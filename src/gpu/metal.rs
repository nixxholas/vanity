use metal::{Device, MTLResourceOptions, CommandQueue, ComputePipelineState};

const THREADS_PER_THREADGROUP: u64 = 256;
const NUM_THREADGROUPS: u64 = 512;

pub struct MetalContext {
    device: Device,
    command_queue: CommandQueue,
    pipeline_state: ComputePipelineState,
}

impl MetalContext {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let device = Device::system_default()
            .ok_or("No Metal device found")?;
        
        let command_queue = device.new_command_queue();
        
        // Load the compiled Metal shader from the build directory
        let library_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/metallib/vanity.metallib");
            
        let library = device.new_library_with_file(library_path)?;
        let kernel = library.get_function("vanity_search", None)?;
        
        let pipeline_state = device
            .new_compute_pipeline_state_with_function(&kernel)?;
            
        Ok(Self {
            device,
            command_queue,
            pipeline_state,
        })
    }
}

pub fn vanity_round(
    _id: i32,
    seed: &[u8],
    base: &[u8],
    owner: &[u8],
    target: &str,
    _case_insensitive: bool,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let ctx = MetalContext::new()?;
    
    // Create input buffers
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
    
    // Create output buffer
    let out_buffer = ctx.device.new_buffer(
        24, // 16 bytes for seed + 8 bytes for count
        MTLResourceOptions::StorageModeShared,
    );
    
    // Create command buffer and encoder
    let command_buffer = ctx.command_queue.new_command_buffer();
    let compute_encoder = command_buffer.new_compute_command_encoder();
    
    // Set pipeline state and buffers
    compute_encoder.set_compute_pipeline_state(&ctx.pipeline_state);
    compute_encoder.set_buffer(0, Some(&seed_buffer), 0);
    compute_encoder.set_buffer(1, Some(&base_buffer), 0);
    compute_encoder.set_buffer(2, Some(&owner_buffer), 0);
    compute_encoder.set_buffer(3, Some(&target_buffer), 0);
    compute_encoder.set_buffer(4, Some(&target_len_buffer), 0);
    compute_encoder.set_buffer(5, Some(&out_buffer), 0);
    
    // Create done and count buffers
    let done = ctx.device.new_buffer(
        std::mem::size_of::<i32>() as u64,
        MTLResourceOptions::StorageModeShared,
    );
    
    let count = ctx.device.new_buffer(
        std::mem::size_of::<u32>() as u64,
        MTLResourceOptions::StorageModeShared,
    );
    
    // Initialize them to zero
    unsafe {
        *(done.contents() as *mut i32) = 0;
        *(count.contents() as *mut u32) = 0;
    }
    
    // Add them to the compute encoder
    compute_encoder.set_buffer(6, Some(&done), 0);
    compute_encoder.set_buffer(7, Some(&count), 0);
    
    // Configure and dispatch threadgroups
    let threads_per_grid = THREADS_PER_THREADGROUP * NUM_THREADGROUPS;
    let threadgroup_size = metal::MTLSize::new(THREADS_PER_THREADGROUP, 1, 1);
    let grid_size = metal::MTLSize::new(threads_per_grid, 1, 1);
    
    compute_encoder.dispatch_threads(grid_size, threadgroup_size);
    compute_encoder.end_encoding();
    
    // Execute and wait for completion
    command_buffer.commit();
    command_buffer.wait_until_completed();
    
    // Read results
    let out_ptr = out_buffer.contents() as *const u8;
    let mut result = vec![0u8; 24];
    unsafe {
        std::ptr::copy_nonoverlapping(out_ptr, result.as_mut_ptr(), 24);
    }
    
    Ok(result)
} 