# Chapter 25: Metaprogramming, Compile-Time Evaluation & Developer Tooling

> **Part VI: The Agam Language Programming Guide**  
> **Target Audience**: Advanced Developers & Language Tool Authors  
> **Compiler Module Focus**: [`agam_macro`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/experiments/agam_macro), [`agam_mir::eval`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/middle/agam_mir), [`agam_driver`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/tooling/agam_driver)

---

## 25.1 Declarative Macros (`macro_rules!`)

Declarative macros provide pattern-matching-based code generation at compile time. They match syntactic patterns against input tokens and expand into Agam source code:

```agam
// Define a declarative macro for creating test assertions
macro_rules! assert_eq {
    ($left:expr, $right:expr) => {
        if $left != $right {
            panic("Assertion failed: " + $left.to_string()
                  + " != " + $right.to_string());
        }
    };
    ($left:expr, $right:expr, $msg:expr) => {
        if $left != $right {
            panic($msg + ": " + $left.to_string()
                  + " != " + $right.to_string());
        }
    };
}

// Usage — expanded at compile time
assert_eq!(compute_factorial(5), 120);
assert_eq!(fibonacci(10), 55, "Fibonacci check");
```

### Pattern Fragment Types

| Fragment | Syntax | Matches |
| :--- | :--- | :--- |
| `$name:expr` | Any expression | `x + 1`, `foo()`, `42` |
| `$name:ty` | Any type | `Int`, `Vec[String]`, `Tensor[Float, 2x3]` |
| `$name:ident` | An identifier | `foo`, `my_var` |
| `$name:stmt` | A statement | `let x = 5;` |
| `$name:block` | A block expression | `{ x + 1 }` |
| `$($name:expr),*` | Repeated comma-separated | `1, 2, 3` |

### Repetition Expansion

```agam
// Macro for creating a vector from literal values
macro_rules! vec_of {
    ($($elem:expr),* $(,)?) => {
        {
            let mut v = Vec.new();
            $(v.push($elem);)*
            v
        }
    };
}

let numbers = vec_of![1, 2, 3, 4, 5];
```

---

## 25.2 Procedural Derive Macros (`@derive`)

Procedural macros operate on the AST representation of a type definition and generate new `impl` blocks automatically. Agam ships four built-in derive macros:

```agam
@derive(Debug, Clone, PartialEq, Default)
struct Config {
    name: String,
    max_retries: Int,
    timeout_ms: Float,
    enabled: Bool,
}

// The @derive annotation automatically generates:
//
// impl Debug for Config {
//     fn debug_fmt(self) -> String { ... }
// }
//
// impl Clone for Config {
//     fn clone(self) -> Config { ... }
// }
//
// impl PartialEq for Config {
//     fn eq(self, other: Config) -> Bool { ... }
// }
//
// impl Default for Config {
//     fn default() -> Config {
//         Config { name: "", max_retries: 0, timeout_ms: 0.0, enabled: false }
//     }
// }
```

### How Procedural Macros Work Internally

```text
Source AST (struct Config { ... })
       │
       ▼
  agam_macro::derive_expand()
       │
       ├── Inspects struct field names and types
       ├── Generates impl block AST nodes
       └── Returns expanded AST fragments
       │
       ▼
  Merged back into the main AST before type checking
```

The macro system operates *before* semantic analysis, ensuring all generated code passes the same type checking as hand-written code.

---

## 25.3 Compile-Time Function Evaluation (`@comptime`)

The `@comptime` annotation forces an expression or block to be fully evaluated during compilation. The result is embedded as a literal constant in the compiled binary:

```agam
// Compile-time constant computation
const TABLE_SIZE: Int = @comptime { 1 << 16 };  // 65536

// Compile-time lookup table generation
const SIN_TABLE: [Float; 360] = @comptime {
    let mut table: [Float; 360] = [0.0; 360];
    for i in 0..360 {
        table[i] = sin(i.to_float() * 3.14159265 / 180.0);
    }
    table
};

// Usage at runtime — zero computation cost, table is pre-baked
fn fast_sin(degrees: Int) -> Float {
    return SIN_TABLE[degrees % 360];
}
```

### Compile-Time Evaluation Engine (`agam_mir::eval`)

The `@comptime` evaluator is a **deterministic MIR interpreter** that executes a subset of Agam at compile time:

**Supported operations:**
- All arithmetic, logical, and comparison operations
- Array and struct construction and field access
- `for`/`while` loops with known bounds
- Pure function calls (no I/O, no allocation, no effects)
- Pattern matching and conditional branching

**Rejected operations (compile-time error):**
- Heap allocation (`Vec.new()`, `String` concatenation beyond literals)
- I/O operations (`println`, file access)
- Effect performance (`perform`)
- Unbounded recursion (enforced by iteration limit)

