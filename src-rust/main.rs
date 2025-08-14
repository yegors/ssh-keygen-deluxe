use clap::{Arg, Command};
use ed25519_dalek::SigningKey;
use memchr::memmem;
use rand::rngs::OsRng;
use ssh_key::{PrivateKey, private::Ed25519Keypair, private::Ed25519PrivateKey, public::Ed25519PublicKey};
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::thread;
use std::fs;

/// Statistics for tracking key generation progress
#[derive(Debug)]
struct Stats {
    attempts: AtomicU64,
    start_time: Instant,
}

impl Stats {
    fn new() -> Self {
        Self {
            attempts: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    fn add(&self, count: u64) {
        self.attempts.fetch_add(count, Ordering::Relaxed);
    }

    fn get_attempts(&self) -> u64 {
        self.attempts.load(Ordering::Relaxed)
    }

    fn get_rate(&self) -> f64 {
        let attempts = self.get_attempts();
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            attempts as f64 / elapsed
        } else {
            0.0
        }
    }

    fn get_elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}

/// Result of a successful key generation
#[derive(Debug)]
struct KeyResult {
    private_key: SigningKey,
    ssh_pub_key: String,
    attempts: u64,
}

/// Configuration for the key generation process
#[derive(Debug, Clone)]
struct Config {
    target: String,
    case_sensitive: bool,
    num_threads: usize,
    private_key_file: String,
    public_key_file: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            target: String::new(),
            case_sensitive: true,
            num_threads: num_cpus::get() * 3,
            private_key_file: "id_ed25519".to_string(),
            public_key_file: "id_ed25519.pub".to_string(),
        }
    }
}

/// Generate a single Ed25519 keypair and check if it matches the target
fn generate_and_check_key(target: &[u8], case_sensitive: bool) -> Option<KeyResult> {
    // Generate Ed25519 keypair directly for maximum performance
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    
    // Convert to SSH format - this is the expensive operation
    let ed25519_keypair = Ed25519Keypair {
        public: Ed25519PublicKey(verifying_key.to_bytes()),
        private: Ed25519PrivateKey::from_bytes(&signing_key.to_bytes()),
    };
    
    let ssh_private = PrivateKey::new(
        ed25519_keypair.into(),
        "".to_string(),
    ).ok()?;
    
    let ssh_public = ssh_private.public_key();
    let public_key_string = ssh_public.to_openssh().ok()?;
    let public_key_bytes = public_key_string.as_bytes();
    
    // Check if the public key contains the target string using optimized search
    let matches = if case_sensitive {
        memmem::find(public_key_bytes, target).is_some()
    } else {
        contains_bytes_ignore_case(public_key_bytes, target)
    };
    
    if matches {
        Some(KeyResult {
            private_key: signing_key,
            ssh_pub_key: public_key_string,
            attempts: 0, // Will be set by caller
        })
    } else {
        None
    }
}

/// Fast case-insensitive byte slice contains check using SIMD optimizations
fn contains_bytes_ignore_case(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if needle.len() > haystack.len() {
        return false;
    }

    // For single byte searches, use memchr's optimized search
    if needle.len() == 1 {
        let target_byte = needle[0];
        let upper_byte = if target_byte >= b'a' && target_byte <= b'z' {
            target_byte - (b'a' - b'A')
        } else {
            target_byte
        };
        
        // Search for both lowercase and uppercase variants using memchr2
        if target_byte != upper_byte {
            return memchr::memchr2(target_byte, upper_byte, haystack).is_some();
        } else {
            return memchr::memchr(target_byte, haystack).is_some();
        }
    }

    // For multi-byte searches, use memchr to find potential starting positions
    // of the first character, then verify the rest manually
    let first_needle_byte = needle[0];
    let first_upper = if first_needle_byte >= b'a' && first_needle_byte <= b'z' {
        first_needle_byte - (b'a' - b'A')
    } else {
        first_needle_byte
    };
    
    let mut start = 0;
    while start <= haystack.len().saturating_sub(needle.len()) {
        // Find next occurrence of first character (case-insensitive)
        let pos = if first_needle_byte != first_upper {
            memchr::memchr2(first_needle_byte, first_upper, &haystack[start..])
        } else {
            memchr::memchr(first_needle_byte, &haystack[start..])
        };
        
        match pos {
            Some(offset) => {
                let actual_pos = start + offset;
                
                // Ensure we don't go out of bounds
                if actual_pos + needle.len() > haystack.len() {
                    break;
                }
                
                // Check if the rest of the bytes match (case-insensitive)
                let mut found = true;
                for j in 1..needle.len() {
                    let haystack_char = to_lowercase(haystack[actual_pos + j]);
                    let needle_char = needle[j]; // Already converted to lowercase
                    if haystack_char != needle_char {
                        found = false;
                        break;
                    }
                }
                
                if found {
                    return true;
                }
                
                start = actual_pos + 1;
            }
            None => break,
        }
    }
    
    false
}

