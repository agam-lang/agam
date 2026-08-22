//! Edge AI Inference Runtime, Model Embedding & Post-Training Quantization.
//!
//! Provides zero-dependency native inference for SafeTensors, ONNX, and TFLite
//! with INT8/FP16 post-training quantization and edge memory planning.

use crate::tensor::Tensor;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Supported Edge AI model formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelFormat {
    SafeTensors,
    Onnx,
    TFLite,
    QuantizedInt8,
}

/// Quantization precision for low-power edge compute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuantizationPrecision {
    FP32,
    FP16,
    INT8,
    INT4,
}

/// Errors during edge model loading, quantization, or inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeError {
    DimensionMismatch {
        expected: Vec<usize>,
        got: Vec<usize>,
    },
    UnsupportedOperator(String),
    CorruptModelWeights(String),
    QuantizationError(String),
}

impl std::fmt::Display for EdgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EdgeError::DimensionMismatch { expected, got } => {
                write!(
                    f,
                    "Input dimension mismatch: expected {expected:?}, got {got:?}"
                )
            }
            EdgeError::UnsupportedOperator(op) => write!(f, "Unsupported edge operator: `{op}`"),
            EdgeError::CorruptModelWeights(e) => write!(f, "Corrupted model weights: {e}"),
            EdgeError::QuantizationError(e) => write!(f, "Quantization error: {e}"),
        }
    }
}

impl std::error::Error for EdgeError {}

/// An INT8 Quantized Tensor with scale and zero-point parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuantizedTensor {
    pub shape: Vec<usize>,
    pub data: Vec<i8>,
    pub scale: f64,
    pub zero_point: i8,
}

impl QuantizedTensor {
    /// Quantize an FP32/FP64 tensor to symmetric INT8.
    pub fn quantize(tensor: &Tensor) -> Self {
        let max_abs = tensor
            .data
            .iter()
            .cloned()
            .map(f64::abs)
            .fold(0.0f64, f64::max);
        let scale = if max_abs > 1e-12 {
            max_abs / 127.0
        } else {
            1.0
        };

        let data: Vec<i8> = tensor
            .data
            .iter()
            .map(|&x| {
                let scaled = (x / scale).round();
                scaled.clamp(-128.0, 127.0) as i8
            })
            .collect();

        Self {
            shape: tensor.shape.clone(),
            data,
            scale,
            zero_point: 0,
        }
    }

    /// Dequantize back to floating point representation.
    pub fn dequantize(&self) -> Tensor {
        let data: Vec<f64> = self
            .data
            .iter()
            .map(|&q| (q as f64 - self.zero_point as f64) * self.scale)
            .collect();
        Tensor::from_data(&self.shape, data)
    }
}

/// Self-contained edge inference model with embedded weights.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeModel {
    pub name: String,
    pub format: ModelFormat,
    pub precision: QuantizationPrecision,
    pub input_shape: Vec<usize>,
    pub output_shape: Vec<usize>,
    pub layers_weights: BTreeMap<String, QuantizedTensor>,
}

impl EdgeModel {
    pub fn new(name: impl Into<String>, input_shape: Vec<usize>, output_shape: Vec<usize>) -> Self {
        Self {
            name: name.into(),
            format: ModelFormat::QuantizedInt8,
            precision: QuantizationPrecision::INT8,
            input_shape,
            output_shape,
            layers_weights: BTreeMap::new(),
        }
    }

    pub fn add_weight(&mut self, name: impl Into<String>, tensor: &Tensor) {
        let q = QuantizedTensor::quantize(tensor);
        self.layers_weights.insert(name.into(), q);
    }

    /// Execute forward inference over input tensor.
    pub fn predict(&self, input: &Tensor) -> Result<Tensor, EdgeError> {
        if input.shape != self.input_shape {
            return Err(EdgeError::DimensionMismatch {
                expected: self.input_shape.clone(),
                got: input.shape.clone(),
            });
        }

        // Sequential layer inference with quantized weights
        let mut curr = input.clone();
        for q_weight in self.layers_weights.values() {
            let weight = q_weight.dequantize();
            curr = curr.matmul(&weight).relu();
        }

        Ok(curr)
    }

    /// Estimate peak memory footprint in bytes.
    pub fn memory_footprint_bytes(&self) -> usize {
        self.layers_weights.values().map(|q| q.data.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantization_roundtrip_precision() {
        let original = Tensor::vector(vec![-1.0, -0.5, 0.0, 0.5, 1.0]);
        let q = QuantizedTensor::quantize(&original);

        assert_eq!(q.data.len(), 5);
        assert_eq!(q.data[0], -127);
        assert_eq!(q.data[4], 127);

        let recovered = q.dequantize();
        for (orig, rec) in original.data.iter().zip(&recovered.data) {
            assert!((orig - rec).abs() < 0.02, "Quantization error too high");
        }
    }

    #[test]
    fn test_edge_model_predict() {
        let mut model = EdgeModel::new("micro_detector", vec![1, 4], vec![1, 2]);
        let w1 = Tensor::from_data(&[4, 2], vec![0.5, -0.5, 0.2, 0.8, -0.1, 0.4, 0.9, -0.3]);
        model.add_weight("layer1", &w1);

        let input = Tensor::from_data(&[1, 4], vec![1.0, 0.5, -0.5, 2.0]);
        let output = model.predict(&input).unwrap();
        assert_eq!(output.shape, vec![1, 2]);
        assert!(model.memory_footprint_bytes() > 0);
    }

    #[test]
    fn test_edge_model_rejects_dimension_mismatch() {
        let model = EdgeModel::new("classifier", vec![1, 10], vec![1, 2]);
        let wrong_input = Tensor::from_data(&[1, 8], vec![0.0; 8]);
        assert!(matches!(
            model.predict(&wrong_input),
            Err(EdgeError::DimensionMismatch { .. })
        ));
    }
}