```text
error[E0701]: operation not permitted at compile time
  ┌─ src/main.agam:3:5
  │
3 │     println("hello");
  │     ^^^^^^^^^^^^^^^^ I/O is not available during @comptime evaluation
  │
  = reason: compile-time execution must be pure and deterministic
```

---

## 25.4 Embedded Domain-Specific Languages

The macro system enables embedded DSLs for specialized domains:

### Neural Network DSL (`@nn`)

```agam
let model = @nn {
    Linear(784, 256),
    ReLU(),
    Dropout(0.2),
    Linear(256, 128),
    ReLU(),
    Linear(128, 10),
    Softmax(),
};

let output = model.forward(input_batch);
```

The `@nn` macro expands into a chain of struct instantiations and a generated `forward()` method that sequentially applies each layer.

---

## 25.5 Interactive JIT REPL (`agamc repl`)

The REPL provides an interactive evaluation environment backed by the Cranelift JIT engine:

```bash
$ agamc repl
Agam v0.1.0 Interactive REPL
>>> let x = 42
x: Int = 42
>>> x * 2
84
>>> struct Point { x: Float, y: Float }
>>> let p = Point { x: 3.0, y: 4.0 }
p: Point = Point { x: 3.0, y: 4.0 }
>>> p.x * p.x + p.y * p.y
25.0
>>> fn fib(n: Int) -> Int => if n <= 1 { n } else { fib(n-1) + fib(n-2) };
>>> fib(20)
6765
```

### REPL Architecture

```text
User Input (text line)
      │
      ▼
  agam_lexer → agam_parser → agam_sema
      │
      ▼
  agam_mir (generate SSA for expression)
      │
      ▼
  agam_jit (Cranelift compile + execute)
      │
      ▼
  Display result + update REPL environment state
```

**Key features:**
- **Persistent environment:** Variables and function definitions persist across REPL lines
- **Incremental compilation:** Only the new expression is compiled; previous definitions remain in JIT memory
- **Multi-line input:** Opening braces `{` trigger multi-line mode until the matching `}` is entered
- **Tab completion:** Identifier names from the current scope are available for tab completion

---

## 25.6 Headless Agent Execution (`agamc exec`)

The `agamc exec` command provides sandboxed, JSON-structured execution for AI agent workflows:

```bash
# Execute Agam code with strict resource limits
agamc exec --json '{
    "source": "println(40 + 2)",
    "memory_limit_mb": 512,
    "timeout_seconds": 30,
    "max_output_bytes": 65536
}'

# Output (JSON stream on stdout)
{"type": "stdout", "data": "42\n"}
{"type": "exit", "code": 0, "duration_ms": 12}
```

### Security Model

All `agamc exec` invocations run inside the **Chāṇakya Durdharṣa** sandbox:
- **Windows:** Process runs inside a Win32 `JobObject` with memory, CPU, and child process limits
- **Linux:** `prctl(PR_SET_NO_NEW_PRIVS)` + `setrlimit()` for memory, file descriptors, and wall-clock timeout
- **Filesystem:** Read-only access to standard library; no write access to host filesystem
- **Network:** All network access is blocked by default

---

## 25.7 Source Code Formatter (`agamc fmt`)

```bash
# Format a single file
agamc fmt src/main.agam

# Format all .agam files in the project
agamc fmt --all

# Check formatting without modifying files (CI mode)
agamc fmt --check src/main.agam
```

The formatter (`agam_fmt`) operates on the **Concrete Syntax Tree (CST)** rather than the AST, preserving comments, blank lines, and documentation annotations while normalizing indentation, brace placement, and expression spacing.

---

## 25.8 Static Linter (`agamc lint`)

```bash
# Run all lint rules
agamc lint src/main.agam

# Run specific lint categories
agamc lint --category performance src/
agamc lint --category style src/
```

Lint categories:
| Category | Example Rules |
| :--- | :--- |
| **correctness** | Unused variables, unreachable code, shadowed imports |
| **performance** | Unnecessary cloning, allocation in hot loops, missing `@inline` |
| **style** | Naming conventions, documentation coverage, import ordering |
| **complexity** | Function too long (>100 lines), nesting depth >5, cyclomatic complexity |

---

## 25.9 Documentation Generator (`agamc doc`)

```bash
# Generate HTML docs for the current project
agamc doc --open

# Generate docs including private items
agamc doc --document-private-items
```

The documentation engine (`agam_doc`) parses `///` doc comments, resolves cross-references to types and functions, and renders a searchable HTML site with:
- Type signature display with syntax highlighting
- Cross-linked symbol references
- Module hierarchy navigation
- Full-text search index
