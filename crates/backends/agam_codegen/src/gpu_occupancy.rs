//! GPU Occupancy Auto-Tuning, Register Pressure Modeling & Shared Memory Conflict Optimization.

use serde::{Deserialize, Serialize};

/// Target GPU hardware device capabilities for occupancy analysis.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GpuDeviceCapability {
    pub name: String,
    pub compute_capability: (u32, u32), // e.g. (8, 0) for Ampere, (9, 0) for Hopper
    pub max_threads_per_sm: u32,
    pub max_warps_per_sm: u32,
    pub max_blocks_per_sm: u32,
    pub max_registers_per_sm: u32,
    pub max_shared_mem_per_sm_bytes: u32,
    pub max_shared_mem_per_block_bytes: u32,
    pub max_threads_per_block: u32,
    pub warp_size: u32,
}

impl GpuDeviceCapability {
    /// Nvidia Ampere Architecture (RTX 3080/3090, A100 - SM 8.0/8.6).
    pub fn nvidia_ampere() -> Self {
        Self {
            name: "Nvidia Ampere (SM 8.0)".into(),
            compute_capability: (8, 0),
            max_threads_per_sm: 2048,
            max_warps_per_sm: 64,
            max_blocks_per_sm: 32,
            max_registers_per_sm: 65536,
            max_shared_mem_per_sm_bytes: 167936, // 164 KB
            max_shared_mem_per_block_bytes: 167936,
            max_threads_per_block: 1024,
            warp_size: 32,
        }
    }

    /// Nvidia Hopper Architecture (H100 - SM 9.0).
    pub fn nvidia_hopper() -> Self {
        Self {
            name: "Nvidia Hopper (SM 9.0)".into(),
            compute_capability: (9, 0),
            max_threads_per_sm: 2048,
            max_warps_per_sm: 64,
            max_blocks_per_sm: 32,
            max_registers_per_sm: 65536,
            max_shared_mem_per_sm_bytes: 233472, // 228 KB
            max_shared_mem_per_block_bytes: 233472,
            max_threads_per_block: 1024,
            warp_size: 32,
        }
    }

    /// Nvidia Blackwell Architecture (B200 - SM 10.0).
    pub fn nvidia_blackwell() -> Self {
        Self {
            name: "Nvidia Blackwell (SM 10.0)".into(),
            compute_capability: (10, 0),
            max_threads_per_sm: 2048,
            max_warps_per_sm: 64,
            max_blocks_per_sm: 32,
            max_registers_per_sm: 65536,
            max_shared_mem_per_sm_bytes: 262144, // 256 KB
            max_shared_mem_per_block_bytes: 262144,
            max_threads_per_block: 1024,
            warp_size: 32,
        }
    }
}

/// Limiting constraint for GPU SM occupancy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OccupancyLimitFactor {
    Warps,
    Registers,
    SharedMemory,
    Blocks,
}

/// Detailed occupancy report for a compiled GPU kernel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OccupancyReport {
    pub theoretical_occupancy_pct: f64,
    pub active_warps_per_sm: u32,
    pub active_blocks_per_sm: u32,
    pub max_active_warps_per_sm: u32,
    pub limiting_factor: OccupancyLimitFactor,
}

/// Compute exact theoretical occupancy based on register, shared memory, and block size constraints.
pub fn calculate_occupancy(
    registers_per_thread: u32,
    shared_mem_per_block_bytes: u32,
    threads_per_block: u32,
    device: &GpuDeviceCapability,
) -> OccupancyReport {
    assert!(threads_per_block > 0 && threads_per_block <= device.max_threads_per_block);
    let warps_per_block = threads_per_block.div_ceil(device.warp_size);

    // 1. Block count limited by max threads / warps per SM
    let blocks_limited_by_warps = device.max_warps_per_sm / warps_per_block;

    // 2. Block count limited by registers
    // In Nvidia GPUs, register allocation granularity is aligned per warp
    let regs_per_warp = registers_per_thread * device.warp_size;
    let regs_per_block = regs_per_warp * warps_per_block;
    let blocks_limited_by_regs = device
        .max_registers_per_sm
        .checked_div(regs_per_block)
        .unwrap_or(device.max_blocks_per_sm);

    // 3. Block count limited by shared memory
    let blocks_limited_by_smem = device
        .max_shared_mem_per_sm_bytes
        .checked_div(shared_mem_per_block_bytes)
        .unwrap_or(device.max_blocks_per_sm);

    // 4. Block count limited by architectural max blocks per SM
    let blocks_limit = device.max_blocks_per_sm;

    // Find actual active blocks per SM
    let active_blocks = blocks_limited_by_warps
        .min(blocks_limited_by_regs)
        .min(blocks_limited_by_smem)
        .min(blocks_limit);

    let active_warps = active_blocks * warps_per_block;
    let occupancy_pct = (active_warps as f64 / device.max_warps_per_sm as f64) * 100.0;

    let limiting_factor = if active_blocks == blocks_limited_by_regs {
        OccupancyLimitFactor::Registers
    } else if active_blocks == blocks_limited_by_smem {
        OccupancyLimitFactor::SharedMemory
    } else if active_blocks == blocks_limited_by_warps {
        OccupancyLimitFactor::Warps
    } else {
        OccupancyLimitFactor::Blocks
    };

    OccupancyReport {
        theoretical_occupancy_pct: occupancy_pct,
        active_warps_per_sm: active_warps,
        active_blocks_per_sm: active_blocks,
        max_active_warps_per_sm: device.max_warps_per_sm,
        limiting_factor,
    }
}

