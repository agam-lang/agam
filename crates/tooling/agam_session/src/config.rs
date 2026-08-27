//! Session configuration and compiler options.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use agam_target::TargetTriple;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackendKind {
    Jit,
    Llvm,
    C,
}

impl Default for BackendKind {
    fn default() -> Self {
        Self::Jit
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptLevel {
    O0,
    O1,
    O2,
    O3,
    Os,
    Oz,
}

impl Default for OptLevel {
    fn default() -> Self {
        Self::O0
    }
}

impl OptLevel {
    pub fn as_u32(&self) -> u32 {
        match self {
            Self::O0 => 0,
            Self::O1 => 1,
            Self::O2 => 2,
            Self::O3 => 3,
            Self::Os | Self::Oz => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionConfig {
    pub target: TargetTriple,
    pub backend: BackendKind,
    pub opt_level: OptLevel,
    pub verbose: bool,
    pub emit_ir: bool,
    pub emit_mir: bool,
    pub emit_ast: bool,
    pub output_path: Option<PathBuf>,
    pub sysroot: Option<PathBuf>,
    pub experimental_crypto: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            target: TargetTriple::host(),
            backend: BackendKind::Jit,
            opt_level: OptLevel::O0,
            verbose: false,
            emit_ir: false,
            emit_mir: false,
            emit_ast: false,
            output_path: None,
            sysroot: None,
            experimental_crypto: false,
        }
    }
}
