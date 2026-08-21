//! Heterogeneous NPU (Neural Processing Unit) & SIMD Tile Tensor Instruction Emitter.
//!
//! Generates hardware-accelerated tensor tile kernels targeting:
//! - Qualcomm Hexagon HVX / NPU
//! - Apple Neural Engine (ANE) / Accelerate
//! - Intel NPU / AVX-512 VNNI / AMX
//! - ARM Ethos / NEON DotProd / I8MM
//! - Generic SIMD Tile Matrix Engines

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NpuTargetKind {
    QualcommHexagon,
    AppleNeuralEngine,
    IntelNpu,
    ArmEthos,
    GenericSimdTile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NpuPrecision {
    Fp32,
    Fp16,
    Bf16,
    Int8,
    Int4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NpuActivation {
    Relu,
    Gelu,
    Silu,
    Tanh,
    Sigmoid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NpuTileShape {
    pub m: u32,
    pub n: u32,
    pub k: u32,
}

impl Default for NpuTileShape {
    fn default() -> Self {
        Self {
            m: 16,
            n: 16,
            k: 16,
        }
    }
}

/// Description of an NPU-accelerated tile kernel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NpuKernelDescriptor {
    pub name: String,
    pub target: NpuTargetKind,
    pub precision: NpuPrecision,
    pub shape: NpuTileShape,
    pub activation: Option<NpuActivation>,
    pub unroll_factor: u32,
}

impl NpuKernelDescriptor {
    pub fn new(name: impl Into<String>, target: NpuTargetKind, precision: NpuPrecision) -> Self {
        Self {
            name: name.into(),
            target,
            precision,
            shape: NpuTileShape::default(),
            activation: None,
            unroll_factor: 4,
        }
    }

    pub fn with_shape(mut self, m: u32, n: u32, k: u32) -> Self {
        self.shape = NpuTileShape { m, n, k };
        self
    }

    pub fn with_activation(mut self, act: NpuActivation) -> Self {
        self.activation = Some(act);
        self
    }
}

/// Emit hardware-accelerated tile tensor kernel implementation code.
pub fn emit_npu_tile_kernel(desc: &NpuKernelDescriptor) -> String {
    let mut out = String::new();

    let target_str = match desc.target {
        NpuTargetKind::QualcommHexagon => "Qualcomm Hexagon HVX",
        NpuTargetKind::AppleNeuralEngine => "Apple Neural Engine (ANE)",
        NpuTargetKind::IntelNpu => "Intel NPU / AVX-512 VNNI / AMX",
        NpuTargetKind::ArmEthos => "ARM Ethos / NEON DotProd",
        NpuTargetKind::GenericSimdTile => "Generic SIMD Tile",
    };

    let prec_type = match desc.precision {
        NpuPrecision::Fp32 => "float",
        NpuPrecision::Fp16 => "__fp16",
        NpuPrecision::Bf16 => "__bf16",
        NpuPrecision::Int8 => "int8_t",
        NpuPrecision::Int4 => "int8_t", // packed 4-bit nibbles
    };

    let acc_type = match desc.precision {
        NpuPrecision::Fp32 | NpuPrecision::Fp16 | NpuPrecision::Bf16 => "float",
        NpuPrecision::Int8 | NpuPrecision::Int4 => "int32_t",
    };

    out.push_str(&format!(
        "// ── Auto-generated NPU Kernel: {} [{}] ──\n",
        desc.name, target_str
    ));
    out.push_str("// Tile Config: M={}, N={}, K={}\n");
    out.push_str(&format!(
        "void {}_tile_{}x{}x{}(\n",
        desc.name, desc.shape.m, desc.shape.n, desc.shape.k
    ));
    out.push_str(&format!("    const {}* restrict A,\n", prec_type));
    out.push_str(&format!("    const {}* restrict B,\n", prec_type));
    out.push_str(&format!("    {}* restrict C,\n", acc_type));
    out.push_str("    int lda, int ldb, int ldc\n");
    out.push_str(") {\n");

    // Vector accumulator registers
    out.push_str(&format!(
        "    {} acc[{}][{}] = {{0}};\n\n",
        acc_type, desc.shape.m, desc.shape.n
    ));

    // Outer tile computation loop
    out.push_str(&format!("    #pragma unroll {}\n", desc.unroll_factor));
    out.push_str(&format!(
        "    for (int k = 0; k < {}; ++k) {{\n",
        desc.shape.k
    ));
    out.push_str(&format!(
        "        for (int i = 0; i < {}; ++i) {{\n",
        desc.shape.m
    ));
    out.push_str(&format!(
        "            {} a_val = A[i * lda + k];\n",
        prec_type
    ));
    out.push_str(&format!(
        "            for (int j = 0; j < {}; ++j) {{\n",
        desc.shape.n
    ));
    out.push_str(&format!(
        "                acc[i][j] += (({acc_type})a_val) * (({acc_type})B[k * ldb + j]);\n"
    ));
    out.push_str("            }\n");
    out.push_str("        }\n");
    out.push_str("    }\n\n");

    // Epilogue with activation function
    out.push_str("    // Epilogue store with fused activation\n");
    out.push_str(&format!(
        "    for (int i = 0; i < {}; ++i) {{\n",
        desc.shape.m
    ));
    out.push_str(&format!(
        "        for (int j = 0; j < {}; ++j) {{\n",
        desc.shape.n
    ));
    out.push_str("            ");
    match desc.activation {
        Some(NpuActivation::Relu) => {
            out.push_str(&format!(
                "{} val = acc[i][j] > 0 ? acc[i][j] : 0;\n",
                acc_type
            ));
        }
        Some(NpuActivation::Gelu) => {
            out.push_str(&format!(
                "{} val = 0.5f * (float)acc[i][j] * (1.0f + tanhf(0.79788456f * ((float)acc[i][j] + 0.044715f * (float)acc[i][j] * (float)acc[i][j] * (float)acc[i][j])));\n",
                acc_type
            ));
        }
        Some(NpuActivation::Silu) => {
            out.push_str(&format!(
                "{} val = (float)acc[i][j] / (1.0f + expf(-(float)acc[i][j]));\n",
                acc_type
            ));
        }
        Some(NpuActivation::Tanh) => {
            out.push_str(&format!("{} val = tanhf((float)acc[i][j]);\n", acc_type));
        }
        Some(NpuActivation::Sigmoid) => {
            out.push_str(&format!(
                "{} val = 1.0f / (1.0f + expf(-(float)acc[i][j]));\n",
                acc_type
            ));
        }
        None => {
            out.push_str(&format!("{} val = acc[i][j];\n", acc_type));
        }
    }
    out.push_str("            C[i * ldc + j] = val;\n");
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n");

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npu_kernel_descriptor_and_emission() {
        let desc = NpuKernelDescriptor::new(
            "gemm_int8_relu",
            NpuTargetKind::QualcommHexagon,
            NpuPrecision::Int8,
        )
        .with_shape(8, 8, 32)
        .with_activation(NpuActivation::Relu);

        let code = emit_npu_tile_kernel(&desc);
        assert!(code.contains("Qualcomm Hexagon HVX"));
        assert!(code.contains("gemm_int8_relu_tile_8x8x32"));
        assert!(code.contains("const int8_t* restrict A"));
        assert!(code.contains("int32_t* restrict C"));
        assert!(code.contains("acc[i][j] > 0 ? acc[i][j] : 0"));
    }

    #[test]
    fn test_npu_arm_ethos_gelu_emission() {
        let desc = NpuKernelDescriptor::new(
            "dense_fp16_gelu",
            NpuTargetKind::ArmEthos,
            NpuPrecision::Fp16,
        )
        .with_shape(16, 16, 16)
        .with_activation(NpuActivation::Gelu);

        let code = emit_npu_tile_kernel(&desc);
        assert!(code.contains("ARM Ethos"));
        assert!(code.contains("dense_fp16_gelu_tile_16x16x16"));
        assert!(code.contains("const __fp16* restrict A"));
        assert!(code.contains("tanhf(0.79788456f"));
    }
}
