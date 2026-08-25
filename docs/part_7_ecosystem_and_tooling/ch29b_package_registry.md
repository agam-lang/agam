# Chapter 29b: Package Registry, Dependency Resolution & Distribution

> **Part VII: Advanced Tooling, Testing & Ecosystem Engineering**  
> **Compiler Module Focus**: [`agam_pkg`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/tooling/agam_pkg)

---

## 29b.1 The Agam Package Ecosystem

The Agam package ecosystem consists of three components: a **manifest format** (`agam.toml`), a **lockfile** (`agam.lock`), and a **registry protocol** for publishing and resolving packages.

```text
Developer Workflow:
  agamc new myapp          → Scaffold project with agam.toml
  agamc add serde@1.0      → Add dependency to manifest
  agamc build              → Resolve deps, download, compile
  agamc publish            → Publish to registry
```

---

## 29b.2 `agam.toml` Manifest Format

Every Agam project is defined by a `agam.toml` manifest at the project root:

```toml
[package]
name = "image-classifier"
version = "0.3.1"
edition = "2026"
authors = ["Agam Team <team@agam-lang.org>"]
description = "GPU-accelerated image classification library"
license = "MIT"
repository = "https://github.com/agam-lang/image-classifier"
keywords = ["gpu", "ai", "image", "tensor"]
categories = ["science", "machine-learning"]

[dependencies]
agam-tensor = "1.2.0"              # Exact version
agam-gpu = "^0.5"                  # Compatible (0.5.x)
agam-http = "~1.0"                 # Patch-level updates only (1.0.x)
agam-crypto = { version = "2.0", features = ["chacha20"] }
local-utils = { path = "../utils" } # Local path dependency

[dev-dependencies]
agam-bench = "0.2.0"
agam-mock = "1.0.0"

[build]
target = "x86_64-unknown-linux-gnu"
opt-level = 2                       # 0=debug, 1=basic, 2=full, 3=aggressive

[features]
default = ["std"]
std = []                            # Standard library support
no-std = []                         # Bare-metal / embedded mode
gpu = ["agam-gpu"]                  # Optional GPU acceleration

[profile.release]
opt-level = 3
lto = true                         # Link-Time Optimization
strip = true                       # Strip debug symbols
```

---

## 29b.3 Semantic Versioning & Compatibility

Agam enforces **Semantic Versioning 2.0** (SemVer) for all published packages:

| Version Bump | Meaning | Example |
| :--- | :--- | :--- |
| **Major** (X.y.z) | Breaking API changes | `1.0.0` → `2.0.0` |
| **Minor** (x.Y.z) | New features, backwards compatible | `1.0.0` → `1.1.0` |
| **Patch** (x.y.Z) | Bug fixes, no API changes | `1.0.0` → `1.0.1` |

### Version Requirement Syntax

| Syntax | Matches | Description |
| :--- | :--- | :--- |
| `"1.2.3"` | Exactly `1.2.3` | Pinned version |
| `"^1.2"` | `≥1.2.0, <2.0.0` | Compatible updates |
| `"~1.2"` | `≥1.2.0, <1.3.0` | Patch-level updates |
| `">=1.0, <2.0"` | `≥1.0.0, <2.0.0` | Range specification |
| `"*"` | Any version | Wildcard (not recommended) |

---

## 29b.4 Dependency Resolution Algorithm

The resolver uses a **SAT-based backtracking algorithm** to find a satisfying assignment of package versions:

```text
Input: Dependency graph from agam.toml (direct + transitive)

1. UNIT PROPAGATION
   For each package with only one candidate version, select it immediately.

2. CONFLICT-DRIVEN CLAUSE LEARNING (CDCL)
   If two packages require incompatible versions of a shared dependency:
     a. Record the conflict clause (e.g., "A@1.0 and B@2.0 cannot coexist")
     b. Backtrack to the most recent decision point
     c. Try the next candidate version
     d. Add the conflict clause to prevent revisiting

3. TOPOLOGICAL RESOLUTION
   Process packages in dependency order (leaves first, root last)
   to minimize backtracking.

4. OUTPUT
   A complete, deterministic version assignment → agam.lock
```

