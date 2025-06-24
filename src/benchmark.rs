use crate::gpu::GpuVanitySearch;
use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};
use std::time::Duration;

const BENCHMARK_DURATION: Duration = Duration::from_secs(5);
const DUMMY_BASE: [u8; 32] = [0; 32];
const DUMMY_OWNER: [u8; 32] = [0; 32];
const DUMMY_TARGET: &str = "1"; // Very easy target for benchmarking throughput

pub fn run_benchmark(num_cpus: u32, num_gpus: u32) {
    println!("🚀 Performance Benchmark ({} seconds)\n", BENCHMARK_DURATION.as_secs());

    // Test individual components
    let mut cpu_rate = 0.0;
    let mut gpu_rate = 0.0;

    if num_cpus > 0 {
        cpu_rate = benchmark_cpu(num_cpus);
    }

    #[cfg(feature = "apple-gpu")]
    if num_gpus > 0 {
        if num_cpus > 0 {
            println!(); // Add spacing between CPU and GPU
        }
        gpu_rate = benchmark_gpu(num_gpus);
    }
    
    // Test hybrid CPU+GPU if both are available
    #[cfg(feature = "apple-gpu")]
    if num_cpus > 0 && num_gpus > 0 {
        println!();
        benchmark_hybrid(num_cpus, num_gpus);
    }
    
    // Summary
    println!("\n🎯 Summary:");
    if cpu_rate > 0.0 {
        println!("  CPU Only: {:.1} MH/s", cpu_rate);
    }
    if gpu_rate > 0.0 {
        println!("  GPU Only: {:.1} MH/s", gpu_rate);
    }
    if cpu_rate > 0.0 && gpu_rate > 0.0 {
        println!("  Theoretical Max: {:.1} MH/s (CPU + GPU)", cpu_rate + gpu_rate);
    }
    
    println!(); // Final spacing
}

fn benchmark_cpu(num_threads: u32) -> f64 {
    println!("🖥️  CPU Benchmark:");
    
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.blue} CPU: Testing {msg} threads...")
        .unwrap());
    pb.set_message(format!("{}", num_threads));
    
    let start = std::time::Instant::now();
    let total_hashes = std::sync::atomic::AtomicU64::new(0);

    rayon::scope(|s| {
        for _ in 0..num_threads {
            s.spawn(|_| {
                let mut local_hashes = 0u64;
                while start.elapsed() < BENCHMARK_DURATION {
                    let seed = rand::random::<[u8; 32]>();
                    let _ = Sha256::digest(&seed);
                    local_hashes += 1;
                    
                    // Update progress periodically
                    if local_hashes % 50000 == 0 {
                        let current_rate = total_hashes.load(std::sync::atomic::Ordering::Relaxed) as f64 
                            / start.elapsed().as_secs_f64() / 1_000_000.0;
                        pb.set_message(format!("{} threads ({:.1} MH/s)", num_threads, current_rate));
                    }
                }
                total_hashes.fetch_add(local_hashes, std::sync::atomic::Ordering::Relaxed);
            });
        }
    });

    let elapsed = start.elapsed().as_secs_f64();
    let hashrate = total_hashes.load(std::sync::atomic::Ordering::Relaxed) as f64 / elapsed;
    
    pb.finish_with_message(format!(
        "CPU: {:.1} MH/s", 
        hashrate / 1_000_000.0
    ));
    
    hashrate / 1_000_000.0
}

