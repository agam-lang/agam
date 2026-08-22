//! Supply Chain Security, Package Provenance, Typosquatting Protection, and SBOM Generation.
//!
//! Provides cryptographic package signing using ML-DSA (NIST FIPS 204),
//! Levenshtein-based typosquatting detection, CycloneDX/SPDX SBOM generation,
//! and automated dependency vulnerability auditing.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use agam_runtime::crypto::sha256_digest;
use agam_runtime::pqc::{MlDsaKeyPair, MlDsaParameter, MlDsaPublicKey, MlDsaSecretKey};

use crate::{WorkspaceLockfile, WorkspaceManifest};

/// Cryptographic signature envelope attached to portable packages and registry releases.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageSignatureEnvelope {
    pub package_name: String,
    pub version: String,
    pub digest_sha256: String,
    pub algorithm: String,
    pub public_key_hex: String,
    pub signature_hex: String,
    pub timestamp_secs: u64,
}

/// Package signing and signature verification engine.
pub struct PackageSigner;

impl PackageSigner {
    /// Sign an artifact package payload using ML-DSA-65 post-quantum keypair.
    pub fn sign_artifact(
        name: &str,
        version: &str,
        payload: &[u8],
        secret_key: &MlDsaSecretKey,
        public_key: &MlDsaPublicKey,
    ) -> PackageSignatureEnvelope {
        let digest = sha256_digest(payload);
        let digest_hex = digest
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();

        let signature = MlDsaKeyPair::sign(secret_key, payload);
        let sig_hex = signature
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();
        let pub_hex = public_key
            .key_bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        PackageSignatureEnvelope {
            package_name: name.to_string(),
            version: version.to_string(),
            digest_sha256: digest_hex,
            algorithm: "ML-DSA-65".to_string(),
            public_key_hex: pub_hex,
            signature_hex: sig_hex,
            timestamp_secs: timestamp,
        }
    }

    /// Verify package signature envelope against raw payload bytes.
    pub fn verify_envelope(envelope: &PackageSignatureEnvelope, payload: &[u8]) -> bool {
        let expected_digest = sha256_digest(payload);
        let expected_digest_hex = expected_digest
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();

        if envelope.digest_sha256 != expected_digest_hex {
            return false;
        }

        let pub_bytes = match hex_decode(&envelope.public_key_hex) {
            Some(b) => b,
            None => return false,
        };

        let sig_bytes = match hex_decode(&envelope.signature_hex) {
            Some(b) => b,
            None => return false,
        };

        let pub_key = MlDsaPublicKey {
            params: MlDsaParameter::MlDsa65,
            key_bytes: pub_bytes,
        };

        MlDsaKeyPair::verify(&pub_key, payload, &sig_bytes)
    }
}

fn hex_decode(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect()
}

/// Typosquatting alert describing potential name confusion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TyposquatAlert {
    pub proposed: String,
    pub existing: String,
    pub edit_distance: usize,
    pub similarity_pct: u32,
}

/// Typosquatting detection engine.
pub struct TyposquatDetector;

impl TyposquatDetector {
    /// Compute Levenshtein distance between two package names.
    pub fn levenshtein(a: &str, b: &str) -> usize {
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        let m = a_chars.len();
        let n = b_chars.len();

        let mut dp = vec![vec![0; n + 1]; m + 1];

        for (i, row) in dp.iter_mut().enumerate().take(m + 1) {
            row[0] = i;
        }
        for (j, cell) in dp[0].iter_mut().enumerate().take(n + 1) {
            *cell = j;
        }

        for i in 1..=m {
            for j in 1..=n {
                let cost = if a_chars[i - 1] == b_chars[j - 1] {
                    0
                } else {
                    1
                };
                dp[i][j] = (dp[i - 1][j] + 1)
                    .min(dp[i][j - 1] + 1)
                    .min(dp[i - 1][j - 1] + cost);
            }
        }

        dp[m][n]
    }

    /// Check proposed package name against a collection of known package names.
    pub fn check(proposed: &str, existing_names: &[&str]) -> Vec<TyposquatAlert> {
        let mut alerts = Vec::new();
        let proposed_lower = proposed.to_ascii_lowercase();

        for &existing in existing_names {
            let existing_lower = existing.to_ascii_lowercase();
            if proposed_lower == existing_lower {
                continue;
            }

            let dist = Self::levenshtein(&proposed_lower, &existing_lower);
            let max_len = proposed_lower.len().max(existing_lower.len());
            let similarity = ((max_len.saturating_sub(dist)) * 100)
                .checked_div(max_len)
                .unwrap_or(100);

            // Alert on 1-2 edit distance or >= 80% similarity
            if dist <= 2 || similarity >= 80 {
                alerts.push(TyposquatAlert {
                    proposed: proposed.to_string(),
                    existing: existing.to_string(),
                    edit_distance: dist,
                    similarity_pct: similarity as u32,
                });
            }
        }

        alerts
    }
}

/// Supported SBOM export formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SbomFormat {
    CycloneDxJson,
    SpdxJson,
}

/// Software Bill of Materials (SBOM) generator.
pub struct SbomGenerator;

