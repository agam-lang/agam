# Chapter 22: First-Class Tensors & Numerical AI Operations

> **Part VI: The Agam Language Programming Guide**  
> **Target Audience**: AI / ML Engineers and Numerical Computing Developers (Advanced Level)

---

## 22.1 First-Class Tensor Primitives

In Agam, multi-dimensional numerical arrays (`Tensor`) are native primitives integrated into the syntax and backend compiler code generator (`agam_codegen`).

```agam
fn main() {
    // 2D Matrix Creation
    let A: Tensor[Float, 2x3] = Tensor.from_array([
        [1.0, 2.0, 3.0],
        [4.0, 5.0, 6.0]
    ]);

    let B: Tensor[Float, 3x2] = Tensor.from_array([
        [7.0, 8.0],
        [9.0, 1.0],
        [2.0, 3.0]
    ]);

    // Matrix Multiplication compiled directly to SIMD / BLAS kernels
    let C: Tensor[Float, 2x2] = A * B;

    println("Result Matrix shape: " + C.shape().to_string());
}
```

---

## 22.2 Tensor Broadcasting & Arithmetic

Agam supports element-wise mathematical operations with automatic shape broadcasting:

```agam
fn main() {
    let X = Tensor.ones([4, 4]); // 4x4 matrix of 1.0s
    let bias = Tensor.from_array([0.5, 1.0, 1.5, 2.0]); // 1x4 vector

    // Automatic broadcasting across rows
    let Y = X + bias; 
    let Z = Tensor.relu(Y); // Native Rectified Linear Unit activation
}
```

---

## 22.3 Neural Network Layer Construction

```agam
struct LinearLayer {
    weights: Tensor[Float],
    bias: Tensor[Float],
}

impl LinearLayer {
    fn new(in_features: Int, out_features: Int) -> LinearLayer {
        return LinearLayer {
            weights: Tensor.random([in_features, out_features]),
            bias: Tensor.zeros([out_features]),
        };
    }

    fn forward(self, input: Tensor[Float]) -> Tensor[Float] {
        return (input * self.weights) + self.bias;
    }
}
```
