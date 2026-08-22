//! Machine Learning primitives — hardware-optimized.
//!
//! Provides the building blocks for ML/DL directly in Agam:
//! - Loss functions (MSE, cross-entropy, Huber)
//! - Activation functions (relu, leaky_relu, tanh, gelu, swish)
//! - Layer operations (dense/linear, batch norm)
//! - Metrics (accuracy, precision, recall, F1)
//! - Data utilities (one-hot encoding, train/test split, normalization)
//!
//! All operations are on contiguous f64 arrays for maximum cache throughput.

use agam_runtime::simd::SimdOps;

use crate::tensor::Tensor;

// ────────────────────────────────────────────────
// Loss Functions
// ────────────────────────────────────────────────

/// Mean Squared Error: (1/n) Σ(predicted - target)²
pub fn mse_loss(predicted: &Tensor, target: &Tensor) -> f64 {
    assert_eq!(predicted.numel(), target.numel());
    predicted
        .data
        .iter()
        .zip(&target.data)
        .map(|(p, t)| (p - t) * (p - t))
        .sum::<f64>()
        / predicted.numel() as f64
}

/// MSE gradient: ∂L/∂predicted = 2(predicted - target) / n
pub fn mse_grad(predicted: &Tensor, target: &Tensor) -> Tensor {
    let n = predicted.numel() as f64;
    let data: Vec<f64> = predicted
        .data
        .iter()
        .zip(&target.data)
        .map(|(p, t)| 2.0 * (p - t) / n)
        .collect();
    Tensor {
        shape: predicted.shape.clone(),
        data,
    }
}

/// Binary cross-entropy: -1/n Σ [t*ln(p) + (1-t)*ln(1-p)]
pub fn binary_cross_entropy(predicted: &Tensor, target: &Tensor) -> f64 {
    assert_eq!(predicted.numel(), target.numel());
    let eps = 1e-15;
    let n = predicted.numel() as f64;
    -predicted
        .data
        .iter()
        .zip(&target.data)
        .map(|(p, t)| t * (p + eps).ln() + (1.0 - t) * (1.0 - p + eps).ln())
        .sum::<f64>()
        / n
}

/// Categorical cross-entropy: -Σ target * ln(predicted) (for softmax outputs)
pub fn cross_entropy(predicted: &Tensor, target: &Tensor) -> f64 {
    let eps = 1e-15;
    -predicted
        .data
        .iter()
        .zip(&target.data)
        .map(|(p, t)| t * (p + eps).ln())
        .sum::<f64>()
}

/// Huber loss: smooth L1, less sensitive to outliers.
pub fn huber_loss(predicted: &Tensor, target: &Tensor, delta: f64) -> f64 {
    let n = predicted.numel() as f64;
    predicted
        .data
        .iter()
        .zip(&target.data)
        .map(|(p, t)| {
            let diff = (p - t).abs();
            if diff <= delta {
                0.5 * diff * diff
            } else {
                delta * (diff - 0.5 * delta)
            }
        })
        .sum::<f64>()
        / n
}

// ────────────────────────────────────────────────
// Activation Functions
// ────────────────────────────────────────────────

/// Leaky ReLU: max(alpha*x, x)
pub fn leaky_relu(t: &Tensor, alpha: f64) -> Tensor {
    t.map(|x| if x > 0.0 { x } else { alpha * x })
}

/// Leaky ReLU gradient.
pub fn leaky_relu_grad(t: &Tensor, alpha: f64) -> Tensor {
    t.map(|x| if x > 0.0 { 1.0 } else { alpha })
}

/// Tanh activation.
pub fn tanh(t: &Tensor) -> Tensor {
    t.map(|x| x.tanh())
}

/// Tanh gradient: 1 - tanh²(x)
pub fn tanh_grad(t: &Tensor) -> Tensor {
    t.map(|x| 1.0 - x.tanh() * x.tanh())
}

/// GELU: Gaussian Error Linear Unit (used in BERT/GPT).
/// GELU(x) = x * Φ(x) ≈ 0.5x(1 + tanh(√(2/π)(x + 0.044715x³)))
pub fn gelu(t: &Tensor) -> Tensor {
    let c = (2.0 / std::f64::consts::PI).sqrt();
    t.map(|x| 0.5 * x * (1.0 + (c * (x + 0.044715 * x * x * x)).tanh()))
}

