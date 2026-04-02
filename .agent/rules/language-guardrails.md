# Language Guardrails

- Agam is its own language. Do not treat `.agam` files as Python with different punctuation.
- Use `.agent/test/*.agam` and `benchmarks/benchmarks/**/*.agam` as concrete syntax references for benchmark-oriented source patterns.
- ML, tensor, dataframe, and autodiff workflows are supposed to be native compiler/runtime features.
- Favor direct native loops, typed lowering, and backend-aware runtime helpers over wrapper-heavy designs.
- Keep the language coherent. New features must strengthen simplicity, safety, native performance, portability, or AI/ML-first usability.
