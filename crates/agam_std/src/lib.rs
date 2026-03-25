//! # agam_std
//!
//! Agam standard library — hardware-optimized scientific computing.
//!
//! All data structures use contiguous memory layouts, `#[repr(C)]` alignment,
//! and cache-friendly access patterns for maximum hardware performance.

pub mod tensor;
pub mod math;
pub mod linalg;
pub mod stats;
pub mod complex;
pub mod units;
pub mod precision;
pub mod numerical;
pub mod dataframe;
pub mod ndarray;
pub mod ml;