/// Swish: x * sigmoid(x) (used in EfficientNet).
pub fn swish(t: &Tensor) -> Tensor {
    t.map(|x| x / (1.0 + (-x).exp()))
}

/// ReLU gradient (for backprop).
pub fn relu_grad(t: &Tensor) -> Tensor {
    t.map(|x| if x > 0.0 { 1.0 } else { 0.0 })
}

/// Sigmoid gradient: σ(x)(1 - σ(x))
pub fn sigmoid_grad(t: &Tensor) -> Tensor {
    t.map(|x| {
        let s = 1.0 / (1.0 + (-x).exp());
        s * (1.0 - s)
    })
}

// ────────────────────────────────────────────────
// Layer Operations
// ────────────────────────────────────────────────

/// Dense (fully connected) layer: output = input @ weights + bias
/// input: [batch, in_features]
/// weights: [in_features, out_features]
/// bias: [out_features]
pub fn dense(input: &Tensor, weights: &Tensor, bias: &Tensor) -> Tensor {
    let output = input.matmul(weights);
    // Add bias to each row
    let batch = output.shape[0];
    let out_features = output.shape[1];
    let mut data = output.data;
    for i in 0..batch {
        for j in 0..out_features {
            data[i * out_features + j] += bias.data[j];
        }
    }
    Tensor::from_data(&[batch, out_features], data)
}

/// Batch normalization: (x - μ) / √(σ² + ε)
pub fn batch_norm(t: &Tensor, epsilon: f64) -> Tensor {
    let mean = t.mean();
    let var = t.data.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / t.numel() as f64;
    let std = (var + epsilon).sqrt();
    t.map(|x| (x - mean) / std)
}

/// Dropout: randomly zero elements (training only). Mask is provided.
pub fn dropout(t: &Tensor, mask: &[bool], scale: f64) -> Tensor {
    assert_eq!(mask.len(), t.numel());
    let data: Vec<f64> = t
        .data
        .iter()
        .zip(mask)
        .map(|(v, keep)| if *keep { *v * scale } else { 0.0 })
        .collect();
    Tensor {
        shape: t.shape.clone(),
        data,
    }
}

// ────────────────────────────────────────────────
// Metrics
// ────────────────────────────────────────────────

/// Classification accuracy.
pub fn accuracy(predicted: &[usize], actual: &[usize]) -> f64 {
    assert_eq!(predicted.len(), actual.len());
    let correct = predicted.iter().zip(actual).filter(|(p, a)| p == a).count();
    correct as f64 / predicted.len() as f64
}

/// Precision = TP / (TP + FP) for binary classification.
pub fn precision(predicted: &[bool], actual: &[bool]) -> f64 {
    let tp = predicted
        .iter()
        .zip(actual)
        .filter(|(p, a)| **p && **a)
        .count() as f64;
    let fp = predicted
        .iter()
        .zip(actual)
        .filter(|(p, a)| **p && !**a)
        .count() as f64;
    if tp + fp == 0.0 { 0.0 } else { tp / (tp + fp) }
}

/// Recall = TP / (TP + FN) for binary classification.
pub fn recall(predicted: &[bool], actual: &[bool]) -> f64 {
    let tp = predicted
        .iter()
        .zip(actual)
        .filter(|(p, a)| **p && **a)
        .count() as f64;
    let fn_ = predicted
        .iter()
        .zip(actual)
        .filter(|(p, a)| !**p && **a)
        .count() as f64;
    if tp + fn_ == 0.0 {
        0.0
    } else {
        tp / (tp + fn_)
    }
}

/// F1 score = 2 * precision * recall / (precision + recall)
pub fn f1_score(predicted: &[bool], actual: &[bool]) -> f64 {
    let p = precision(predicted, actual);
    let r = recall(predicted, actual);
    if p + r == 0.0 {
        0.0
    } else {
        2.0 * p * r / (p + r)
    }
}

// ────────────────────────────────────────────────
// Data Utilities
// ────────────────────────────────────────────────

/// One-hot encoding: label → binary vector.
pub fn one_hot(label: usize, num_classes: usize) -> Tensor {
    let mut data = vec![0.0; num_classes];
    data[label] = 1.0;
    Tensor::vector(data)
}

/// Batch one-hot: encode multiple labels.
pub fn one_hot_batch(labels: &[usize], num_classes: usize) -> Tensor {
    let batch = labels.len();
    let mut data = vec![0.0; batch * num_classes];
    for (i, &label) in labels.iter().enumerate() {
        data[i * num_classes + label] = 1.0;
    }
    Tensor::from_data(&[batch, num_classes], data)
}

