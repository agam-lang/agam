//! Portable Agam package format and helpers.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use agam_ast::Module;
use agam_ast::decl::DeclKind;
use agam_errors::{SourceFile, SourceId};
use agam_mir::ir::MirModule;
use agam_runtime::contract::{
    RUNTIME_ABI_VERSION, RuntimeBackend, RuntimeManifest, portable_runtime_manifest,
};
use serde::{Deserialize, Serialize};

/// First portable package format version.
pub const PACKAGE_FORMAT_VERSION: u32 = 1;
/// First SDK distribution manifest format version.
pub const SDK_DISTRIBUTION_FORMAT_VERSION: u32 = 1;
/// First source workspace manifest format version.
pub const WORKSPACE_MANIFEST_FORMAT_VERSION: u32 = 1;
/// First dependency lockfile format version.
pub const LOCKFILE_FORMAT_VERSION: u32 = 1;

/// The version compatibility policy for `agam.toml` manifests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestCompatibility {
    V1Stable,
    Unsupported(u32),
}

/// Check if a manifest version is supported by this compiler.
pub fn check_manifest_compatibility(version: u32) -> ManifestCompatibility {
    if version == WORKSPACE_MANIFEST_FORMAT_VERSION {
        ManifestCompatibility::V1Stable
    } else {
        ManifestCompatibility::Unsupported(version)
    }
}

/// A portable Agam package carrying verified MIR plus runtime metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortablePackage {
    pub format_version: u32,
    pub manifest: PackageManifest,
    pub runtime: RuntimeManifest,
    pub mir: MirModule,
}

/// A host-native Agam SDK distribution manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SdkDistributionManifest {
    pub format_version: u32,
    pub sdk_name: String,
    pub host_platform: String,
    pub compiler_binary: String,
    pub llvm_bundle_root: Option<String>,
    pub preferred_llvm_driver: Option<String>,
    pub supported_targets: Vec<SdkTargetProfile>,
    pub notes: Vec<String>,
}

/// A target profile carried by an SDK distribution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SdkTargetProfile {
    pub name: String,
    pub target_triple: String,
    pub backend: RuntimeBackend,
    pub sysroot_env: Option<String>,
    pub sdk_env: Option<String>,
}

/// High-level package metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageManifest {
    pub name: String,
    pub source_path: String,
    pub entry_function: String,
    pub preferred_backend: RuntimeBackend,
    pub verified_ir: VerifiedIrSummary,
    pub source_map: Vec<SourceMapEntry>,
    pub effects: EffectMetadata,
}

/// A summarized view of the verified IR attached to the package.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerifiedIrSummary {
    pub optimized: bool,
    pub function_count: usize,
    pub functions: Vec<VerifiedFunctionSummary>,
}

/// Per-function summary for package inspection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerifiedFunctionSummary {
    pub name: String,
    pub params: usize,
    pub basic_blocks: usize,
    pub instructions: usize,
}

/// Coarse source mapping for top-level declarations in the package.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceMapEntry {
    pub symbol: String,
    pub kind: SourceSymbolKind,
    pub line: usize,
    pub column: usize,
}

/// Kinds of declaration recorded in the coarse source map.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SourceSymbolKind {
    Function,
    Effect,
    Handler,
}

/// Package-facing effect metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EffectMetadata {
    pub declared_effects: Vec<String>,
    pub handlers: Vec<HandlerBinding>,
}

/// Declared effect handler attached to the package.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HandlerBinding {
    pub handler: String,
    pub effect: String,
}

/// Build a portable package from verified MIR and the parsed source module.
pub fn build_portable_package(
    source_path: &Path,
    source: &str,
    module: &Module,
    mir: &MirModule,
    preferred_backend: RuntimeBackend,
) -> PortablePackage {
    let runtime = portable_runtime_manifest(preferred_backend, true);
    let source_file = SourceFile::new(
        SourceId(0),
        source_path.to_string_lossy().to_string(),
        source.to_string(),
    );

    PortablePackage {
        format_version: PACKAGE_FORMAT_VERSION,
        manifest: PackageManifest {
            name: source_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("package")
                .to_string(),
            source_path: source_path.to_string_lossy().to_string(),
            entry_function: "main".to_string(),
            preferred_backend,
            verified_ir: verified_ir_summary(mir),
            source_map: collect_source_map(module, &source_file),
            effects: collect_effect_metadata(module),
        },
        runtime,
        mir: mir.clone(),
    }
}

/// Write a portable package to disk as pretty JSON.
pub fn write_package_to_path(path: &Path, package: &PortablePackage) -> Result<(), String> {
    let json = serde_json::to_string_pretty(package)
        .map_err(|e| format!("failed to serialize package: {e}"))?;
    std::fs::write(path, json)
        .map_err(|e| format!("failed to write package `{}`: {e}", path.display()))
}

/// Read a portable package from disk.
pub fn read_package_from_path(path: &Path) -> Result<PortablePackage, String> {
    let json = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read package `{}`: {e}", path.display()))?;
    let package = serde_json::from_str(&json)
        .map_err(|e| format!("failed to parse package `{}`: {e}", path.display()))?;
    Ok(package)
}

/// Write an SDK distribution manifest to disk as pretty JSON.
pub fn write_sdk_distribution_manifest_to_path(
    path: &Path,
    manifest: &SdkDistributionManifest,
) -> Result<(), String> {
    let json = serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("failed to serialize SDK distribution manifest: {e}"))?;
    std::fs::write(path, json).map_err(|e| {
        format!(
            "failed to write SDK distribution manifest `{}`: {e}",
            path.display()
        )
    })
}

/// Read an SDK distribution manifest from disk.
pub fn read_sdk_distribution_manifest_from_path(
    path: &Path,
) -> Result<SdkDistributionManifest, String> {
    let json = std::fs::read_to_string(path).map_err(|e| {
        format!(
            "failed to read SDK distribution manifest `{}`: {e}",
            path.display()
        )
    })?;
    let manifest = serde_json::from_str(&json).map_err(|e| {
        format!(
            "failed to parse SDK distribution manifest `{}`: {e}",
            path.display()
        )
    })?;
    Ok(manifest)
}

/// Default package output path for a given source file.
pub fn default_package_path(source_path: &Path) -> PathBuf {
    let stem = source_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("package");
    source_path.with_file_name(format!("{stem}.agpkg.json"))
}

/// Source-level package and workspace contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceManifest {
    #[serde(default = "default_workspace_manifest_format_version")]
    pub format_version: u32,
    #[serde(alias = "package")]
    pub project: ProjectManifest,
    #[serde(default)]
    pub workspace: WorkspaceDefinition,
    #[serde(default)]
    pub dependencies: BTreeMap<String, DependencySpec>,
    #[serde(default, rename = "dev-dependencies")]
    pub dev_dependencies: BTreeMap<String, DependencySpec>,
    #[serde(default, rename = "build-dependencies")]
    pub build_dependencies: BTreeMap<String, DependencySpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toolchain: Option<ToolchainRequirement>,
    #[serde(default)]
    pub environments: BTreeMap<String, EnvironmentSpec>,
}

/// The first-party project section carried by `agam.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectManifest {
    pub name: String,
    pub version: String,
    pub agam: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
}

/// Optional multi-member workspace metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceDefinition {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<String>,
    #[serde(
        default,
        rename = "default-members",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub default_members: Vec<String>,
}

/// A single source dependency declaration.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencySpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub optional: bool,
}

