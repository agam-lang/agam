//! Runtime CPU topology detection and hardware introspection.
//!
//! Detects the machine's hardware capabilities at runtime:
//! - CPU architecture (x86_64, aarch64)
//! - Cache hierarchy (L1/L2/L3 sizes and line sizes)
//! - Core counts (physical, logical)
//! - SIMD feature flags (SSE, AVX, AVX-512, NEON)
//!
//! All queries are cached after first invocation for zero-cost subsequent access.

use std::sync::OnceLock;

/// SIMD instruction set capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimdCapabilities {
    pub sse2: bool,
    pub sse4_1: bool,
    pub sse4_2: bool,
    pub avx: bool,
    pub avx2: bool,
    pub avx512f: bool,
    pub fma: bool,
    pub neon: bool,
}

impl SimdCapabilities {
    /// Detect at runtime using `is_x86_feature_detected!` / target arch.
    pub fn detect() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            Self {
                sse2: std::is_x86_feature_detected!("sse2"),
                sse4_1: std::is_x86_feature_detected!("sse4.1"),
                sse4_2: std::is_x86_feature_detected!("sse4.2"),
                avx: std::is_x86_feature_detected!("avx"),
                avx2: std::is_x86_feature_detected!("avx2"),
                avx512f: std::is_x86_feature_detected!("avx512f"),
                fma: std::is_x86_feature_detected!("fma"),
                neon: false,
            }
        }
        #[cfg(target_arch = "aarch64")]
        {
            Self {
                sse2: false,
                sse4_1: false,
                sse4_2: false,
                avx: false,
                avx2: false,
                avx512f: false,
                fma: false,
                neon: true, // NEON is mandatory on AArch64
            }
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            Self {
                sse2: false,
                sse4_1: false,
                sse4_2: false,
                avx: false,
                avx2: false,
                avx512f: false,
                fma: false,
                neon: false,
            }
        }
    }

    /// Best available SIMD width in bytes.
    pub fn best_simd_width(&self) -> usize {
        if self.avx512f {
            64
        } else if self.avx2 || self.avx {
            32
        } else if self.sse2 {
            16
        } else if self.neon {
            16
        } else {
            8
        } // scalar fallback
    }

    /// Best SIMD tier name.
    pub fn best_tier(&self) -> SimdTier {
        if self.avx512f {
            SimdTier::Avx512
        } else if self.avx2 {
            SimdTier::Avx2
        } else if self.avx {
            SimdTier::Avx
        } else if self.sse4_2 {
            SimdTier::Sse42
        } else if self.sse2 {
            SimdTier::Sse2
        } else if self.neon {
            SimdTier::Neon
        } else {
            SimdTier::Scalar
        }
    }
}

/// SIMD dispatch tiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SimdTier {
    Scalar = 0,
    Sse2 = 1,
    Sse42 = 2,
    Neon = 3,
    Avx = 4,
    Avx2 = 5,
    Avx512 = 6,
}

impl SimdTier {
    pub fn width_bytes(&self) -> usize {
        match self {
            SimdTier::Scalar => 8,
            SimdTier::Sse2 | SimdTier::Sse42 | SimdTier::Neon => 16,
            SimdTier::Avx | SimdTier::Avx2 => 32,
            SimdTier::Avx512 => 64,
        }
    }

    /// Number of f64 values per SIMD register.
    pub fn f64_lanes(&self) -> usize {
        self.width_bytes() / 8
    }

    /// Number of f32 values per SIMD register.
    pub fn f32_lanes(&self) -> usize {
        self.width_bytes() / 4
    }
}

/// Cache level information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheInfo {
    /// Cache size in bytes.
    pub size: usize,
    /// Cache line size in bytes (typically 64).
    pub line_size: usize,
    /// Associativity.
    pub associativity: usize,
}