#[cfg(feature = "apple-gpu")]
fn benchmark_gpu(num_gpus: u32) -> f64 {
    println!("🔥 GPU Benchmark:");
    
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} GPU: Running parallel benchmark...")
        .unwrap());

    use std::sync::{Arc, Mutex};
    use std::thread;
    
    let total_iterations = Arc::new(Mutex::new(0u64));
    let start_time = std::time::Instant::now();
    
    // On Apple Silicon, num_gpus represents parallel GPU contexts, not physical devices
    // Limit to reasonable values to avoid resource contention
    let num_workers = if num_gpus <= 8 {
        num_gpus // Use the requested number directly for reasonable values
    } else {
        println!("⚠️  Warning: Apple Silicon has 1 integrated GPU. Using 8 workers instead of {}.", num_gpus);
        8 // Cap at 8 workers to avoid excessive contention
    };
    let mut handles = vec![];
    
    for worker_id in 0..num_workers {
        let total_iterations_clone = total_iterations.clone();
        let pb_clone = pb.clone();
        
        let handle = thread::spawn(move || {
            let gpu = GpuVanitySearch::new();
            let mut local_iterations = 0u64;
            let mut rounds = 0u32;
            
            while start_time.elapsed() < BENCHMARK_DURATION {
                let seed = rand::random::<[u8; 32]>();
                
                match gpu.vanity_round_with_timeout(
                    worker_id as i32,
                    &seed,
                    &DUMMY_BASE,
                    &DUMMY_OWNER,
                    DUMMY_TARGET,
                    false,
                    1, // 1 second timeout for benchmark rounds
                ) {
                    Ok(result) => {
                        // Extract iteration count from the result
                        if result.len() >= 24 {
                            let count_bytes = &result[16..24];
                            let count = u64::from_le_bytes(count_bytes.try_into().unwrap_or_default());
                            local_iterations += count;
                        }
                        rounds += 1;
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        if error_msg.contains("No match found") {
                            // Extract iteration count from error message
                            if let Some(count_str) = error_msg.split("after ").nth(1) {
                                if let Some(count_str) = count_str.split(" iterations").next() {
                                    if let Ok(count) = count_str.parse::<u64>() {
                                        local_iterations += count;
                                        rounds += 1;
                                    }
                                }
                            }
                        } else if error_msg.contains("interrupted") || error_msg.contains("timed out") {
                            // Expected timeout for benchmark
                            break;
                        } else {
                            // GPU error, stop this worker
                            break;
                        }
                    }
                }
                
                // Update progress every 3 rounds
                if rounds % 3 == 0 {
                    let mut total = total_iterations_clone.lock().unwrap();
                    *total += local_iterations;
                    let current_rate = *total as f64 / start_time.elapsed().as_secs_f64() / 1_000_000.0;
                    pb_clone.set_message(format!("GPU: {:.1} MH/s ({} workers)", current_rate, num_workers));
                    local_iterations = 0; // Reset to avoid double counting
                }
            }
            
            // Add remaining iterations
            let mut total = total_iterations_clone.lock().unwrap();
            *total += local_iterations;
        });
        
        handles.push(handle);
    }
    
    // Wait for all workers to complete
    for handle in handles {
        let _ = handle.join();
    }
    
    let elapsed = start_time.elapsed().as_secs_f64();
    let total_its = *total_iterations.lock().unwrap();
    let hashrate = total_its as f64 / elapsed;
    
    pb.finish_with_message(format!(
        "GPU: {:.1} MH/s ({} parallel workers)", 
        hashrate / 1_000_000.0,
        num_workers
    ));
    
    println!("\n📊 Results:");
    println!("  GPU Total: {:.1} MH/s", hashrate / 1_000_000.0);
    println!("  Parallel Workers: {} GPU contexts on 1 Apple Silicon GPU", num_workers);
    println!("  Per-Worker: {:.0} iterations/sec", total_its as f64 / num_workers as f64 / elapsed);
    
    if num_workers > 6 {
        println!("  💡 Tip: 4-6 workers usually perform better on Apple Silicon");
    }
    
    hashrate / 1_000_000.0
}

