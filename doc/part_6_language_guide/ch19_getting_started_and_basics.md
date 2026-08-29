# Chapter 19: Getting Started & Basics of Agam

> **Part VI: The Agam Language Programming Guide**  
> **Target Audience**: Software Engineers learning to write code in Agam (Basic Level)

---

## 19.1 Introduction to Agam Programming

Agam is a next-generation compiled programming language designed to combine Python-level readability, Rust-level memory safety, and C/LLVM native execution speed.

Key language characteristics:
- **Static Typing with Local Type Inference**: Strongly typed at compile time without verbose annotations.
- **Native Tensor Operations**: Multi-dimensional numerical arrays are first-class language constructs.
- **Algebraic Effect Handlers**: Structured side-effect management replacing callbacks and complex error hierarchies.
- **Zero-Overhead Memory Safety**: Automatic memory management without a global stop-the-world garbage collector.

---

## 19.2 "Hello, World!" in Agam

Create a file named `hello.agam`:

```agam
fn main() {
    println("Hello, Agam World!");
}
```

### Compiling and Running
Use the `agamc` CLI tool:

```bash
# Build a native standalone binary
agamc build hello.agam

# Run directly
agamc run hello.agam
```

---

## 19.3 Variables, Mutability & Constants

In Agam, variables are declared using `let` and are **immutable by default**. To make a variable mutable, append `mut`:

```agam
fn main() {
    // Immutable variable binding
    let name: String = "Agam";
    let version = 1; // Type inferred as Int

    // Mutable variable binding
    let mut score: Int = 100;
    score = score + 50;

    // Constants (evaluated at compile time)
    const MAX_CONNECTIONS: Int = 1024;

    println("Language: " + name);
    println("Score: " + score.to_string());
}
```

---

## 19.4 Primitive Data Types

Agam supports primitive types:

| Type | Description | Example |
| :--- | :--- | :--- |
| `Int` | 64-bit signed integer | `42`, `-100` |
| `Float` | 64-bit IEEE 754 floating point | `3.14159`, `-0.5` |
| `Bool` | Boolean truth value | `true`, `false` |
| `Char` | 32-bit Unicode scalar character | `'A'`, `'α'` |
| `String` | UTF-8 encoded text string | `"Agam Language"` |
| `Nil` | Unit/empty value | `()` |

---

## 19.5 Functions & Signatures

Functions are declared with `fn`, followed by parameter names, type annotations, and an optional return type (`-> Type`):

```agam
// Function taking arguments and returning a Float
fn calculate_bmi(weight_kg: Float, height_m: Float) -> Float {
    let bmi = weight_kg / (height_m * height_m);
    return bmi;
}

// Single-expression implicit return syntax
fn add(a: Int, b: Int) -> Int => a + b;

fn main() {
    let result = calculate_bmi(70.0, 1.75);
    println("BMI Result: " + result.to_string());
}
```
