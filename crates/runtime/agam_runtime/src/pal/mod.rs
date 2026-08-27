//! Platform Abstraction Layer (PAL) for direct OS kernel interaction.
//!
//! Provides bare-metal virtual memory management, async event loops, and direct syscalls.

pub mod memory;

pub use memory::{
    AllocationFlags, MemoryProtection, PageAllocation, PalMemoryError, align_to_page_size,
    system_page_size,
};
