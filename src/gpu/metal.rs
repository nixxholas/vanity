use metal::{Device, MTLResourceOptions, CommandQueue, ComputePipelineState, MTLSize};
use std::sync::atomic::{AtomicI32, Ordering, AtomicBool};
use std::sync::Arc;
use lazy_static::lazy_static;

// Create a global static for the Ctrl-C handler state
lazy_static! {
    static ref INTERRUPT_HANDLER_INSTALLED: AtomicBool = AtomicBool::new(false);
    static ref GPU_RUNNING: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
}

pub struct MetalContext {
    device: Device,
    command_queue: CommandQueue,
    pipeline_state: ComputePipelineState,
    max_threads_per_threadgroup: u64,
    num_threadgroups: u64,
    threads_per_threadgroup: u64,
    total_threads: u64,
}

impl MetalContext {
    pub fn new(case_insensitive: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let device = Device::system_default()
            .ok_or("No Metal device found")?;
        
        // Get device capabilities
        let max_threads_per_threadgroup = device.max_threads_per_threadgroup().width as u64;
        let _max_total_threads_per_threadgroup = device.max_threads_per_threadgroup().depth as u64;
        
        // Optimize for Apple Silicon GPUs (M1/M2/M3)
        // Use 256 threads per threadgroup for optimal occupancy
        let threads_per_threadgroup = 256u64.min(max_threads_per_threadgroup);
        
        // Calculate optimal number of threadgroups based on GPU cores
        // Apple Silicon GPUs have high thread occupancy
        let gpu_family = device.supports_family(metal::MTLGPUFamily::Apple7); // M1 and newer
        let num_threadgroups = if gpu_family {
            // For Apple Silicon, use more threadgroups for better occupancy
            // M1: 8 cores, M1 Pro: 16 cores, M1 Max: 32 cores, M2 similar scaling
            let estimated_cores = device.recommended_max_working_set_size() / (1024 * 1024 * 64);
            (estimated_cores * 32).max(256).min(4096) // 32 threadgroups per core
        } else {
            // Fallback for older Intel Macs
            device.recommended_max_working_set_size()
                .max(512 * 1024 * 1024) / (256 * 1024) as u64
        };
        
        let total_threads = threads_per_threadgroup * num_threadgroups;
        
        let command_queue = device.new_command_queue();
        
        let library_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/metallib/vanity.metallib");
            
        let library = device.new_library_with_file(library_path)?;
        
        // Create function constants
        let constants = metal::FunctionConstantValues::new();
        constants.set_constant_value_at_index(
            &case_insensitive as *const bool as *const _,
            metal::MTLDataType::Bool,
            0
        );
        
        // Get specialized kernel function with constants
        let kernel = library.get_function("vanity_search", Some(constants))?;
        
        let pipeline_state = device
            .new_compute_pipeline_state_with_function(&kernel)?;
            
        Ok(Self {
            device,
            command_queue,
            pipeline_state,
            max_threads_per_threadgroup,
            num_threadgroups,
            threads_per_threadgroup,
            total_threads,
        })
    }
    
    pub fn device_info(&self) -> String {
        let gpu_family = if self.device.supports_family(metal::MTLGPUFamily::Apple7) {
            "Apple Silicon GPU (M1/M2/M3 family)"
        } else if self.device.supports_family(metal::MTLGPUFamily::Mac2) {
            "AMD GPU"
        } else {
            "Intel GPU"
        };
        
        format!(
            "GPU: {}\n  Family: {}\n  Max threads/threadgroup: {}\n  Threadgroups: {}\n  Threads/threadgroup: {}\n  Total threads: {}",
            self.device.name(),
            gpu_family,
            self.max_threads_per_threadgroup,
            self.num_threadgroups,
            self.threads_per_threadgroup,
            self.total_threads
        )
    }
}

pub fn vanity_round(
    _id: i32,
    seed: &[u8],
    base: &[u8],
    owner: &[u8],
    target: &str,
    case_insensitive: bool,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    vanity_round_with_timeout(_id, seed, base, owner, target, case_insensitive, 60)
}

