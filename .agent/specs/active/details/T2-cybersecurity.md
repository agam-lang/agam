# Phase T2-cybersecurity � World-Class Cybersecurity Programming

**Status:** open
**Tier:** 2 (Runtime and Concurrency — security is a runtime+compiler co-concern)
**Priority:** Critical differentiator — no language today offers all three: memory safety + capability security + formal verification in one package

## Vision

Make Agam the **first language where security vulnerabilities are structurally impossible**, not just discouraged. Buffer overflows, null pointer dereferences, injection attacks, and unauthorized resource access must be prevented by the compiler and runtime — not by developer discipline.

## Pillar A: Strict Memory Safety (Compile-Time Guarantees)

### Buffer Overflow Prevention
- [ ] All array/slice accesses bounds-checked at compile time where provable, at runtime otherwise
- [ ] No raw pointer arithmetic outside `unsafe` blocks
- [ ] Stack canary insertion for functions that use variable-length buffers
- [ ] Compile-time proof that no buffer overflow is possible in `@safe` functions

### Null Safety
- [ ] No null pointers in safe Agam — `Option<T>` is the only way to represent absence
- [ ] Compiler rejects code that dereferences without matching on `Some`/`None`
- [ ] FFI boundary automatically wraps nullable C pointers in `Option`

### Use-After-Free / Double-Free Prevention
- [ ] Enforced by the memory model (Phase T2-memory-model) — whether ownership, ARC, or effect-based
- [ ] Compile-time guarantee: no dangling references escape their scope
- [ ] Runtime: poisoned-memory detection in debug builds

## Pillar B: Capability-Based Security Model

### Principle of Least Authority (POLA)
- [ ] `@capabilities(fs, net)` annotation on modules/functions declaring what resources they access
- [ ] A math library with no capability annotations **cannot** touch the filesystem or network — compiler enforced
- [ ] Capability propagation: if function A calls function B, A must have at least B's capabilities
- [ ] `agamc audit --capabilities` reports the full capability graph of a program

### Sandboxed Execution Domains
- [ ] `@sandbox` annotation creates an isolated execution domain
- [ ] Sandboxed code cannot access parent scope's mutable state
- [ ] Inter-sandbox communication only through explicit channels
- [ ] Extends existing Phase T2-os-sandbox OS-level sandbox into language-level enforcement

### Supply Chain Security
- [ ] Package capability manifests in `agam.toml`: `[capabilities] fs = false, net = false`
- [ ] `agamc install` refuses packages that request capabilities not declared by the consumer
- [ ] Cryptographic package signing and provenance verification
- [ ] SBOM (Software Bill of Materials) generation: `agamc sbom`

## Pillar C: Cryptographic Primitives (Language-Native)

### Standard Crypto Library
- [ ] `agam_std::crypto` with audited implementations (not wrappers)
- [ ] AES-256-GCM, ChaCha20-Poly1305 symmetric encryption
- [ ] Ed25519, X25519 asymmetric operations
- [ ] SHA-256, SHA-3, BLAKE3 hashing
- [ ] HMAC, HKDF key derivation
- [ ] Constant-time comparison for all secret-dependent operations
- [ ] Side-channel resistance annotations: `@constant_time` verified by compiler

### Secure Random
- [ ] `agam_std::crypto::random` backed by OS CSPRNG (CryptGenRandom / getrandom)
- [ ] No `rand()` equivalent that could be accidentally used for security
- [ ] Compile-time warning if non-crypto random is used in `@security` contexts

### TLS/Certificate Handling
- [ ] First-party TLS 1.3 support in `agam_std::crypto::tls`
- [ ] Certificate pinning utilities
- [ ] Automatic certificate validation with clear error messages

## Pillar D: Secure Coding Enforcement

### Taint Tracking
- [ ] `@tainted` type qualifier for user-supplied input
- [ ] Tainted data cannot be used in SQL queries, shell commands, or file paths without explicit sanitization
- [ ] Compiler error: `Cannot use tainted String in sql_query() — sanitize first`
- [ ] Built-in sanitizers for SQL, HTML, shell, path traversal

### Integer Overflow Protection
- [ ] Checked arithmetic by default in `@safe` mode — overflow panics in debug, wraps in release
- [ ] `@overflow(wrap)`, `@overflow(saturate)`, `@overflow(panic)` annotations for explicit control
- [ ] `WrappingInt`, `SaturatingInt` types for intentional overflow behavior

### Secrets Management
- [ ] `Secret<T>` type that:
  - Zeroes memory on drop (zeroize)
  - Cannot be printed or logged (no `Display`/`Debug` impl)
  - Cannot be serialized without explicit `@allow_serialize`
  - Triggers compiler warning if assigned to a non-secret variable

### Compile-Time Security Lints
- [ ] `agam_lint` security rules enabled by default:
  - Hardcoded credentials detection
  - Insecure hash function usage (MD5, SHA-1 for security)
  - Cleartext secret in source code
  - Unsafe deserialization patterns
  - Path traversal in file operations

## Pillar E: Formal Verification for Security-Critical Code

### Integrated Theorem Prover (extends Phase T6-formal-verification)
- [ ] `@verify` annotation with pre/post-conditions for crypto and security functions
- [ ] `requires(key.len() == 32)` — compile-time proof obligation
- [ ] `ensures(result.is_encrypted())` — output contract verification
- [ ] Loop invariant inference for crypto algorithms
- [ ] SMT-backed verification using Z3/CVC5

### Security Proofs
- [ ] Prove absence of buffer overflows in annotated functions
- [ ] Prove constant-time execution of annotated crypto code
- [ ] Prove capability compliance (no escalation)
- [ ] `agamc verify --security` runs all security proof obligations

## Pillar F: Network Security

### Secure Defaults
- [ ] All network connections use TLS by default — plaintext requires explicit `@allow_plaintext`
- [ ] DNS-over-HTTPS by default
- [ ] Certificate validation enabled by default — disabling requires `@allow_insecure_tls`

### Firewall Integration
- [ ] `@network(allow: ["api.example.com:443"])` — compile-time network allowlisting
- [ ] Runtime enforcement: connections to non-allowed hosts are rejected
- [ ] `agamc audit --network` reports all network endpoints in the program

## Responsible Crates

- `agam_sema` — capability checking, taint tracking, overflow mode enforcement
- `agam_runtime` — sandbox enforcement, secret zeroization, secure random
- `agam_std` — crypto primitives, TLS, secure networking
- `agam_lint` — security lint rules
- `agam_smt` — formal verification (extends X2)
- `agam_pkg` — capability manifests, package signing, SBOM

## Dependencies

- Phase T2-memory-model (memory model) — memory safety guarantees depend on chosen model
- Phase T0-type-system (type system) — generics needed for `Secret<T>`, `Option<T>`, capability types
- Phase T0-object-model (object model) — traits needed for crypto trait abstractions
- Phase T0-module-system (modules) — visibility rules needed for capability enforcement
- Phase T6-formal-verification (formal verification) — theorem prover shared infrastructure

## Test Strategy

- Security-specific fuzzing: AFL/libFuzzer integration for all crypto primitives
- Capability violation tests: programs that violate capabilities must be rejected
- Taint tracking tests: tainted data reaching sinks without sanitization = compile error
- Constant-time verification: timing tests for crypto operations
- CVE regression tests: known vulnerability patterns must be caught by lints
