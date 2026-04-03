//! Repo-local persistent cache for package and native build artifacts.

use std::collections::BTreeMap;
use std::fs;
use std::hash::Hasher;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::contract::{RUNTIME_ABI_VERSION, RuntimeBackend, host_runtime};

const CACHE_SCHEMA_VERSION: u32 = 1;
const MAX_CACHE_ENTRIES: usize = 256;
const MAX_CACHE_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheKey {
    pub source_hash: String,
    pub semantic_hash: String,
    pub backend: RuntimeBackend,
    pub opt_level: u8,
    pub runtime_abi: u32,
    pub host_os: String,
    pub host_arch: String,
    pub host_pointer_width: u8,
    pub feature_signature: String,
}

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
)]
pub enum CacheArtifactKind {
    #[default]
    NativeBinary,
    LlvmIr,
    CSource,
    PortablePackage,
    ProfileJson,
}

impl CacheArtifactKind {
    pub fn label(self) -> &'static str {
        match self {
            CacheArtifactKind::NativeBinary => "native-binary",
            CacheArtifactKind::LlvmIr => "llvm-ir",
            CacheArtifactKind::CSource => "c-source",
            CacheArtifactKind::PortablePackage => "portable-package",
            CacheArtifactKind::ProfileJson => "profile-json",
        }
    }
}

#[derive(Clone, Debug)]
pub struct CacheStore {
    root: PathBuf,
    entries_dir: PathBuf,
}

#[derive(Clone, Debug)]
pub struct CacheEntry {
    pub artifact_kind: CacheArtifactKind,
    pub source_path: String,
    pub last_used_unix_ms: u128,
    pub bytes: u64,
}

#[derive(Clone, Debug)]
pub struct CacheHit {
    pub id: String,
    pub artifact_path: PathBuf,
    pub entry: CacheEntry,
}

#[derive(Clone, Debug)]
pub struct CacheStatus {
    pub root: PathBuf,
    pub entry_count: usize,
    pub total_bytes: u64,
    pub by_kind: Vec<CacheStatusByKind>,
    pub recent_entries: Vec<CacheStatusEntry>,
}

#[derive(Clone, Debug)]
pub struct CacheStatusByKind {
    pub kind: CacheArtifactKind,
    pub entries: usize,
    pub bytes: u64,
}

#[derive(Clone, Debug)]
pub struct CacheStatusEntry {
    pub artifact_kind: CacheArtifactKind,
    pub source_path: String,
    pub last_used_unix_ms: u128,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StoredCacheEntry {
    schema_version: u32,
    id: String,
    key: CacheKey,
    artifact_kind: CacheArtifactKind,
    source_path: String,
    artifact_name: String,
    bytes: u64,
    created_unix_ms: u128,
    last_used_unix_ms: u128,
}

impl CacheStore {
    pub fn for_path(path: &Path) -> Result<Self, String> {
        let root = cache_root_for_path(path)?;
        let entries_dir = root.join("entries");
        fs::create_dir_all(&entries_dir)
            .map_err(|e| format!("failed to create cache root `{}`: {e}", entries_dir.display()))?;
        Ok(Self { root, entries_dir })
    }

    pub fn lookup(&self, key: &CacheKey) -> Result<Option<CacheHit>, String> {
        let id = key_id(key)?;
        let Some(mut entry) = self.read_entry(&id)? else {
            return Ok(None);
        };
        let artifact_path = self.entry_dir(&id).join(&entry.artifact_name);
        if !artifact_path.is_file() {
            return Ok(None);
        }
        entry.last_used_unix_ms = now_unix_ms();
        self.write_entry(&entry)?;
        Ok(Some(CacheHit {
            id,
            artifact_path,
            entry: public_entry(&entry),
        }))
    }

    pub fn store_bytes(
        &self,
        key: &CacheKey,
        artifact_kind: CacheArtifactKind,
        source_path: &Path,
        artifact_name: &str,
        bytes: &[u8],
    ) -> Result<CacheHit, String> {
        let id = key_id(key)?;
        let entry_dir = self.entry_dir(&id);
        fs::create_dir_all(&entry_dir).map_err(|e| {
            format!(
                "failed to create cache entry directory `{}`: {e}",
                entry_dir.display()
            )
        })?;
        let artifact_path = entry_dir.join(artifact_name);
        fs::write(&artifact_path, bytes)
            .map_err(|e| format!("failed to write cache artifact `{}`: {e}", artifact_path.display()))?;

        let now = now_unix_ms();
        let entry = StoredCacheEntry {
            schema_version: CACHE_SCHEMA_VERSION,
            id: id.clone(),
            key: key.clone(),
            artifact_kind,
            source_path: source_path.to_string_lossy().to_string(),
            artifact_name: artifact_name.to_string(),
            bytes: bytes.len() as u64,
            created_unix_ms: now,
            last_used_unix_ms: now,
        };
        self.write_entry(&entry)?;
        self.enforce_limits()?;

        Ok(CacheHit {
            id,
            artifact_path,
            entry: public_entry(&entry),
        })
    }

