# Backend Capability Matrix

Defines explicit feature support per codegen target:

| Feature Area | Native LLVM | NVPTX64 GPU | C Emitter | JIT Engine |
| :--- | :--- | :--- | :--- | :--- |
| **Scalar Math & Control Flow** | ✅ Full | ✅ Full | ✅ Full | ✅ Full |
| **Fixed-Size Arrays `[T; N]`** | ✅ Full | ✅ Device DRAM | ✅ Full | ✅ Full |
| **Shared Memory `addrspace(3)`** | ❌ N/A | ✅ Block Scratchpad | 💬 Comment stub | ❌ N/A |
| **Algebraic Effects (`perform`)** | ✅ Extern Call | ❌ Prohibited | ✅ Static Dispatch | ✅ In-process |
| **Generics & Monomorphization** | ✅ Full | ✅ Full | ✅ Full | ✅ Full |
| **OS Sandboxing Enforcement** | ✅ Full | ❌ N/A | ✅ Full | ✅ Full |