### Diamond Dependency Resolution

```text
     myapp
    /     \
   A@1.0   B@2.0
    \     /
     C@???

A requires C@^1.0 (≥1.0, <2.0)
B requires C@^1.5 (≥1.5, <2.0)

Resolution: C@1.5.x (intersection of both ranges)
```

If the ranges are incompatible (e.g., A requires `C@^1.0` and B requires `C@^2.0`), the resolver emits a clear diagnostic:

```text
error: incompatible dependency versions
  Package `A@1.0.0` requires `C@^1.0`
  Package `B@2.0.0` requires `C@^2.0`
  
  No version of `C` satisfies both constraints.
  
  help: upgrade `A` to a version compatible with `C@2.x`
```

---

## 29b.5 `agam.lock` Lockfile

The lockfile captures the exact resolved versions for reproducible builds:

```toml
# agam.lock — auto-generated, DO NOT EDIT
[[package]]
name = "agam-tensor"
version = "1.2.3"
source = "registry+https://registry.agam-lang.org"
checksum = "sha256:a1b2c3d4e5f6..."
dependencies = ["agam-runtime@0.1.0"]

[[package]]
name = "agam-gpu"
version = "0.5.2"
source = "registry+https://registry.agam-lang.org"
checksum = "sha256:f6e5d4c3b2a1..."
dependencies = ["agam-tensor@1.2.3", "agam-runtime@0.1.0"]
```

**Lockfile guarantees:**
- Every CI build and developer machine resolves to **identical** dependency versions
- Checksums verify package integrity (defense against supply-chain attacks)
- `agam.lock` is committed to version control

---

## 29b.6 Registry Protocol

The Agam package registry provides an HTTP API for package discovery, download, and publishing:

### API Endpoints

| Endpoint | Method | Description |
| :--- | :---: | :--- |
| `/api/v1/packages` | `GET` | List all packages (paginated) |
| `/api/v1/packages/{name}` | `GET` | Get package metadata |
| `/api/v1/packages/{name}/{version}` | `GET` | Get specific version metadata |
| `/api/v1/packages/{name}/{version}/download` | `GET` | Download package tarball |
| `/api/v1/packages/new` | `PUT` | Publish new package version |
| `/api/v1/packages/{name}/owners` | `GET/PUT` | Manage package owners |
| `/api/v1/search?q={query}` | `GET` | Full-text search |

### Publishing Workflow

```bash
# Login with API token
agamc login --token agam_tok_abc123

# Verify package before publishing
agamc publish --dry-run

# Publish to registry
agamc publish
```

```text
Publishing Sequence:
  1. agamc publish
  2. Build package from source (verify it compiles)
  3. Run test suite (verify tests pass)
  4. Create source tarball (.tar.gz)
  5. Compute SHA-256 checksum
  6. PUT to registry API with auth token
  7. Registry validates: SemVer, no yanked deps, checksum
  8. Registry indexes package for search
  9. Package available for `agamc add`
```

### Package Yanking

Published versions cannot be deleted (to prevent breaking downstream users), but they can be **yanked** to prevent new dependencies:

```bash
# Yank a version (existing users can still download)
agamc yank image-classifier@0.2.0 --reason "Security vulnerability in CVE-2026-1234"

# Un-yank (restore availability)
agamc yank --undo image-classifier@0.2.0
```

---

## 29b.7 Local & Private Registries

Organizations can host private registries for internal packages:

```toml
# In agam.toml
[registries]
internal = { url = "https://packages.internal.corp/api/v1" }

[dependencies]
internal-auth = { version = "3.0", registry = "internal" }
```

### Registry Priority

```text
1. Local path dependencies        (highest priority)
2. Private registries              (organization-scoped)
3. Public registry (agam-lang.org) (default fallback)
```
