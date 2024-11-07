fn main() {
    #[cfg(feature = "cuda-gpu")]
    build_cuda_libs();

    #[cfg(feature = "apple-gpu")]
    compile_metal_shaders();
}

#[cfg(feature = "cuda-gpu")]
fn build_cuda_libs() {
    println!("cargo::rerun-if-changed=kernels/");

    cc::Build::new()
        .cuda(true)
        .file("kernels/utils.cu")
        .file("kernels/vanity.cu")
        .file("kernels/base58.cu")
        .file("kernels/sha256.cu")
        .flag("-cudart=static")
        .flag("-gencode=arch=compute_89,code=sm_89")
        .flag("-gencode=arch=compute_89,code=compute_89")
        .compile("libvanity.a");

    // Add link directory
    println!("cargo:rustc-link-search=native=/usr/local/cuda/lib64");
    println!("cargo:rustc-link-lib=cudart");
    println!("cargo:rustc-link-lib=cuda");

    // Emit the location of the compiled library
    let out_dir = std::env::var("OUT_DIR").unwrap();
    println!("cargo:rustc-link-search=native={}", out_dir);
}

#[cfg(feature = "apple-gpu")]
fn compile_metal_shaders() {
    use std::process::Command;

    println!("cargo:rerun-if-changed=kernels/vanity.metal");

    // Create the metallib directory if it doesn't exist
    std::fs::create_dir_all("target/metallib").unwrap();

    // Compile Metal shader to metallib
    let status = Command::new("xcrun")
        .args([
            "-sdk", "macosx", "metal",
            "-c", "kernels/vanity.metal",
            "-o", "target/metallib/vanity.air"
        ])
        .status()
        .expect("Failed to compile Metal shader");

    if !status.success() {
        panic!("Metal shader compilation failed");
    }

    let status = Command::new("xcrun")
        .args([
            "-sdk", "macosx", "metallib",
            "target/metallib/vanity.air",
            "-o", "target/metallib/vanity.metallib"
        ])
        .status()
        .expect("Failed to create metallib");

    if !status.success() {
        panic!("Metallib creation failed");
    }
}
