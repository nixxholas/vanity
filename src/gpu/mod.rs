#[cfg(feature = "apple-gpu")]
mod metal;

pub struct GpuVanitySearch;

impl GpuVanitySearch {
    pub fn new() -> Self {
        Self
    }

    pub fn vanity_round(
        &self,
        id: i32,
        seed: &[u8],
        base: &[u8],
        owner: &[u8],
        target: &str,
        case_insensitive: bool,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        #[cfg(feature = "apple-gpu")]
        return metal::vanity_round(id, seed, base, owner, target, case_insensitive);

        #[cfg(not(feature = "apple-gpu"))]
        Err("No GPU implementation available".into())
    }
} 