#[cfg(feature = "apple-gpu")]
fn benchmark_hybrid(num_cpus: u32, num_gpus: u32) {
    println!("🚀 Hybrid CPU+GPU Benchmark:");
    
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.yellow} Hybrid: {msg}")
        .unwrap());
    pb.set_message("Running CPU+GPU simultaneously...");

    use std::sync::{Arc, Mutex};
    use std::thread;
    
    let total_iterations = Arc::new(Mutex::new(0u64));
    let start_time = std::time::Instant::now();
    let mut handles = vec![];
    
    // CPU workers
    for _ in 0..num_cpus {
        let total_iterations_clone = total_iterations.clone();
        
        let handle = thread::spawn(move || {
            let mut local_iterations = 0u64;
            
            while start_time.elapsed() < BENCHMARK_DURATION {
                let seed = rand::random::<[u8; 32]>();
                let _ = sha2::Sha256::digest(&seed);
                local_iterations += 1;
                
                // Update shared counter periodically
                if local_iterations % 10000 == 0 {
                    let mut total = total_iterations_clone.lock().unwrap();
                    *total += 10000;
                }
            }
            
            // Add remaining iterations
            let mut total = total_iterations_clone.lock().unwrap();
            *total += local_iterations % 10000;
        });
        
        handles.push(handle);
    }
    
    // GPU workers  
    let optimal_gpu_workers = if num_gpus <= 6 { num_gpus } else { 6 };
    
    for worker_id in 0..optimal_gpu_workers {
        let total_iterations_clone = total_iterations.clone();
        
        let handle = thread::spawn(move || {
            let gpu = GpuVanitySearch::new();
            let mut local_iterations = 0u64;
            
            while start_time.elapsed() < BENCHMARK_DURATION {
                let seed = rand::random::<[u8; 32]>();
                
                match gpu.vanity_round_with_timeout(
                    worker_id as i32,
                    &seed,
                    &DUMMY_BASE,
                    &DUMMY_OWNER,
                    DUMMY_TARGET,
                    false,
                    1, // 1 second timeout
                ) {
                    Ok(result) => {
                        if result.len() >= 24 {
                            let count_bytes = &result[16..24];
                            let count = u64::from_le_bytes(count_bytes.try_into().unwrap_or_default());
                            local_iterations += count;
                        }
                    }
                    Err(_) => {
                        // Continue on timeout/error
                    }
                }
                
                // Update shared counter periodically
                if local_iterations > 1000000 {
                    let mut total = total_iterations_clone.lock().unwrap();
                    *total += local_iterations;
                    local_iterations = 0;
                }
            }
            
            // Add remaining iterations
            let mut total = total_iterations_clone.lock().unwrap();
            *total += local_iterations;
        });
        
        handles.push(handle);
    }
    
    // Update progress
    let progress_handle = {
        let total_iterations_clone = total_iterations.clone();
        let pb_clone = pb.clone();
        
        thread::spawn(move || {
            while start_time.elapsed() < BENCHMARK_DURATION {
                thread::sleep(Duration::from_millis(500));
                let total = *total_iterations_clone.lock().unwrap();
                let current_rate = total as f64 / start_time.elapsed().as_secs_f64() / 1_000_000.0;
                pb_clone.set_message(format!("CPU+GPU: {:.1} MH/s", current_rate));
            }
        })
    };
    
    // Wait for all workers
    for handle in handles {
        let _ = handle.join();
    }
    let _ = progress_handle.join();
    
    let elapsed = start_time.elapsed().as_secs_f64();
    let total_its = *total_iterations.lock().unwrap();
    let hashrate = total_its as f64 / elapsed / 1_000_000.0;
    
    pb.finish_with_message(format!(
        "Hybrid: {:.1} MH/s", 
        hashrate
    ));
    
    println!("\n📊 Hybrid Results:");
    println!("  Combined Rate: {:.1} MH/s", hashrate);
    println!("  CPU Workers: {}", num_cpus);
    println!("  GPU Workers: {}", optimal_gpu_workers);
    println!("  Total Workers: {}", num_cpus + optimal_gpu_workers);
} 