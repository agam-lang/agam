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
pub mod ebpf;
pub mod edge_ai;
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
pub mod json;
pub mod precision;
pub mod probabilistic;
pub mod process;
pub mod quantum;
pub mod random;
pub mod serial;
pub mod sparse;
pub mod stats;
pub mod string;
pub mod tensor;
pub mod time;
pub mod units;

pub use collections::{
    CompactGraph, Counter, FastHashMap, FastHashSet, FastRingBuffer, FastVec, OrderedMap,
};
pub use io::{AgamPath, FastBufReader, FastBufWriter, IoError};
pub use json::{
    JsonError, JsonValue, parse as parse_json, stringify as stringify_json,
    stringify_pretty as stringify_json_pretty,
};
pub use random::{
    Rng, choice as random_choice, float as random_float, int_range as random_int_range,
    shuffle as random_shuffle,
};
pub use string::{StringBuilder, Utf8Scanner, case_fold_eq};
pub use time::{DateTime, Instant, TimeError, sleep_micros, sleep_ms};
pub use combinatorics::{MatrixCover, ramsey_multi_color_lower_bound, saturated_matrix_cover};
pub use complex::Complex;
pub use ebpf::{
    EbpfInstruction, EbpfMap, EbpfMapKind, EbpfProgram, EbpfProgramKind, EbpfVerifier,
    VerifierError,
};
pub use edge_ai::{EdgeError, EdgeModel, ModelFormat, QuantizationPrecision, QuantizedTensor};
pub use ehrhart::{
    barycentric_convex_volume_upper_bound, bergman_initial_slope, bergman_potential_ray,
    simplex_volume,
};
pub use fft::{blackman_window, fft, hamming_window, hanning_window, ifft};
pub use gpu::{AsyncPipelineStage, Extent, GpuBuffer, GpuError, PartitionView, Tile, tile_matmul};
pub use group_theory::{LeavittAlgebraElement, kazhdan_constant_property_t};
pub use hankel::{HankelError, HankelMatrix, solve_hankel_system};
pub use ipc::{IpcError, SharedMemoryRegion, SpscRingBuffer};
pub use linalg::{Matrix, dot, glynn_permanent, matmul, ryser_permanent};
pub use net::{HttpHeaders, HttpMethod, HttpRequest, HttpResponse, NetError, NetworkManager, Url};
pub use packing::{cohn_elkies_bound, fourier_sign_uncertainty_radius, mellin_hankel_transform};
pub use probabilistic::{BayesianInference, Distribution, ModelTrace};
pub use quantum::{QuantumState, entangled_game_value_decay, quantum_correlated_sampling};
pub use serial::{
    SerialEnvelopeHeader, SerialError, ZeroCopy, decode_envelope, encode_envelope, from_bytes,
    from_bytes_slice, to_bytes, to_bytes_slice,
};
pub use sparse::{CooMatrix, CsrMatrix};
pub use tensor::{Tensor, TensorError, TensorView, default_strides};