impl SbomGenerator {
    /// Generate CycloneDX 1.5 JSON SBOM representation.
    pub fn generate_cyclonedx(
        manifest: &WorkspaceManifest,
        lockfile: &WorkspaceLockfile,
    ) -> serde_json::Value {
        let components: Vec<serde_json::Value> = lockfile
            .packages
            .iter()
            .map(|pkg| {
                serde_json::json!({
                    "type": "library",
                    "name": pkg.name,
                    "version": pkg.version,
                    "purl": format!("pkg:agam/{}@{}", pkg.name, pkg.version),
                    "hashes": [
                        {
                            "alg": "SHA-256",
                            "content": pkg.content_hash
                        }
                    ]
                })
            })
            .collect();

        serde_json::json!({
            "bomFormat": "CycloneDX",
            "specVersion": "1.5",
            "version": 1,
            "metadata": {
                "component": {
                    "type": "application",
                    "name": manifest.project.name,
                    "version": manifest.project.version
                }
            },
            "components": components
        })
    }

    /// Generate SPDX 2.3 JSON SBOM representation.
    pub fn generate_spdx(
        manifest: &WorkspaceManifest,
        lockfile: &WorkspaceLockfile,
    ) -> serde_json::Value {
        let packages: Vec<serde_json::Value> = lockfile
            .packages
            .iter()
            .enumerate()
            .map(|(idx, pkg)| {
                serde_json::json!({
                    "SPDXID": format!("SPDXRef-Package-{}", idx + 1),
                    "name": pkg.name,
                    "versionInfo": pkg.version,
                    "downloadLocation": "NOASSERTION",
                    "checksums": [
                        {
                            "algorithm": "SHA256",
                            "checksumValue": pkg.content_hash
                        }
                    ]
                })
            })
            .collect();

        serde_json::json!({
            "spdxVersion": "SPDX-2.3",
            "dataLicense": "CC0-1.0",
            "SPDXID": "SPDXRef-DOCUMENT",
            "name": manifest.project.name,
            "documentNamespace": format!("https://agam-lang.org/spdx/{}-{}", manifest.project.name, manifest.project.version),
            "packages": packages
        })
    }
}

/// Security vulnerability advisory.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecurityAdvisory {
    pub id: String,
    pub package: String,
    pub vulnerable_version_range: String,
    pub severity: String,
    pub title: String,
    pub patched_version: String,
}

/// Vulnerability audit report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AuditReport {
    pub total_packages_scanned: usize,
    pub vulnerabilities_found: Vec<SecurityAdvisory>,
    pub capability_grants: BTreeMap<String, Vec<String>>,
}

/// Dependency vulnerability audit engine.
pub struct AuditEngine;

impl AuditEngine {
    /// Scan locked dependencies against security advisory database.
    pub fn audit(lockfile: &WorkspaceLockfile) -> AuditReport {
        let mut report = AuditReport {
            total_packages_scanned: lockfile.packages.len(),
            vulnerabilities_found: Vec::new(),
            capability_grants: BTreeMap::new(),
        };

        // Simulated advisory database checking
        for pkg in &lockfile.packages {
            if pkg.name == "vulnerable-lib" && pkg.version == "0.1.0" {
                report.vulnerabilities_found.push(SecurityAdvisory {
                    id: "AGAM-2026-0001".to_string(),
                    package: pkg.name.clone(),
                    vulnerable_version_range: "< 0.2.0".to_string(),
                    severity: "HIGH".to_string(),
                    title: "Buffer overflow in legacy serialization parser".to_string(),
                    patched_version: "0.2.0".to_string(),
                });
            }
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LOCKFILE_FORMAT_VERSION, LockedWorkspace};

    #[test]
    fn test_package_signing_and_verification_roundtrip() {
        let seed = b"quantum_supply_chain_test_seed_65";
        let keypair = MlDsaKeyPair::from_seed(MlDsaParameter::MlDsa65, seed);

        let payload = b"AGAM_PACKAGE_PAYLOAD_BINARY_BLOB_V1";
        let envelope = PackageSigner::sign_artifact(
            "fast-matrix",
            "1.2.0",
            payload,
            &keypair.secret_key,
            &keypair.public_key,
        );

        assert_eq!(envelope.package_name, "fast-matrix");
        assert_eq!(envelope.version, "1.2.0");
        assert!(PackageSigner::verify_envelope(&envelope, payload));

        // Tampered payload fails verification
        assert!(!PackageSigner::verify_envelope(
            &envelope,
            b"TAMPERED_PAYLOAD"
        ));
    }

    #[test]
    fn test_typosquatting_detection() {
        let known = ["math", "tensor", "dataframe", "network", "crypto"];
        let alerts = TyposquatDetector::check("matth", &known);
        assert!(!alerts.is_empty());
        assert_eq!(alerts[0].existing, "math");
        assert_eq!(alerts[0].edit_distance, 1);

        let safe = TyposquatDetector::check("super_quantum_sim", &known);
        assert!(safe.is_empty());
    }

    #[test]
    fn test_sbom_generation() {
        let manifest_toml = r#"
[project]
name = "my-app"
version = "0.1.0"
agam = "0.1.0"
"#;
        let manifest: WorkspaceManifest = toml::from_str(manifest_toml).unwrap();
        let lockfile = WorkspaceLockfile {
            format_version: LOCKFILE_FORMAT_VERSION,
            workspace: LockedWorkspace {
                name: "my-app".to_string(),
                version: "0.1.0".to_string(),
            },
            packages: Vec::new(),
            environments: BTreeMap::new(),
        };

        let cyclonedx = SbomGenerator::generate_cyclonedx(&manifest, &lockfile);
        assert_eq!(cyclonedx["bomFormat"], "CycloneDX");
        assert_eq!(cyclonedx["metadata"]["component"]["name"], "my-app");

        let spdx = SbomGenerator::generate_spdx(&manifest, &lockfile);
        assert_eq!(spdx["spdxVersion"], "SPDX-2.3");
        assert_eq!(spdx["name"], "my-app");
    }
}
