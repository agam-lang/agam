# Evaluation Case: [Feature or Bug Name]

## 1. Goal

[Define the exact outcome required. What are we trying to achieve?]

## 2. Input State

[What files or structures are we starting with? E.g., "The AST parser currently fails on `let x: int = 5;`"]

## 3. Verifiable Expected Output

[What is the exact, measurable expected outcome? E.g., "The parser must successfully build an `AstNode::VarDecl` without returning an `agam_errors::ParseError`"]

## 4. Verification Script (The "Eval")

[Provide the exact command that the AI must run to prove the task is done. The AI is NOT ALLOWED to stop working until this command exits with a `0` success code.]

```powershell
# Example:
cargo test test_var_decl_parsing --manifest-path agam_parser/Cargo.toml
```

## AI Agent Instructions

If you are an AI reading this eval:

1. Implement the fix.
2. Run the Verification Script.
3. If it fails, analyze the output, adjust your code, and loop. Do not ask for human intervention unless you have exhausted all logical approaches or are stuck in an infinite loop.
