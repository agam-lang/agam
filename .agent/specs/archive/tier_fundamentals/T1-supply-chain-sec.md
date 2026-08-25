# Phase T1-supply-chain-sec � Supply Chain Security

**Status:** open
**Tier:** 2 (Runtime — supply chain security is a safety concern)
**Pillar:** 20 — The Supply Chain Security Pillar

## Vision

The Agam package registry is **not a public dumping ground**. Every package has provable origins, enforced capability sandboxing, and typosquatting protection. Installing an Agam package is as safe as installing an app from a curated app store.

## Deliverables

### Provable Origins
- [ ] Every published package must be **cryptographically signed** by the developer's key
- [ ] `agamc publish` requires signing with developer's Ed25519 key
- [ ] `agamc install` verifies signature before download
- [ ] Key rotation support with revocation lists
- [ ] Sigstore-style transparency log: every publish is publicly recorded
- [ ] Developer identity verification (GitHub/GitLab account linking)

### Capability Sandboxing for Packages
- [ ] Every package declares capabilities in `agam.toml`:
  ```toml
  [capabilities]
  filesystem = false
  network = false
  process = false
  ffi = false
  ```
- [ ] Build system **enforces** declared capabilities at compile time
- [ ] A "string formatting" package with `network = true` triggers a **warning** at install
- [ ] `agamc audit --capabilities` shows full capability tree of all dependencies
- [ ] Permission grant model: consumer must explicitly allow capabilities:
  ```toml
  [dependencies.some-package]
  version = "1.0"
  allow = ["network"]  # Explicit grant
  ```

### Typosquatting Protection
- [ ] Package name similarity detection at publish time
- [ ] `agamc add strng-utils` → "Did you mean `string-utils`? (98% name similarity)"
- [ ] Reserved namespace for official packages: `agam/*`
- [ ] Mandatory README and license for published packages
- [ ] Cooldown period for new packages (24h review window)

### Vulnerability Management
- [ ] `agamc audit` — scan all dependencies against advisory database
- [ ] Automatic security advisory notifications on `agamc build`
- [ ] `agamc audit --fix` — auto-update to patched versions
- [ ] Advisory database maintained by Agam organization
- [ ] CVE cross-reference with RustSec and OSV databases

### SBOM Generation
- [ ] `agamc sbom` — generate SPDX or CycloneDX Software Bill of Materials
- [ ] Includes: all dependencies, versions, licenses, hashes, capabilities
- [ ] Required for enterprise and government compliance
- [ ] Machine-readable JSON and human-readable report output

## Responsible Crates

- `agam_pkg` — signing, capability enforcement, typosquat detection, SBOM
- `agam_driver` — `agamc audit`, `agamc sbom` commands
- `agam_sema` — capability checking during compilation

## Dependencies

- Phase T2-cybersecurity (security) — cryptographic infrastructure shared
- Phase T3-pkg-manager-maturity (package manager) — registry protocol
- Phase T1-cross-platform-pkg (reproducibility) — hash verification infrastructure
