# Chapter 24b: Security Architecture, Cryptography & Sandboxing

> **Part VI: The Agam Language Programming Guide**  
> **Compiler Module Focus**: [`agam_runtime::security`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/runtime/agam_runtime), [`agam_runtime::crypto`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/runtime/agam_runtime), [`agam_runtime::sandbox`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/runtime/agam_runtime)

---

## 24b.1 Security Design Principles

Agam is designed with security as a first-class engineering constraint, not an afterthought:

| Principle | Implementation |
| :--- | :--- |
| **Memory safety by default** | ARC + affine ownership — no use-after-free, no double-free |
| **No null pointers** | `Option[T]` monad — compiler-enforced null handling |
| **No data races** | Structured concurrency + Mutex/RwLock — statically checked |
| **Defense in depth** | OS sandbox + capability model + crypto primitives |
| **Secure by construction** | Zeroization on drop, constant-time comparison |

---

## 24b.2 Memory Safety Guarantees

### Secret Value Zeroization

Sensitive data (keys, passwords, tokens) must be erased from memory when no longer needed. Agam's `Secret[T]` wrapper guarantees this:

```agam
let api_key = Secret.new("sk-prod-abc123xyz");

// Use the secret value
let response = await http.get(url, headers: {
    "Authorization": "Bearer " + api_key.expose()
});

// When api_key goes out of scope:
// 1. The underlying memory is overwritten with zeros
// 2. The compiler prevents accidental logging/serialization
// 3. Debug output shows "Secret<***>" instead of the value
```

**Zeroization guarantees:**
- Memory is overwritten with zeros using `volatile_set_memory` (prevents compiler from optimizing away the write)
- Works for both stack and heap allocations
- Applies to all intermediate copies created during computation

### Constant-Time Comparison

To prevent timing side-channel attacks, Agam provides constant-time comparison for security-critical values:

```agam
// INSECURE: Standard comparison leaks information via timing
if token == expected_token { ... }  // Early-exit reveals prefix match length

// SECURE: Constant-time comparison
if crypto.constant_time_eq(token, expected_token) { ... }
// Always examines ALL bytes, regardless of mismatch position
```

---

## 24b.3 Cryptographic Primitives

The `agam_runtime::crypto` module provides verified implementations of essential cryptographic algorithms:

### Hash Functions

```agam
// SHA-256 digest
let hash = crypto.sha256("Hello, Agam!");
// hash: "a1b2c3d4..." (64-character hex string)

// Incremental hashing for large data
let mut hasher = crypto.Sha256Hasher.new();
hasher.update(chunk_1);
hasher.update(chunk_2);
hasher.update(chunk_3);
let digest = hasher.finalize();
```

### HMAC (Hash-Based Message Authentication Code)

```agam
// HMAC-SHA256 for message authentication
let key = Secret.new("my-secret-key");
let mac = crypto.hmac_sha256(key.expose(), "message to authenticate");

// Verify HMAC (constant-time comparison)
let is_valid = crypto.hmac_sha256_verify(key.expose(), "message", received_mac);
```

### Stream Cipher (ChaCha20)

```agam
// ChaCha20 encryption
let key: [u8; 32] = crypto.random_bytes(32);
let nonce: [u8; 12] = crypto.random_bytes(12);

let plaintext = "Sensitive data to encrypt";
let ciphertext = crypto.chacha20_encrypt(key, nonce, plaintext);
let decrypted = crypto.chacha20_decrypt(key, nonce, ciphertext);

assert_eq!(decrypted, plaintext);
```

### Cryptographically Secure Random Number Generator (CSPRNG)

```agam
// Generate cryptographically secure random bytes
let random_bytes: [u8; 32] = crypto.random_bytes(32);

// Generate a random integer in range
let random_id: u64 = crypto.random_u64();

// Generate a random token (URL-safe base64)
let token: String = crypto.random_token(32); // 32-byte token, base64 encoded
```

The CSPRNG reads from the operating system's entropy source (`/dev/urandom` on Linux, `BCryptGenRandom` on Windows).

---

## 24b.4 OS-Level Process Sandboxing

The **Chāṇakya Durdharṣa** sandbox enforces operating system-level isolation for untrusted code execution:

### Windows Sandbox (JobObject)

```text
┌───────────────────────────────────────────┐
│           Windows Job Object               │
│                                            │
│  Limits:                                   │
│    • Memory: max 512 MB                    │
│    • CPU Rate: max 50%                     │
│    • Wall Clock: max 30 seconds            │
│    • Child Processes: 0 (cannot spawn)     │
│    • I/O: Restricted to sandbox directory  │
│                                            │
│  ┌─────────────────────────────────────┐  │
│  │  Agam Process (sandboxed)           │  │
│  │                                      │  │
│  │  agamc exec --json '...'             │  │
│  └─────────────────────────────────────┘  │
└───────────────────────────────────────────┘
```

### Linux Sandbox (prctl + cgroups)

```text
┌───────────────────────────────────────────┐
│         Linux Sandbox Stack                │
│                                            │
│  Layer 1: prctl(PR_SET_NO_NEW_PRIVS)       │
│    → Cannot gain elevated privileges       │
│                                            │
│  Layer 2: setrlimit()                      │
│    → RLIMIT_AS:    512 MB max memory       │
│    → RLIMIT_NOFILE: 64 max file descriptors│
│    → RLIMIT_CPU:    30 sec CPU time        │
│    → RLIMIT_FSIZE:  0 (no file writes)     │
│                                            │
│  Layer 3: Filesystem chroot                │
│    → Read-only access to stdlib            │
│    → No access to host filesystem          │
│                                            │
│  Layer 4: Network namespace isolation      │
│    → No network access by default          │
└───────────────────────────────────────────┘
```

### Sandbox Configuration

```toml
# In agam.toml (for agamc exec)
[sandbox]
memory_limit_mb = 512
timeout_seconds = 30
max_file_descriptors = 64
allow_network = false
allow_filesystem_write = false
allow_child_processes = false
```

---

## 24b.5 Capability-Based Security Model

Agam's type system can enforce fine-grained capability restrictions at compile time:

```agam
// Capability tokens — zero-cost type-level permissions
capability FileRead;
capability FileWrite;
capability NetworkAccess;

// Functions declare required capabilities
fn read_config(cap: FileRead) -> Config {
    return File.read("config.toml").parse();
}

fn send_telemetry(cap: NetworkAccess, data: Metrics) {
    http.post("https://telemetry.example.com", body: data);
}

// Capabilities must be explicitly granted at the entry point
fn main() {
    let file_cap = grant!(FileRead);
    let config = read_config(file_cap);

    // Compile error: NetworkAccess not granted
    // send_telemetry(???, metrics);  // Error: missing capability
}
```

This ensures that untrusted library code cannot perform privileged operations without explicit permission from the application's entry point.

---

## 24b.6 Taint Tracking

The compiler tracks **tainted** data (user input, network data) and prevents it from reaching sensitive operations without explicit sanitization:

```agam
fn handle_request(input: @tainted String) -> String {
    // Compile error: tainted data cannot be used in SQL directly
    // let result = db.query("SELECT * FROM users WHERE name = '" + input + "'");

    // Must sanitize first
    let safe_input = sanitize.sql_escape(input);  // Returns @clean String
    let result = db.query("SELECT * FROM users WHERE name = '" + safe_input + "'");
    return result;
}
```
