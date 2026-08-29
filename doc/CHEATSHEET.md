# Agam Language Syntax Cheat Sheet

*A One-Page Syntax & CLI Quick Reference for Agam Developers*

---

## 1. Variables & Primitive Types

```agam
let x: Int = 42;                 // Immutable integer
let mut count = 0;               // Mutable integer (type inferred)
let ratio: Float = 3.14;         // 64-bit float
let active: Bool = true;         // Boolean
let name: String = "Agam";       // UTF-8 String
const MAX_LIMIT: Int = 1000;     // Compile-time constant
```

---

## 2. Functions & Control Flow

```agam
// Standard function
fn add(a: Int, b: Int) -> Int {
    return a + b;
}

// Implicit expression return syntax
fn square(n: Int) -> Int => n * n;

// Conditionals as expressions
let status = if score >= 50 { "Pass" } else { "Fail" };

// Loops
while count < 10 { count = count + 1; }
for i in 0..5 { println(i.to_string()); }
```

---

## 3. Structs & Methods

```agam
struct Point { x: Float, y: Float }

impl Point {
    fn origin() -> Point => Point { x: 0.0, y: 0.0 };
    fn distance(self) -> Float => (self.x * self.x + self.y * self.y).sqrt();
}
```

---

## 4. Enums & Pattern Matching

```agam
enum Status {
    Idle,
    Processing(percent: Int),
    Error(String),
}

let msg = match status {
    Status.Idle => "System Idle",
    Status.Processing(p) => "Progress: " + p.to_string() + "%",
    Status.Error(err) => "Error: " + err,
};
```

---

## 5. First-Class Tensors

```agam
let A: Tensor[Float, 2x2] = Tensor.from_array([[1.0, 2.0], [3.0, 4.0]]);
let B = Tensor.ones([2, 2]);

let C = A * B;             // Matrix multiplication
let D = Tensor.relu(C);    // Activation function
```

---

## 6. Algebraic Effects

```agam
effect Logger { fn log(msg: String) -> Nil; }

fn compute() {
    perform Logger.log("Computing...");
}

fn main() {
    handle compute() {
        Logger.log(msg) => { println("LOG: " + msg); resume(); }
    }
}
```

---

## 7. CLI Reference (`agamc`)

```bash
agamc build main.agam                  # Build native binary
agamc run main.agam                    # Compile and execute
agamc check main.agam                  # Fast type check
agamc repl                             # Launch interactive JIT REPL
agamc fmt main.agam                    # Format source code
agamc lint main.agam                   # Run static linter
agamc dev                              # Start daemon incremental loop
agamc exec --json '{"source":"..."}'    # Sandboxed headless execution
```