/// Full hardware topology for the current machine.
#[derive(Debug, Clone)]
pub struct HardwareInfo {
    pub arch: &'static str,
    pub physical_cores: usize,
    pub logical_cores: usize,
    pub l1_data: CacheInfo,
    pub l2: CacheInfo,
    pub l3: CacheInfo,
    pub simd: SimdCapabilities,
    pub pointer_width: usize,
    pub endianness: Endianness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endianness {
    Little,
    Big,
}

impl HardwareInfo {
    /// Detect the current machine's hardware topology.
    pub fn detect() -> Self {
        let logical_cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        // Heuristic: physical cores ≈ logical / 2 on hyperthreaded systems
        let physical_cores = (logical_cores / 2).max(1);

        // Conservative defaults for cache — actual CPUID parsing can be added later
        let l1_data = CacheInfo {
            size: 32 * 1024,
            line_size: 64,
            associativity: 8,
        };
        let l2 = CacheInfo {
            size: 256 * 1024,
            line_size: 64,
            associativity: 4,
        };
        let l3 = CacheInfo {
            size: 8 * 1024 * 1024,
            line_size: 64,
            associativity: 16,
        };

        let endianness = if cfg!(target_endian = "little") {
            Endianness::Little
        } else {
            Endianness::Big
        };

        Self {
            arch: std::env::consts::ARCH,
            physical_cores,
            logical_cores,
            l1_data,
            l2,
            l3,
            simd: SimdCapabilities::detect(),
            pointer_width: std::mem::size_of::<usize>() * 8,
            endianness,
        }
    }

    /// Optimal tile size for blocked matrix multiply (fits in L1).
    pub fn optimal_tile_size(&self) -> usize {
        // Each tile is tile_size × tile_size × 8 bytes (f64)
        // We want 3 tiles (A, B, C blocks) to fit in L1
        let available = self.l1_data.size / 3;
        let max_elements = available / 8;
        (max_elements as f64).sqrt() as usize
    }

    /// Optimal chunk size for parallel work division.
    pub fn optimal_chunk_size(&self, total_work: usize) -> usize {
        let per_core = total_work / self.logical_cores;
        // Ensure chunk is cache-line aligned
        let line_elements = self.l1_data.line_size / 8;
        ((per_core / line_elements) * line_elements).max(line_elements)
    }
}

/// Global cached hardware info (initialized once, read many).
static HWINFO: OnceLock<HardwareInfo> = OnceLock::new();

/// Get hardware info (cached after first call).
pub fn hwinfo() -> &'static HardwareInfo {
    HWINFO.get_or_init(HardwareInfo::detect)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hwinfo_detect() {
        let hw = HardwareInfo::detect();
        assert!(hw.logical_cores >= 1);
        assert!(hw.physical_cores >= 1);
        assert!(hw.pointer_width == 32 || hw.pointer_width == 64);
    }

    #[test]
    fn test_cache_defaults() {
        let hw = HardwareInfo::detect();
        assert_eq!(hw.l1_data.size, 32 * 1024);
        assert_eq!(hw.l1_data.line_size, 64);
        assert_eq!(hw.l2.size, 256 * 1024);
    }

    #[test]
    fn test_simd_detect() {
        let simd = SimdCapabilities::detect();
        let width = simd.best_simd_width();
        assert!(width >= 8); // at least scalar
        assert!(width <= 64); // at most AVX-512
    }

    #[test]
    fn test_simd_tier_lanes() {
        assert_eq!(SimdTier::Avx2.f64_lanes(), 4);
        assert_eq!(SimdTier::Sse2.f64_lanes(), 2);
        assert_eq!(SimdTier::Avx512.f64_lanes(), 8);
        assert_eq!(SimdTier::Scalar.f64_lanes(), 1);
    }

    #[test]
    fn test_simd_tier_ordering() {
        assert!(SimdTier::Avx512 > SimdTier::Avx2);
        assert!(SimdTier::Avx2 > SimdTier::Sse2);
        assert!(SimdTier::Sse2 > SimdTier::Scalar);
    }

    #[test]
    fn test_optimal_tile_size() {
        let hw = HardwareInfo::detect();
        let tile = hw.optimal_tile_size();
        // With 32KB L1, ~10KB per tile, ~1280 elements, sqrt ≈ 35
        assert!(tile > 10 && tile < 100, "tile={}", tile);
    }

    #[test]
    fn test_hwinfo_cached() {
        let a = hwinfo();
        let b = hwinfo();
        // Both should point to the same data
        assert_eq!(a.arch, b.arch);
        assert_eq!(a.logical_cores, b.logical_cores);
    }

    #[test]
    fn test_endianness() {
        let hw = HardwareInfo::detect();
        // Most modern systems are little-endian
        #[cfg(target_endian = "little")]
        assert_eq!(hw.endianness, Endianness::Little);
    }

    #[test]
    fn test_optimal_chunk_size() {
        let hw = HardwareInfo::detect();
        let chunk = hw.optimal_chunk_size(10000);
        assert!(chunk >= 8); // at least one cache line of f64s
        assert!(chunk <= 10000);
    }
}
