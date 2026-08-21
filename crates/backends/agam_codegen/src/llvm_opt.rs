//! Advanced LLVM 22.1+ Optimization Pipeline: ThinLTO, PGO, and SIMD Auto-Vectorization.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Supported LLVM Toolchain Major Versions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LlvmVersion {
    Llvm20,
    Llvm21,
    #[default]
    Llvm22_1,
    Llvm23,
}

impl LlvmVersion {
    /// Return true if this LLVM version supports `ptrtoaddr` IR instructions for provenance-free alias analysis.
    pub fn supports_ptrtoaddr(self) -> bool {
        self >= Self::Llvm22_1
    }

    /// Return true if LLVM 23 floating-point hex literal format (`f0x...`) is required.
    pub fn uses_f0x_float_literals(self) -> bool {
        self >= Self::Llvm23
    }
}

/// Link-Time Optimization (LTO) Mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LtoMode {
    #[default]
    None,
    Thin,
    ThinParallel,
    Full,
}

/// Profile-Guided Optimization (PGO) Mode.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PgoMode {
    #[default]
    None,
    Generate {
        profile_dir: PathBuf,
    },
    Use {
        profile_file: PathBuf,
    },
}

/// SIMD Vectorization Target Capabilities.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimdConfig {
    pub auto_vectorize: bool,
    pub preferred_vector_width: u32,
    pub target_features: Vec<String>,
}

impl Default for SimdConfig {
    fn default() -> Self {
        Self {
            auto_vectorize: true,
            preferred_vector_width: 256,
            target_features: vec!["+avx2".into(), "+fma".into()],
        }
    }
}

/// Complete LLVM 22.1+ Optimization Configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlvmOptConfig {
    pub version: LlvmVersion,
    pub opt_level: u8, // 0, 1, 2, 3
    pub lto: LtoMode,
    pub pgo: PgoMode,
    pub simd: SimdConfig,
    pub loop_unroll: bool,
    pub loop_fusion: bool,
}

impl Default for LlvmOptConfig {
    fn default() -> Self {
        Self {
            version: LlvmVersion::Llvm22_1,
            opt_level: 3,
            lto: LtoMode::Thin,
            pgo: PgoMode::None,
            simd: SimdConfig::default(),
            loop_unroll: true,
            loop_fusion: true,
        }
    }
}

impl LlvmOptConfig {
    /// Build clang / opt / lld command-line arguments for the configured optimization pipeline.
    pub fn build_clang_opt_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        // 1. Optimization level
        args.push(format!("-O{}", self.opt_level));

        // 2. LTO flags
        match self.lto {
            LtoMode::None => {}
            LtoMode::Thin | LtoMode::ThinParallel => {
                args.push("-flto=thin".into());
            }
            LtoMode::Full => {
                args.push("-flto=full".into());
            }
        }

        // 3. PGO flags
        match &self.pgo {
            PgoMode::None => {}
            PgoMode::Generate { profile_dir } => {
                args.push(format!("-fprofile-generate={}", profile_dir.display()));
            }
            PgoMode::Use { profile_file } => {
                args.push(format!("-fprofile-use={}", profile_file.display()));
            }
        }

        // 4. SIMD & Target Architecture features
        if self.simd.auto_vectorize {
            args.push("-fvectorize".into());
            args.push("-fslp-vectorize".into());
            for feat in &self.simd.target_features {
                args.push(format!("-mllvm=-mattr={}", feat));
            }
        } else {
            args.push("-fno-vectorize".into());
            args.push("-fno-slp-vectorize".into());
        }

        // 5. Loop optimizations
        if self.loop_unroll {
            args.push("-funroll-loops".into());
        }
        if self.loop_fusion {
            args.push("-mllvm=-enable-loop-fusion".into());
        }

        args
    }

    /// Emit LLVM IR Module-level optimization attributes.
    pub fn emit_module_attributes(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("; LLVM Version: {:?}\n", self.version));
        out.push_str(&format!("; LTO Mode: {:?}\n", self.lto));
        out.push_str(&format!(
            "; Vector Width: {} bits\n",
            self.simd.preferred_vector_width
        ));

        if self.lto != LtoMode::None {
            out.push_str("!llvm.module.flags = !{!0}\n");
            out.push_str("!0 = !{i32 1, !\"ThinLTO\", i32 1}\n");
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llvm_22_version_capabilities() {
        assert!(LlvmVersion::Llvm22_1.supports_ptrtoaddr());
        assert!(!LlvmVersion::Llvm20.supports_ptrtoaddr());
        assert!(LlvmVersion::Llvm23.uses_f0x_float_literals());
        assert!(!LlvmVersion::Llvm22_1.uses_f0x_float_literals());
    }

    #[test]
    fn test_build_thin_lto_pgo_clang_args() {
        let config = LlvmOptConfig {
            version: LlvmVersion::Llvm22_1,
            opt_level: 3,
            lto: LtoMode::Thin,
            pgo: PgoMode::Use {
                profile_file: PathBuf::from("default.profdata"),
            },
            simd: SimdConfig::default(),
            loop_unroll: true,
            loop_fusion: true,
        };

        let args = config.build_clang_opt_args();
        assert!(args.contains(&"-O3".to_string()));
        assert!(args.contains(&"-flto=thin".to_string()));
        assert!(args.contains(&"-fprofile-use=default.profdata".to_string()));
        assert!(args.contains(&"-fvectorize".to_string()));
        assert!(args.contains(&"-funroll-loops".to_string()));
        assert!(args.contains(&"-mllvm=-enable-loop-fusion".to_string()));
    }
}