/// Toolchain and runtime expectations attached to a source workspace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolchainRequirement {
    pub agam: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdk: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_abi: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_backend: Option<RuntimeBackend>,
}

/// A named project-level environment definition.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnvironmentSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compiler: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdk: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_backend: Option<RuntimeBackend>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub profiles: Vec<String>,
}

/// Resolved dependency graph contract stored in `agam.lock`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceLockfile {
    #[serde(default = "default_lockfile_format_version")]
    pub format_version: u32,
    pub workspace: LockedWorkspace,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<LockedPackage>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub environments: BTreeMap<String, LockedEnvironment>,
}

/// Root workspace identity stored in the lockfile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockedWorkspace {
    pub name: String,
    pub version: String,
}

/// A resolved package entry recorded in the lockfile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockedPackage {
    pub name: String,
    pub version: String,
    pub source: LockedPackageSource,
    pub content_hash: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<String>,
}

/// The provenance of a resolved package.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockedPackageSource {
    pub kind: String,
    pub location: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
}

/// A resolved environment view recorded in the lockfile.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockedEnvironment {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compiler: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdk: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_backend: Option<RuntimeBackend>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<String>,
}

/// Shared workspace layout resolved from a manifest root, source file, or directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceLayout {
    pub root: PathBuf,
    pub manifest_path: Option<PathBuf>,
    pub project_name: String,
    pub entry_file: PathBuf,
    pub source_files: Vec<PathBuf>,
    pub test_files: Vec<PathBuf>,
}

/// Parsed workspace metadata paired with the resolved first-party layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSession {
    pub layout: WorkspaceLayout,
    pub manifest: Option<WorkspaceManifest>,
    pub members: Vec<WorkspaceSession>,
}

/// Fingerprint for one manifest, source, or test file in a workspace snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceFileSnapshot {
    pub path: PathBuf,
    pub content_hash: String,
    pub bytes: u64,
    pub modified_unix_seconds: Option<u64>,
}

/// Point-in-time workspace manifest/source/test snapshot for future daemon reuse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSnapshot {
    pub session: WorkspaceSession,
    pub manifests: Vec<WorkspaceFileSnapshot>,
    pub source_files: Vec<WorkspaceFileSnapshot>,
    pub test_files: Vec<WorkspaceFileSnapshot>,
}

impl WorkspaceSnapshot {
    /// Check if another snapshot differs from this one in any way.
    pub fn is_stale(&self, other: &WorkspaceSnapshot) -> bool {
        let diff = diff_workspace_snapshots(self, other);
        !diff.added_files.is_empty()
            || !diff.changed_files.is_empty()
            || !diff.removed_files.is_empty()
    }
}

/// Diff between two workspace snapshots.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkspaceSnapshotDiff {
    pub added_files: Vec<PathBuf>,
    pub changed_files: Vec<PathBuf>,
    pub removed_files: Vec<PathBuf>,
    pub unchanged_files: Vec<PathBuf>,
}

/// Create the first-party scaffold manifest for a new workspace.
pub fn scaffold_workspace_manifest(project_name: &str) -> WorkspaceManifest {
    WorkspaceManifest {
        format_version: WORKSPACE_MANIFEST_FORMAT_VERSION,
        project: ProjectManifest {
            name: project_name.to_string(),
            version: "0.1.0".into(),
            agam: "0.1".into(),
            entry: Some("src/main.agam".into()),
            keywords: Vec::new(),
        },
        workspace: WorkspaceDefinition::default(),
        dependencies: BTreeMap::new(),
        dev_dependencies: BTreeMap::new(),
        build_dependencies: BTreeMap::new(),
        toolchain: Some(ToolchainRequirement {
            agam: "0.1".into(),
            sdk: None,
            target: None,
            runtime_abi: Some(RUNTIME_ABI_VERSION),
            preferred_backend: None,
        }),
        environments: BTreeMap::new(),
    }
}

/// Default source manifest path for a workspace root.
pub fn default_manifest_path(root: &Path) -> PathBuf {
    root.join("agam.toml")
}

/// Read a source workspace manifest from TOML.
pub fn read_workspace_manifest_from_path(path: &Path) -> Result<WorkspaceManifest, String> {
    let manifest = std::fs::read_to_string(path).map_err(|e| {
        format!(
            "failed to read workspace manifest `{}`: {e}",
            path.display()
        )
    })?;
    let manifest: WorkspaceManifest = toml::from_str(&manifest).map_err(|e| {
        format!(
            "failed to parse workspace manifest `{}`: {e}",
            path.display()
        )
    })?;
    let root = path.parent().ok_or_else(|| {
        format!(
            "workspace manifest `{}` has no parent directory",
            path.display()
        )
    })?;
    validate_workspace_manifest(root, &manifest)?;
    Ok(manifest)
}

/// Validate the user-facing `agam.toml` workspace contract.
pub fn validate_workspace_manifest(
    root: &Path,
    manifest: &WorkspaceManifest,
) -> Result<(), String> {
    if manifest.format_version != WORKSPACE_MANIFEST_FORMAT_VERSION {
        return Err(format!(
            "unsupported `format_version` `{}`; expected `{WORKSPACE_MANIFEST_FORMAT_VERSION}`",
            manifest.format_version
        ));
    }

    validate_required_field(&manifest.project.name, "`project.name`")?;
    validate_required_field(&manifest.project.version, "`project.version`")?;
    validate_required_field(&manifest.project.agam, "`project.agam`")?;

    if let Some(entry) = manifest.project.entry.as_deref() {
        workspace_relative_path(root, entry, "`project.entry`")?;
    }

    if let Some(toolchain) = manifest.toolchain.as_ref() {
        validate_required_field(&toolchain.agam, "`toolchain.agam`")?;
        validate_optional_field(toolchain.sdk.as_deref(), "`toolchain.sdk`")?;
        validate_optional_field(toolchain.target.as_deref(), "`toolchain.target`")?;
        if matches!(toolchain.runtime_abi, Some(0)) {
            return Err("`toolchain.runtime_abi` must be greater than zero".into());
        }
    }

    let members =
        validate_workspace_member_list(root, &manifest.workspace.members, "workspace.members")?;
    let default_members = validate_workspace_member_list(
        root,
        &manifest.workspace.default_members,
        "workspace.default-members",
    )?;
    for member in default_members {
        if !members.contains(&member) {
            return Err(format!(
                "`workspace.default-members` entry `{member}` must also appear in `workspace.members`"
            ));
        }
    }

    validate_dependency_table(root, "dependencies", &manifest.dependencies)?;
    validate_dependency_table(root, "dev-dependencies", &manifest.dev_dependencies)?;
    validate_dependency_table(root, "build-dependencies", &manifest.build_dependencies)?;
    validate_environments(&manifest.environments)?;

    Ok(())
}

/// Write a source workspace manifest to TOML.
pub fn write_workspace_manifest_to_path(
    path: &Path,
    manifest: &WorkspaceManifest,
) -> Result<(), String> {
    let toml = toml::to_string_pretty(manifest)
        .map_err(|e| format!("failed to serialize workspace manifest: {e}"))?;
    std::fs::write(path, toml).map_err(|e| {
        format!(
            "failed to write workspace manifest `{}`: {e}",
            path.display()
        )
    })
}

/// Resolve the canonical workspace layout from a directory, manifest path, or source file.
pub fn resolve_workspace_layout(path: Option<PathBuf>) -> Result<WorkspaceLayout, String> {
    Ok(resolve_workspace_session(path)?.layout)
}

