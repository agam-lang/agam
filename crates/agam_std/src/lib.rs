//! # agam_std
//!
//! Agam standard library — hardware-optimized scientific computing.
//!
//! All data structures use contiguous memory layouts, `#[repr(C)]` alignment,
//! and cache-friendly access patterns for maximum hardware performance.
//!
//! ## Modules
//! - **tensor** — N-dimensional arrays with matmul, relu, sigmoid, softmax
//! - **math** — Integration (Simpson, Gauss), FFT, root-finding, gamma
//! - **linalg** — LU decomposition, inverse, eigenvalues, Ax=b solver
//! - **stats** — Descriptive stats, xoshiro256** PRNG, distributions, t-test
//! - **complex** — Complex numbers (#[repr(C)]), quaternions for 3D
//! - **units** — Compile-time SI dimensional analysis (zero runtime cost)
//! - **precision** — BigUint (hardware-aligned u32 limbs), interval arithmetic
//! - **numerical** — RK4 ODE solver, gradient descent, Adam optimizer

pub mod tensor;
pub mod math;
pub mod linalg;
pub mod stats;
pub mod complex;
pub mod units;
pub mod precision;
pub mod numerical;
