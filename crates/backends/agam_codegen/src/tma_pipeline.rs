//! Asynchronous Memory Transfer and Hardware TMA Copy Pipeline Engine.
//!
//! Provides descriptors and code emitters for multi-stage asynchronous copy pipelines
//! (TMA / `memcpy_async`) moving multi-dimensional tensor partition views directly
//! from global memory into shared-memory tiles without stalling compute execution.

use serde::{Deserialize, Serialize};

/// Multi-dimensional TMA box copy geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TmaCopyDimension {
    pub box_rows: usize,
    pub box_cols: usize,
    pub global_stride_bytes: usize,
    pub elem_size_bytes: usize,
}

impl TmaCopyDimension {
    pub fn total_bytes(&self) -> usize {
        self.box_rows * self.box_cols * self.elem_size_bytes
    }
}

/// Descriptor configuring an asynchronous 2D/3D tile copy from global VRAM to shared memory.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TmaCopyDescriptor {
    pub src_global_symbol: String,
    pub dst_smem_offset: usize,
    pub dimensions: TmaCopyDimension,
    pub swizzle_mode: bool,
}

/// Multi-stage asynchronous copy pipeline tracker.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AsyncPipelineTracker {
    pub num_stages: usize,
    pub current_producer_stage: usize,
    pub current_consumer_stage: usize,
}

impl AsyncPipelineTracker {
    pub fn new(num_stages: usize) -> Self {
        Self {
            num_stages: num_stages.max(1),
            current_producer_stage: 0,
            current_consumer_stage: 0,
        }
    }

    /// Advance the producer stage (when issuing a new async tile transfer).
    pub fn advance_producer(&mut self) {
        self.current_producer_stage = (self.current_producer_stage + 1) % self.num_stages;
    }

    /// Advance the consumer stage (when finishing computation of a buffered tile).
    pub fn advance_consumer(&mut self) {
        self.current_consumer_stage = (self.current_consumer_stage + 1) % self.num_stages;
    }

    /// Emit SPIR-V / NVVM low-level intrinsic pseudo-code for an async TMA copy.
    pub fn emit_async_copy_instruction(&self, desc: &TmaCopyDescriptor) -> String {
        format!(
            "__tma_async_copy_2d(dst_smem + {}, {}, rows={}, cols={}, stride={});",
            desc.dst_smem_offset,
            desc.src_global_symbol,
            desc.dimensions.box_rows,
            desc.dimensions.box_cols,
            desc.dimensions.global_stride_bytes
        )
    }

    /// Emit pipeline commit instruction.
    pub fn emit_pipeline_commit(&self) -> String {
        "__pipeline_commit_group();".into()
    }

    /// Emit pipeline wait instruction ensuring $N$ stages prior have completed transfer.
    pub fn emit_pipeline_wait_prior(&self, stages_prior: usize) -> String {
        format!("__pipeline_wait_prior({});", stages_prior)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tma_copy_dimension_and_descriptor() {
        let dim = TmaCopyDimension {
            box_rows: 16,
            box_cols: 16,
            global_stride_bytes: 1024,
            elem_size_bytes: 4,
        };
        assert_eq!(dim.total_bytes(), 16 * 16 * 4); // 1024 bytes

        let desc = TmaCopyDescriptor {
            src_global_symbol: "g_tensor_A".into(),
            dst_smem_offset: 2048,
            dimensions: dim,
            swizzle_mode: true,
        };

        let tracker = AsyncPipelineTracker::new(2);
        let instr = tracker.emit_async_copy_instruction(&desc);
        assert!(instr.contains("__tma_async_copy_2d"));
        assert!(instr.contains("g_tensor_A"));
        assert!(instr.contains("rows=16"));
    }

    #[test]
    fn test_async_pipeline_stages_and_emissions() {
        let mut tracker = AsyncPipelineTracker::new(3);
        assert_eq!(tracker.current_producer_stage, 0);

        let commit_code = tracker.emit_pipeline_commit();
        assert_eq!(commit_code, "__pipeline_commit_group();");

        let wait_code = tracker.emit_pipeline_wait_prior(2);
        assert_eq!(wait_code, "__pipeline_wait_prior(2);");

        tracker.advance_producer();
        assert_eq!(tracker.current_producer_stage, 1);
        tracker.advance_consumer();
        assert_eq!(tracker.current_consumer_stage, 1);
    }
}