/// Resolve a parsed workspace session from a directory, manifest path, or source file.
pub fn resolve_workspace_session(path: Option<PathBuf>) -> Result<WorkspaceSession, String> {
    let hint = match path {
        Some(path) => path,
        None => {
            std::env::current_dir().map_err(|e| format!("failed to read current directory: {e}"))?
        }
    };

    resolve_workspace_session_from_path(&hint)
}

/// Discover and resolve all explicit members in a workspace package.
pub fn resolve_workspace_members(
    session: &WorkspaceSession,
) -> Result<Vec<WorkspaceSession>, String> {
    Ok(session.members.clone())
}

/// Expand user-provided formatter/test inputs into concrete Agam source files.
pub fn expand_agam_inputs(files: Vec<PathBuf>) -> Result<Vec<PathBuf>, String> {
    let inputs = if files.is_empty() {
        vec![
            std::env::current_dir()
                .map_err(|e| format!("could not read current directory: {e}"))?,
        ]
    } else {
        files
    };

    let mut expanded = Vec::new();
    for input in inputs {
        expand_agam_input(&input, &mut expanded)?;
    }
    expanded.sort();
    expanded.dedup();
    Ok(expanded)
}

/// Capture a fingerprinted workspace snapshot for future incremental invalidation work.
pub fn snapshot_workspace(path: Option<PathBuf>) -> Result<WorkspaceSnapshot, String> {
    Ok(snapshot_workspace_session(resolve_workspace_session(
        path,
    )?)?)
}

/// Capture a fingerprinted workspace snapshot from an explicit filesystem path.
pub fn snapshot_workspace_from_path(path: &Path) -> Result<WorkspaceSnapshot, String> {
    Ok(snapshot_workspace_session(
        resolve_workspace_session_from_path(path)?,
    )?)
}

/// Fingerprint a resolved workspace session.
pub fn snapshot_workspace_session(session: WorkspaceSession) -> Result<WorkspaceSnapshot, String> {
    let manifests = snapshot_workspace_manifests(&session)?;
    let source_files = session
        .layout
        .source_files
        .iter()
        .map(|path| snapshot_file(path))
        .collect::<Result<Vec<_>, _>>()?;
    let test_files = session
        .layout
        .test_files
        .iter()
        .map(|path| snapshot_file(path))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(WorkspaceSnapshot {
        session,
        manifests,
        source_files,
        test_files,
    })
}

/// Compare two workspace snapshots so later daemon work can invalidate only changed files.
pub fn diff_workspace_snapshots(
    previous: &WorkspaceSnapshot,
    next: &WorkspaceSnapshot,
) -> WorkspaceSnapshotDiff {
    let previous_files = tracked_snapshot_hashes(previous);
    let mut next_files = tracked_snapshot_hashes(next);
    let mut diff = WorkspaceSnapshotDiff::default();

    for (path, previous_hash) in previous_files {
        match next_files.remove(&path) {
            Some(next_hash) if next_hash == previous_hash => diff.unchanged_files.push(path),
            Some(_) => diff.changed_files.push(path),
            None => diff.removed_files.push(path),
        }
    }

    for (path, _) in next_files {
        diff.added_files.push(path);
    }

    diff
}

/// Resolve the canonical workspace layout from an explicit filesystem path.
pub fn resolve_workspace_layout_from_path(path: &Path) -> Result<WorkspaceLayout, String> {
    Ok(resolve_workspace_session_from_path(path)?.layout)
}

/// Resolve a parsed workspace session from an explicit filesystem path.
pub fn resolve_workspace_session_from_path(path: &Path) -> Result<WorkspaceSession, String> {
    if path.is_file() {
        if path.file_name().and_then(|name| name.to_str()) == Some("agam.toml") {
            let root = path
                .parent()
                .ok_or_else(|| format!("manifest `{}` has no parent directory", path.display()))?;
            return workspace_session_from_root(root, Some(path.to_path_buf()), None);
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("agam") {
            return Err(format!(
                "`{}` is not an Agam source file or `agam.toml` manifest",
                path.display()
            ));
        }
        let parent = path
            .parent()
            .ok_or_else(|| format!("source file `{}` has no parent directory", path.display()))?;
        let manifest = find_workspace_manifest(parent);
        let root = manifest
            .as_ref()
            .and_then(|manifest_path| manifest_path.parent())
            .unwrap_or(parent)
            .to_path_buf();
        return workspace_session_from_root(&root, manifest, Some(path.to_path_buf()));
    }

    if !path.exists() {
        return Err(format!("`{}` does not exist", path.display()));
    }
    if !path.is_dir() {
        return Err(format!("`{}` is not a directory", path.display()));
    }

    let manifest = find_workspace_manifest(path);
    let root = manifest
        .as_ref()
        .and_then(|manifest_path| manifest_path.parent())
        .unwrap_or(path)
        .to_path_buf();
    workspace_session_from_root(&root, manifest, None)
}

/// Default dependency lockfile path for a workspace root.
pub fn default_lockfile_path(root: &Path) -> PathBuf {
    root.join("agam.lock")
}

/// Read a dependency lockfile from TOML.
pub fn read_lockfile_from_path(path: &Path) -> Result<WorkspaceLockfile, String> {
    let lockfile = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read lockfile `{}`: {e}", path.display()))?;
    toml::from_str(&lockfile)
        .map_err(|e| format!("failed to parse lockfile `{}`: {e}", path.display()))
}

/// Write a dependency lockfile to TOML.
pub fn write_lockfile_to_path(path: &Path, lockfile: &WorkspaceLockfile) -> Result<(), String> {
    let toml = toml::to_string_pretty(lockfile)
        .map_err(|e| format!("failed to serialize lockfile: {e}"))?;
    std::fs::write(path, toml)
        .map_err(|e| format!("failed to write lockfile `{}`: {e}", path.display()))
}

fn verified_ir_summary(mir: &MirModule) -> VerifiedIrSummary {
    VerifiedIrSummary {
        optimized: true,
        function_count: mir.functions.len(),
        functions: mir
            .functions
            .iter()
            .map(|function| VerifiedFunctionSummary {
                name: function.name.clone(),
                params: function.params.len(),
                basic_blocks: function.blocks.len(),
                instructions: function
                    .blocks
                    .iter()
                    .map(|block| block.instructions.len())
                    .sum(),
            })
            .collect(),
    }
}

fn collect_agam_files(path: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    if path.is_file() {
        if path.extension().and_then(|ext| ext.to_str()) == Some("agam") {
            out.push(path.to_path_buf());
        }
        return Ok(());
    }

    if !path.is_dir() {
        return Err(format!("`{}` is not a file or directory", path.display()));
    }

    for entry in std::fs::read_dir(path)
        .map_err(|e| format!("could not read directory `{}`: {e}", path.display()))?
    {
        let entry = entry.map_err(|e| format!("could not read directory entry: {e}"))?;
        let child = entry.path();
        if child.is_dir() {
            collect_agam_files(&child, out)?;
        } else if child.extension().and_then(|ext| ext.to_str()) == Some("agam") {
            out.push(child);
        }
    }

    Ok(())
}

fn snapshot_file(path: &Path) -> Result<WorkspaceFileSnapshot, String> {
    let bytes = std::fs::read(path)
        .map_err(|e| format!("failed to read workspace file `{}`: {e}", path.display()))?;
    let metadata = std::fs::metadata(path)
        .map_err(|e| format!("failed to read metadata for `{}`: {e}", path.display()))?;
    let modified_unix_seconds = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs());

    Ok(WorkspaceFileSnapshot {
        path: path.to_path_buf(),
        content_hash: agam_runtime::cache::hash_bytes(&bytes),
        bytes: bytes.len() as u64,
        modified_unix_seconds,
    })
}

