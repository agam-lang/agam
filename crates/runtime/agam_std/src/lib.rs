//! # agam_std
//!
//! Agam standard library — hardware-optimized scientific computing.
//!
//! All data structures use contiguous memory layouts, `#[repr(C)]` alignment,
//! and cache-friendly access patterns for maximum hardware performance.

pub mod collections;
pub mod combinatorics;
pub mod complex;
pub mod dataframe;
pub mod effects;
pub mod ehrhart;
pub mod env;
pub mod fft;
pub mod gpu;
pub mod group_theory;
pub mod hankel;
pub mod io;
pub mod ipc;
pub mod linalg;
pub mod math;
pub mod ml;
pub mod ndarray;
pub mod net;
pub mod numerical;
pub mod packing;
pub mod precision;
pub mod process;
pub mod quantum;
pub mod serial;
pub mod sparse;
pub mod stats;
pub mod tensor;
pub mod units;

pub use collections::{CompactGraph, FastRingBuffer};
pub use combinatorics::{MatrixCover, ramsey_multi_color_lower_bound, saturated_matrix_cover};
pub use complex::Complex;
pub use ehrhart::{
    barycentric_convex_volume_upper_bound, bergman_initial_slope, bergman_potential_ray,
    simplex_volume,
};
pub use fft::{blackman_window, fft, hamming_window, hanning_window, ifft};
pub use gpu::{AsyncPipelineStage, Extent, GpuBuffer, GpuError, PartitionView, Tile, tile_matmul};
pub use group_theory::{LeavittAlgebraElement, kazhdan_constant_property_t};
pub use hankel::{HankelError, HankelMatrix, solve_hankel_system};
pub use ipc::{IpcError, SharedMemoryRegion, SpscRingBuffer};
pub use linalg::{Matrix, glynn_permanent, ryser_permanent};
pub use packing::{cohn_elkies_bound, fourier_sign_uncertainty_radius, mellin_hankel_transform};
pub use quantum::{QuantumState, entangled_game_value_decay, quantum_correlated_sampling};
pub use serial::{
    SerialEnvelopeHeader, SerialError, ZeroCopy, decode_envelope, encode_envelope, from_bytes,
    from_bytes_slice, to_bytes, to_bytes_slice,
};
pub use sparse::{CooMatrix, CsrMatrix};