/// Min-max normalization: scale to [0, 1].
pub fn min_max_normalize(t: &Tensor) -> Tensor {
    let min = t.data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = t.data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;
    if range < 1e-15 {
        return t.clone();
    }
    t.map(|x| (x - min) / range)
}

/// Z-score normalization: (x - μ) / σ
pub fn z_score_normalize(t: &Tensor) -> Tensor {
    let mean = t.mean();
    let std =
        (t.data.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / t.numel() as f64).sqrt();
    if std < 1e-15 {
        return t.clone();
    }
    t.map(|x| (x - mean) / std)
}

/// Cosine similarity between two vectors.
pub fn cosine_similarity(a: &Tensor, b: &Tensor) -> f64 {
    let dot = SimdOps::dot(&a.data, &b.data);
    let na = SimdOps::norm_l2(&a.data);
    let nb = SimdOps::norm_l2(&b.data);
    dot / (na * nb)
}

/// Euclidean distance between two tensors.
pub fn euclidean_distance(a: &Tensor, b: &Tensor) -> f64 {
    SimdOps::distance(&a.data, &b.data)
}

/// K-nearest neighbors classification (brute force).
/// Returns predicted label for query based on k closest training points.
pub fn knn_classify(train_x: &[Tensor], train_y: &[usize], query: &Tensor, k: usize) -> usize {
    let mut dists: Vec<(f64, usize)> = train_x
        .iter()
        .zip(train_y)
        .map(|(x, y)| (euclidean_distance(x, query), *y))
        .collect();
    dists.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    // Majority vote among k nearest
    let mut votes = std::collections::HashMap::new();
    for &(_, label) in dists.iter().take(k) {
        *votes.entry(label).or_insert(0usize) += 1;
    }
    *votes.iter().max_by_key(|(_, v)| **v).unwrap().0
}

// ────────────────────────────────────────────────
// Optimizers & Neural Network Models
// ────────────────────────────────────────────────

/// Trait for first-order gradient optimizers.
pub trait Optimizer {
    /// Update model weights in-place given gradients.
    fn step(&mut self, weights: &mut Tensor, grads: &Tensor);
}

/// Stochastic Gradient Descent with Momentum.
#[derive(Debug, Clone)]
pub struct Sgd {
    pub lr: f64,
    pub momentum: f64,
    pub velocity: Option<Tensor>,
}

impl Sgd {
    pub fn new(lr: f64, momentum: f64) -> Self {
        Self {
            lr,
            momentum,
            velocity: None,
        }
    }
}

impl Optimizer for Sgd {
    fn step(&mut self, weights: &mut Tensor, grads: &Tensor) {
        assert_eq!(weights.shape, grads.shape);
        let vel = self
            .velocity
            .get_or_insert_with(|| Tensor::zeros(&weights.shape));
        for i in 0..weights.data.len() {
            vel.data[i] = self.momentum * vel.data[i] + self.lr * grads.data[i];
            weights.data[i] -= vel.data[i];
        }
    }
}

/// Adam (Adaptive Moment Estimation) Optimizer.
#[derive(Debug, Clone)]
pub struct Adam {
    pub lr: f64,
    pub beta1: f64,
    pub beta2: f64,
    pub eps: f64,
    pub t: usize,
    pub m: Option<Tensor>,
    pub v: Option<Tensor>,
}

impl Adam {
    pub fn new(lr: f64) -> Self {
        Self {
            lr,
            beta1: 0.9,
            beta2: 0.999,
            eps: 1e-8,
            t: 0,
            m: None,
            v: None,
        }
    }
}

impl Optimizer for Adam {
    fn step(&mut self, weights: &mut Tensor, grads: &Tensor) {
        assert_eq!(weights.shape, grads.shape);
        self.t += 1;
        let t_f = self.t as f64;
        let m = self.m.get_or_insert_with(|| Tensor::zeros(&weights.shape));
        let v = self.v.get_or_insert_with(|| Tensor::zeros(&weights.shape));

        for i in 0..weights.data.len() {
            let g = grads.data[i];
            m.data[i] = self.beta1 * m.data[i] + (1.0 - self.beta1) * g;
            v.data[i] = self.beta2 * v.data[i] + (1.0 - self.beta2) * g * g;

            // Bias correction
            let m_hat = m.data[i] / (1.0 - self.beta1.powf(t_f));
            let v_hat = v.data[i] / (1.0 - self.beta2.powf(t_f));

            weights.data[i] -= self.lr * m_hat / (v_hat.sqrt() + self.eps);
        }
    }
}