    pub fn store_file(
        &self,
        key: &CacheKey,
        artifact_kind: CacheArtifactKind,
        source_path: &Path,
        artifact_path: &Path,
    ) -> Result<CacheHit, String> {
        let bytes = fs::read(artifact_path)
            .map_err(|e| format!("failed to read cache source artifact `{}`: {e}", artifact_path.display()))?;
        let artifact_name = artifact_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("artifact.bin");
        self.store_bytes(key, artifact_kind, source_path, artifact_name, &bytes)
    }

    pub fn restore_to_path(&self, hit: &CacheHit, destination: &Path) -> Result<(), String> {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "failed to create restore directory `{}`: {e}",
                    parent.display()
                )
            })?;
        }
        fs::copy(&hit.artifact_path, destination).map_err(|e| {
            format!(
                "failed to restore cache artifact `{}` to `{}`: {e}",
                hit.artifact_path.display(),
                destination.display()
            )
        })?;
        Ok(())
    }

    pub fn status(&self, recent: usize) -> Result<CacheStatus, String> {
        let entries = self.list_entries()?;
        let entry_count = entries.len();
        let total_bytes = entries.iter().map(|entry| entry.bytes).sum();

        let mut by_kind: BTreeMap<CacheArtifactKind, (usize, u64)> = BTreeMap::new();
        for entry in &entries {
            let stats = by_kind.entry(entry.artifact_kind).or_insert((0, 0));
            stats.0 += 1;
            stats.1 = stats.1.saturating_add(entry.bytes);
        }

        let mut recent_entries: Vec<_> = entries
            .iter()
            .map(|entry| CacheStatusEntry {
                artifact_kind: entry.artifact_kind,
                source_path: entry.source_path.clone(),
                last_used_unix_ms: entry.last_used_unix_ms,
            })
            .collect();
        recent_entries.sort_by(|left, right| {
            right
                .last_used_unix_ms
                .cmp(&left.last_used_unix_ms)
                .then_with(|| left.source_path.cmp(&right.source_path))
        });
        recent_entries.truncate(recent);

        Ok(CacheStatus {
            root: self.root.clone(),
            entry_count,
            total_bytes,
            by_kind: by_kind
                .into_iter()
                .map(|(kind, (entries, bytes))| CacheStatusByKind {
                    kind,
                    entries,
                    bytes,
                })
                .collect(),
            recent_entries,
        })
    }

    fn entry_dir(&self, id: &str) -> PathBuf {
        self.entries_dir.join(id)
    }

    fn metadata_path(&self, id: &str) -> PathBuf {
        self.entry_dir(id).join("metadata.json")
    }

    fn read_entry(&self, id: &str) -> Result<Option<StoredCacheEntry>, String> {
        let metadata_path = self.metadata_path(id);
        if !metadata_path.is_file() {
            return Ok(None);
        }
        let metadata = fs::read_to_string(&metadata_path).map_err(|e| {
            format!(
                "failed to read cache metadata `{}`: {e}",
                metadata_path.display()
            )
        })?;
        let entry: StoredCacheEntry = serde_json::from_str(&metadata).map_err(|e| {
            format!(
                "failed to parse cache metadata `{}`: {e}",
                metadata_path.display()
            )
        })?;
        Ok(Some(entry))
    }

    fn write_entry(&self, entry: &StoredCacheEntry) -> Result<(), String> {
        let metadata_path = self.metadata_path(&entry.id);
        let json = serde_json::to_string_pretty(entry)
            .map_err(|e| format!("failed to serialize cache metadata: {e}"))?;
        fs::write(&metadata_path, json).map_err(|e| {
            format!(
                "failed to write cache metadata `{}`: {e}",
                metadata_path.display()
            )
        })
    }

    fn list_entries(&self) -> Result<Vec<StoredCacheEntry>, String> {
        let mut entries = Vec::new();
        if !self.entries_dir.is_dir() {
            return Ok(entries);
        }

        for child in fs::read_dir(&self.entries_dir).map_err(|e| {
            format!(
                "failed to inspect cache directory `{}`: {e}",
                self.entries_dir.display()
            )
        })? {
            let child = child.map_err(|e| format!("failed to read cache directory entry: {e}"))?;
            if !child.path().is_dir() {
                continue;
            }
            let metadata_path = child.path().join("metadata.json");
            if !metadata_path.is_file() {
                continue;
            }
            let metadata = fs::read_to_string(&metadata_path).map_err(|e| {
                format!(
                    "failed to read cache metadata `{}`: {e}",
                    metadata_path.display()
                )
            })?;
            let entry: StoredCacheEntry = serde_json::from_str(&metadata).map_err(|e| {
                format!(
                    "failed to parse cache metadata `{}`: {e}",
                    metadata_path.display()
                )
            })?;
            entries.push(entry);
        }

        Ok(entries)
    }

    fn enforce_limits(&self) -> Result<(), String> {
        let mut entries = self.list_entries()?;
        let mut total_bytes: u64 = entries.iter().map(|entry| entry.bytes).sum();

        entries.sort_by(|left, right| {
            left.last_used_unix_ms
                .cmp(&right.last_used_unix_ms)
                .then_with(|| left.id.cmp(&right.id))
        });

        while entries.len() > MAX_CACHE_ENTRIES || total_bytes > MAX_CACHE_BYTES {
            let Some(entry) = entries.first().cloned() else {
                break;
            };
            let entry_dir = self.entry_dir(&entry.id);
            if entry_dir.exists() {
                fs::remove_dir_all(&entry_dir).map_err(|e| {
                    format!(
                        "failed to evict cache entry `{}`: {e}",
                        entry_dir.display()
                    )
                })?;
            }
            total_bytes = total_bytes.saturating_sub(entry.bytes);
            entries.remove(0);
        }

        Ok(())
    }
}

