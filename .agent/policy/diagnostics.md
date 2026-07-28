# Compiler Diagnostic Error Code Policy

All compiler diagnostics in `agam_sema`, `agam_hir`, `agam_mir`, and backends use standardized error ranges:

| Error Code Range | Domain | Description |
| :--- | :--- | :--- |
| **`E0001` – `E0999`** | Syntax & Parsing | Lexer errors, unclosed brackets, invalid AST nodes |
| **`E1001` – `E1999`** | Semantic Analysis | Type mismatch, unresolved identifier, invalid cast |
| **`E2001` – `E2999`** | Algebraic Effects | Unhandled effect, perform prohibited in `@target.iot` |
| **`E3001` – `E3999`** | GPU & NVPTX | Prohibited heap allocation in `@gpu`, shared alloc error |
| **`E4001` – `E4999`** | MIR & Codegen | Unreachable SSA arm, invalid terminator, codegen crash |
