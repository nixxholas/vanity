mod benchmark;
mod gpu;

use clap::Parser;
use logfather::{Level, Logger};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use sha2::{Digest, Sha256};
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "apple-gpu")]
use crate::gpu::GpuVanitySearch;

#[derive(Parser)]
#[command(name = "vanity")]
#[command(about = "A high-performance Solana vanity address generator optimized for Apple Silicon")]
#[command(long_about = "Generate Solana Program Derived Addresses (PDAs) with custom prefixes using GPU acceleration.

UNDERSTANDING THE PARAMETERS:
• BASE: The base public key used in PDA derivation (usually a program ID)
• OWNER: The owner public key that will control the generated PDA  
• TARGET: The desired prefix for the vanity address (case-sensitive by default)

EXAMPLE USAGE:
  vanity --base 11111111111111111111111111111112 \\
         --owner So11111111111111111111111111111111111111112 \\
         --target Sol

PERFORMANCE TIPS:
• Use --case-insensitive for faster results when case doesn't matter
• Shorter prefixes are exponentially faster to find
• Apple Silicon GPUs provide significant speedup over CPU-only search")]
pub struct Args {
    /// Base public key (32-byte Solana public key in base58 format)
    /// Example: 11111111111111111111111111111112
    /// This is typically your program's public key or a known base address
    #[clap(long, value_parser = parse_pubkey)]
    #[clap(required_unless_present = "benchmark")]
    pub base: Option<[u8; 32]>,

    /// Owner public key (32-byte Solana public key in base58 format)  
    /// Example: So11111111111111111111111111111111111111112
    /// This is the address that will own the generated Program Derived Address (PDA)
    #[clap(long, value_parser = parse_pubkey)]
    #[clap(required_unless_present = "benchmark")]
    pub owner: Option<[u8; 32]>,

    /// Target prefix to search for in the generated vanity address
    /// Examples: 'Sol' (3 chars), 'Lucky' (5 chars), '123' (3 chars), 'ABCD' (4 chars)
    /// Valid characters: 123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz
    /// Note: Each additional character increases search time exponentially (~58x harder)
    /// Difficulty estimates: 3 chars = ~200k attempts, 4 chars = ~11M attempts, 5 chars = ~660M attempts
    #[clap(long)]
    #[clap(required_unless_present = "benchmark")]
    pub target: Option<String>,

    /// Enable case-insensitive matching (faster search, matches more addresses)
    /// Example: 'Sol' will match 'Sol', 'SOL', 'sol', 'SoL', etc.
    #[clap(long, default_value_t = false)]
    pub case_insensitive: bool,

    /// Optional log file path to save search progress and results
    /// Example: --logfile vanity_search.log
    #[clap(long)]
    pub logfile: Option<String>,

    /// Number of GPU workers to use for address generation
    /// On Apple Silicon: Controls parallel GPU contexts (1-8 recommended)  
    /// On discrete GPUs: Number of physical GPU devices
    /// Default: 1 (use 0 to disable GPU acceleration)
    #[clap(long, default_value_t = 1)]
    #[cfg(any(feature = "cuda-gpu", feature = "apple-gpu"))]
    pub num_gpus: u32,

    /// Number of CPU threads to use for address generation
    /// Default: 0 (auto-detect), set to 0 to use all available cores
    #[clap(long, default_value_t = 0)]
    pub num_cpus: u32,

    /// Run performance benchmark instead of searching for addresses
    /// Tests GPU and CPU performance without requiring --base, --owner, --target
    #[clap(long)]
    pub benchmark: bool,
}

static EXIT: AtomicBool = AtomicBool::new(false);