/// Fast ASCII lowercase conversion (similar to Go implementation)
fn to_lowercase(b: u8) -> u8 {
    if b >= b'A' && b <= b'Z' {
        b + (b'a' - b'A')
    } else {
        b
    }
}

/// Worker function that continuously generates keys until a match is found
fn worker(
    config: Arc<Config>,
    stats: Arc<Stats>,
    found: Arc<AtomicBool>,
) -> Option<KeyResult> {
    let batch_size = 1000u64; // Match Go implementation batch size
    let mut attempts = 0u64;
    
    // Prepare target bytes for efficient search
    let target_bytes = if config.case_sensitive {
        config.target.as_bytes().to_vec()
    } else {
        config.target.to_lowercase().as_bytes().to_vec()
    };

    while !found.load(Ordering::Relaxed) {
        // Process a batch without checking found flag for maximum performance
        for _ in 0..batch_size {
            attempts += 1;
            
            if let Some(mut key_result) =
                generate_and_check_key(&target_bytes, config.case_sensitive) {
                // Found a match!
                let total_attempts = stats.get_attempts() + attempts;
                key_result.attempts = total_attempts;
                
                // Signal other workers to stop
                found.store(true, Ordering::Relaxed);
                return Some(key_result);
            }
            
            // Early exit check within batch for responsiveness
            if attempts % 100 == 0 && found.load(Ordering::Relaxed) {
                return None;
            }
        }
        
        // Update global counter after processing the batch
        stats.add(batch_size);
        attempts = 0;
    }
    None
}

/// Display progress statistics
fn display_progress(stats: Arc<Stats>, found: Arc<AtomicBool>, ci_mode: bool) {
    let mut last_attempts = 0u64;
    let mut last_time = Instant::now();
    
    while !found.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_secs(1));
        
        let current_time = Instant::now();
        let current = stats.get_attempts();
        let time_diff = current_time.duration_since(last_time).as_secs_f64();
        
        // Calculate current rate (attempts in the last second)
        let rate = if time_diff > 0.0 {
            ((current.saturating_sub(last_attempts)) as f64 / time_diff) as u64
        } else {
            0
        };
        
        let elapsed = stats.get_elapsed();
        let avg_rate = stats.get_rate();
        
        // Format elapsed time as MMmSSs like the Go version
        let elapsed_secs = elapsed.as_secs();
        let minutes = elapsed_secs / 60;
        let seconds = elapsed_secs % 60;
        let elapsed_str = format!("{}m{:02}s", minutes, seconds);
        
        if ci_mode {
            // For CI mode, print each update on a new line
            println!("Attempts: {} | Rate: {}/s | Avg: {:.0}/s | Elapsed: {}",
                     current, rate, avg_rate, elapsed_str);
        } else {
            // For interactive mode, overwrite the line
            print!("\rAttempts: {} | Rate: {}/s | Avg: {:.0}/s | Elapsed: {}",
                   current, rate, avg_rate, elapsed_str);
            use std::io::{self, Write};
            io::stdout().flush().unwrap();
        }
        
        last_attempts = current;
        last_time = current_time;
    }
}