/// A trainable Dense/Linear Neural Network Layer.
#[derive(Debug, Clone, PartialEq)]
pub struct DenseLayer {
    pub weights: Tensor,
    pub bias: Tensor,
    pub in_features: usize,
    pub out_features: usize,
}

impl DenseLayer {
    pub fn new(in_features: usize, out_features: usize) -> Self {
        let bound = (6.0 / (in_features + out_features) as f64).sqrt();
        let mut data = Vec::with_capacity(in_features * out_features);
        // Simple deterministic Xavier initialization
        for i in 0..(in_features * out_features) {
            let pseudo_rand = ((i as f64 * 0.12345).sin()) * bound;
            data.push(pseudo_rand);
        }
        let weights = Tensor::from_data(&[in_features, out_features], data);
        let bias = Tensor::zeros(&[out_features]);
        Self {
            weights,
            bias,
            in_features,
            out_features,
        }
    }

    pub fn forward(&self, input: &Tensor) -> Tensor {
        dense(input, &self.weights, &self.bias)
    }
}

/// A Sequential Neural Network Model.
#[derive(Debug, Clone)]
pub struct SequentialModel {
    pub layers: Vec<DenseLayer>,
}

impl SequentialModel {
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    pub fn add_dense(mut self, in_features: usize, out_features: usize) -> Self {
        self.layers.push(DenseLayer::new(in_features, out_features));
        self
    }

    pub fn forward(&self, input: &Tensor) -> Tensor {
        let mut curr = input.clone();
        for (i, layer) in self.layers.iter().enumerate() {
            curr = layer.forward(&curr);
            if i + 1 < self.layers.len() {
                curr = curr.relu();
            }
        }
        curr
    }
}