fn tracked_snapshot_hashes(snapshot: &WorkspaceSnapshot) -> BTreeMap<PathBuf, String> {
    let mut files = BTreeMap::new();
    for manifest in &snapshot.manifests {
        files.insert(manifest.path.clone(), manifest.content_hash.clone());
    }
    for file in &snapshot.source_files {
        files.insert(file.path.clone(), file.content_hash.clone());
    }
    for file in &snapshot.test_files {
        files.insert(file.path.clone(), file.content_hash.clone());
    }
    files
}

fn expand_agam_input(path: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    if path.is_file() {
        if path.file_name().and_then(|name| name.to_str()) == Some("agam.toml") {
            let layout = resolve_workspace_layout(Some(path.to_path_buf()))?;
            out.extend(layout.source_files);
            out.extend(layout.test_files);
            return Ok(());
        }
        if path.extension().and_then(|ext| ext.to_str()) == Some("agam") {
            out.push(path.to_path_buf());
            return Ok(());
        }
        return Err(format!(
            "`{}` is not an Agam source file or `agam.toml` manifest",
            path.display()
        ));
    }

    if !path.is_dir() {
        return Err(format!("`{}` is not a file or directory", path.display()));
    }

    match resolve_workspace_layout_from_path(path) {
        Ok(layout) => {
            out.extend(layout.source_files);
            out.extend(layout.test_files);
            Ok(())
        }
        Err(error) => {
            if find_workspace_manifest(path).is_some()
                || path.join("agam.toml").is_file()
                || path.join("src").join("main.agam").is_file()
            {
                return Err(error);
            }
            collect_agam_files(path, out)
        }
    }
}

fn find_workspace_manifest(start: &Path) -> Option<PathBuf> {
    for ancestor in start.ancestors() {
        let manifest = ancestor.join("agam.toml");
        if manifest.is_file() {
            return Some(manifest);
        }
    }
    None
}

fn workspace_session_from_root(
    root: &Path,
    manifest_path: Option<PathBuf>,
    entry_override: Option<PathBuf>,
) -> Result<WorkspaceSession, String> {
    workspace_session_from_root_inner(root, manifest_path, entry_override, true)
}

fn workspace_session_from_root_inner(
    root: &Path,
    manifest_path: Option<PathBuf>,
    entry_override: Option<PathBuf>,
    include_members: bool,
) -> Result<WorkspaceSession, String> {
    let manifest = manifest_path
        .as_ref()
        .map(|path| read_workspace_manifest_from_path(path))
        .transpose()?;
    let project_name = manifest
        .as_ref()
        .map(|manifest| manifest.project.name.clone())
        .or_else(|| {
            root.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.to_string())
        })
        .unwrap_or_else(|| "agam-workspace".into());
    let entry_file = match entry_override {
        Some(path) => path,
        None => manifest
            .as_ref()
            .map(|manifest| manifest_entry_path(root, manifest))
            .transpose()?
            .unwrap_or_else(|| root.join("src").join("main.agam")),
    };
    if !entry_file.is_file() {
        return Err(format!(
            "could not find entry file `{}`; create a project with `agamc new <name>` or pass an explicit `.agam` file",
            entry_file.display()
        ));
    }

    let mut source_files = collect_workspace_source_files(root, &entry_file)?;
    let mut test_files = collect_workspace_test_files(root)?;
    let members = if include_members {
        manifest
            .as_ref()
            .map(|manifest| resolve_workspace_member_sessions(root, manifest))
            .transpose()?
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    for member in &members {
        source_files.extend(member.layout.source_files.iter().cloned());
        test_files.extend(member.layout.test_files.iter().cloned());
    }
    source_files.sort();
    source_files.dedup();
    test_files.sort();
    test_files.dedup();

    Ok(WorkspaceSession {
        layout: WorkspaceLayout {
            root: root.to_path_buf(),
            manifest_path,
            project_name,
            entry_file,
            source_files,
            test_files,
        },
        manifest,
        members,
    })
}

fn snapshot_workspace_manifests(
    session: &WorkspaceSession,
) -> Result<Vec<WorkspaceFileSnapshot>, String> {
    let mut manifest_paths = Vec::new();
    collect_workspace_manifest_paths(session, &mut manifest_paths);
    manifest_paths.sort();
    manifest_paths.dedup();
    manifest_paths
        .iter()
        .map(|path| snapshot_file(path))
        .collect::<Result<Vec<_>, _>>()
}

fn collect_workspace_manifest_paths(session: &WorkspaceSession, out: &mut Vec<PathBuf>) {
    if let Some(path) = session.layout.manifest_path.as_ref() {
        out.push(path.clone());
    }
    for member in &session.members {
        collect_workspace_manifest_paths(member, out);
    }
}

fn collect_workspace_source_files(root: &Path, entry_file: &Path) -> Result<Vec<PathBuf>, String> {
    let mut source_files = Vec::new();
    let src_dir = root.join("src");
    if src_dir.is_dir() {
        collect_agam_files(&src_dir, &mut source_files)?;
    }
    if !source_files.iter().any(|file| file == entry_file) {
        source_files.push(entry_file.to_path_buf());
    }
    source_files.sort();
    source_files.dedup();
    Ok(source_files)
}

fn collect_workspace_test_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut test_files = Vec::new();
    let tests_dir = root.join("tests");
    if tests_dir.is_dir() {
        collect_agam_files(&tests_dir, &mut test_files)?;
    }
    test_files.sort();
    test_files.dedup();
    Ok(test_files)
}

fn resolve_workspace_member_sessions(
    root: &Path,
    manifest: &WorkspaceManifest,
) -> Result<Vec<WorkspaceSession>, String> {
    let mut members = Vec::new();
    for member in &manifest.workspace.members {
        let member_path = workspace_relative_path(root, member, "workspace.members")?;
        if !member_path.is_dir() {
            return Err(format!(
                "workspace member `{}` does not exist or is not a directory",
                member_path.display()
            ));
        }
        let member_manifest_path = default_manifest_path(&member_path);
        if !member_manifest_path.is_file() {
            return Err(format!(
                "workspace member `{}` is missing `agam.toml`",
                member_path.display()
            ));
        }
        let member_session = workspace_session_from_root_inner(
            &member_path,
            Some(member_manifest_path),
            None,
            false,
        )?;
        members.push(member_session);
    }
    members.sort_by(|left, right| left.layout.root.cmp(&right.layout.root));
    Ok(members)
}

fn manifest_entry_path(root: &Path, manifest: &WorkspaceManifest) -> Result<PathBuf, String> {
    let entry = manifest.project.entry.as_deref().unwrap_or("src/main.agam");
    workspace_relative_path(root, entry, "`project.entry`")
}

fn workspace_relative_path(
    root: &Path,
    relative: &str,
    field_name: &str,
) -> Result<PathBuf, String> {
    let path = Path::new(relative);
    if relative.trim().is_empty() {
        return Err(format!("{field_name} cannot be empty"));
    }
    if path.is_absolute() {
        return Err(format!(
            "{field_name} must stay relative to the workspace root; got `{}`",
            relative
        ));
    }
    if path.components().any(|component| {
        matches!(
            component,
            std::path::Component::ParentDir
                | std::path::Component::RootDir
                | std::path::Component::Prefix(_)
        )
    }) {
        return Err(format!(
            "{field_name} must stay inside the workspace root; got `{}`",
            relative
        ));
    }
    Ok(root.join(path))
}