pub fn vanity_round_with_timeout(
    _id: i32,
    seed: &[u8],
    base: &[u8],
    owner: &[u8],
    target: &str,
    case_insensitive: bool,
    timeout_secs: u64,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Validate inputs
    if seed.len() != 32 {
        return Err("Seed must be 32 bytes".into());
    }
    if base.len() != 32 {
        return Err("Base public key must be 32 bytes".into());
    }
    if owner.len() != 32 {
        return Err("Owner public key must be 32 bytes".into());
    }
    if target.is_empty() || target.len() > 44 {
        return Err("Target must be between 1 and 44 characters".into());
    }
    // Install global Ctrl-C handler only once
    if !INTERRUPT_HANDLER_INSTALLED.load(Ordering::SeqCst) {
        if INTERRUPT_HANDLER_INSTALLED.compare_exchange(
            false,
            true,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ).is_ok() {
            let gpu_running = GPU_RUNNING.clone();
            ctrlc::set_handler(move || {
                gpu_running.store(false, Ordering::SeqCst);
            })?;
        }
    }

    // Check if we should continue
    if !GPU_RUNNING.load(Ordering::SeqCst) {
        return Err("GPU computation interrupted".into());
    }

    let ctx = MetalContext::new(case_insensitive)?;
    
    // Only show GPU info for longer running tasks (timeout > 5 seconds)
    if timeout_secs > 5 {
        println!("\n{}", ctx.device_info());
    }
    
    let seed_buffer = ctx.device.new_buffer_with_data(
        seed.as_ptr() as *const _,
        seed.len() as u64,
        MTLResourceOptions::StorageModeShared,
    );
    
    let base_buffer = ctx.device.new_buffer_with_data(
        base.as_ptr() as *const _,
        base.len() as u64,
        MTLResourceOptions::StorageModeShared,
    );
    
    let owner_buffer = ctx.device.new_buffer_with_data(
        owner.as_ptr() as *const _,
        owner.len() as u64,
        MTLResourceOptions::StorageModeShared,
    );
    
    let target_buffer = ctx.device.new_buffer_with_data(
        target.as_ptr() as *const _,
        target.len() as u64,
        MTLResourceOptions::StorageModeShared,
    );
    
    let target_len = target.len() as u64;
    let target_len_buffer = ctx.device.new_buffer_with_data(
        &target_len as *const u64 as *const _,
        std::mem::size_of::<u64>() as u64,
        MTLResourceOptions::StorageModeShared,
    );
    
    // Allocate output buffer for result
    let out_buffer = ctx.device.new_buffer(
        16, // 16 bytes for the found seed
        MTLResourceOptions::StorageModeShared,
    );
    
    let command_buffer = ctx.command_queue.new_command_buffer();
    let command_buffer_label = format!("VanitySearch_GPU{}", _id);
    command_buffer.set_label(&command_buffer_label);
    
    let compute_encoder = command_buffer.new_compute_command_encoder();
    
    compute_encoder.set_compute_pipeline_state(&ctx.pipeline_state);
    compute_encoder.set_buffer(0, Some(&seed_buffer), 0);
    compute_encoder.set_buffer(1, Some(&base_buffer), 0);
    compute_encoder.set_buffer(2, Some(&owner_buffer), 0);
    compute_encoder.set_buffer(3, Some(&target_buffer), 0);
    compute_encoder.set_buffer(4, Some(&target_len_buffer), 0);
    compute_encoder.set_buffer(5, Some(&out_buffer), 0);
    
    let done = ctx.device.new_buffer(
        std::mem::size_of::<i32>() as u64,
        MTLResourceOptions::StorageModeShared,
    );
    
    let count = ctx.device.new_buffer(
        std::mem::size_of::<u32>() as u64,
        MTLResourceOptions::StorageModeShared,
    );
    
    let done_buffer = Arc::new(done);
    let _done_buffer_clone = done_buffer.clone();
    
    let done_atomic = Arc::new(AtomicI32::new(0));
    let _done_atomic_clone = done_atomic.clone();

    unsafe {
        *(done_buffer.contents() as *mut i32) = 0;
        *(count.contents() as *mut u32) = 0;
    }
    
    compute_encoder.set_buffer(6, Some(&done_buffer), 0);
    compute_encoder.set_buffer(7, Some(&count), 0);
    
    // Use optimized threadgroup configuration
    let threadgroup_size = MTLSize::new(ctx.threads_per_threadgroup, 1, 1);
    let grid_size = MTLSize::new(ctx.total_threads, 1, 1);
    
    compute_encoder.dispatch_threads(grid_size, threadgroup_size);
    compute_encoder.end_encoding();

    command_buffer.commit();

    let start_time = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(timeout_secs);
    let check_interval = std::time::Duration::from_millis(10);

    loop {
        let status = command_buffer.status();
        
        match status {
            metal::MTLCommandBufferStatus::Completed => break,
            metal::MTLCommandBufferStatus::Error => {
                return Err("Metal command buffer execution failed".into());
            }
            _ => {
                if start_time.elapsed() > timeout || !GPU_RUNNING.load(Ordering::SeqCst) {
                    unsafe {
                        *(done_buffer.contents() as *mut i32) = 1;
                    }
                    return Err("GPU computation interrupted or timed out".into());
                }
                std::thread::sleep(check_interval);
            }
        }
    }

    // Extract the result and iteration count
    let count_val = unsafe { *(count.contents() as *const u32) };
    let done_val = unsafe { *(done_buffer.contents() as *const i32) };
    
    if done_val != 1 {
        // No match found in this round - this is expected for benchmark
        if timeout_secs <= 2 {
            // This is likely a benchmark run, return the iteration count
            let mut result = vec![0u8; 24];
            let count_bytes = (count_val as u64).to_le_bytes();
            unsafe {
                std::ptr::copy_nonoverlapping(count_bytes.as_ptr(), result.as_mut_ptr().add(16), 8);
            }
            return Ok(result);
        }
        return Err(format!("No match found after {} iterations", count_val).into());
    }
    
    let out_ptr = out_buffer.contents() as *const u8;
    let mut result = vec![0u8; 24];
    unsafe {
        // Copy the 16-byte seed
        std::ptr::copy_nonoverlapping(out_ptr, result.as_mut_ptr(), 16);
        // Add the iteration count as the last 8 bytes
        let count_bytes = (count_val as u64).to_le_bytes();
        std::ptr::copy_nonoverlapping(count_bytes.as_ptr(), result.as_mut_ptr().add(16), 8);
    }
    
    // Only log for non-benchmark runs (longer timeouts)
    if timeout_secs > 5 {
        println!("GPU {} found match after {} iterations", _id, count_val);
    }
    
    Ok(result)
} 