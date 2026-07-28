---
name: cargo-lens
description: Compresses cargo build/check output to show only errors and warnings, saving context tokens.
---

# cargo-lens

**Purpose**: Cargo dumps massive walls of text during build failures. This skill enforces parsing the output to extract only what is needed, preventing token window exhaustion.

## Usage

When you need to verify a build or run tests, do NOT run raw `cargo check` if you anticipate a massive failure trace. Instead, run:

```powershell
cargo check --message-format=short
```

If the output is still too large, pipe the output to a file and read only the first 50 lines, or use `grep` to extract only lines containing `error:`.

**Strict Rule**: Never dump raw 500+ line `cargo build` logs into the chat context.
