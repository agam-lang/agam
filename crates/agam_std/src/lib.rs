//! # agam_std
//!
//! Agam standard library — hardware-optimized scientific computing.
//!
//! All data structures use contiguous memory layouts, `#[repr(C)]` alignment,
//! and cache-friendly access patterns for maximum hardware performance.

pub mod complex;
pub mod dataframe;
pub mod effects;
pub mod io;
pub mod linalg;
pub mod math;
pub mod ml;
pub mod ndarray;
pub mod numerical;
pub mod precision;
pub mod stats;
pub mod tensor;
pub mod units;