fn main() {
    let args = Args::parse();
    
    if args.benchmark {
        #[cfg(any(feature = "cuda-gpu", feature = "apple-gpu"))]
        let num_gpus = args.num_gpus;
        #[cfg(not(any(feature = "cuda-gpu", feature = "apple-gpu")))]
        let num_gpus = 0;
        
        benchmark::run_benchmark(args.num_cpus, num_gpus);
        return;
    }

    // Initialize logger with optional logfile
    let mut logger = Logger::new();
    if let Some(ref logfile) = args.logfile {
        logger.file(true);
        logger.path(logfile);
    }

    // Slightly more compact log format
    logger.log_format("[{timestamp} {level}] {message}");
    logger.timestamp_format("%Y-%m-%d %H:%M:%S");
    logger.level(Level::Info);

    // Print resource usage
    logfather::info!("using {} threads", args.num_cpus);
    #[cfg(any(feature = "cuda-gpu", feature = "apple-gpu"))]
    logfather::info!("using {} gpus", args.num_gpus);

    // Hybrid CPU+GPU processing - run simultaneously for maximum performance
    let mut handles = vec![];
    
    let base = args.base.expect("Base is required for processing");
    let owner = args.owner.expect("Owner is required for processing");
    let target = get_validated_target(&args);
    
    // GPU processing
    #[cfg(feature = "apple-gpu")]
    if args.num_gpus > 0 {
        logfather::info!("Starting {} GPU workers", args.num_gpus);
        
        for gpu_index in 0..args.num_gpus {
            let base = base;
            let owner = owner;
            let target = target.clone();
            
            let handle = std::thread::spawn(move || {
                let mut iteration = 0u64;
                let gpu = GpuVanitySearch::new();
                
                // Only show progress for first GPU worker to avoid spam
                let pb = if gpu_index == 0 {
                    let pb = ProgressBar::new_spinner();
                    pb.set_style(ProgressStyle::default_spinner()
                        .template("{spinner:.green} [{elapsed_precise}] GPU: {msg}")
                        .unwrap());
                    pb.set_message("Searching...");
                    Some(pb)
                } else {
                    None
                };
                
                loop {
                    if EXIT.load(Ordering::SeqCst) {
                        if let Some(pb) = &pb {
                            pb.finish_and_clear();
                        }
                        return;
                    }

                    let seed = new_gpu_seed(gpu_index, iteration);
                    
                    match gpu.vanity_round(
                        gpu_index as i32,
                        &seed,
                        &base,
                        &owner,
                        &target,
                        args.case_insensitive,
                    ) {
                        Ok(result) => {
                            // Extract the found seed (first 16 bytes)
                            let found_seed = &result[..16];
                            
                            // Reconstruct the full public key
                            let mut hasher = sha2::Sha256::new();
                            hasher.update(&base);
                            hasher.update(found_seed);
                            hasher.update(&owner);
                            let pubkey = hasher.finalize();
                            
                            let address = bs58::encode(&pubkey).into_string();
                            
                            if let Some(pb) = &pb {
                                pb.finish_with_message(format!("✨ GPU found: {}", address));
                            }
                            
                            logfather::info!("GPU {} found seed: {}", gpu_index, bs58::encode(found_seed).into_string());
                            logfather::info!("Full address: {}", address);
                            
                            EXIT.store(true, Ordering::SeqCst);
                            return;
                        }
                        Err(e) => {
                            if e.to_string().contains("interrupted") || e.to_string().contains("timed out") {
                                EXIT.store(true, Ordering::SeqCst);
                                if let Some(pb) = &pb {
                                    pb.finish_and_clear();
                                }
                                return;
                            }
                            if !e.to_string().contains("No match found") {
                                if let Some(pb) = &pb {
                                    pb.set_message(format!("GPU {} error: {}", gpu_index, e));
                                }
                            }
                        }
                    }
                    
                    iteration += 1;
                    
                    // Update progress occasionally for main GPU worker
                    if gpu_index == 0 && iteration % 10 == 0 {
                        if let Some(pb) = &pb {
                            pb.set_message(format!("Searching... (round {})", iteration));
                        }
                    }
                }
            });
            
            handles.push(handle);
        }
    }

    // CPU processing - runs simultaneously with GPU
    let mut num_cpus = args.num_cpus;
    maybe_update_num_cpus(&mut num_cpus);
    
    if num_cpus > 0 {
        logfather::info!("Starting {} CPU workers", num_cpus);
        
        for cpu_index in 0..num_cpus {
            let base = base;
            let owner = owner;
            let target = target.clone();
            
            let handle = std::thread::spawn(move || {
                let mut iteration = 0u64;
                let base_sha = Sha256::new().chain_update(&base);
                
                loop {
                    if EXIT.load(Ordering::SeqCst) {
                        return;
                    }

                    let seed = new_cpu_seed(cpu_index, iteration);
                    
                    // Calculate the PDA
                    let mut hasher = base_sha.clone();
                    hasher.update(&seed);
                    hasher.update(&owner);
                    let pubkey = hasher.finalize();
                    
                    // Check if it matches target
                    let address = bs58::encode(&pubkey).into_string();
                    let matches = if args.case_insensitive {
                        address.to_lowercase().starts_with(&target.to_lowercase())
                    } else {
                        address.starts_with(&target)
                    };
                    
                    if matches {
                        logfather::info!("CPU {} found seed: {}", cpu_index, bs58::encode(&seed).into_string());
                        logfather::info!("Full address: {}", address);
                        
                        EXIT.store(true, Ordering::SeqCst);
                        return;
                    }
                    
                    iteration += 1;
                }
            });
            
            handles.push(handle);
        }
    }
    
    // Wait for any worker (CPU or GPU) to find a result
    for handle in handles {
        let _ = handle.join();
    }
}