/// Auto-tuned grid and block configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoTunedLaunchConfig {
    pub grid_dim: (u32, u32, u32),
    pub block_dim: (u32, u32, u32),
    pub dynamic_shared_mem_bytes: u32,
    pub expected_occupancy_pct: u32,
}

/// Auto-tune optimal launch dimensions for a 1D/2D workload.
pub fn auto_tune_kernel_launch(
    total_elements: usize,
    registers_per_thread: u32,
    shared_mem_per_thread_bytes: u32,
    device: &GpuDeviceCapability,
) -> AutoTunedLaunchConfig {
    let candidate_block_sizes = [64, 128, 256, 512];
    let mut best_block = 256;
    let mut best_occupancy = 0.0;

    for &block_size in &candidate_block_sizes {
        let smem = block_size * shared_mem_per_thread_bytes;
        if smem > device.max_shared_mem_per_block_bytes {
            continue;
        }

        let report = calculate_occupancy(registers_per_thread, smem, block_size, device);
        if report.theoretical_occupancy_pct > best_occupancy {
            best_occupancy = report.theoretical_occupancy_pct;
            best_block = block_size;
        }
    }

    let grid_size = (total_elements as u32).div_ceil(best_block);
    let dynamic_smem = best_block * shared_mem_per_thread_bytes;

    AutoTunedLaunchConfig {
        grid_dim: (grid_size, 1, 1),
        block_dim: (best_block, 1, 1),
        dynamic_shared_mem_bytes: dynamic_smem,
        expected_occupancy_pct: best_occupancy as u32,
    }
}

/// Shared memory bank conflict optimizer.
pub struct SharedMemLayoutOptimizer;

impl SharedMemLayoutOptimizer {
    /// Calculate optimal padded stride for a 2D tile to eliminate 32-bank conflicts.
    ///
    /// On Nvidia GPUs (32 banks, 4 bytes per bank), accessing a column of a matrix
    /// stored with stride `cols` produces an $N$-way bank conflict if `cols % 32 == 0`.
    /// Adding padding (e.g. `cols + 1` or `cols + 4`) resolves the conflict.
    pub fn calculate_conflict_free_stride(cols: usize, element_size_bytes: usize) -> usize {
        let words_per_row = (cols * element_size_bytes) / 4;
        if words_per_row.is_multiple_of(32) {
            cols + (4 / element_size_bytes).max(1)
        } else {
            cols
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_occupancy_calculation_ampere() {
        let device = GpuDeviceCapability::nvidia_ampere();
        // 32 registers per thread, 0 shared memory, 256 threads per block
        let report = calculate_occupancy(32, 0, 256, &device);
        assert_eq!(report.active_warps_per_sm, 64);
        assert_eq!(report.theoretical_occupancy_pct, 100.0);
    }

    #[test]
    fn test_auto_tune_launch_config() {
        let device = GpuDeviceCapability::nvidia_hopper();
        let config = auto_tune_kernel_launch(1_000_000, 32, 8, &device);
        assert!(config.block_dim.0 >= 64);
        assert!(config.grid_dim.0 > 0);
        assert!(config.expected_occupancy_pct > 50);
    }

    #[test]
    fn test_shared_memory_bank_conflict_padding() {
        // A 32x32 f32 matrix has stride 32 (a multiple of 32 -> 32-way bank conflict on columns)
        let padded = SharedMemLayoutOptimizer::calculate_conflict_free_stride(32, 4);
        assert_eq!(padded, 33); // Padded with +1 element to stagger banks across rows

        // Stride 31 is already relatively prime to 32 -> no padding needed
        let unpadded = SharedMemLayoutOptimizer::calculate_conflict_free_stride(31, 4);
        assert_eq!(unpadded, 31);
    }
}