/// Save the generated keys to files
fn save_keys(
    private_key: &SigningKey,
    public_key_string: &str,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create SSH private key
    let ed25519_keypair = Ed25519Keypair {
        public: Ed25519PublicKey(private_key.verifying_key().to_bytes()),
        private: Ed25519PrivateKey::from_bytes(&private_key.to_bytes()),
    };
    
    let ssh_private = PrivateKey::new(
        ed25519_keypair.into(),
        "".to_string(),
    )?;
    
    // Save private key in OpenSSH format
    let private_key_pem = ssh_private.to_openssh(ssh_key::LineEnding::LF)?;
    fs::write(&config.private_key_file, private_key_pem.as_bytes())?;
    
    // Save public key
    fs::write(&config.public_key_file, public_key_string.as_bytes())?;
    
    // Set appropriate permissions for private key (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&config.private_key_file)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&config.private_key_file, perms)?;
    }
    
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments (simplified version matching Go implementation)
    let matches = Command::new("ssh-keygen")
        .version("0.1.0")
        .about("Generate SSH Ed25519 keys with specific patterns")
        .arg(
            Arg::new("case-insensitive")
                .long("ci")
                .help("CI mode - reduced output for automated environments")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("target")
                .help("Target string to search for in public key")
                .required(true)
                .index(1),
        )
        .get_matches();

    // Build configuration
    let mut config = Config::default();
    config.target = matches.get_one::<String>("target").unwrap().clone();
    let ci_mode = matches.get_flag("case-insensitive");
    config.case_sensitive = true; // Always case-sensitive by default, --ci is for output mode

    if config.target.is_empty() {
        eprintln!("Error: target sequence cannot be empty");
        std::process::exit(1);
    }

    println!(
        "Searching for ed25519 key containing: {} (case-sensitive)",
        config.target
    );
    println!(
        "Using {} cores, {} workers",
        num_cpus::get(),
        config.num_threads
    );

    // Initialize shared state
    let config = Arc::new(config);
    let stats = Arc::new(Stats::new());
    let found = Arc::new(AtomicBool::new(false));

    // Set up signal handling for graceful shutdown
    let found_signal = found.clone();
    ctrlc::set_handler(move || {
        found_signal.store(true, Ordering::Relaxed);
    }).expect("Error setting Ctrl-C handler");

    // Start progress display thread
    let stats_clone = stats.clone();
    let found_clone = found.clone();
    let progress_handle = thread::spawn(move || {
        display_progress(stats_clone, found_clone, ci_mode);
    });

    // Start parallel key generation using rayon
    use rayon::prelude::*;
    
    let result = (0..config.num_threads)
        .into_par_iter()
        .map(|_| {
            worker(config.clone(), stats.clone(), found.clone())
        })
        .find_any(|result| result.is_some())
        .flatten();
    
    // Signal completion and wait for progress thread
    found.store(true, Ordering::Relaxed);
    progress_handle.join().unwrap();
    
    match result {
        Some(key_result) => {
            if !ci_mode {
                println!(); // Add newline after progress display
            }
            println!("\nMatch found after {} attempts!", key_result.attempts);
            
            // Save the generated keys
            if let Err(e) = save_keys(&key_result.private_key, &key_result.ssh_pub_key, &config) {
                eprintln!("Error saving keys: {}", e);
                std::process::exit(1);
            }
            
            println!("Keys written to {} and {}", config.private_key_file, config.public_key_file);
            println!("Public key: {}", key_result.ssh_pub_key.trim());
            
            let final_attempts = stats.get_attempts();
            println!("Total attempts across all workers: {}", final_attempts);
        }
        None => {
            if !ci_mode {
                println!(); // Add newline after progress display
            }
            println!("\nSearch interrupted by user");
            std::process::exit(1);
        }
    }
    Ok(())
}