fn validate_dependency_table(
    root: &Path,
    table_name: &str,
    dependencies: &BTreeMap<String, DependencySpec>,
) -> Result<(), String> {
    for (name, spec) in dependencies {
        if name.trim().is_empty() {
            return Err(format!("dependency name in `{table_name}` cannot be empty"));
        }

        let field_prefix = format!("`{table_name}.{name}`");
        let has_version =
            validate_optional_field(spec.version.as_deref(), &format!("{field_prefix}.version"))?;
        let has_registry = validate_optional_field(
            spec.registry.as_deref(),
            &format!("{field_prefix}.registry"),
        )?;
        let has_path = if let Some(path) = spec.path.as_deref() {
            validate_required_field(path, &format!("{field_prefix}.path"))?;
            workspace_relative_path(root, path, &format!("{field_prefix}.path"))?;
            true
        } else {
            false
        };
        let has_git = validate_optional_field(spec.git.as_deref(), &format!("{field_prefix}.git"))?;
        let has_rev = validate_optional_field(spec.rev.as_deref(), &format!("{field_prefix}.rev"))?;
        let has_branch =
            validate_optional_field(spec.branch.as_deref(), &format!("{field_prefix}.branch"))?;
        validate_optional_field(spec.package.as_deref(), &format!("{field_prefix}.package"))?;

        if has_branch && has_rev {
            return Err(format!("{field_prefix} cannot set both `branch` and `rev`"));
        }
        if (has_branch || has_rev) && !has_git {
            return Err(format!(
                "{field_prefix} requires `git` when `branch` or `rev` is set"
            ));
        }
        if has_path && has_git {
            return Err(format!(
                "{field_prefix} cannot mix `path` and `git` sources"
            ));
        }
        if has_path && has_registry {
            return Err(format!(
                "{field_prefix} cannot mix `path` and `registry` sources"
            ));
        }
        if has_git && has_registry {
            return Err(format!(
                "{field_prefix} cannot mix `git` and `registry` sources"
            ));
        }
        if !(has_version || has_registry || has_path || has_git) {
            return Err(format!(
                "{field_prefix} must declare at least one source selector (`version`, `path`, `git`, or `registry`)"
            ));
        }

        validate_string_list(&spec.features, &format!("{field_prefix}.features"))?;
    }

    Ok(())
}

fn validate_environments(environments: &BTreeMap<String, EnvironmentSpec>) -> Result<(), String> {
    for (name, environment) in environments {
        if name.trim().is_empty() {
            return Err("environment name cannot be empty".into());
        }

        let field_prefix = format!("`environments.{name}`");
        let has_compiler = validate_optional_field(
            environment.compiler.as_deref(),
            &format!("{field_prefix}.compiler"),
        )?;
        let has_sdk =
            validate_optional_field(environment.sdk.as_deref(), &format!("{field_prefix}.sdk"))?;
        let has_target = validate_optional_field(
            environment.target.as_deref(),
            &format!("{field_prefix}.target"),
        )?;
        validate_string_list(&environment.profiles, &format!("{field_prefix}.profiles"))?;

        if !(has_compiler
            || has_sdk
            || has_target
            || environment.preferred_backend.is_some()
            || !environment.profiles.is_empty())
        {
            return Err(format!(
                "{field_prefix} must set at least one compiler, sdk, target, preferred backend, or profile"
            ));
        }
    }

    Ok(())
}

fn validate_workspace_member_list(
    root: &Path,
    members: &[String],
    field_name: &str,
) -> Result<BTreeSet<String>, String> {
    let mut seen = BTreeSet::new();
    for (index, member) in members.iter().enumerate() {
        let item_field = format!("`{field_name}[{index}]`");
        validate_required_field(member, &item_field)?;
        workspace_relative_path(root, member, &item_field)?;
        if !seen.insert(member.clone()) {
            return Err(format!(
                "`{field_name}` contains duplicate entry `{member}`"
            ));
        }
    }
    Ok(seen)
}

fn validate_required_field(value: &str, field_name: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{field_name} cannot be empty"));
    }
    Ok(())
}

fn validate_optional_field(value: Option<&str>, field_name: &str) -> Result<bool, String> {
    if let Some(value) = value {
        validate_required_field(value, field_name)?;
        return Ok(true);
    }
    Ok(false)
}

fn validate_string_list(values: &[String], field_name: &str) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        let item_field = format!("`{field_name}[{index}]`");
        validate_required_field(value, &item_field)?;
        if !seen.insert(value.clone()) {
            return Err(format!("{field_name} contains duplicate entry `{value}`"));
        }
    }
    Ok(())
}

fn collect_source_map(module: &Module, source_file: &SourceFile) -> Vec<SourceMapEntry> {
    let mut entries = Vec::new();

    for decl in &module.declarations {
        match &decl.kind {
            DeclKind::Function(function) => {
                let (line, column) = source_file.offset_to_line_col(function.span.start as usize);
                entries.push(SourceMapEntry {
                    symbol: function.name.name.clone(),
                    kind: SourceSymbolKind::Function,
                    line: line + 1,
                    column: column + 1,
                });
            }
            DeclKind::Effect(effect) => {
                let (line, column) = source_file.offset_to_line_col(effect.span.start as usize);
                entries.push(SourceMapEntry {
                    symbol: effect.name.name.clone(),
                    kind: SourceSymbolKind::Effect,
                    line: line + 1,
                    column: column + 1,
                });
            }
            DeclKind::Handler(handler) => {
                let (line, column) = source_file.offset_to_line_col(handler.span.start as usize);
                entries.push(SourceMapEntry {
                    symbol: handler.name.name.clone(),
                    kind: SourceSymbolKind::Handler,
                    line: line + 1,
                    column: column + 1,
                });
            }
            _ => {}
        }
    }

    entries
}

fn collect_effect_metadata(module: &Module) -> EffectMetadata {
    let mut declared_effects = Vec::new();
    let mut handlers = Vec::new();

    for decl in &module.declarations {
        match &decl.kind {
            DeclKind::Effect(effect) => declared_effects.push(effect.name.name.clone()),
            DeclKind::Handler(handler) => handlers.push(HandlerBinding {
                handler: handler.name.name.clone(),
                effect: handler.effect_name.name.clone(),
            }),
            _ => {}
        }
    }

    EffectMetadata {
        declared_effects,
        handlers,
    }
}

fn default_workspace_manifest_format_version() -> u32 {
    WORKSPACE_MANIFEST_FORMAT_VERSION
}

