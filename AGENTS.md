# Agam Compiler — Agent Instructions

> Full briefing: CLAUDE.md.

## Quick Orientation

- **What:** Agam compiler (Rust workspace, 27 crates).
- **Target:** Native LLVM for Windows, Linux, Android.
- **Pipeline:** Source → agam_lexer → agam_parser → agam_ast → agam_sema → agam_hir → agam_mir → agam_codegen/agam_jit
- **CLI:** gamc {build,run,check,lock,new,dev,daemon,fmt,test,lsp,repl,exec,doctor,env,publish,registry,cache,package}

## Rules

1. Agam is own language. Check .agam syntax in enchmarks/ and xamples/.
2. Smallest responsible crate. Preserve spans/diagnostics via gam_errors. No panic .unwrap().
3. Benchmark before/after for optimizations.
4. Verify: cargo check and cargo test.
5. Keep commits clean and verified.

## Layout

| Path | Purpose |
|---|---|
| crates/ | core/, middle/, ackends/, 
untime/, 	ooling/, xperiments/ |
| xamples/ | .agam examples |
| enchmarks/ | Benchmarks and performance tests |
| .agent/ | Specs, rules, skills, memory |
| CLAUDE.md | Full compiler briefing |

## Skills

caveman, caveman-compress, cargo-lens, spec-archiver, enchmark-guard, language-guard, graphify.
