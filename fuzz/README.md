# Coverage-Guided Fuzzing Infrastructure (`agam/fuzz`)

This directory contains libFuzzer-backed, coverage-guided fuzz targets for the Agam compiler infrastructure.

## Fuzz Targets

1. **`fuzz_parser`** (`fuzz_targets/fuzz_parser.rs`):
   - Fuzzes the Pratt parser and token stream against arbitrary adversarial byte streams and malformed syntax.
2. **`fuzz_bindgen`** (`fuzz_targets/fuzz_bindgen.rs`):
   - Fuzzes the C header parser in `agam_ffi::bindgen` against untrusted, corrupted, or complex C header files.

## Running Fuzzers

Prerequisites:
```bash
cargo install cargo-fuzz
rustup default nightly
```

Run a fuzzing campaign:
```bash
cargo fuzz run fuzz_parser
cargo fuzz run fuzz_bindgen
```