fn get_validated_target(args: &Args) -> String {
    // Static string of BS58 characters
    const BS58_CHARS: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

    let target_str = args.target.as_ref().expect("Target is required for non-benchmark mode");
    
    // Validate target (i.e. does it include 0, O, I, l)
    //
    // maybe TODO: technically we could accept I or o if case-insensitivity but I suspect
    // most users will provide lowercase targets for case-insensitive searches
    for c in target_str.chars() {
        assert!(
            BS58_CHARS.contains(c),
            "your target contains invalid bs58: {}",
            c
        );
    }

    // bs58-aware lowercase converison
    maybe_bs58_aware_lowercase(target_str, args.case_insensitive)
}

fn maybe_bs58_aware_lowercase(target: &str, case_insensitive: bool) -> String {
    // L is only char that shouldn't be converted to lowercase in case-insensitivity case
    const LOWERCASE_EXCEPTIONS: &str = "L";

    if case_insensitive {
        target
            .chars()
            .map(|c| {
                if LOWERCASE_EXCEPTIONS.contains(c) {
                    c
                } else {
                    c.to_ascii_lowercase()
                }
            })
            .collect::<String>()
    } else {
        target.to_string()
    }
}

#[cfg(feature = "cuda-gpu")]
extern "C" {
    pub fn vanity_round(
        gpus: u32,
        seed: *const u8,
        base: *const u8,
        owner: *const u8,
        target: *const u8,
        target_len: u64,
        out: *mut u8,
        case_insensitive: bool,
    );
}

#[cfg(any(feature = "cuda-gpu", feature = "apple-gpu"))]
fn new_gpu_seed(gpu_id: u32, iteration: u64) -> [u8; 32] {
    Sha256::new()
        .chain_update(rand::random::<[u8; 32]>())
        .chain_update(gpu_id.to_le_bytes())
        .chain_update(iteration.to_le_bytes())
        .finalize()
        .into()
}

fn parse_pubkey(input: &str) -> Result<[u8; 32], String> {
    match bs58::decode(input).into_vec() {
        Ok(bytes) => {
            if bytes.len() == 32 {
                let mut array = [0u8; 32];
                array.copy_from_slice(&bytes);
                Ok(array)
            } else {
                Err(format!("Public key must be 32 bytes, got {}", bytes.len()))
            }
        }
        Err(e) => Err(format!("Invalid base58: {}", e))
    }
}

fn maybe_update_num_cpus(num_cpus: &mut u32) {
    if *num_cpus == 0 {
        *num_cpus = rayon::current_num_threads() as u32;
    }
}

fn new_cpu_seed(cpu_id: u32, iteration: u64) -> [u8; 32] {
    Sha256::new()
        .chain_update(rand::random::<[u8; 32]>())
        .chain_update(cpu_id.to_le_bytes())
        .chain_update(iteration.to_le_bytes())
        .finalize()
        .into()
}
