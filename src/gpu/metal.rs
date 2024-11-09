use metal::{Device, MTLResourceOptions, CommandQueue, ComputePipelineState};
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
}

impl MetalContext {
    pub fn new(case_insensitive: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let device = Device::system_default()
            .ok_or("No Metal device found")?;
        
        let max_threads_per_threadgroup = device.max_threads_per_threadgroup().width as u64;
        
        let num_threadgroups = device.recommended_max_working_set_size()
            .max(512 * 1024 * 1024) / (256 * 1024) as u64;
        
        let command_queue = device.new_command_queue();
        
        let library_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/metallib/vanity.metallib");
            
        let library = device.new_library_with_file(library_path)?;
        
        // Create function constants
        let mut constants = metal::FunctionConstantValues::new();
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
        })
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
    
    println!("GPU Info:");
    println!("  Max threads per threadgroup: {}", ctx.max_threads_per_threadgroup);
    println!("  Number of threadgroups: {}", ctx.num_threadgroups);
    println!("  Total threads: {}", ctx.max_threads_per_threadgroup * ctx.num_threadgroups);
    
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
    
    let out_buffer = ctx.device.new_buffer(
        24, // 16 bytes for seed + 8 bytes for count
        MTLResourceOptions::StorageModeShared,
    );
    
    let command_buffer = ctx.command_queue.new_command_buffer();
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
    let done_buffer_clone = done_buffer.clone();
    
    let done_atomic = Arc::new(AtomicI32::new(0));
    let done_atomic_clone = done_atomic.clone();

    unsafe {
        *(done_buffer.contents() as *mut i32) = 0;
        *(count.contents() as *mut u32) = 0;
    }
    
    compute_encoder.set_buffer(6, Some(&done_buffer), 0);
    compute_encoder.set_buffer(7, Some(&count), 0);
    
    let threads_per_threadgroup = 256;
    let num_threadgroups = ctx.num_threadgroups;
    let threadgroup_size = metal::MTLSize::new(threads_per_threadgroup, 1, 1);
    let grid_size = metal::MTLSize::new(threads_per_threadgroup * num_threadgroups, 1, 1);
    
    compute_encoder.dispatch_threads(grid_size, threadgroup_size);
    compute_encoder.end_encoding();

    command_buffer.commit();

    let start_time = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(30);

    while command_buffer.status() != metal::MTLCommandBufferStatus::Completed {
        if start_time.elapsed() > timeout || !GPU_RUNNING.load(Ordering::SeqCst) {
            unsafe {
                *(done_buffer.contents() as *mut i32) = 1;
            }
            command_buffer.commit();
            return Err("GPU computation interrupted".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    let out_ptr = out_buffer.contents() as *const u8;
    let mut result = vec![0u8; 24];
    unsafe {
        std::ptr::copy_nonoverlapping(out_ptr, result.as_mut_ptr(), 24);
    }
    
    Ok(result)
} 