pub fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hasher.write(bytes);
    format!("{:016x}", hasher.finish())
}

pub fn hash_serializable<T: Serialize>(value: &T) -> Result<String, String> {
    let bytes = serde_json::to_vec(value)
        .map_err(|e| format!("failed to serialize cache payload for hashing: {e}"))?;
    Ok(hash_bytes(&bytes))
}

pub fn default_cache_key(
    source_hash: String,
    semantic_hash: String,
    backend: RuntimeBackend,
    opt_level: u8,
    feature_signature: String,
) -> CacheKey {
    let host = host_runtime();
    CacheKey {
        source_hash,
        semantic_hash,
        backend,
        opt_level,
        runtime_abi: RUNTIME_ABI_VERSION,
        host_os: host.os,
        host_arch: host.arch,
        host_pointer_width: host.pointer_width,
        feature_signature,
    }
}

fn cache_root_for_path(path: &Path) -> Result<PathBuf, String> {
    let base = if path.is_dir() {
        path.to_path_buf()
    } else if path.file_name().and_then(|name| name.to_str()) == Some("agam.toml") {
        path.parent()
            .ok_or_else(|| format!("manifest `{}` has no parent directory", path.display()))?
            .to_path_buf()
    } else if let Some(parent) = path.parent() {
        parent.to_path_buf()
    } else {
        return Err(format!("cannot determine cache root for `{}`", path.display()));
    };
    Ok(base.join(".agam_cache"))
}

fn key_id(key: &CacheKey) -> Result<String, String> {
    hash_serializable(key)
}

fn public_entry(entry: &StoredCacheEntry) -> CacheEntry {
    CacheEntry {
        artifact_kind: entry.artifact_kind,
        source_path: entry.source_path.clone(),
        last_used_unix_ms: entry.last_used_unix_ms,
        bytes: entry.bytes,
    }
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(prefix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "agam_runtime_cache_{prefix}_{}_{}",
            std::process::id(),
            now_unix_ms()
        ));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn stores_and_restores_cached_bytes() {
        let dir = temp_dir("store");
        let cache = CacheStore::for_path(&dir).expect("create cache");
        let key = default_cache_key(
            "src".into(),
            "sem".into(),
            RuntimeBackend::Llvm,
            3,
            "feat=llvm".into(),
        );
        let hit = cache
            .store_bytes(
                &key,
                CacheArtifactKind::PortablePackage,
                &dir.join("sample.agam"),
                "sample.agpkg.json",
                b"{\"ok\":true}",
            )
            .expect("store cache entry");
        let restored = dir.join("out").join("sample.agpkg.json");
        cache
            .restore_to_path(&hit, &restored)
            .expect("restore cache entry");
        assert_eq!(fs::read(&restored).expect("read restored"), b"{\"ok\":true}");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn reports_status_by_kind() {
        let dir = temp_dir("status");
        let cache = CacheStore::for_path(&dir).expect("create cache");
        let key = default_cache_key(
            "src".into(),
            "sem".into(),
            RuntimeBackend::C,
            2,
            "feat=c".into(),
        );
        cache
            .store_bytes(
                &key,
                CacheArtifactKind::CSource,
                &dir.join("sample.agam"),
                "sample.c",
                b"int main(void) { return 0; }",
            )
            .expect("store cache entry");

        let status = cache.status(5).expect("read cache status");
        assert_eq!(status.entry_count, 1);
        assert_eq!(status.by_kind.len(), 1);
        assert_eq!(status.by_kind[0].kind, CacheArtifactKind::CSource);
        assert_eq!(status.recent_entries.len(), 1);
        let _ = fs::remove_dir_all(dir);
    }
}
