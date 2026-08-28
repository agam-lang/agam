//! Platform Abstraction Layer (PAL) for direct OS kernel interaction.
//!
//! Provides bare-metal virtual memory management, async event loops, raw non-blocking sockets, and direct syscalls.

pub mod event;
pub mod memory;
pub mod net;

pub use event::{Event, EventDemuxer, EventInterest, PalEventError, PollTimeout, Token};
pub use memory::{
    AllocationFlags, MemoryProtection, PageAllocation, PalMemoryError, align_to_page_size,
    system_page_size,
};
pub use net::{PalNetError, PalTcpListener, PalTcpStream, PalUdpSocket, ShutdownKind};