fn default_lockfile_format_version() -> u32 {
    LOCKFILE_FORMAT_VERSION
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use agam_runtime::contract::RUNTIME_ABI_VERSION;

    fn sample_source() -> &'static str {
        "@lang.advance\nfn main() -> i32 {\n    return 0;\n}\n"
    }

    fn temp_dir(prefix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "agam_pkg_{prefix}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time should move forward")
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn builds_package_with_verified_ir_and_source_map() {
        let tokens = agam_lexer::tokenize(sample_source(), SourceId(0));
        let module = agam_parser::parse(tokens, SourceId(0)).expect("source should parse");
        let mut hir_lowering = agam_hir::lower::HirLowering::new();
        let hir = hir_lowering.lower_module(&module);
        let mut mir_lowering = agam_mir::lower::MirLowering::new();
        let mut mir = mir_lowering.lower_module(&hir);
        let _ = agam_mir::opt::optimize_module(&mut mir);

        let package = build_portable_package(
            Path::new("sample.agam"),
            sample_source(),
            &module,
            &mir,
            RuntimeBackend::Jit,
        );

        assert_eq!(package.format_version, PACKAGE_FORMAT_VERSION);
        assert_eq!(package.manifest.name, "sample");
        assert_eq!(package.manifest.entry_function, "main");
        assert_eq!(package.manifest.verified_ir.function_count, 1);
        assert_eq!(package.manifest.source_map[0].symbol, "main");
    }

    #[test]
    fn round_trips_package_json() {
        let package = PortablePackage {
            format_version: PACKAGE_FORMAT_VERSION,
            manifest: PackageManifest {
                name: "sample".into(),
                source_path: "sample.agam".into(),
                entry_function: "main".into(),
                preferred_backend: RuntimeBackend::Jit,
                verified_ir: VerifiedIrSummary {
                    optimized: true,
                    function_count: 0,
                    functions: Vec::new(),
                },
                source_map: Vec::new(),
                effects: EffectMetadata {
                    declared_effects: Vec::new(),
                    handlers: Vec::new(),
                },
            },
            runtime: portable_runtime_manifest(RuntimeBackend::Jit, true),
            mir: MirModule {
                functions: Vec::new(),
            },
        };

        let json = serde_json::to_string(&package).expect("serialize package");
        let decoded: PortablePackage = serde_json::from_str(&json).expect("deserialize package");
        assert_eq!(decoded.format_version, PACKAGE_FORMAT_VERSION);
        assert_eq!(decoded.manifest.name, "sample");
        assert_eq!(
            decoded.runtime.requirements.preferred_backend,
            RuntimeBackend::Jit
        );
    }

    #[test]
    fn round_trips_sdk_distribution_manifest_json() {
        let manifest = SdkDistributionManifest {
            format_version: SDK_DISTRIBUTION_FORMAT_VERSION,
            sdk_name: "agam-sdk".into(),
            host_platform: "windows-x86_64".into(),
            compiler_binary: "bin/agamc.exe".into(),
            llvm_bundle_root: Some("toolchains/llvm".into()),
            preferred_llvm_driver: Some("toolchains/llvm/windows-x86_64/bin/clang.exe".into()),
            supported_targets: vec![
                SdkTargetProfile {
                    name: "host-native".into(),
                    target_triple: "x86_64-pc-windows-msvc".into(),
                    backend: RuntimeBackend::Llvm,
                    sysroot_env: None,
                    sdk_env: None,
                },
                SdkTargetProfile {
                    name: "android-arm64".into(),
                    target_triple: "aarch64-linux-android21".into(),
                    backend: RuntimeBackend::Llvm,
                    sysroot_env: Some("AGAM_LLVM_SYSROOT".into()),
                    sdk_env: None,
                },
            ],
            notes: vec!["native llvm is the preferred production backend".into()],
        };

        let json = serde_json::to_string(&manifest).expect("serialize sdk distribution manifest");
        let decoded: SdkDistributionManifest =
            serde_json::from_str(&json).expect("deserialize sdk distribution manifest");
        assert_eq!(decoded.format_version, SDK_DISTRIBUTION_FORMAT_VERSION);
        assert_eq!(decoded.host_platform, "windows-x86_64");
        assert_eq!(decoded.supported_targets.len(), 2);
        assert_eq!(
            decoded.supported_targets[1].target_triple,
            "aarch64-linux-android21"
        );
    }

    #[test]
    fn scaffold_workspace_manifest_uses_first_party_defaults() {
        let manifest = scaffold_workspace_manifest("hello-agam");
        assert_eq!(manifest.format_version, WORKSPACE_MANIFEST_FORMAT_VERSION);
        assert_eq!(manifest.project.name, "hello-agam");
        assert_eq!(manifest.project.entry.as_deref(), Some("src/main.agam"));
        assert_eq!(
            manifest
                .toolchain
                .as_ref()
                .and_then(|toolchain| toolchain.runtime_abi),
            Some(RUNTIME_ABI_VERSION)
        );
    }

    #[test]
    fn round_trips_workspace_manifest_toml() {
        let mut manifest = scaffold_workspace_manifest("tensor-lab");
        manifest.workspace.members = vec!["packages/core".into(), "packages/models".into()];
        manifest.dependencies.insert(
            "tensor-core".into(),
            DependencySpec {
                version: Some("^0.4".into()),
                registry: Some("agam".into()),
                features: vec!["simd".into()],
                ..DependencySpec::default()
            },
        );
        manifest.dev_dependencies.insert(
            "lint-kit".into(),
            DependencySpec {
                git: Some("https://github.com/agam-lang/lint-kit".into()),
                branch: Some("main".into()),
                ..DependencySpec::default()
            },
        );
        manifest.environments.insert(
            "android-arm64".into(),
            EnvironmentSpec {
                sdk: Some("android-r26d".into()),
                target: Some("aarch64-linux-android21".into()),
                preferred_backend: Some(RuntimeBackend::Llvm),
                profiles: vec!["release".into()],
                ..EnvironmentSpec::default()
            },
        );

        let dir = temp_dir("manifest");
        let path = default_manifest_path(&dir);
        write_workspace_manifest_to_path(&path, &manifest).expect("write workspace manifest");
        let decoded = read_workspace_manifest_from_path(&path).expect("read workspace manifest");
        assert_eq!(decoded, manifest);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn reads_legacy_project_manifest_shape() {
        let manifest = r#"
[project]
name = "legacy-app"
version = "0.1.0"
agam = "0.1"
"#;

        let decoded: WorkspaceManifest =
            toml::from_str(manifest).expect("parse legacy project manifest");
        assert_eq!(decoded.format_version, WORKSPACE_MANIFEST_FORMAT_VERSION);
        assert_eq!(decoded.project.name, "legacy-app");
        assert!(decoded.dependencies.is_empty());
    }

    #[test]
    fn read_workspace_manifest_rejects_git_revision_without_git_source() {
        let dir = temp_dir("manifest_invalid_dep");
        let path = default_manifest_path(&dir);
        fs::write(
            &path,
            r#"
[project]
name = "invalid-dependency"
version = "0.1.0"
agam = "0.1"

[dependencies.tensor-core]
rev = "abc123"
"#,
        )
        .expect("write invalid manifest");

        let error = read_workspace_manifest_from_path(&path)
            .expect_err("manifest should reject bad git metadata");

        assert!(error.contains("requires `git`"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn read_workspace_manifest_rejects_empty_environment_metadata() {
        let dir = temp_dir("manifest_invalid_env");
        let path = default_manifest_path(&dir);
        fs::write(
            &path,
            r#"
[project]
name = "invalid-environment"
version = "0.1.0"
agam = "0.1"

[environments.dev]
"#,
        )
        .expect("write invalid manifest");

        let error = read_workspace_manifest_from_path(&path)
            .expect_err("manifest should reject empty environment metadata");

        assert!(error.contains(
            "must set at least one compiler, sdk, target, preferred backend, or profile"
        ));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn resolve_workspace_session_includes_manifest_and_layout() {
        let dir = temp_dir("workspace_session");
        let entry = dir.join("src").join("main.agam");
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        let mut manifest = scaffold_workspace_manifest("session-workspace");
        manifest.dependencies.insert(
            "tensor-core".into(),
            DependencySpec {
                version: Some("^0.4".into()),
                registry: Some("agam".into()),
                ..DependencySpec::default()
            },
        );
        write_workspace_manifest_to_path(&default_manifest_path(&dir), &manifest)
            .expect("write workspace manifest");
        fs::write(&entry, sample_source()).expect("write entry");

        let session =
            resolve_workspace_session(Some(dir.clone())).expect("resolve workspace session");

        assert_eq!(session.layout.root, dir);
        assert_eq!(session.layout.project_name, "session-workspace");
        assert_eq!(session.layout.entry_file, entry);
        assert_eq!(
            session
                .manifest
                .as_ref()
                .expect("manifest should exist")
                .dependencies
                .len(),
            1
        );

        let _ = fs::remove_dir_all(session.layout.root);
    }

    #[test]
    fn snapshot_workspace_captures_manifest_sources_and_tests() {
        let dir = temp_dir("workspace_snapshot");
        let entry = dir.join("src").join("main.agam");
        let helper = dir.join("src").join("helper.agam");
        let test_file = dir.join("tests").join("smoke.agam");
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        fs::create_dir_all(test_file.parent().expect("test parent")).expect("create tests");
        write_workspace_manifest_to_path(
            &default_manifest_path(&dir),
            &scaffold_workspace_manifest("snapshot"),
        )
        .expect("write workspace manifest");
        fs::write(&entry, sample_source()).expect("write entry");
        fs::write(&helper, sample_source()).expect("write helper");
        fs::write(&test_file, "@test\nfn smoke() -> bool:\n    return true\n").expect("write test");

        let snapshot = snapshot_workspace(Some(dir.clone())).expect("snapshot workspace");

        assert_eq!(snapshot.session.layout.project_name, "snapshot");
        assert_eq!(snapshot.manifests.len(), 1);
        assert_eq!(snapshot.source_files.len(), 2);
        assert_eq!(snapshot.test_files.len(), 1);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn diff_workspace_snapshots_reports_added_changed_and_removed_files() {
        let dir = temp_dir("workspace_snapshot_diff");
        let entry = dir.join("src").join("main.agam");
        let helper = dir.join("src").join("helper.agam");
        let extra = dir.join("src").join("extra.agam");
        let test_file = dir.join("tests").join("smoke.agam");
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        fs::create_dir_all(test_file.parent().expect("test parent")).expect("create tests");
        write_workspace_manifest_to_path(
            &default_manifest_path(&dir),
            &scaffold_workspace_manifest("snapshot-diff"),
        )
        .expect("write workspace manifest");
        fs::write(&entry, sample_source()).expect("write entry");
        fs::write(&helper, sample_source()).expect("write helper");
        fs::write(&test_file, "@test\nfn smoke() -> bool:\n    return true\n").expect("write test");

        let previous = snapshot_workspace(Some(dir.clone())).expect("snapshot workspace");

        fs::write(
            default_manifest_path(&dir),
            r#"
[project]
name = "snapshot-diff"
version = "0.2.0"
agam = "0.1"
entry = "src/main.agam"
"#,
        )
        .expect("rewrite manifest");
        fs::write(
            &entry,
            "@lang.advance\nfn main() -> i32 {\n    return 1;\n}\n",
        )
        .expect("rewrite entry");
        fs::write(&extra, sample_source()).expect("write extra source");
        fs::remove_file(&test_file).expect("remove test file");

        let next = snapshot_workspace(Some(dir.clone())).expect("snapshot workspace");
        let diff = diff_workspace_snapshots(&previous, &next);

        assert!(diff.changed_files.contains(&default_manifest_path(&dir)));
        assert!(diff.changed_files.contains(&entry));
        assert!(diff.added_files.contains(&extra));
        assert!(diff.removed_files.contains(&test_file));
        assert!(diff.unchanged_files.contains(&helper));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn round_trips_lockfile_toml() {
        let mut environments = BTreeMap::new();
        environments.insert(
            "dev".into(),
            LockedEnvironment {
                compiler: Some("0.1.0".into()),
                sdk: Some("host-native".into()),
                preferred_backend: Some(RuntimeBackend::Jit),
                packages: vec!["tensor-core@0.4.0".into()],
                ..LockedEnvironment::default()
            },
        );

        let lockfile = WorkspaceLockfile {
            format_version: LOCKFILE_FORMAT_VERSION,
            workspace: LockedWorkspace {
                name: "tensor-lab".into(),
                version: "0.1.0".into(),
            },
            packages: vec![LockedPackage {
                name: "tensor-core".into(),
                version: "0.4.0".into(),
                source: LockedPackageSource {
                    kind: "registry".into(),
                    location: "agam".into(),
                    reference: Some("tensor-core".into()),
                },
                content_hash: "blake3:tensor-core-0.4.0".into(),
                dependencies: vec!["simd-kernel@0.2.0".into()],
            }],
            environments,
        };

        let dir = temp_dir("lockfile");
        let path = default_lockfile_path(&dir);
        write_lockfile_to_path(&path, &lockfile).expect("write lockfile");
        let decoded = read_lockfile_from_path(&path).expect("read lockfile");
        assert_eq!(decoded, lockfile);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn resolves_workspace_layout_uses_manifest_root_entry_and_tests() {
        let root = temp_dir("workspace");
        let manifest = root.join("agam.toml");
        let entry = root.join("src").join("main.agam");
        let test_file = root.join("tests").join("smoke.agam");
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        fs::create_dir_all(test_file.parent().expect("test parent")).expect("create tests");
        write_workspace_manifest_to_path(&manifest, &scaffold_workspace_manifest("workspace"))
            .expect("write manifest");
        fs::write(&entry, sample_source()).expect("write entry");
        fs::write(
            &test_file,
            "@test\nfn arithmetic_is_sound() -> bool:\n    return true\n",
        )
        .expect("write test");

        let layout = resolve_workspace_layout(Some(root.clone())).expect("resolve workspace");

        assert_eq!(layout.root, root);
        assert_eq!(layout.manifest_path.as_ref(), Some(&manifest));
        assert_eq!(layout.project_name, "workspace");
        assert_eq!(layout.entry_file, entry);
        assert_eq!(layout.test_files, vec![test_file]);

        let _ = fs::remove_dir_all(layout.root);
    }

    #[test]
    fn resolves_workspace_layout_uses_manifest_declared_entry_path() {
        let root = temp_dir("workspace_entry");
        let manifest = root.join("agam.toml");
        let entry = root.join("app").join("main.agam");
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create app");

        let mut workspace_manifest = scaffold_workspace_manifest("workspace-entry");
        workspace_manifest.project.entry = Some("app/main.agam".into());
        write_workspace_manifest_to_path(&manifest, &workspace_manifest).expect("write manifest");
        fs::write(&entry, sample_source()).expect("write entry");

        let layout = resolve_workspace_layout(Some(root.clone())).expect("resolve workspace");

        assert_eq!(layout.manifest_path.as_ref(), Some(&manifest));
        assert_eq!(layout.project_name, "workspace-entry");
        assert_eq!(layout.entry_file, entry);
        assert_eq!(layout.source_files, vec![layout.entry_file.clone()]);

        let _ = fs::remove_dir_all(layout.root);
    }

    #[test]
    fn resolves_workspace_layout_rejects_manifest_entry_outside_workspace() {
        let root = temp_dir("workspace_invalid_entry");
        let manifest = root.join("agam.toml");

        let mut workspace_manifest = scaffold_workspace_manifest("workspace-invalid");
        workspace_manifest.project.entry = Some("../escape.agam".into());
        write_workspace_manifest_to_path(&manifest, &workspace_manifest).expect("write manifest");

        let error = resolve_workspace_layout(Some(root.clone())).expect_err("manifest should fail");
        assert!(error.contains("must stay inside the workspace root"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn resolves_workspace_layout_supports_single_source_file_without_manifest() {
        let root = temp_dir("single_file");
        let file = root.join("script.agam");
        fs::write(&file, sample_source()).expect("write source");

        let layout = resolve_workspace_layout(Some(file.clone())).expect("resolve single source");

        assert!(layout.manifest_path.is_none());
        assert_eq!(layout.entry_file, file);
        assert_eq!(layout.source_files, vec![layout.entry_file.clone()]);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn expand_agam_inputs_prefers_workspace_contract_for_directories() {
        let root = temp_dir("expand_workspace");
        let entry = root.join("src").join("main.agam");
        let test_file = root.join("tests").join("smoke.agam");
        let loose_file = root.join("benchmarks").join("scratch.agam");
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        fs::create_dir_all(test_file.parent().expect("test parent")).expect("create tests");
        fs::create_dir_all(loose_file.parent().expect("loose parent")).expect("create loose dir");
        write_workspace_manifest_to_path(
            &root.join("agam.toml"),
            &scaffold_workspace_manifest("expand"),
        )
        .expect("write manifest");
        fs::write(&entry, sample_source()).expect("write entry");
        fs::write(&test_file, "@test\nfn smoke() -> bool:\n    return true\n").expect("write test");
        fs::write(&loose_file, sample_source()).expect("write loose file");

        let expanded = expand_agam_inputs(vec![root.clone()]).expect("expand workspace inputs");

        assert_eq!(expanded, vec![entry, test_file]);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn expand_agam_inputs_keeps_explicit_source_files_narrow() {
        let root = temp_dir("expand_file");
        let entry = root.join("src").join("main.agam");
        let helper = root.join("src").join("helper.agam");
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        write_workspace_manifest_to_path(
            &root.join("agam.toml"),
            &scaffold_workspace_manifest("expand-file"),
        )
        .expect("write manifest");
        fs::write(&entry, sample_source()).expect("write entry");
        fs::write(&helper, sample_source()).expect("write helper");

        let expanded = expand_agam_inputs(vec![helper.clone()]).expect("expand explicit file");

        assert_eq!(expanded, vec![helper]);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn resolve_workspace_session_aggregates_explicit_workspace_members() {
        let root = temp_dir("workspace_members_session");
        let root_entry = root.join("src").join("main.agam");
        let root_test = root.join("tests").join("smoke.agam");
        let member_root = root.join("packages").join("core");
        let member_entry = member_root.join("src").join("main.agam");
        let member_test = member_root.join("tests").join("core.agam");
        fs::create_dir_all(root_entry.parent().expect("root entry parent"))
            .expect("create root src");
        fs::create_dir_all(root_test.parent().expect("root test parent"))
            .expect("create root tests");
        fs::create_dir_all(member_entry.parent().expect("member entry parent"))
            .expect("create member src");
        fs::create_dir_all(member_test.parent().expect("member test parent"))
            .expect("create member tests");

        let mut root_manifest = scaffold_workspace_manifest("workspace-root");
        root_manifest.workspace.members = vec!["packages/core".into()];
        write_workspace_manifest_to_path(&root.join("agam.toml"), &root_manifest)
            .expect("write root manifest");
        write_workspace_manifest_to_path(
            &member_root.join("agam.toml"),
            &scaffold_workspace_manifest("workspace-core"),
        )
        .expect("write member manifest");
        fs::write(&root_entry, sample_source()).expect("write root entry");
        fs::write(
            &root_test,
            "@test\nfn root_smoke() -> bool:\n    return true\n",
        )
        .expect("write root test");
        fs::write(&member_entry, sample_source()).expect("write member entry");
        fs::write(
            &member_test,
            "@test\nfn member_smoke() -> bool:\n    return true\n",
        )
        .expect("write member test");

        let session =
            resolve_workspace_session(Some(root.clone())).expect("resolve workspace session");

        assert_eq!(session.members.len(), 1);
        assert_eq!(session.members[0].layout.root, member_root);
        assert!(session.layout.source_files.contains(&root_entry));
        assert!(session.layout.source_files.contains(&member_entry));
        assert!(session.layout.test_files.contains(&root_test));
        assert!(session.layout.test_files.contains(&member_test));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn snapshot_workspace_tracks_member_manifests_and_manifest_diffs() {
        let root = temp_dir("workspace_member_snapshot");
        let root_entry = root.join("src").join("main.agam");
        let member_root = root.join("packages").join("core");
        let member_entry = member_root.join("src").join("main.agam");
        fs::create_dir_all(root_entry.parent().expect("root entry parent"))
            .expect("create root src");
        fs::create_dir_all(member_entry.parent().expect("member entry parent"))
            .expect("create member src");

        let mut root_manifest = scaffold_workspace_manifest("workspace-root");
        root_manifest.workspace.members = vec!["packages/core".into()];
        write_workspace_manifest_to_path(&root.join("agam.toml"), &root_manifest)
            .expect("write root manifest");
        write_workspace_manifest_to_path(
            &member_root.join("agam.toml"),
            &scaffold_workspace_manifest("workspace-core"),
        )
        .expect("write member manifest");
        fs::write(&root_entry, sample_source()).expect("write root entry");
        fs::write(&member_entry, sample_source()).expect("write member entry");

        let previous = snapshot_workspace(Some(root.clone())).expect("snapshot workspace");
        assert_eq!(previous.manifests.len(), 2);

        let mut member_manifest = scaffold_workspace_manifest("workspace-core");
        member_manifest.project.version = "0.2.0".into();
        write_workspace_manifest_to_path(&member_root.join("agam.toml"), &member_manifest)
            .expect("rewrite member manifest");

        let next = snapshot_workspace(Some(root.clone())).expect("snapshot workspace");
        let diff = diff_workspace_snapshots(&previous, &next);

        assert!(previous.is_stale(&next));
        assert_eq!(diff.changed_files, vec![member_root.join("agam.toml")]);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn expand_agam_inputs_includes_workspace_member_files() {
        let root = temp_dir("workspace_members_expand");
        let root_entry = root.join("src").join("main.agam");
        let member_root = root.join("packages").join("core");
        let member_entry = member_root.join("src").join("main.agam");
        let member_test = member_root.join("tests").join("core.agam");
        fs::create_dir_all(root_entry.parent().expect("root entry parent"))
            .expect("create root src");
        fs::create_dir_all(member_entry.parent().expect("member entry parent"))
            .expect("create member src");
        fs::create_dir_all(member_test.parent().expect("member test parent"))
            .expect("create member tests");

        let mut root_manifest = scaffold_workspace_manifest("workspace-root");
        root_manifest.workspace.members = vec!["packages/core".into()];
        write_workspace_manifest_to_path(&root.join("agam.toml"), &root_manifest)
            .expect("write root manifest");
        write_workspace_manifest_to_path(
            &member_root.join("agam.toml"),
            &scaffold_workspace_manifest("workspace-core"),
        )
        .expect("write member manifest");
        fs::write(&root_entry, sample_source()).expect("write root entry");
        fs::write(&member_entry, sample_source()).expect("write member entry");
        fs::write(
            &member_test,
            "@test\nfn member_smoke() -> bool:\n    return true\n",
        )
        .expect("write member test");

        let expanded = expand_agam_inputs(vec![root.clone()]).expect("expand workspace inputs");

        assert!(expanded.contains(&root_entry));
        assert!(expanded.contains(&member_entry));
        assert!(expanded.contains(&member_test));

        let _ = fs::remove_dir_all(root);
    }
}
