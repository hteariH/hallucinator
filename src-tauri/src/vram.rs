const FOUR_GIB: u64 = 4 * 1024 * 1024 * 1024;

pub fn recommended_model(vram_bytes: u64) -> &'static str {
    if vram_bytes >= FOUR_GIB { "gemma-3-4b" } else { "gemma-3-1b" }
}

/// Largest dedicated video memory across hardware adapters, via DXGI.
#[cfg(windows)]
pub fn max_dedicated_vram() -> u64 {
    use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE};
    unsafe {
        let Ok(factory) = CreateDXGIFactory1::<IDXGIFactory1>() else { return 0 };
        let mut max = 0u64;
        let mut i = 0;
        while let Ok(adapter) = factory.EnumAdapters1(i) {
            if let Ok(desc) = adapter.GetDesc1() {
                if desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 == 0 {
                    max = max.max(desc.DedicatedVideoMemory as u64);
                }
            }
            i += 1;
        }
        max
    }
}

#[cfg(not(windows))]
pub fn max_dedicated_vram() -> u64 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threshold_is_four_gib() {
        assert_eq!(recommended_model(0), "gemma-3-1b");
        assert_eq!(recommended_model(FOUR_GIB - 1), "gemma-3-1b");
        assert_eq!(recommended_model(FOUR_GIB), "gemma-3-4b");
        assert_eq!(recommended_model(8 * FOUR_GIB), "gemma-3-4b");
    }

    #[test]
    fn detection_does_not_panic() {
        println!("dedicated VRAM: {} MiB", max_dedicated_vram() / 1024 / 1024);
    }
}
