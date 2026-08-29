# Chapter 1: The C Execution & Memory Model

> **Core Literature Grounding**: *The C Programming Language (K&R)* by Brian W. Kernighan & Dennis M. Ritchie  
> **Compiler Module Focus**: [`agam_runtime`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/runtime/agam_runtime), [`agam_codegen`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/backends/agam_codegen)

---

## 1.1 Physical Memory Layout

Compilers translate abstract programming semantics into raw memory operations. Physical process memory allocated by the operating system is partitioned into several distinct segments:

```text
+-----------------------------------+ High Memory Address (e.g., 0x7FFFFFFF)
|            Stack Frame            | (Grows Downward toward low addresses)
|  Local Variables, Frame Pointers  |  |
|                                   |  v
:                                   :
:                                   :
|                                   |  ^
|            Heap Memory            |  |
|   Dynamic Allocation (malloc)     | (Grows Upward toward high addresses)
+-----------------------------------+
|      BSS (Uninitialized Globals)  |
+-----------------------------------+
|      Data (Initialized Globals)   |
+-----------------------------------+
|      Text (Executable Machine Code)| Low Memory Address (e.g., 0x00400000)
+-----------------------------------+
```

- **Text Segment**: Contains immutable binary instructions executed directly by the CPU instruction pointer (`rip`).
- **Data Segment**: Holds initialized global and static variables.
- **BSS Segment**: Holds uninitialized global variables, zeroed by the OS kernel upon process launch.
- **Heap**: Dynamic memory managed programmatically via allocators (`malloc`/`free`, bump allocators).
- **Stack**: Automatic memory managed via CPU stack pointer manipulation (`rsp`).

---

## 1.2 Data Alignment & Struct Padding

Modern CPU architectures access multi-byte primitive types (e.g., 32-bit integers, 64-bit pointers) most efficiently when located at addresses divisible by their size. Unaligned memory accesses can trigger performance penalties or CPU bus faults.

### Struct Alignment Rule
The compiler calculates struct layout offsets using alignment formulas:

$$\text{Offset}(X_{i+1}) = \text{AlignUp}(\text{Offset}(X_i) + \text{sizeof}(X_i), \text{AlignOf}(X_{i+1}))$$

### Layout Example
Consider a composite type definition:

```c
struct SystemHeader {
    char  id;        // 1 byte  (Offset 0)
                     // 3 bytes padding inserted by compiler
    int   flags;     // 4 bytes (Offset 4)
    short version;   // 2 bytes (Offset 8)
                     // 2 bytes padding inserted to align total size to 4-byte boundary
};                   // Total Size: 12 bytes
```

In `agam_sema` and `agam_mir`, struct layout calculators compute these exact padding byte offsets to guarantee ABI compatibility with target C runtimes.

---

## 1.3 Pointer Arithmetic & Memory Addressing

Pointers represent physical memory addresses. In C and generated target IR, adding an integer `k` to a pointer `p` scales `k` by the size of the referenced type $T$:

$$\text{Address}(p + k) = \text{Address}(p) + k \times \text{sizeof}(T)$$

Compilers emit pointer offset calculations explicitly using indexed memory operand instructions (e.g., `mov rax, [rbx + rdi*8]`).