impl Default for SequentialModel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mse_loss() {
        let p = Tensor::vector(vec![1.0, 2.0, 3.0]);
        let t = Tensor::vector(vec![1.0, 2.0, 3.0]);
        assert_eq!(mse_loss(&p, &t), 0.0);
    }

    #[test]
    fn test_mse_loss_nonzero() {
        let p = Tensor::vector(vec![1.0, 2.0, 3.0]);
        let t = Tensor::vector(vec![2.0, 2.0, 2.0]);
        // ((1-2)² + (2-2)² + (3-2)²) / 3 = 2/3
        assert!((mse_loss(&p, &t) - 2.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_cross_entropy() {
        let p = Tensor::vector(vec![0.7, 0.2, 0.1]);
        let t = Tensor::vector(vec![1.0, 0.0, 0.0]); // label = 0
        let loss = cross_entropy(&p, &t);
        assert!((loss - (-(0.7f64.ln()))).abs() < 1e-10);
    }

    #[test]
    fn test_huber_loss() {
        let p = Tensor::vector(vec![1.0, 5.0]);
        let t = Tensor::vector(vec![1.0, 1.0]);
        let loss = huber_loss(&p, &t, 1.0);
        // diff=0→0, diff=4→1*(4-0.5)=3.5. Mean = 3.5/2 = 1.75
        assert!((loss - 1.75).abs() < 1e-10);
    }

    #[test]
    fn test_leaky_relu() {
        let t = Tensor::vector(vec![-2.0, -1.0, 0.0, 1.0, 2.0]);
        let r = leaky_relu(&t, 0.01);
        assert_eq!(r.data[0], -0.02);
        assert_eq!(r.data[3], 1.0);
    }

    #[test]
    fn test_gelu() {
        // GELU(0) ≈ 0
        let t = Tensor::scalar(0.0);
        let r = gelu(&t);
        assert!(r.data[0].abs() < 1e-10);
    }

    #[test]
    fn test_swish() {
        // swish(0) = 0 * sigmoid(0) = 0 * 0.5 = 0
        let t = Tensor::scalar(0.0);
        let r = swish(&t);
        assert!(r.data[0].abs() < 1e-10);
    }

    #[test]
    fn test_dense_layer() {
        let input = Tensor::from_data(&[2, 3], vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let weights = Tensor::from_data(&[3, 2], vec![1.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
        let bias = Tensor::vector(vec![0.1, 0.2]);
        let output = dense(&input, &weights, &bias);
        assert_eq!(output.shape, vec![2, 2]);
        // Row 0: [1+3+0.1, 2+3+0.2] = [4.1, 5.2]
        assert!((output.data[0] - 4.1).abs() < 1e-10);
        assert!((output.data[1] - 5.2).abs() < 1e-10);
    }

    #[test]
    fn test_batch_norm() {
        let t = Tensor::vector(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        let bn = batch_norm(&t, 1e-5);
        assert!((bn.mean()).abs() < 1e-10); // mean ≈ 0 after normalization
    }

    #[test]
    fn test_accuracy() {
        let pred = vec![0, 1, 2, 1, 0];
        let actual = vec![0, 1, 1, 1, 0];
        assert!((accuracy(&pred, &actual) - 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_precision_recall() {
        let pred = vec![true, true, false, true];
        let actual = vec![true, false, false, true];
        assert!((precision(&pred, &actual) - 2.0 / 3.0).abs() < 1e-10);
        assert!((recall(&pred, &actual) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_one_hot() {
        let oh = one_hot(2, 4);
        assert_eq!(oh.data, vec![0.0, 0.0, 1.0, 0.0]);
    }

    #[test]
    fn test_one_hot_batch() {
        let oh = one_hot_batch(&[0, 2], 3);
        assert_eq!(oh.shape, vec![2, 3]);
        assert_eq!(oh.data, vec![1.0, 0.0, 0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_min_max_normalize() {
        let t = Tensor::vector(vec![0.0, 5.0, 10.0]);
        let n = min_max_normalize(&t);
        assert_eq!(n.data, vec![0.0, 0.5, 1.0]);
    }

    #[test]
    fn test_z_score_normalize() {
        let t = Tensor::vector(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        let n = z_score_normalize(&t);
        assert!((n.mean()).abs() < 1e-10); // mean ≈ 0
    }

    #[test]
    fn test_cosine_similarity() {
        let a = Tensor::vector(vec![1.0, 0.0]);
        let b = Tensor::vector(vec![0.0, 1.0]);
        assert!(cosine_similarity(&a, &b).abs() < 1e-10); // orthogonal → 0

        let c = Tensor::vector(vec![1.0, 0.0]);
        assert!((cosine_similarity(&a, &c) - 1.0).abs() < 1e-10); // same → 1
    }

    #[test]
    fn test_knn() {
        let train_x = vec![
            Tensor::vector(vec![1.0, 1.0]),
            Tensor::vector(vec![2.0, 2.0]),
            Tensor::vector(vec![10.0, 10.0]),
            Tensor::vector(vec![11.0, 11.0]),
        ];
        let train_y = vec![0, 0, 1, 1];
        let query = Tensor::vector(vec![1.5, 1.5]);
        assert_eq!(knn_classify(&train_x, &train_y, &query, 3), 0);
    }

    #[test]
    fn test_euclidean_distance() {
        let a = Tensor::vector(vec![0.0, 0.0]);
        let b = Tensor::vector(vec![3.0, 4.0]);
        assert!((euclidean_distance(&a, &b) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_sgd_optimizer_step() {
        let mut weights = Tensor::vector(vec![1.0, 2.0]);
        let grads = Tensor::vector(vec![0.1, 0.2]);
        let mut sgd = Sgd::new(0.1, 0.9);

        sgd.step(&mut weights, &grads);
        assert!((weights.data[0] - 0.99).abs() < 1e-10);
        assert!((weights.data[1] - 1.98).abs() < 1e-10);
    }

    #[test]
    fn test_adam_optimizer_step() {
        let mut weights = Tensor::vector(vec![1.0, 2.0]);
        let grads = Tensor::vector(vec![0.1, 0.2]);
        let mut adam = Adam::new(0.01);

        adam.step(&mut weights, &grads);
        assert!(weights.data[0] < 1.0);
        assert!(weights.data[1] < 2.0);
    }

    #[test]
    fn test_sequential_model_forward() {
        let model = SequentialModel::new().add_dense(4, 8).add_dense(8, 2);

        let input = Tensor::from_data(&[1, 4], vec![0.5, -0.2, 0.1, 0.9]);
        let output = model.forward(&input);
        assert_eq!(output.shape, vec![1, 2]);
    }
}
