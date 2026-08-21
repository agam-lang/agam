//! # agam_std
//!
//! Agam standard library — hardware-optimized scientific computing.
//!
//! All data structures use contiguous memory layouts, `#[repr(C)]` alignment,
//! and cache-friendly access patterns for maximum hardware performance.

pub mod collections;
pub mod complex;
pub mod dataframe;
pub mod effects;
pub mod env;
pub mod fft;
pub mod gpu;
pub mod io;
pub mod linalg;
pub mod math;
pub mod ml;
pub mod ndarray;
pub mod net;
pub mod numerical;
pub mod precision;
pub mod process;
pub mod sparse;
pub mod stats;
pub mod tensor;
pub mod units;

pub use collections::{CompactGraph, FastRingBuffer};
pub use complex::Complex;
pub use fft::{blackman_window, fft, hamming_window, hanning_window, ifft};
pub use gpu::{GpuBuffer, GpuError, Tile, tile_matmul};
pub use sparse::{CooMatrix, CsrMatrix};
