//! Portable Agam package format and helpers.

use std::cmp::Ordering;
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
/// First daemon warm-state index format version.
pub const DAEMON_WARM_INDEX_FORMAT_VERSION: u32 = 1;
/// First standard library metadata format version.
pub const STDLIB_METADATA_FORMAT_VERSION: u32 = 1;
/// Default environment variable used to point registry dependencies at a local index.
const DEFAULT_REGISTRY_INDEX_ENV: &str = "AGAM_REGISTRY_INDEX";

/// Metadata for a standard library module distributed through the package ecosystem.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StdlibModuleMetadata {
    /// Module name (e.g. "io", "math", "net").
    pub name: String,
    /// Version string following the same semver contract as packages.
    pub version: String,
    /// Minimum compiler version required by this stdlib module.
    pub min_compiler_version: String,
    /// List of effect contracts this module participates in.
    pub effects: Vec<String>,
    /// Whether this module is a candidate for first-party distribution profiles.
    pub distribution_eligible: bool,
}

/// Registry of all known first-party standard library modules.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct StdlibRegistry {
    pub format_version: u32,
    pub modules: Vec<StdlibModuleMetadata>,
}

/// Create the first-party stdlib registry with the current shipped modules.
pub fn builtin_stdlib_registry() -> StdlibRegistry {
    StdlibRegistry {
        format_version: STDLIB_METADATA_FORMAT_VERSION,
        modules: vec![StdlibModuleMetadata {
            name: "io".into(),
            version: "0.1.0".into(),
            min_compiler_version: "0.1".into(),
            effects: vec!["FileSystem".into()],
            distribution_eligible: true,
        }],
    }
}

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
    pub packaged_sysroot: Option<String>,
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

/// A fully resolved project-local environment view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedEnvironment {
    pub name: String,
    pub compiler: String,
    pub sdk: Option<String>,
    pub target: Option<String>,
    pub runtime_abi: Option<u32>,
    pub preferred_backend: Option<RuntimeBackend>,
    pub profiles: Vec<String>,
    pub packages: Vec<String>,
}

/// Which dependency table declared a local path dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorkspaceDependencyTable {
    Main,
    Dev,
    Build,
}

impl WorkspaceDependencyTable {
    pub fn manifest_label(self) -> &'static str {
        match self {
            Self::Main => "dependencies",
            Self::Dev => "dev-dependencies",
            Self::Build => "build-dependencies",
        }
    }
}

/// Direct local path-dependency metadata carried by the shared workspace session contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePathDependency {
    pub dependency_key: String,
    pub table: WorkspaceDependencyTable,
    pub root: PathBuf,
    pub relative_path: String,
    pub manifest_path: Option<PathBuf>,
    pub manifest: Option<WorkspaceManifest>,
    pub path_dependencies: Vec<WorkspacePathDependency>,
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
    pub path_dependencies: Vec<WorkspacePathDependency>,
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

/// How far the daemon has warmed a particular file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DaemonWarmLevel {
    /// Source was parsed but not semantically checked.
    Parsed,
    /// Source was parsed, resolved, and type-checked.
    Checked,
    /// Source was parsed, checked, and lowered to optimized MIR.
    Lowered,
}

/// Per-file entry in the daemon warm-state index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonWarmFileEntry {
    pub content_hash: String,
    pub mir_hash: Option<String>,
    pub artifact_path: Option<String>,
    pub warm_level: DaemonWarmLevel,
}

/// Multi-file daemon warm-state index persisted at `.agam_cache/daemon/warm_index.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonWarmIndex {
    #[serde(default = "default_daemon_warm_index_format_version")]
    pub format_version: u32,
    pub files: BTreeMap<String, DaemonWarmFileEntry>,
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

/// Discover all direct local path dependencies declared in a workspace manifest.
pub fn resolve_workspace_path_dependencies(
    session: &WorkspaceSession,
) -> Result<Vec<WorkspacePathDependency>, String> {
    Ok(session.path_dependencies.clone())
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

/// Default daemon warm-state index path for a workspace root.
pub fn daemon_warm_index_path(root: &Path) -> PathBuf {
    root.join(".agam_cache")
        .join("daemon")
        .join("warm_index.json")
}

/// Read the daemon warm-state index for a workspace, returning `None` if it does not exist.
pub fn read_daemon_warm_index(root: &Path) -> Result<Option<DaemonWarmIndex>, String> {
    let path = daemon_warm_index_path(root);
    if !path.is_file() {
        return Ok(None);
    }
    let json = std::fs::read_to_string(&path)
        .map_err(|e| format!("failed to read daemon warm index `{}`: {e}", path.display()))?;
    let index: DaemonWarmIndex = serde_json::from_str(&json).map_err(|e| {
        format!(
            "failed to parse daemon warm index `{}`: {e}",
            path.display()
        )
    })?;
    if index.format_version != DAEMON_WARM_INDEX_FORMAT_VERSION {
        return Ok(None);
    }
    Ok(Some(index))
}

/// Write the daemon warm-state index for a workspace.
pub fn write_daemon_warm_index(root: &Path, index: &DaemonWarmIndex) -> Result<(), String> {
    let path = daemon_warm_index_path(root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            format!(
                "failed to create daemon warm index directory `{}`: {e}",
                parent.display()
            )
        })?;
    }
    let json = serde_json::to_vec_pretty(index)
        .map_err(|e| format!("failed to serialize daemon warm index: {e}"))?;
    std::fs::write(&path, json).map_err(|e| {
        format!(
            "failed to write daemon warm index `{}`: {e}",
            path.display()
        )
    })
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
    let path_dependencies = manifest
        .as_ref()
        .map(|manifest| resolve_workspace_path_dependencies_from_manifest(root, manifest))
        .transpose()?
        .unwrap_or_default();
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
        path_dependencies,
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
    for dependency in &session.path_dependencies {
        collect_path_dependency_manifest_paths(dependency, out);
    }
    for member in &session.members {
        collect_workspace_manifest_paths(member, out);
    }
}

fn collect_path_dependency_manifest_paths(
    dependency: &WorkspacePathDependency,
    out: &mut Vec<PathBuf>,
) {
    if let Some(path) = dependency.manifest_path.as_ref() {
        out.push(path.clone());
    }
    for child in &dependency.path_dependencies {
        collect_path_dependency_manifest_paths(child, out);
    }
}

fn resolve_workspace_path_dependencies_from_manifest(
    root: &Path,
    manifest: &WorkspaceManifest,
) -> Result<Vec<WorkspacePathDependency>, String> {
    let mut ancestry = vec![root.to_path_buf()];
    resolve_workspace_path_dependencies_from_manifest_inner(root, manifest, &mut ancestry)
}

fn resolve_workspace_path_dependencies_from_manifest_inner(
    root: &Path,
    manifest: &WorkspaceManifest,
    ancestry: &mut Vec<PathBuf>,
) -> Result<Vec<WorkspacePathDependency>, String> {
    let mut path_dependencies = Vec::new();
    collect_workspace_path_dependency_table(
        root,
        WorkspaceDependencyTable::Main,
        &manifest.dependencies,
        &mut path_dependencies,
        ancestry,
    )?;
    collect_workspace_path_dependency_table(
        root,
        WorkspaceDependencyTable::Dev,
        &manifest.dev_dependencies,
        &mut path_dependencies,
        ancestry,
    )?;
    collect_workspace_path_dependency_table(
        root,
        WorkspaceDependencyTable::Build,
        &manifest.build_dependencies,
        &mut path_dependencies,
        ancestry,
    )?;
    sort_workspace_path_dependencies(&mut path_dependencies);
    Ok(path_dependencies)
}

fn sort_workspace_path_dependencies(path_dependencies: &mut [WorkspacePathDependency]) {
    path_dependencies.sort_by(|left, right| {
        left.root
            .cmp(&right.root)
            .then_with(|| left.table.cmp(&right.table))
            .then_with(|| left.dependency_key.cmp(&right.dependency_key))
    });
    for dependency in path_dependencies {
        sort_workspace_path_dependencies(&mut dependency.path_dependencies);
    }
}

fn collect_workspace_path_dependency_table(
    root: &Path,
    table: WorkspaceDependencyTable,
    dependencies: &BTreeMap<String, DependencySpec>,
    out: &mut Vec<WorkspacePathDependency>,
    ancestry: &mut Vec<PathBuf>,
) -> Result<(), String> {
    for (dependency_key, spec) in dependencies {
        let Some(relative_path) = spec.path.as_deref() else {
            continue;
        };

        let field_name = format!("`{}.{dependency_key}.path`", table.manifest_label());
        let dependency_root = workspace_relative_path(root, relative_path, &field_name)?;
        let (manifest_path, manifest, path_dependencies) = if dependency_root.is_dir() {
            let manifest_path = default_manifest_path(&dependency_root);
            if manifest_path.is_file() {
                let manifest =
                    read_workspace_manifest_from_path(&manifest_path).map_err(|error| {
                        format!(
                            "failed to resolve local path dependency `{dependency_key}` from {}: {error}",
                            table.manifest_label()
                        )
                    })?;
                let path_dependencies =
                    if ancestry.iter().any(|ancestor| ancestor == &dependency_root) {
                        Vec::new()
                    } else {
                        ancestry.push(dependency_root.clone());
                        let nested = resolve_workspace_path_dependencies_from_manifest_inner(
                            &dependency_root,
                            &manifest,
                            ancestry,
                        )?;
                        ancestry.pop();
                        nested
                    };
                (Some(manifest_path), Some(manifest), path_dependencies)
            } else {
                (None, None, Vec::new())
            }
        } else {
            (None, None, Vec::new())
        };

        out.push(WorkspacePathDependency {
            dependency_key: dependency_key.clone(),
            table,
            root: dependency_root,
            relative_path: relative_path.to_string(),
            manifest_path,
            manifest,
            path_dependencies,
        });
    }

    Ok(())
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

fn default_daemon_warm_index_format_version() -> u32 {
    DAEMON_WARM_INDEX_FORMAT_VERSION
}

// ---------------------------------------------------------------------------
// Phase 17B — Deterministic Dependency Resolver
// ---------------------------------------------------------------------------

/// The kind of source a resolved dependency was drawn from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DependencySourceKind {
    /// A local path dependency (`path = "libs/core"`).
    Path,
    /// A git repository dependency (`git = "https://..."`).
    Git,
    /// A registry dependency (`version = "^1.0"`, optionally with `registry`).
    Registry,
    /// An intra-workspace member dependency.
    Workspace,
}

impl DependencySourceKind {
    /// Stable string label used in `LockedPackageSource.kind`.
    pub fn label(self) -> &'static str {
        match self {
            DependencySourceKind::Path => "path",
            DependencySourceKind::Git => "git",
            DependencySourceKind::Registry => "registry",
            DependencySourceKind::Workspace => "workspace",
        }
    }
}

/// Deterministically hash all `.agam` files in a directory.
///
/// The hash is computed by sorting file paths lexicographically, concatenating
/// `<relative-path>\n<file-content>` for each file, and hashing the result
/// with the shared `agam_runtime::cache::hash_bytes` function.
pub fn content_hash_directory(directory: &Path) -> Result<String, String> {
    if !directory.is_dir() {
        return Err(format!(
            "content hash target `{}` is not a directory",
            directory.display()
        ));
    }

    let mut agam_files = Vec::new();
    collect_agam_files(directory, &mut agam_files)?;
    agam_files.sort();

    let mut payload = Vec::new();
    for file in &agam_files {
        let relative = file
            .strip_prefix(directory)
            .unwrap_or(file)
            .to_string_lossy();
        let content = std::fs::read(file)
            .map_err(|e| format!("failed to read `{}` for content hash: {e}", file.display()))?;
        payload.extend_from_slice(relative.as_bytes());
        payload.push(b'\n');
        payload.extend_from_slice(&content);
    }

    Ok(agam_runtime::cache::hash_bytes(&payload))
}

/// Resolve a `path`-based dependency into a locked package record.
fn resolve_path_dependency(
    root: &Path,
    name: &str,
    spec: &DependencySpec,
) -> Result<LockedPackage, String> {
    let rel_path = spec
        .path
        .as_deref()
        .ok_or_else(|| format!("dependency `{name}` is marked as path but has no `path` field"))?;
    let abs_path = workspace_relative_path(root, rel_path, &format!("`dependencies.{name}.path`"))?;

    let content_hash = if abs_path.is_dir() {
        content_hash_directory(&abs_path)?
    } else if abs_path.is_file() {
        let bytes = std::fs::read(&abs_path).map_err(|e| {
            format!(
                "failed to read path dependency `{}`: {e}",
                abs_path.display()
            )
        })?;
        agam_runtime::cache::hash_bytes(&bytes)
    } else {
        return Err(format!(
            "path dependency `{name}` target `{}` does not exist",
            abs_path.display()
        ));
    };

    let version = spec.version.as_deref().unwrap_or("0.0.0").to_string();
    Ok(LockedPackage {
        name: spec.package.as_deref().unwrap_or(name).to_string(),
        version,
        source: LockedPackageSource {
            kind: DependencySourceKind::Path.label().to_string(),
            location: rel_path.to_string(),
            reference: None,
        },
        content_hash,
        dependencies: Vec::new(),
    })
}

/// Resolve a `git`-based dependency into a locked package record.
///
/// This records the URL and ref/branch but does not clone the repository.
/// Actual fetching is deferred to Phase 17C (registry layer).
fn resolve_git_dependency(name: &str, spec: &DependencySpec) -> Result<LockedPackage, String> {
    let url = spec
        .git
        .as_deref()
        .ok_or_else(|| format!("dependency `{name}` is marked as git but has no `git` field"))?;

    let reference = spec
        .rev
        .as_deref()
        .or(spec.branch.as_deref())
        .map(|s| s.to_string());

    // Build a deterministic content hash from the URL + ref so lockfile records
    // are stable across resolver runs that see the same inputs.
    let hash_input = format!("git:{url}:{}", reference.as_deref().unwrap_or("HEAD"));
    let content_hash = agam_runtime::cache::hash_bytes(hash_input.as_bytes());

    let version = spec.version.as_deref().unwrap_or("0.0.0").to_string();
    Ok(LockedPackage {
        name: spec.package.as_deref().unwrap_or(name).to_string(),
        version,
        source: LockedPackageSource {
            kind: DependencySourceKind::Git.label().to_string(),
            location: url.to_string(),
            reference,
        },
        content_hash,
        dependencies: Vec::new(),
    })
}

/// Resolve a registry-based dependency into a locked package record.
///
/// This produces a placeholder record. Actual registry fetching is Phase 17C.
fn resolve_registry_dependency(name: &str, spec: &DependencySpec) -> Result<LockedPackage, String> {
    let version = spec
        .version
        .as_deref()
        .ok_or_else(|| format!("registry dependency `{name}` must declare a `version`"))?;
    let package_name = spec.package.as_deref().unwrap_or(name);

    let registry = spec.registry.as_deref().unwrap_or("agam").to_string();

    if let Some(index_root) = configured_registry_index_path(&registry) {
        return resolve_registry_dependency_from_index(&index_root, package_name, version);
    }

    let hash_input = format!("registry:{registry}:{package_name}@{version}");
    let content_hash = agam_runtime::cache::hash_bytes(hash_input.as_bytes());

    Ok(LockedPackage {
        name: package_name.to_string(),
        version: version.to_string(),
        source: LockedPackageSource {
            kind: DependencySourceKind::Registry.label().to_string(),
            location: registry,
            reference: Some(format!("{package_name}@{version}")),
        },
        content_hash,
        dependencies: Vec::new(),
    })
}

/// Resolve an intra-workspace member as a path dependency.
fn resolve_workspace_member_dependency(
    root: &Path,
    member_session: &WorkspaceSession,
    name: &str,
) -> Result<LockedPackage, String> {
    let member_root = &member_session.layout.root;
    let content_hash = content_hash_directory(member_root)?;
    let relative = member_root
        .strip_prefix(root)
        .unwrap_or(member_root)
        .to_string_lossy()
        .to_string();
    let version = member_session
        .manifest
        .as_ref()
        .map(|m| m.project.version.clone())
        .unwrap_or_else(|| "0.0.0".to_string());

    Ok(LockedPackage {
        name: name.to_string(),
        version,
        source: LockedPackageSource {
            kind: DependencySourceKind::Workspace.label().to_string(),
            location: relative,
            reference: None,
        },
        content_hash,
        dependencies: Vec::new(),
    })
}

/// Classify which source kind a `DependencySpec` uses.
fn classify_dependency_source(spec: &DependencySpec) -> DependencySourceKind {
    if spec.path.is_some() {
        DependencySourceKind::Path
    } else if spec.git.is_some() {
        DependencySourceKind::Git
    } else {
        DependencySourceKind::Registry
    }
}

/// Resolve a single dependency spec into a locked package record.
fn resolve_single_dependency(
    root: &Path,
    name: &str,
    spec: &DependencySpec,
    workspace_members: &[WorkspaceSession],
) -> Result<LockedPackage, String> {
    // Check if this dependency name matches a workspace member first.
    if let Some(member) = workspace_members
        .iter()
        .find(|m| m.layout.project_name == name)
    {
        return resolve_workspace_member_dependency(root, member, name);
    }

    match classify_dependency_source(spec) {
        DependencySourceKind::Path => resolve_path_dependency(root, name, spec),
        DependencySourceKind::Git => resolve_git_dependency(name, spec),
        DependencySourceKind::Registry | DependencySourceKind::Workspace => {
            resolve_registry_dependency(name, spec)
        }
    }
}

/// Resolve all dependencies in a manifest into locked package records.
///
/// Resolution order is deterministic: dependencies are processed alphabetically
/// by name, then by table order (`dependencies` → `dev-dependencies` →
/// `build-dependencies`). Duplicate package names across tables are unified
/// to the first occurrence.
///
/// Path and workspace dependencies that carry their own `agam.toml` with
/// dependency sections are resolved transitively (up to a depth limit).
fn resolve_dependency_tables(
    root: &Path,
    manifest: &WorkspaceManifest,
    workspace_members: &[WorkspaceSession],
    path_dependencies: &[WorkspacePathDependency],
) -> Result<Vec<LockedPackage>, String> {
    let mut packages = Vec::new();
    let mut seen = BTreeSet::new();

    resolve_dependency_tables_recursive(
        root,
        manifest,
        workspace_members,
        path_dependencies,
        &mut packages,
        &mut seen,
        0,
    )?;

    Ok(packages)
}

/// Maximum transitive dependency resolution depth.
const MAX_RESOLVE_DEPTH: usize = 16;

/// Recursively resolve dependency tables, walking transitive path/workspace deps.
fn resolve_dependency_tables_recursive(
    root: &Path,
    manifest: &WorkspaceManifest,
    workspace_members: &[WorkspaceSession],
    path_dependencies: &[WorkspacePathDependency],
    packages: &mut Vec<LockedPackage>,
    seen: &mut BTreeSet<String>,
    depth: usize,
) -> Result<(), String> {
    if depth > MAX_RESOLVE_DEPTH {
        return Err(format!(
            "dependency resolution depth exceeded {MAX_RESOLVE_DEPTH}; possible cyclic dependency"
        ));
    }

    let tables: [&BTreeMap<String, DependencySpec>; 3] = [
        &manifest.dependencies,
        &manifest.dev_dependencies,
        &manifest.build_dependencies,
    ];

    for table in tables {
        // BTreeMap iteration is already alphabetically sorted.
        for (name, spec) in table {
            if seen.contains(name) {
                continue;
            }
            seen.insert(name.clone());
            let mut locked = resolve_single_dependency(root, name, spec, workspace_members)?;

            // Attempt transitive resolution for path and workspace deps.
            let transitive_deps = resolve_transitive_deps(
                root,
                name,
                spec,
                workspace_members,
                path_dependencies,
                packages,
                seen,
                depth,
            )?;
            for tdep in &transitive_deps {
                let dep_ref = format!("{}@{}", tdep, "0.0.0");
                if !locked.dependencies.contains(&dep_ref) {
                    locked.dependencies.push(dep_ref);
                }
            }

            packages.push(locked);
        }
    }

    Ok(())
}

/// Walk a single dependency's own manifest (if it exists) to discover transitive deps.
///
/// Returns the names of the transitive dependencies that were added.
fn resolve_transitive_deps(
    root: &Path,
    name: &str,
    spec: &DependencySpec,
    workspace_members: &[WorkspaceSession],
    path_dependencies: &[WorkspacePathDependency],
    packages: &mut Vec<LockedPackage>,
    seen: &mut BTreeSet<String>,
    depth: usize,
) -> Result<Vec<String>, String> {
    let mut added = Vec::new();

    // Only path and workspace deps can have discoverable transitive manifests.
    let (dep_root, dep_manifest, dep_workspace_members, dep_path_dependencies) =
        if let Some(member) = workspace_members
            .iter()
            .find(|m| m.layout.project_name == name)
        {
            (
                member.layout.root.clone(),
                member.manifest.clone(),
                member.members.clone(),
                member.path_dependencies.clone(),
            )
        } else if let Some(path_dependency) = path_dependencies
            .iter()
            .find(|dependency| dependency.dependency_key == name)
        {
            (
                path_dependency.root.clone(),
                path_dependency.manifest.clone(),
                Vec::new(),
                path_dependency.path_dependencies.clone(),
            )
        } else if let Some(rel_path) = spec.path.as_deref() {
            match workspace_relative_path(root, rel_path, &format!("`dependencies.{name}.path`")) {
                Ok(path) if path.is_dir() => {
                    let manifest_path = default_manifest_path(&path);
                    let manifest = if manifest_path.is_file() {
                        match read_workspace_manifest_from_path(&manifest_path) {
                            Ok(manifest) => Some(manifest),
                            Err(_) => return Ok(added),
                        }
                    } else {
                        None
                    };
                    let nested_path_dependencies = manifest
                        .as_ref()
                        .map(|manifest| {
                            resolve_workspace_path_dependencies_from_manifest(&path, manifest)
                        })
                        .transpose()?
                        .unwrap_or_default();
                    (path, manifest, Vec::new(), nested_path_dependencies)
                }
                _ => return Ok(added),
            }
        } else {
            return Ok(added);
        };

    let Some(dep_manifest) = dep_manifest else {
        return Ok(added);
    };

    // Check if there are any dependencies to resolve.
    let has_deps = !dep_manifest.dependencies.is_empty()
        || !dep_manifest.dev_dependencies.is_empty()
        || !dep_manifest.build_dependencies.is_empty();
    if !has_deps {
        return Ok(added);
    }

    // Capture names before recursive resolution.
    let before_count = packages.len();

    resolve_dependency_tables_recursive(
        &dep_root,
        &dep_manifest,
        &dep_workspace_members,
        &dep_path_dependencies,
        packages,
        seen,
        depth + 1,
    )?;

    for pkg in packages.iter().skip(before_count) {
        added.push(pkg.name.clone());
    }

    Ok(added)
}

/// Resolve all locked environment records from a manifest.
fn resolve_locked_environments(
    manifest: &WorkspaceManifest,
    packages: &[LockedPackage],
) -> BTreeMap<String, LockedEnvironment> {
    let mut locked_envs = BTreeMap::new();

    for (name, env) in &manifest.environments {
        let package_refs: Vec<String> = packages
            .iter()
            .map(|p| format!("{}@{}", p.name, p.version))
            .collect();

        locked_envs.insert(
            name.clone(),
            LockedEnvironment {
                compiler: env.compiler.clone(),
                sdk: env.sdk.clone(),
                target: env.target.clone(),
                preferred_backend: env.preferred_backend,
                packages: package_refs,
            },
        );
    }

    locked_envs
}

/// Select the default environment name for a manifest.
///
/// Rules:
/// - `dev` wins when present.
/// - otherwise the sole environment wins.
/// - otherwise there is no implicit default.
pub fn default_environment_name(manifest: &WorkspaceManifest) -> Option<String> {
    if manifest.environments.contains_key("dev") {
        return Some("dev".into());
    }
    if manifest.environments.len() == 1 {
        return manifest.environments.keys().next().cloned();
    }
    None
}

/// Resolve an explicit or implicit environment name for a manifest.
pub fn select_environment_name(
    manifest: &WorkspaceManifest,
    requested: Option<&str>,
) -> Result<Option<String>, String> {
    if let Some(requested) = requested {
        let requested = requested.trim();
        if requested.is_empty() {
            return Err("environment name cannot be empty".into());
        }
        if manifest.environments.contains_key(requested) {
            return Ok(Some(requested.to_string()));
        }
        return Err(format!("workspace has no environment named `{requested}`"));
    }

    if manifest.environments.is_empty() {
        return Ok(None);
    }

    if let Some(default_name) = default_environment_name(manifest) {
        return Ok(Some(default_name));
    }

    let available = manifest
        .environments
        .keys()
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!(
        "workspace defines multiple environments with no implicit default; choose one explicitly ({available})"
    ))
}

/// Resolve every named environment against the current manifest and lockfile.
pub fn resolve_environment_catalog(
    manifest: &WorkspaceManifest,
    lockfile: &WorkspaceLockfile,
) -> BTreeMap<String, ResolvedEnvironment> {
    let package_refs = lockfile
        .packages
        .iter()
        .map(|package| format!("{}@{}", package.name, package.version))
        .collect::<Vec<_>>();
    let toolchain = manifest.toolchain.as_ref();
    let fallback_compiler = toolchain
        .map(|toolchain| toolchain.agam.clone())
        .unwrap_or_else(|| manifest.project.agam.clone());
    let fallback_sdk = toolchain.and_then(|toolchain| toolchain.sdk.clone());
    let fallback_target = toolchain.and_then(|toolchain| toolchain.target.clone());
    let fallback_runtime_abi = toolchain.and_then(|toolchain| toolchain.runtime_abi);
    let fallback_backend = toolchain.and_then(|toolchain| toolchain.preferred_backend);

    manifest
        .environments
        .iter()
        .map(|(name, environment)| {
            let locked = lockfile.environments.get(name);
            (
                name.clone(),
                ResolvedEnvironment {
                    name: name.clone(),
                    compiler: environment
                        .compiler
                        .clone()
                        .or_else(|| locked.and_then(|locked| locked.compiler.clone()))
                        .unwrap_or_else(|| fallback_compiler.clone()),
                    sdk: environment
                        .sdk
                        .clone()
                        .or_else(|| locked.and_then(|locked| locked.sdk.clone()))
                        .or_else(|| fallback_sdk.clone()),
                    target: environment
                        .target
                        .clone()
                        .or_else(|| locked.and_then(|locked| locked.target.clone()))
                        .or_else(|| fallback_target.clone()),
                    runtime_abi: fallback_runtime_abi,
                    preferred_backend: environment
                        .preferred_backend
                        .or_else(|| locked.and_then(|locked| locked.preferred_backend))
                        .or(fallback_backend),
                    profiles: environment.profiles.clone(),
                    packages: locked
                        .map(|locked| locked.packages.clone())
                        .unwrap_or_else(|| package_refs.clone()),
                },
            )
        })
        .collect()
}

/// Resolve one named environment, applying the implicit default-selection rules when needed.
pub fn resolve_environment(
    manifest: &WorkspaceManifest,
    lockfile: &WorkspaceLockfile,
    requested: Option<&str>,
) -> Result<Option<ResolvedEnvironment>, String> {
    let selected = select_environment_name(manifest, requested)?;
    let catalog = resolve_environment_catalog(manifest, lockfile);
    Ok(selected.and_then(|name| catalog.get(&name).cloned()))
}

/// Deterministically resolve all dependencies declared in a workspace session
/// into a complete `WorkspaceLockfile`.
///
/// This is the main resolver entry point. Every run with the same inputs will
/// produce byte-identical output.
pub fn resolve_dependencies(session: &WorkspaceSession) -> Result<WorkspaceLockfile, String> {
    let manifest = match session.manifest.as_ref() {
        Some(m) => m,
        None => {
            // No manifest means no dependencies — produce a minimal lockfile.
            return Ok(WorkspaceLockfile {
                format_version: LOCKFILE_FORMAT_VERSION,
                workspace: LockedWorkspace {
                    name: session.layout.project_name.clone(),
                    version: "0.0.0".to_string(),
                },
                packages: Vec::new(),
                environments: BTreeMap::new(),
            });
        }
    };

    let root = &session.layout.root;
    let packages =
        resolve_dependency_tables(root, manifest, &session.members, &session.path_dependencies)?;
    let environments = resolve_locked_environments(manifest, &packages);

    Ok(WorkspaceLockfile {
        format_version: LOCKFILE_FORMAT_VERSION,
        workspace: LockedWorkspace {
            name: manifest.project.name.clone(),
            version: manifest.project.version.clone(),
        },
        packages,
        environments,
    })
}

/// Check whether an existing lockfile is still fresh relative to the current
/// workspace manifest declarations.
///
/// A lockfile is fresh when every dependency in the manifest has a corresponding
/// locked package and there are no extra locked packages.
fn expected_locked_dependency_name(name: &str, spec: &DependencySpec) -> String {
    spec.package.as_deref().unwrap_or(name).to_string()
}

fn lockfile_package_matches_spec(
    package: &LockedPackage,
    dependency_key: &str,
    spec: &DependencySpec,
) -> bool {
    let expected_name = expected_locked_dependency_name(dependency_key, spec);
    if package.name != expected_name {
        return false;
    }

    match classify_dependency_source(spec) {
        DependencySourceKind::Path => {
            package.version == spec.version.as_deref().unwrap_or("0.0.0")
                && package.source.kind == DependencySourceKind::Path.label()
                && package.source.location == spec.path.as_deref().unwrap_or_default()
                && package.source.reference.is_none()
        }
        DependencySourceKind::Git => {
            package.version == spec.version.as_deref().unwrap_or("0.0.0")
                && package.source.kind == DependencySourceKind::Git.label()
                && package.source.location == spec.git.as_deref().unwrap_or_default()
                && package.source.reference
                    == spec
                        .rev
                        .as_deref()
                        .or(spec.branch.as_deref())
                        .map(str::to_string)
        }
        DependencySourceKind::Registry | DependencySourceKind::Workspace => {
            package.source.kind == DependencySourceKind::Registry.label()
                && package.source.location == spec.registry.as_deref().unwrap_or("agam")
                && spec
                    .version
                    .as_deref()
                    .map(|requirement| {
                        package.version == requirement
                            || version_matches(&package.version, requirement)
                    })
                    .unwrap_or(false)
        }
    }
}

fn expected_locked_environments_for_lockfile(
    manifest: &WorkspaceManifest,
    lockfile: &WorkspaceLockfile,
) -> BTreeMap<String, LockedEnvironment> {
    resolve_locked_environments(manifest, &lockfile.packages)
}

pub fn is_lockfile_fresh(manifest: &WorkspaceManifest, lockfile: &WorkspaceLockfile) -> bool {
    let mut expected_specs = BTreeMap::new();
    for table in [
        &manifest.dependencies,
        &manifest.dev_dependencies,
        &manifest.build_dependencies,
    ] {
        for (name, spec) in table {
            expected_specs.insert(expected_locked_dependency_name(name, spec), (name, spec));
        }
    }

    let locked_names: BTreeSet<String> = lockfile.packages.iter().map(|p| p.name.clone()).collect();
    let expected_names: BTreeSet<String> = expected_specs.keys().cloned().collect();

    // Check workspace identity.
    if lockfile.workspace.name != manifest.project.name
        || lockfile.workspace.version != manifest.project.version
    {
        return false;
    }

    if expected_names != locked_names {
        return false;
    }

    for (locked_name, (dependency_key, spec)) in expected_specs {
        let Some(package) = lockfile
            .packages
            .iter()
            .find(|package| package.name == locked_name)
        else {
            return false;
        };
        if !lockfile_package_matches_spec(package, dependency_key, spec) {
            return false;
        }
    }

    if expected_locked_environments_for_lockfile(manifest, lockfile) != lockfile.environments {
        return false;
    }

    true
}

/// Generate a new lockfile or return the existing one if it is still fresh.
///
/// When the lockfile is regenerated it is written to disk at the default
/// `agam.lock` path inside the workspace root.
pub fn generate_or_refresh_lockfile(
    session: &WorkspaceSession,
) -> Result<WorkspaceLockfile, String> {
    let lockfile_path = default_lockfile_path(&session.layout.root);

    // Try to read an existing lockfile.
    if let Ok(existing) = read_lockfile_from_path(&lockfile_path) {
        if let Some(manifest) = session.manifest.as_ref() {
            if is_lockfile_fresh(manifest, &existing)
                && lockfile_content_drift(&session.layout.root, &existing).is_empty()
            {
                return Ok(existing);
            }
        }
    }

    // Re-resolve from scratch.
    let lockfile = resolve_dependencies(session)?;
    write_lockfile_to_path(&lockfile_path, &lockfile)?;
    Ok(lockfile)
}

/// Produce human-readable diagnostics about lockfile freshness.
///
/// Returns an empty list when the lockfile perfectly matches the manifest.
pub fn lockfile_diagnostics(
    manifest: &WorkspaceManifest,
    lockfile: &WorkspaceLockfile,
) -> Vec<String> {
    let mut diagnostics = Vec::new();

    let mut expected_specs = BTreeMap::new();
    for table in [
        &manifest.dependencies,
        &manifest.dev_dependencies,
        &manifest.build_dependencies,
    ] {
        for (name, spec) in table {
            expected_specs.insert(expected_locked_dependency_name(name, spec), (name, spec));
        }
    }

    let locked_names: BTreeSet<String> = lockfile.packages.iter().map(|p| p.name.clone()).collect();
    let expected_names: BTreeSet<String> = expected_specs.keys().cloned().collect();

    for name in expected_names.difference(&locked_names) {
        diagnostics.push(format!("missing locked package: `{name}`"));
    }
    for name in locked_names.difference(&expected_names) {
        diagnostics.push(format!("extra locked package: `{name}` (not in manifest)"));
    }
    for (locked_name, (dependency_key, spec)) in expected_specs {
        if let Some(package) = lockfile
            .packages
            .iter()
            .find(|package| package.name == locked_name)
        {
            if !lockfile_package_matches_spec(package, dependency_key, spec) {
                diagnostics.push(format!(
                    "locked package `{locked_name}` no longer matches dependency `{dependency_key}`"
                ));
            }
        }
    }

    if lockfile.workspace.name != manifest.project.name {
        diagnostics.push(format!(
            "lockfile workspace name `{}` does not match manifest `{}`",
            lockfile.workspace.name, manifest.project.name
        ));
    }
    if lockfile.workspace.version != manifest.project.version {
        diagnostics.push(format!(
            "lockfile workspace version `{}` does not match manifest `{}`",
            lockfile.workspace.version, manifest.project.version
        ));
    }

    let expected_environments = expected_locked_environments_for_lockfile(manifest, lockfile);
    let locked_environment_names: BTreeSet<String> =
        lockfile.environments.keys().cloned().collect();
    let expected_environment_names: BTreeSet<String> =
        expected_environments.keys().cloned().collect();

    for name in expected_environment_names.difference(&locked_environment_names) {
        diagnostics.push(format!("missing locked environment: `{name}`"));
    }
    for name in locked_environment_names.difference(&expected_environment_names) {
        diagnostics.push(format!(
            "extra locked environment: `{name}` (not in manifest)"
        ));
    }
    for (name, expected) in expected_environments {
        if let Some(locked) = lockfile.environments.get(&name) {
            if locked != &expected {
                diagnostics.push(format!(
                    "locked environment `{name}` no longer matches manifest"
                ));
            }
        }
    }

    diagnostics.sort();
    diagnostics
}

/// Compare the content hashes of path-based locked packages against their
/// current live filesystem state.
///
/// Returns a list of `(package_name, lockfile_hash, live_hash)` triples for
/// every path dependency whose content hash has drifted since the lockfile
/// was generated. An empty result means all path deps are consistent.
pub fn lockfile_content_drift(
    root: &Path,
    lockfile: &WorkspaceLockfile,
) -> Vec<(String, String, String)> {
    let mut drifted = Vec::new();

    for pkg in &lockfile.packages {
        if pkg.source.kind != DependencySourceKind::Path.label()
            && pkg.source.kind != DependencySourceKind::Workspace.label()
        {
            continue;
        }

        let dep_path = root.join(&pkg.source.location);
        let live_hash = if dep_path.is_dir() {
            match content_hash_directory(&dep_path) {
                Ok(h) => h,
                Err(_) => continue,
            }
        } else if dep_path.is_file() {
            match std::fs::read(&dep_path) {
                Ok(bytes) => agam_runtime::cache::hash_bytes(&bytes),
                Err(_) => continue,
            }
        } else {
            continue;
        };

        if live_hash != pkg.content_hash {
            drifted.push((pkg.name.clone(), pkg.content_hash.clone(), live_hash));
        }
    }

    drifted
}

// ---------------------------------------------------------------------------
// Phase 17C — Registry Index and Publish Protocol
// ---------------------------------------------------------------------------

/// First registry index format version.
pub const REGISTRY_INDEX_FORMAT_VERSION: u32 = 1;

/// Minimum allowed package name length.
const PACKAGE_NAME_MIN_LENGTH: usize = 2;

/// Maximum allowed package name length.
const PACKAGE_NAME_MAX_LENGTH: usize = 64;

/// Reserved package name prefix for official Agam packages.
const RESERVED_PREFIX: &str = "agam-";

/// Top-level configuration stored at `config.json` in a registry index root.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryConfig {
    pub format_version: u32,
    /// API endpoint URL for registry operations (future use).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_url: Option<String>,
    /// Base URL for downloading release tarballs (future use).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download_url: Option<String>,
    /// Human-readable name of this registry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// One package's metadata entry in the registry index.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryPackageEntry {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owners: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub releases: Vec<RegistryRelease>,
}

/// One immutable release record for a package.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryRelease {
    pub version: String,
    /// SHA-256 checksum of the published source artifact.
    pub checksum: String,
    /// Minimum Agam language version required.
    pub agam_version: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<RegistryReleaseDependency>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<RegistryReleaseProvenance>,
    pub published_at: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub yanked: bool,
}

/// Provenance metadata recorded alongside a published release.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryReleaseProvenance {
    pub source_checksum: String,
    pub manifest_checksum: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_repository: Option<String>,
}

/// A dependency edge declared by a registry release.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryReleaseDependency {
    pub name: String,
    pub version_req: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub optional: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
}

/// The artifact submitted during `agamc publish`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublishManifest {
    pub name: String,
    pub version: String,
    pub agam_version: String,
    /// SHA-256 checksum of the published source artifact.
    pub checksum: String,
    /// SHA-256 checksum of the workspace manifest at publish time.
    pub manifest_checksum: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download_url: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<RegistryReleaseDependency>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
}

/// Receipt returned by the registry after a successful publish.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublishReceipt {
    pub name: String,
    pub version: String,
    pub checksum: String,
    pub published_at: String,
    pub index_path: String,
}

/// One official package recommendation inside a curated first-party profile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FirstPartyPackageRecommendation {
    pub name: String,
    pub version_req: String,
    pub rationale: String,
}

/// A curated first-party distribution profile built on top of the registry contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FirstPartyDistributionProfile {
    pub name: String,
    pub summary: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<FirstPartyPackageRecommendation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

/// The official-package governance contract for the first Agam registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OfficialPackageGovernance {
    pub registry: String,
    pub reserved_prefix: String,
    pub repository_namespace: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owner_handles: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub publication_rules: Vec<String>,
}

/// Return the read-only governance contract for official Agam packages.
pub fn official_package_governance() -> OfficialPackageGovernance {
    OfficialPackageGovernance {
        registry: "agam".into(),
        reserved_prefix: RESERVED_PREFIX.into(),
        repository_namespace: "github.com/agam-lang/*".into(),
        owner_handles: vec!["agam-lang".into()],
        publication_rules: vec![
            "the reserved `agam-` prefix is for first-party packages only".into(),
            "official packages publish through the canonical `agam` registry namespace".into(),
            "official package repositories live under the Agam organization namespace".into(),
            "curated profiles describe install guidance but do not hide real dependency boundaries"
                .into(),
        ],
    }
}

fn official_repository_matches_namespace(
    repository: &str,
    governance: &OfficialPackageGovernance,
) -> bool {
    let normalized = repository
        .trim()
        .trim_end_matches(".git")
        .to_ascii_lowercase();
    let namespace = governance
        .repository_namespace
        .trim()
        .trim_end_matches('*')
        .trim_end_matches('/')
        .to_ascii_lowercase();
    if namespace.is_empty() {
        return false;
    }

    let https_prefix = format!("https://{namespace}/");
    let http_prefix = format!("http://{namespace}/");
    let direct_prefix = format!("{namespace}/");
    let ssh_prefix = if let Some((host, owner)) = namespace.split_once('/') {
        format!("git@{host}:{owner}/")
    } else {
        String::new()
    };

    normalized.starts_with(&https_prefix)
        || normalized.starts_with(&http_prefix)
        || normalized.starts_with(&direct_prefix)
        || (!ssh_prefix.is_empty() && normalized.starts_with(&ssh_prefix))
}

/// Return the first curated official-package profile taxonomy for Agam.
pub fn first_party_distribution_profiles() -> Vec<FirstPartyDistributionProfile> {
    vec![
        FirstPartyDistributionProfile {
            name: "base".into(),
            summary: "Small default language support profile".into(),
            description: "Keeps the default install compact while covering the core language, standard library, and testing story.".into(),
            packages: vec![
                FirstPartyPackageRecommendation {
                    name: "agam-std".into(),
                    version_req: "^0.1".into(),
                    rationale: "core first-party standard-library surface once the package ecosystem stabilizes".into(),
                },
                FirstPartyPackageRecommendation {
                    name: "agam-test".into(),
                    version_req: "^0.1".into(),
                    rationale: "first-party testing helpers aligned with the compiler and diagnostics contract".into(),
                },
            ],
            notes: vec![
                "intended as the smallest coherent first-party starting point".into(),
                "does not imply that every official package ships in the default install".into(),
            ],
        },
        FirstPartyDistributionProfile {
            name: "systems".into(),
            summary: "Native and foreign-interop profile".into(),
            description: "Adds explicit systems-facing and interoperability packages without folding foreign package managers into Agam's base contract.".into(),
            packages: vec![
                FirstPartyPackageRecommendation {
                    name: "agam-ffi".into(),
                    version_req: "^0.1".into(),
                    rationale: "native foreign-function interop stays layered and explicit".into(),
                },
                FirstPartyPackageRecommendation {
                    name: "agam-debug".into(),
                    version_req: "^0.1".into(),
                    rationale: "systems-oriented inspection and debugging support".into(),
                },
            ],
            notes: vec![
                "interop remains opt-in instead of part of the base package manager contract".into(),
            ],
        },
        FirstPartyDistributionProfile {
            name: "data-ai".into(),
            summary: "Numerical, tensor, and AI-oriented profile".into(),
            description: "Layers data and AI packages on top of the base language/runtime stack instead of bloating the default install.".into(),
            packages: vec![
                FirstPartyPackageRecommendation {
                    name: "agam-tensor".into(),
                    version_req: "^0.1".into(),
                    rationale: "first-party tensor and numerical primitives".into(),
                },
                FirstPartyPackageRecommendation {
                    name: "agam-dataframe".into(),
                    version_req: "^0.1".into(),
                    rationale: "structured data workflows built for Agam's package ecosystem".into(),
                },
            ],
            notes: vec![
                "intended to be selected explicitly through project-local environment choices".into(),
            ],
        },
    ]
}

/// Look up one curated first-party distribution profile by name.
pub fn first_party_distribution_profile(name: &str) -> Option<FirstPartyDistributionProfile> {
    first_party_distribution_profiles()
        .into_iter()
        .find(|profile| profile.name == name)
}

fn validate_package_name_with_policy(
    name: &str,
    allow_reserved_prefix: bool,
    require_reserved_prefix: bool,
) -> Result<(), String> {
    if name.len() < PACKAGE_NAME_MIN_LENGTH {
        return Err(format!(
            "package name `{name}` is too short (minimum {PACKAGE_NAME_MIN_LENGTH} characters)"
        ));
    }
    if name.len() > PACKAGE_NAME_MAX_LENGTH {
        return Err(format!(
            "package name `{name}` is too long (maximum {PACKAGE_NAME_MAX_LENGTH} characters)"
        ));
    }

    let bytes = name.as_bytes();
    if !is_pkg_name_start(bytes[0]) {
        return Err(format!(
            "package name `{name}` must start with a lowercase letter or digit"
        ));
    }
    if !is_pkg_name_start(bytes[bytes.len() - 1]) {
        return Err(format!(
            "package name `{name}` must end with a lowercase letter or digit"
        ));
    }

    for (i, &b) in bytes.iter().enumerate() {
        if !is_pkg_name_char(b) {
            return Err(format!(
                "package name `{name}` contains invalid character `{}` at position {i}",
                b as char
            ));
        }
        // No consecutive hyphens/underscores.
        if i > 0 && is_separator(b) && is_separator(bytes[i - 1]) {
            return Err(format!(
                "package name `{name}` has consecutive separators at position {i}"
            ));
        }
    }

    if require_reserved_prefix && !name.starts_with(RESERVED_PREFIX) {
        return Err(format!(
            "official package name `{name}` must use the reserved `{RESERVED_PREFIX}` prefix"
        ));
    }

    if !allow_reserved_prefix && name.starts_with(RESERVED_PREFIX) {
        return Err(format!(
            "package name `{name}` uses the reserved `{RESERVED_PREFIX}` prefix (reserved for official packages)"
        ));
    }

    Ok(())
}

/// Validate a package name against Agam naming rules.
///
/// Rules:
/// - Length must be between 2 and 64 characters (inclusive).
/// - Only lowercase ASCII letters, digits, hyphens, and underscores allowed.
/// - Must start with a lowercase letter or digit.
/// - Must end with a lowercase letter or digit.
/// - No consecutive hyphens or underscores.
/// - The `agam-` prefix is reserved for official packages.
pub fn validate_package_name(name: &str) -> Result<(), String> {
    validate_package_name_with_policy(name, false, false)
}

/// Validate an official first-party package name that is allowed to use the
/// reserved `agam-` namespace.
pub fn validate_official_package_name(name: &str) -> Result<(), String> {
    validate_package_name_with_policy(name, true, true)
}

/// Map a package name to its sharded index file path.
///
/// Uses Cargo-style prefix sharding:
/// - 1-char names: `1/<name>`
/// - 2-char names: `2/<name>`
/// - 3-char names: `3/<first-char>/<name>`
/// - 4+ char names: `<first-2-chars>/<next-2-chars>/<name>`
pub fn registry_index_path(name: &str) -> String {
    match name.len() {
        0 => String::new(),
        1 => format!("1/{name}"),
        2 => format!("2/{name}"),
        3 => format!("3/{}/{name}", &name[..1]),
        _ => format!("{}/{}/{name}", &name[..2], &name[2..4]),
    }
}

fn registry_specific_index_env(registry: &str) -> String {
    registry_index_env_var(registry)
}

/// Return the environment variable used to point a registry name at a local index.
pub fn registry_index_env_var(registry: &str) -> String {
    let normalized = registry
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("AGAM_REGISTRY_{normalized}_INDEX")
}

fn configured_registry_index_path(registry: &str) -> Option<PathBuf> {
    let specific = registry_specific_index_env(registry);
    std::env::var_os(&specific)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os(DEFAULT_REGISTRY_INDEX_ENV)
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        })
}

fn registry_index_identity(index_root: &Path) -> String {
    read_registry_config(index_root)
        .ok()
        .and_then(|config| config.name)
        .filter(|name| !name.trim().is_empty())
        .or_else(|| {
            index_root
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.to_string())
        })
        .unwrap_or_else(|| "agam".to_string())
}

fn default_registry_release_download_url(
    index_root: &Path,
    name: &str,
    version: &str,
) -> Option<String> {
    let base = read_registry_config(index_root).ok()?.download_url?;
    let base = base.trim().trim_end_matches('/');
    if base.is_empty() {
        return None;
    }
    Some(format!(
        "{base}/{name}/{version}/{name}-{version}.agam-src.tar.gz"
    ))
}

/// Write a `config.json` to a registry index root.
pub fn write_registry_config(path: &Path, config: &RegistryConfig) -> Result<(), String> {
    let config_path = path.join("config.json");
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            format!(
                "failed to create registry index directory `{}`: {e}",
                parent.display()
            )
        })?;
    }
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("failed to serialize registry config: {e}"))?;
    std::fs::write(&config_path, json).map_err(|e| {
        format!(
            "failed to write registry config `{}`: {e}",
            config_path.display()
        )
    })
}

/// Read a `config.json` from a registry index root.
pub fn read_registry_config(path: &Path) -> Result<RegistryConfig, String> {
    let config_path = path.join("config.json");
    let json = std::fs::read_to_string(&config_path).map_err(|e| {
        format!(
            "failed to read registry config `{}`: {e}",
            config_path.display()
        )
    })?;
    serde_json::from_str(&json).map_err(|e| {
        format!(
            "failed to parse registry config `{}`: {e}",
            config_path.display()
        )
    })
}

/// Write a package entry to its sharded path within a registry index.
pub fn write_registry_package_entry(
    index_root: &Path,
    entry: &RegistryPackageEntry,
) -> Result<(), String> {
    let index_path = registry_index_path(&entry.name);
    if index_path.is_empty() {
        return Err("cannot write index entry for empty package name".into());
    }
    let file_path = index_root.join(&index_path);
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            format!(
                "failed to create index directory `{}`: {e}",
                parent.display()
            )
        })?;
    }
    let json = serde_json::to_string_pretty(entry)
        .map_err(|e| format!("failed to serialize package entry `{}`: {e}", entry.name))?;
    std::fs::write(&file_path, json).map_err(|e| {
        format!(
            "failed to write package entry `{}`: {e}",
            file_path.display()
        )
    })
}

/// Read a package entry from its sharded path within a registry index.
pub fn read_registry_package_entry(
    index_root: &Path,
    name: &str,
) -> Result<RegistryPackageEntry, String> {
    let index_path = registry_index_path(name);
    if index_path.is_empty() {
        return Err(format!("cannot read index entry for empty package name"));
    }
    let file_path = index_root.join(&index_path);
    let json = std::fs::read_to_string(&file_path).map_err(|e| {
        format!(
            "failed to read package entry `{}`: {e}",
            file_path.display()
        )
    })?;
    serde_json::from_str(&json).map_err(|e| {
        format!(
            "failed to parse package entry `{}`: {e}",
            file_path.display()
        )
    })
}

/// Append a release record to an existing package entry in the index.
///
/// The release version must not already exist (immutability rule).
/// If the package entry does not exist, it is created automatically.
pub fn append_release_to_index(
    index_root: &Path,
    name: &str,
    release: &RegistryRelease,
) -> Result<PublishReceipt, String> {
    let index_path = registry_index_path(name);
    if index_path.is_empty() {
        return Err("cannot append release for empty package name".into());
    }

    let file_path = index_root.join(&index_path);
    let mut entry = if file_path.is_file() {
        read_registry_package_entry(index_root, name)?
    } else {
        RegistryPackageEntry {
            name: name.to_string(),
            owners: Vec::new(),
            description: None,
            keywords: Vec::new(),
            homepage: None,
            repository: None,
            created_at: release.published_at.clone(),
            releases: Vec::new(),
        }
    };

    // Immutability check: no duplicate versions.
    if entry.releases.iter().any(|r| r.version == release.version) {
        return Err(format!(
            "version `{}` of package `{name}` already exists in the index (immutable)",
            release.version
        ));
    }

    entry.releases.push(release.clone());
    write_registry_package_entry(index_root, &entry)?;

    Ok(PublishReceipt {
        name: name.to_string(),
        version: release.version.clone(),
        checksum: release.checksum.clone(),
        published_at: release.published_at.clone(),
        index_path,
    })
}

/// Mark an existing package release as yanked or available again.
pub fn set_registry_release_yanked(
    index_root: &Path,
    name: &str,
    version: &str,
    yanked: bool,
) -> Result<RegistryRelease, String> {
    let mut entry = read_registry_package_entry(index_root, name)?;
    let release = entry
        .releases
        .iter_mut()
        .find(|release| release.version == version)
        .ok_or_else(|| format!("package `{name}` has no release `{version}` in the index"))?;
    release.yanked = yanked;
    let updated = release.clone();
    write_registry_package_entry(index_root, &entry)?;
    Ok(updated)
}

fn merge_registry_entry_metadata(
    entry: &mut RegistryPackageEntry,
    manifest: &PublishManifest,
    owners: &[String],
) {
    let mut merged_owners = entry.owners.iter().cloned().collect::<BTreeSet<_>>();
    for owner in owners {
        let owner = owner.trim();
        if !owner.is_empty() {
            merged_owners.insert(owner.to_string());
        }
    }
    entry.owners = merged_owners.into_iter().collect();

    let mut merged_keywords = entry.keywords.iter().cloned().collect::<BTreeSet<_>>();
    for keyword in &manifest.keywords {
        let keyword = keyword.trim();
        if !keyword.is_empty() {
            merged_keywords.insert(keyword.to_string());
        }
    }
    entry.keywords = merged_keywords.into_iter().collect();

    if let Some(description) = manifest.description.as_ref() {
        entry.description = Some(description.clone());
    }
    if let Some(homepage) = manifest.homepage.as_ref() {
        entry.homepage = Some(homepage.clone());
    }
    if let Some(repository) = manifest.repository.as_ref() {
        entry.repository = Some(repository.clone());
    }
}

fn publish_validated_manifest_to_registry_index(
    index_root: &Path,
    manifest: &PublishManifest,
    owners: &[String],
    published_at: &str,
) -> Result<PublishReceipt, String> {
    let index_path = registry_index_path(&manifest.name);
    if index_path.is_empty() {
        return Err("cannot publish an empty package name".into());
    }

    let file_path = index_root.join(&index_path);
    let mut entry = if file_path.is_file() {
        read_registry_package_entry(index_root, &manifest.name)?
    } else {
        RegistryPackageEntry {
            name: manifest.name.clone(),
            owners: Vec::new(),
            description: None,
            keywords: Vec::new(),
            homepage: None,
            repository: None,
            created_at: published_at.to_string(),
            releases: Vec::new(),
        }
    };

    if entry
        .releases
        .iter()
        .any(|release| release.version == manifest.version)
    {
        return Err(format!(
            "version `{}` of package `{}` already exists in the index (immutable)",
            manifest.version, manifest.name
        ));
    }

    merge_registry_entry_metadata(&mut entry, manifest, owners);
    entry.releases.push(RegistryRelease {
        version: manifest.version.clone(),
        checksum: manifest.checksum.clone(),
        agam_version: manifest.agam_version.clone(),
        dependencies: manifest.dependencies.clone(),
        features: manifest.features.clone(),
        download_url: manifest.download_url.clone().or_else(|| {
            default_registry_release_download_url(index_root, &manifest.name, &manifest.version)
        }),
        provenance: Some(RegistryReleaseProvenance {
            source_checksum: manifest.checksum.clone(),
            manifest_checksum: manifest.manifest_checksum.clone(),
            published_by: owners.first().cloned(),
            source_repository: manifest.repository.clone(),
        }),
        published_at: published_at.to_string(),
        yanked: false,
    });
    write_registry_package_entry(index_root, &entry)?;

    Ok(PublishReceipt {
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        checksum: manifest.checksum.clone(),
        published_at: published_at.to_string(),
        index_path,
    })
}

/// Publish a validated manifest into a local registry index.
///
/// Each published version is immutable. Package-level metadata such as owners,
/// description, homepage, repository, and keywords is merged into the index
/// entry when provided.
pub fn publish_to_registry_index(
    index_root: &Path,
    manifest: &PublishManifest,
    owners: &[String],
    published_at: &str,
) -> Result<PublishReceipt, String> {
    validate_publish_manifest(manifest)?;
    publish_validated_manifest_to_registry_index(index_root, manifest, owners, published_at)
}

/// Publish an official first-party package into a local registry index.
pub fn publish_official_package_to_registry_index(
    index_root: &Path,
    manifest: &PublishManifest,
    owners: &[String],
    published_at: &str,
    registry_name: &str,
) -> Result<PublishReceipt, String> {
    validate_official_publish_manifest(manifest, registry_name, owners)?;
    publish_validated_manifest_to_registry_index(index_root, manifest, owners, published_at)
}

/// Build a `PublishManifest` from the current workspace session.
///
/// The checksum is computed from the content hash of the workspace source files.
pub fn build_publish_manifest(session: &WorkspaceSession) -> Result<PublishManifest, String> {
    let manifest = session.manifest.as_ref().ok_or_else(|| {
        "cannot build publish manifest: no `agam.toml` manifest found".to_string()
    })?;
    let manifest_path = session.layout.manifest_path.as_ref().ok_or_else(|| {
        "cannot build publish manifest: no `agam.toml` manifest found".to_string()
    })?;

    // Compute a source tarball checksum from all workspace source files.
    let mut payload = Vec::new();
    for file in &session.layout.source_files {
        let content = std::fs::read(file).map_err(|e| {
            format!(
                "failed to read source file `{}` for publish checksum: {e}",
                file.display()
            )
        })?;
        let relative = file
            .strip_prefix(&session.layout.root)
            .unwrap_or(file)
            .to_string_lossy();
        payload.extend_from_slice(relative.as_bytes());
        payload.push(b'\n');
        payload.extend_from_slice(&content);
    }
    let checksum = agam_runtime::cache::hash_bytes(&payload);
    let manifest_bytes = std::fs::read(manifest_path).map_err(|e| {
        format!(
            "failed to read workspace manifest `{}` for publish checksum: {e}",
            manifest_path.display()
        )
    })?;
    let manifest_checksum = agam_runtime::cache::hash_bytes(&manifest_bytes);

    // Collect dependencies from the manifest.
    let dependencies: Vec<RegistryReleaseDependency> = manifest
        .dependencies
        .iter()
        .map(|(name, spec)| RegistryReleaseDependency {
            name: spec.package.as_deref().unwrap_or(name).to_string(),
            version_req: spec.version.as_deref().unwrap_or("*").to_string(),
            registry: spec.registry.clone(),
            optional: spec.optional,
            features: spec.features.clone(),
        })
        .collect();

    Ok(PublishManifest {
        name: manifest.project.name.clone(),
        version: manifest.project.version.clone(),
        agam_version: manifest.project.agam.clone(),
        checksum,
        manifest_checksum,
        description: None,
        keywords: manifest.project.keywords.clone(),
        homepage: None,
        repository: None,
        download_url: None,
        dependencies,
        features: Vec::new(),
    })
}

fn validate_publish_manifest_fields(manifest: &PublishManifest) -> Result<(), String> {
    if manifest.name.is_empty() {
        return Err("publish manifest `name` must not be empty".into());
    }
    if manifest.version.is_empty() {
        return Err("publish manifest `version` must not be empty".into());
    }
    if manifest.checksum.is_empty() {
        return Err("publish manifest `checksum` must not be empty".into());
    }
    if manifest.manifest_checksum.is_empty() {
        return Err("publish manifest `manifest_checksum` must not be empty".into());
    }
    if manifest.agam_version.is_empty() {
        return Err("publish manifest `agam_version` must not be empty".into());
    }

    Ok(())
}

/// Validate a `PublishManifest` for completeness.
pub fn validate_publish_manifest(manifest: &PublishManifest) -> Result<(), String> {
    validate_publish_manifest_fields(manifest)?;
    validate_package_name(&manifest.name)
}

/// Validate an official first-party publish manifest against the Agam governance contract.
pub fn validate_official_publish_manifest(
    manifest: &PublishManifest,
    registry_name: &str,
    owners: &[String],
) -> Result<(), String> {
    validate_publish_manifest_fields(manifest)?;

    let governance = official_package_governance();
    validate_official_package_name(&manifest.name)?;

    if registry_name.trim() != governance.registry {
        return Err(format!(
            "official package `{}` must publish to registry `{}` instead of `{}`",
            manifest.name, governance.registry, registry_name
        ));
    }

    let allowed_owner = owners.iter().any(|owner| {
        let owner = owner.trim();
        !owner.is_empty()
            && governance
                .owner_handles
                .iter()
                .any(|allowed| owner == allowed)
    });
    if !allowed_owner {
        return Err(format!(
            "official package `{}` requires at least one owner from [{}]",
            manifest.name,
            governance.owner_handles.join(", ")
        ));
    }

    let repository = manifest.repository.as_deref().ok_or_else(|| {
        format!(
            "official package `{}` must declare a repository under `{}`",
            manifest.name, governance.repository_namespace
        )
    })?;
    if !official_repository_matches_namespace(repository, &governance) {
        return Err(format!(
            "official package `{}` repository `{repository}` must live under `{}`",
            manifest.name, governance.repository_namespace
        ));
    }

    Ok(())
}

/// Resolve a registry dependency by reading from a local index.
///
/// This upgrades the placeholder `resolve_registry_dependency()` to look up
/// real release metadata. If no local index is available, falls back to the
/// placeholder behavior.
pub fn resolve_registry_dependency_from_index(
    index_root: &Path,
    name: &str,
    version_req: &str,
) -> Result<LockedPackage, String> {
    let registry = registry_index_identity(index_root);
    let release = select_registry_release(index_root, name, Some(version_req))?;

    let dep_names: Vec<String> = release
        .dependencies
        .iter()
        .map(|d| format!("{}@{}", d.name, d.version_req))
        .collect();

    Ok(LockedPackage {
        name: name.to_string(),
        version: release.version.clone(),
        source: LockedPackageSource {
            kind: DependencySourceKind::Registry.label().to_string(),
            location: registry,
            reference: Some(format!("{name}@{}", release.version)),
        },
        content_hash: release.checksum.clone(),
        dependencies: dep_names,
    })
}

/// Select the newest non-yanked registry release matching a version requirement.
pub fn select_registry_release(
    index_root: &Path,
    name: &str,
    version_req: Option<&str>,
) -> Result<RegistryRelease, String> {
    let entry = read_registry_package_entry(index_root, name)?;
    select_registry_release_from_entry(&entry, version_req).ok_or_else(|| {
        match version_req.filter(|req| !req.trim().is_empty()) {
            Some(version_req) => {
                format!("no matching release for `{name}` with version requirement `{version_req}`")
            }
            None => format!("package `{name}` has no non-yanked releases in the index"),
        }
    })
}

/// Produce human-readable audit lines for a registry package.
pub fn audit_registry_package(index_root: &Path, name: &str) -> Result<Vec<String>, String> {
    let entry = read_registry_package_entry(index_root, name)?;
    let mut lines = Vec::new();

    lines.push(format!("package: {}", entry.name));
    if let Some(ref desc) = entry.description {
        lines.push(format!("description: {desc}"));
    }
    if !entry.owners.is_empty() {
        lines.push(format!("owners: {}", entry.owners.join(", ")));
    }
    lines.push(format!("created: {}", entry.created_at));
    lines.push(format!("releases: {}", entry.releases.len()));

    for release in &entry.releases {
        let yanked = if release.yanked { " [yanked]" } else { "" };
        lines.push(format!(
            "  {} (checksum: {}, published: {}{})",
            release.version, release.checksum, release.published_at, yanked
        ));
        if let Some(download_url) = release.download_url.as_deref() {
            lines.push(format!("    download: {download_url}"));
        }
        if let Some(provenance) = release.provenance.as_ref() {
            lines.push(format!(
                "    provenance: source={}, manifest={}",
                provenance.source_checksum, provenance.manifest_checksum
            ));
            if let Some(published_by) = provenance.published_by.as_deref() {
                lines.push(format!("    published_by: {published_by}"));
            }
            if let Some(source_repository) = provenance.source_repository.as_deref() {
                lines.push(format!("    source_repository: {source_repository}"));
            }
        }
        for dep in &release.dependencies {
            let opt = if dep.optional { " [optional]" } else { "" };
            lines.push(format!("    dep: {} {}{opt}", dep.name, dep.version_req));
        }
    }

    Ok(lines)
}

/// Simple version matching: exact match or wildcard `*`.
///
/// Full semver range matching (^, ~, >=, etc.) is a future concern.
fn version_matches(version: &str, requirement: &str) -> bool {
    let requirement = requirement.trim();
    if requirement.is_empty() || requirement == "*" {
        return true;
    }
    // Strip leading ^ or ~ for basic prefix matching.
    let req = requirement.trim_start_matches('^').trim_start_matches('~');
    version == req || version.starts_with(&format!("{req}."))
}

fn select_registry_release_from_entry(
    entry: &RegistryPackageEntry,
    version_req: Option<&str>,
) -> Option<RegistryRelease> {
    entry
        .releases
        .iter()
        .filter(|release| !release.yanked)
        .filter(|release| {
            version_req
                .filter(|req| !req.trim().is_empty())
                .map(|req| version_matches(&release.version, req))
                .unwrap_or(true)
        })
        .max_by(|left, right| compare_registry_versions(&left.version, &right.version))
        .cloned()
}

fn compare_registry_versions(left: &str, right: &str) -> Ordering {
    let (left_core, left_prerelease) = split_registry_version(left);
    let (right_core, right_prerelease) = split_registry_version(right);

    let core_order = compare_version_identifiers(
        left_core.split('.').filter(|segment| !segment.is_empty()),
        right_core.split('.').filter(|segment| !segment.is_empty()),
        true,
    );
    if core_order != Ordering::Equal {
        return core_order;
    }

    match (left_prerelease, right_prerelease) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(left_pre), Some(right_pre)) => compare_version_identifiers(
            left_pre.split('.').filter(|segment| !segment.is_empty()),
            right_pre.split('.').filter(|segment| !segment.is_empty()),
            false,
        ),
    }
}

fn split_registry_version(version: &str) -> (&str, Option<&str>) {
    let without_build = version.split('+').next().unwrap_or(version);
    match without_build.split_once('-') {
        Some((core, prerelease)) => (core, Some(prerelease)),
        None => (without_build, None),
    }
}

fn compare_version_identifiers<'a>(
    mut left: impl Iterator<Item = &'a str>,
    mut right: impl Iterator<Item = &'a str>,
    core: bool,
) -> Ordering {
    loop {
        match (left.next(), right.next()) {
            (Some(left_id), Some(right_id)) => {
                let order = compare_version_identifier(left_id, right_id, core);
                if order != Ordering::Equal {
                    return order;
                }
            }
            (Some(left_id), None) => {
                if core {
                    if !identifier_is_zero(left_id) {
                        return Ordering::Greater;
                    }
                } else {
                    return Ordering::Greater;
                }
            }
            (None, Some(right_id)) => {
                if core {
                    if !identifier_is_zero(right_id) {
                        return Ordering::Less;
                    }
                } else {
                    return Ordering::Less;
                }
            }
            (None, None) => return Ordering::Equal,
        }
    }
}

fn compare_version_identifier(left: &str, right: &str, core: bool) -> Ordering {
    match (left.parse::<u64>(), right.parse::<u64>()) {
        (Ok(left_num), Ok(right_num)) => left_num.cmp(&right_num),
        (Ok(_), Err(_)) if !core => Ordering::Less,
        (Err(_), Ok(_)) if !core => Ordering::Greater,
        _ => left.cmp(right),
    }
}

fn identifier_is_zero(identifier: &str) -> bool {
    identifier
        .parse::<u64>()
        .map(|value| value == 0)
        .unwrap_or(false)
}

fn is_pkg_name_start(b: u8) -> bool {
    b.is_ascii_lowercase() || b.is_ascii_digit()
}

fn is_pkg_name_char(b: u8) -> bool {
    b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_'
}

fn is_separator(b: u8) -> bool {
    b == b'-' || b == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::{Mutex, OnceLock};

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

    fn registry_env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct RegistryEnvRestore {
        key: String,
        previous: Option<std::ffi::OsString>,
    }

    impl RegistryEnvRestore {
        fn capture(key: &str) -> Self {
            Self {
                key: key.to_string(),
                previous: std::env::var_os(key),
            }
        }
    }

    impl Drop for RegistryEnvRestore {
        fn drop(&mut self) {
            match self.previous.as_ref() {
                Some(previous) => unsafe {
                    std::env::set_var(&self.key, previous);
                },
                None => unsafe {
                    std::env::remove_var(&self.key);
                },
            }
        }
    }

    fn with_registry_env<R>(key: &str, value: &Path, f: impl FnOnce() -> R) -> R {
        let _guard = registry_env_lock()
            .lock()
            .expect("registry env lock should not be poisoned");
        let restore = RegistryEnvRestore::capture(key);
        unsafe {
            std::env::set_var(key, value);
        }
        let result = f();
        drop(restore);
        result
    }

    fn without_registry_env<R>(key: &str, f: impl FnOnce() -> R) -> R {
        let _guard = registry_env_lock()
            .lock()
            .expect("registry env lock should not be poisoned");
        let restore = RegistryEnvRestore::capture(key);
        unsafe {
            std::env::remove_var(key);
        }
        let result = f();
        drop(restore);
        result
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
                    packaged_sysroot: None,
                },
                SdkTargetProfile {
                    name: "android-arm64".into(),
                    target_triple: "aarch64-linux-android21".into(),
                    backend: RuntimeBackend::Llvm,
                    sysroot_env: Some("AGAM_LLVM_SYSROOT".into()),
                    sdk_env: None,
                    packaged_sysroot: Some("target-packs/android-arm64/sysroot".into()),
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
        assert_eq!(
            decoded.supported_targets[1].packaged_sysroot.as_deref(),
            Some("target-packs/android-arm64/sysroot")
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
    fn default_environment_name_prefers_dev() {
        let mut manifest = scaffold_workspace_manifest("env-default");
        manifest.environments.insert(
            "release".into(),
            EnvironmentSpec {
                target: Some("x86_64-unknown-linux-gnu".into()),
                ..EnvironmentSpec::default()
            },
        );
        manifest.environments.insert(
            "dev".into(),
            EnvironmentSpec {
                preferred_backend: Some(RuntimeBackend::Jit),
                ..EnvironmentSpec::default()
            },
        );

        assert_eq!(default_environment_name(&manifest).as_deref(), Some("dev"));
    }

    #[test]
    fn select_environment_name_rejects_ambiguous_implicit_choice() {
        let mut manifest = scaffold_workspace_manifest("env-select");
        manifest.environments.insert(
            "release".into(),
            EnvironmentSpec {
                target: Some("x86_64-unknown-linux-gnu".into()),
                ..EnvironmentSpec::default()
            },
        );
        manifest.environments.insert(
            "android-arm64".into(),
            EnvironmentSpec {
                target: Some("aarch64-linux-android21".into()),
                ..EnvironmentSpec::default()
            },
        );

        let error = select_environment_name(&manifest, None)
            .expect_err("multiple environments without dev should require an explicit name");
        assert!(error.contains("multiple environments"));
    }

    #[test]
    fn resolve_environment_merges_manifest_toolchain_and_locked_packages() {
        let mut manifest = scaffold_workspace_manifest("env-resolve");
        manifest.toolchain = Some(ToolchainRequirement {
            agam: "0.2.0".into(),
            sdk: Some("host-native".into()),
            target: Some("x86_64-pc-windows-msvc".into()),
            runtime_abi: Some(RUNTIME_ABI_VERSION),
            preferred_backend: Some(RuntimeBackend::Llvm),
        });
        manifest.environments.insert(
            "dev".into(),
            EnvironmentSpec {
                preferred_backend: Some(RuntimeBackend::Jit),
                profiles: vec!["debug".into()],
                ..EnvironmentSpec::default()
            },
        );
        let lockfile = WorkspaceLockfile {
            format_version: LOCKFILE_FORMAT_VERSION,
            workspace: LockedWorkspace {
                name: "env-resolve".into(),
                version: "0.1.0".into(),
            },
            packages: vec![LockedPackage {
                name: "tensor-core".into(),
                version: "1.4.0".into(),
                source: LockedPackageSource {
                    kind: "registry".into(),
                    location: "agam".into(),
                    reference: Some("tensor-core@1.4.0".into()),
                },
                content_hash: "sha256-tensor-core".into(),
                dependencies: vec![],
            }],
            environments: BTreeMap::new(),
        };

        let environment =
            resolve_environment(&manifest, &lockfile, None).expect("resolve environment");
        let environment = environment.expect("dev environment should be selected");
        assert_eq!(environment.name, "dev");
        assert_eq!(environment.compiler, "0.2.0");
        assert_eq!(environment.sdk.as_deref(), Some("host-native"));
        assert_eq!(
            environment.target.as_deref(),
            Some("x86_64-pc-windows-msvc")
        );
        assert_eq!(environment.runtime_abi, Some(RUNTIME_ABI_VERSION));
        assert_eq!(environment.preferred_backend, Some(RuntimeBackend::Jit));
        assert_eq!(environment.profiles, vec!["debug".to_string()]);
        assert_eq!(environment.packages, vec!["tensor-core@1.4.0".to_string()]);
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

    #[test]
    fn resolve_workspace_session_collects_direct_path_dependency_metadata() {
        let root = temp_dir("workspace_path_dependency_session");
        let root_entry = root.join("src").join("main.agam");
        let dep_root = root.join("libs").join("core");
        let dep_entry = dep_root.join("src").join("main.agam");
        let dep_manifest_path = dep_root.join("agam.toml");
        fs::create_dir_all(root_entry.parent().expect("root entry parent"))
            .expect("create root src");
        fs::create_dir_all(dep_entry.parent().expect("dep entry parent")).expect("create dep src");

        let mut root_manifest = scaffold_workspace_manifest("workspace-root");
        root_manifest.dependencies.insert(
            "core".into(),
            DependencySpec {
                path: Some("libs/core".into()),
                ..DependencySpec::default()
            },
        );
        write_workspace_manifest_to_path(&root.join("agam.toml"), &root_manifest)
            .expect("write root manifest");

        let mut dep_manifest = scaffold_workspace_manifest("workspace-core");
        dep_manifest.project.entry = Some("src/main.agam".into());
        write_workspace_manifest_to_path(&dep_manifest_path, &dep_manifest)
            .expect("write dependency manifest");

        fs::write(&root_entry, sample_source()).expect("write root entry");
        fs::write(&dep_entry, sample_source()).expect("write dependency entry");

        let session =
            resolve_workspace_session(Some(root.clone())).expect("resolve workspace session");

        assert_eq!(session.path_dependencies.len(), 1);
        let dependency = &session.path_dependencies[0];
        assert_eq!(dependency.dependency_key, "core");
        assert_eq!(dependency.table, WorkspaceDependencyTable::Main);
        assert_eq!(dependency.root, dep_root);
        assert_eq!(
            dependency.manifest_path.as_deref(),
            Some(dep_manifest_path.as_path())
        );
        assert_eq!(
            dependency
                .manifest
                .as_ref()
                .map(|manifest| manifest.project.name.as_str()),
            Some("workspace-core")
        );
        assert!(dependency.path_dependencies.is_empty());
        assert!(
            !session.layout.source_files.contains(&dep_entry),
            "direct path dependency sources should stay outside the workspace source expansion"
        );

        let resolved = resolve_workspace_path_dependencies(&session)
            .expect("resolve path dependency metadata from session");
        assert_eq!(resolved, session.path_dependencies);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn resolve_workspace_session_collects_transitive_path_dependency_metadata() {
        let root = temp_dir("workspace_transitive_path_dependency_session");
        let root_entry = root.join("src").join("main.agam");
        let alpha_root = root.join("libs").join("alpha");
        let beta_root = alpha_root.join("beta");
        let alpha_manifest_path = alpha_root.join("agam.toml");
        let beta_manifest_path = beta_root.join("agam.toml");
        fs::create_dir_all(root_entry.parent().expect("root entry parent"))
            .expect("create root src");
        fs::create_dir_all(&alpha_root).expect("create alpha root");
        fs::create_dir_all(&beta_root).expect("create beta root");

        let mut root_manifest = scaffold_workspace_manifest("workspace-root");
        root_manifest.dependencies.insert(
            "alpha".into(),
            DependencySpec {
                path: Some("libs/alpha".into()),
                ..DependencySpec::default()
            },
        );
        write_workspace_manifest_to_path(&root.join("agam.toml"), &root_manifest)
            .expect("write root manifest");

        let mut alpha_manifest = scaffold_workspace_manifest("workspace-alpha");
        alpha_manifest.dependencies.insert(
            "beta".into(),
            DependencySpec {
                path: Some("beta".into()),
                ..DependencySpec::default()
            },
        );
        write_workspace_manifest_to_path(&alpha_manifest_path, &alpha_manifest)
            .expect("write alpha manifest");
        write_workspace_manifest_to_path(
            &beta_manifest_path,
            &scaffold_workspace_manifest("workspace-beta"),
        )
        .expect("write beta manifest");
        fs::write(&root_entry, sample_source()).expect("write root entry");

        let session =
            resolve_workspace_session(Some(root.clone())).expect("resolve workspace session");

        assert_eq!(session.path_dependencies.len(), 1);
        let alpha = &session.path_dependencies[0];
        assert_eq!(alpha.dependency_key, "alpha");
        assert_eq!(alpha.root, alpha_root);
        assert_eq!(alpha.path_dependencies.len(), 1);

        let beta = &alpha.path_dependencies[0];
        assert_eq!(beta.dependency_key, "beta");
        assert_eq!(beta.root, beta_root);
        assert_eq!(
            beta.manifest
                .as_ref()
                .map(|manifest| manifest.project.name.as_str()),
            Some("workspace-beta")
        );
        assert!(beta.path_dependencies.is_empty());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn snapshot_workspace_tracks_path_dependency_manifests_and_manifest_diffs() {
        let root = temp_dir("workspace_path_dependency_snapshot");
        let root_entry = root.join("src").join("main.agam");
        let dep_root = root.join("libs").join("core");
        let dep_manifest_path = dep_root.join("agam.toml");
        fs::create_dir_all(root_entry.parent().expect("root entry parent"))
            .expect("create root src");
        fs::create_dir_all(&dep_root).expect("create dependency root");

        let mut root_manifest = scaffold_workspace_manifest("workspace-root");
        root_manifest.dependencies.insert(
            "core".into(),
            DependencySpec {
                path: Some("libs/core".into()),
                ..DependencySpec::default()
            },
        );
        write_workspace_manifest_to_path(&root.join("agam.toml"), &root_manifest)
            .expect("write root manifest");
        write_workspace_manifest_to_path(
            &dep_manifest_path,
            &scaffold_workspace_manifest("workspace-core"),
        )
        .expect("write dependency manifest");
        fs::write(&root_entry, sample_source()).expect("write root entry");

        let previous = snapshot_workspace(Some(root.clone())).expect("snapshot workspace");
        assert_eq!(previous.manifests.len(), 2);
        assert!(
            previous
                .manifests
                .iter()
                .any(|snapshot| snapshot.path == dep_manifest_path)
        );

        let mut dep_manifest = scaffold_workspace_manifest("workspace-core");
        dep_manifest.project.version = "0.2.0".into();
        write_workspace_manifest_to_path(&dep_manifest_path, &dep_manifest)
            .expect("rewrite dependency manifest");

        let next = snapshot_workspace(Some(root.clone())).expect("snapshot workspace");
        let diff = diff_workspace_snapshots(&previous, &next);

        assert!(previous.is_stale(&next));
        assert_eq!(diff.changed_files, vec![dep_manifest_path]);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn snapshot_workspace_tracks_transitive_path_dependency_manifests() {
        let root = temp_dir("workspace_transitive_path_dependency_snapshot");
        let root_entry = root.join("src").join("main.agam");
        let alpha_root = root.join("libs").join("alpha");
        let beta_root = alpha_root.join("beta");
        let alpha_manifest_path = alpha_root.join("agam.toml");
        let beta_manifest_path = beta_root.join("agam.toml");
        fs::create_dir_all(root_entry.parent().expect("root entry parent"))
            .expect("create root src");
        fs::create_dir_all(&alpha_root).expect("create alpha root");
        fs::create_dir_all(&beta_root).expect("create beta root");

        let mut root_manifest = scaffold_workspace_manifest("workspace-root");
        root_manifest.dependencies.insert(
            "alpha".into(),
            DependencySpec {
                path: Some("libs/alpha".into()),
                ..DependencySpec::default()
            },
        );
        write_workspace_manifest_to_path(&root.join("agam.toml"), &root_manifest)
            .expect("write root manifest");

        let mut alpha_manifest = scaffold_workspace_manifest("workspace-alpha");
        alpha_manifest.dependencies.insert(
            "beta".into(),
            DependencySpec {
                path: Some("beta".into()),
                ..DependencySpec::default()
            },
        );
        write_workspace_manifest_to_path(&alpha_manifest_path, &alpha_manifest)
            .expect("write alpha manifest");
        write_workspace_manifest_to_path(
            &beta_manifest_path,
            &scaffold_workspace_manifest("workspace-beta"),
        )
        .expect("write beta manifest");
        fs::write(&root_entry, sample_source()).expect("write root entry");

        let previous = snapshot_workspace(Some(root.clone())).expect("snapshot workspace");
        assert_eq!(previous.manifests.len(), 3);
        assert!(
            previous
                .manifests
                .iter()
                .any(|snapshot| snapshot.path == beta_manifest_path)
        );

        let mut beta_manifest = scaffold_workspace_manifest("workspace-beta");
        beta_manifest.project.version = "0.2.0".into();
        write_workspace_manifest_to_path(&beta_manifest_path, &beta_manifest)
            .expect("rewrite beta manifest");

        let next = snapshot_workspace(Some(root.clone())).expect("snapshot workspace");
        let diff = diff_workspace_snapshots(&previous, &next);
        assert!(diff.changed_files.contains(&beta_manifest_path));

        let _ = fs::remove_dir_all(root);
    }

    // -----------------------------------------------------------------------
    // Phase 17B — Deterministic Resolver and Lockfile Tests
    // -----------------------------------------------------------------------

    #[test]
    fn resolves_empty_dependencies_to_empty_lockfile() {
        let dir = temp_dir("resolve_empty");
        let entry = dir.join("src").join("main.agam");
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        fs::write(&entry, sample_source()).expect("write entry");
        write_workspace_manifest_to_path(
            &default_manifest_path(&dir),
            &scaffold_workspace_manifest("empty-deps"),
        )
        .expect("write manifest");

        let session =
            resolve_workspace_session(Some(dir.clone())).expect("resolve workspace session");
        let lockfile = resolve_dependencies(&session).expect("resolve dependencies");

        assert_eq!(lockfile.format_version, LOCKFILE_FORMAT_VERSION);
        assert_eq!(lockfile.workspace.name, "empty-deps");
        assert!(lockfile.packages.is_empty());
        assert!(lockfile.environments.is_empty());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn resolves_path_dependency_with_content_hash() {
        let dir = temp_dir("resolve_path_dep");
        let entry = dir.join("src").join("main.agam");
        let lib_dir = dir.join("libs").join("core");
        let lib_file = lib_dir.join("lib.agam");
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        fs::create_dir_all(&lib_dir).expect("create lib dir");
        fs::write(&entry, sample_source()).expect("write entry");
        fs::write(&lib_file, "fn helper() -> i32:\n    return 42\n").expect("write lib");

        let mut manifest = scaffold_workspace_manifest("path-dep-project");
        manifest.dependencies.insert(
            "core".into(),
            DependencySpec {
                path: Some("libs/core".into()),
                version: Some("0.1.0".into()),
                ..DependencySpec::default()
            },
        );
        write_workspace_manifest_to_path(&default_manifest_path(&dir), &manifest)
            .expect("write manifest");

        let lockfile = without_registry_env(DEFAULT_REGISTRY_INDEX_ENV, || {
            let session =
                resolve_workspace_session(Some(dir.clone())).expect("resolve workspace session");
            resolve_dependencies(&session).expect("resolve dependencies")
        });

        assert_eq!(lockfile.packages.len(), 1);
        let locked = &lockfile.packages[0];
        assert_eq!(locked.name, "core");
        assert_eq!(locked.version, "0.1.0");
        assert_eq!(locked.source.kind, "path");
        assert_eq!(locked.source.location, "libs/core");
        assert!(!locked.content_hash.is_empty());

        // Running again should produce the same hash.
        let lockfile2 = without_registry_env(DEFAULT_REGISTRY_INDEX_ENV, || {
            let session =
                resolve_workspace_session(Some(dir.clone())).expect("resolve workspace session");
            resolve_dependencies(&session).expect("resolve dependencies again")
        });
        assert_eq!(
            lockfile.packages[0].content_hash,
            lockfile2.packages[0].content_hash
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn resolves_git_dependency_records_url_and_ref() {
        let dir = temp_dir("resolve_git_dep");
        let entry = dir.join("src").join("main.agam");
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        fs::write(&entry, sample_source()).expect("write entry");

        let mut manifest = scaffold_workspace_manifest("git-dep-project");
        manifest.dependencies.insert(
            "tensor-ops".into(),
            DependencySpec {
                git: Some("https://github.com/agam-lang/tensor-ops".into()),
                rev: Some("abc123".into()),
                ..DependencySpec::default()
            },
        );
        write_workspace_manifest_to_path(&default_manifest_path(&dir), &manifest)
            .expect("write manifest");

        let session =
            resolve_workspace_session(Some(dir.clone())).expect("resolve workspace session");
        let lockfile = resolve_dependencies(&session).expect("resolve dependencies");

        assert_eq!(lockfile.packages.len(), 1);
        let locked = &lockfile.packages[0];
        assert_eq!(locked.name, "tensor-ops");
        assert_eq!(locked.source.kind, "git");
        assert_eq!(
            locked.source.location,
            "https://github.com/agam-lang/tensor-ops"
        );
        assert_eq!(locked.source.reference.as_deref(), Some("abc123"));
        assert!(!locked.content_hash.is_empty());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn resolves_registry_dependency_placeholder() {
        let dir = temp_dir("resolve_registry_dep");
        let entry = dir.join("src").join("main.agam");
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        fs::write(&entry, sample_source()).expect("write entry");

        let mut manifest = scaffold_workspace_manifest("registry-project");
        manifest.dependencies.insert(
            "math-lib".into(),
            DependencySpec {
                version: Some("^1.2".into()),
                registry: Some("agam".into()),
                ..DependencySpec::default()
            },
        );
        write_workspace_manifest_to_path(&default_manifest_path(&dir), &manifest)
            .expect("write manifest");

        let lockfile = without_registry_env(DEFAULT_REGISTRY_INDEX_ENV, || {
            let session =
                resolve_workspace_session(Some(dir.clone())).expect("resolve workspace session");
            resolve_dependencies(&session).expect("resolve dependencies")
        });

        assert_eq!(lockfile.packages.len(), 1);
        let locked = &lockfile.packages[0];
        assert_eq!(locked.name, "math-lib");
        assert_eq!(locked.version, "^1.2");
        assert_eq!(locked.source.kind, "registry");
        assert_eq!(locked.source.location, "agam");
        assert_eq!(locked.source.reference.as_deref(), Some("math-lib@^1.2"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn resolves_registry_dependency_from_configured_local_index() {
        let dir = temp_dir("resolve_registry_dep_index");
        let entry = dir.join("src").join("main.agam");
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        fs::write(&entry, sample_source()).expect("write entry");

        let index_root = dir.join("registry-index");
        write_registry_config(
            &index_root,
            &RegistryConfig {
                format_version: REGISTRY_INDEX_FORMAT_VERSION,
                api_url: None,
                download_url: None,
                name: Some("agam".into()),
            },
        )
        .expect("write registry config");
        append_release_to_index(
            &index_root,
            "math-lib",
            &RegistryRelease {
                version: "1.2.3".into(),
                checksum: "sha256-math".into(),
                agam_version: "0.1".into(),
                dependencies: vec![RegistryReleaseDependency {
                    name: "core".into(),
                    version_req: "^1.0".into(),
                    registry: None,
                    optional: false,
                    features: vec![],
                }],
                features: vec!["simd".into()],
                download_url: None,
                provenance: None,
                published_at: "2026-04-10T00:00:00Z".into(),
                yanked: false,
            },
        )
        .expect("publish registry release");

        let mut manifest = scaffold_workspace_manifest("registry-project");
        manifest.dependencies.insert(
            "math-lib".into(),
            DependencySpec {
                version: Some("1.2.3".into()),
                registry: Some("agam".into()),
                ..DependencySpec::default()
            },
        );
        write_workspace_manifest_to_path(&default_manifest_path(&dir), &manifest)
            .expect("write manifest");

        let lockfile = with_registry_env(DEFAULT_REGISTRY_INDEX_ENV, &index_root, || {
            let session =
                resolve_workspace_session(Some(dir.clone())).expect("resolve workspace session");
            resolve_dependencies(&session).expect("resolve dependencies")
        });

        assert_eq!(lockfile.packages.len(), 1);
        let locked = &lockfile.packages[0];
        assert_eq!(locked.name, "math-lib");
        assert_eq!(locked.version, "1.2.3");
        assert_eq!(locked.source.kind, "registry");
        assert_eq!(locked.source.location, "agam");
        assert_eq!(locked.content_hash, "sha256-math");
        assert_eq!(locked.dependencies, vec!["core@^1.0".to_string()]);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn lockfile_freshness_detects_added_dependency() {
        let manifest = scaffold_workspace_manifest("fresh-test");
        let lockfile = WorkspaceLockfile {
            format_version: LOCKFILE_FORMAT_VERSION,
            workspace: LockedWorkspace {
                name: "fresh-test".into(),
                version: "0.1.0".into(),
            },
            packages: Vec::new(),
            environments: BTreeMap::new(),
        };

        // Empty deps → empty lockfile is fresh.
        assert!(is_lockfile_fresh(&manifest, &lockfile));

        // Add a dep to the manifest → lockfile becomes stale.
        let mut manifest_with_dep = manifest.clone();
        manifest_with_dep.dependencies.insert(
            "new-dep".into(),
            DependencySpec {
                version: Some("1.0".into()),
                ..DependencySpec::default()
            },
        );
        assert!(!is_lockfile_fresh(&manifest_with_dep, &lockfile));
    }

    #[test]
    fn lockfile_freshness_detects_removed_dependency() {
        let mut manifest = scaffold_workspace_manifest("remove-test");
        manifest.dependencies.insert(
            "old-dep".into(),
            DependencySpec {
                version: Some("1.0".into()),
                ..DependencySpec::default()
            },
        );

        let lockfile = WorkspaceLockfile {
            format_version: LOCKFILE_FORMAT_VERSION,
            workspace: LockedWorkspace {
                name: "remove-test".into(),
                version: "0.1.0".into(),
            },
            packages: vec![LockedPackage {
                name: "old-dep".into(),
                version: "1.0".into(),
                source: LockedPackageSource {
                    kind: "registry".into(),
                    location: "agam".into(),
                    reference: None,
                },
                content_hash: "abc".into(),
                dependencies: Vec::new(),
            }],
            environments: BTreeMap::new(),
        };

        assert!(is_lockfile_fresh(&manifest, &lockfile));

        // Remove the dep from the manifest → stale.
        manifest.dependencies.clear();
        assert!(!is_lockfile_fresh(&manifest, &lockfile));
    }

    #[test]
    fn lockfile_freshness_accepts_matching_lockfile() {
        let mut manifest = scaffold_workspace_manifest("match-test");
        manifest.dependencies.insert(
            "alpha".into(),
            DependencySpec {
                version: Some("1.0".into()),
                ..DependencySpec::default()
            },
        );
        manifest.dev_dependencies.insert(
            "beta".into(),
            DependencySpec {
                git: Some("https://example.com/beta".into()),
                ..DependencySpec::default()
            },
        );

        let lockfile = WorkspaceLockfile {
            format_version: LOCKFILE_FORMAT_VERSION,
            workspace: LockedWorkspace {
                name: "match-test".into(),
                version: "0.1.0".into(),
            },
            packages: vec![
                LockedPackage {
                    name: "alpha".into(),
                    version: "1.0".into(),
                    source: LockedPackageSource {
                        kind: "registry".into(),
                        location: "agam".into(),
                        reference: None,
                    },
                    content_hash: "hash_a".into(),
                    dependencies: Vec::new(),
                },
                LockedPackage {
                    name: "beta".into(),
                    version: "0.0.0".into(),
                    source: LockedPackageSource {
                        kind: "git".into(),
                        location: "https://example.com/beta".into(),
                        reference: None,
                    },
                    content_hash: "hash_b".into(),
                    dependencies: Vec::new(),
                },
            ],
            environments: BTreeMap::new(),
        };

        assert!(is_lockfile_fresh(&manifest, &lockfile));
    }

    #[test]
    fn lockfile_freshness_detects_changed_registry_version_requirement() {
        let mut manifest = scaffold_workspace_manifest("version-drift");
        manifest.dependencies.insert(
            "json".into(),
            DependencySpec {
                version: Some("^1".into()),
                ..DependencySpec::default()
            },
        );

        let lockfile = WorkspaceLockfile {
            format_version: LOCKFILE_FORMAT_VERSION,
            workspace: LockedWorkspace {
                name: "version-drift".into(),
                version: "0.1.0".into(),
            },
            packages: vec![LockedPackage {
                name: "json".into(),
                version: "1.4.0".into(),
                source: LockedPackageSource {
                    kind: "registry".into(),
                    location: "agam".into(),
                    reference: Some("json@1.4.0".into()),
                },
                content_hash: "sha256-json-140".into(),
                dependencies: Vec::new(),
            }],
            environments: BTreeMap::new(),
        };

        assert!(is_lockfile_fresh(&manifest, &lockfile));

        manifest.dependencies.insert(
            "json".into(),
            DependencySpec {
                version: Some("^2".into()),
                ..DependencySpec::default()
            },
        );

        assert!(!is_lockfile_fresh(&manifest, &lockfile));
        assert!(
            lockfile_diagnostics(&manifest, &lockfile)
                .iter()
                .any(|diagnostic| { diagnostic.contains("no longer matches dependency `json`") })
        );
    }

    #[test]
    fn lockfile_freshness_accepts_package_aliases_using_locked_package_name() {
        let mut manifest = scaffold_workspace_manifest("alias-freshness");
        manifest.dependencies.insert(
            "my-json".into(),
            DependencySpec {
                package: Some("json".into()),
                version: Some("^1".into()),
                ..DependencySpec::default()
            },
        );

        let lockfile = WorkspaceLockfile {
            format_version: LOCKFILE_FORMAT_VERSION,
            workspace: LockedWorkspace {
                name: "alias-freshness".into(),
                version: "0.1.0".into(),
            },
            packages: vec![LockedPackage {
                name: "json".into(),
                version: "1.2.0".into(),
                source: LockedPackageSource {
                    kind: "registry".into(),
                    location: "agam".into(),
                    reference: Some("json@1.2.0".into()),
                },
                content_hash: "sha256-json-120".into(),
                dependencies: Vec::new(),
            }],
            environments: BTreeMap::new(),
        };

        assert!(is_lockfile_fresh(&manifest, &lockfile));
        assert!(lockfile_diagnostics(&manifest, &lockfile).is_empty());
    }

    #[test]
    fn lockfile_freshness_accepts_matching_environment_records() {
        let mut manifest = scaffold_workspace_manifest("env-freshness");
        manifest.dependencies.insert(
            "json".into(),
            DependencySpec {
                version: Some("^1".into()),
                ..DependencySpec::default()
            },
        );
        manifest.environments.insert(
            "dev".into(),
            EnvironmentSpec {
                compiler: Some("0.2.0".into()),
                sdk: Some("host-native".into()),
                target: Some("x86_64-unknown-linux-gnu".into()),
                preferred_backend: Some(RuntimeBackend::Jit),
                ..EnvironmentSpec::default()
            },
        );

        let mut environments = BTreeMap::new();
        environments.insert(
            "dev".into(),
            LockedEnvironment {
                compiler: Some("0.2.0".into()),
                sdk: Some("host-native".into()),
                target: Some("x86_64-unknown-linux-gnu".into()),
                preferred_backend: Some(RuntimeBackend::Jit),
                packages: vec!["json@1.2.0".into()],
            },
        );

        let lockfile = WorkspaceLockfile {
            format_version: LOCKFILE_FORMAT_VERSION,
            workspace: LockedWorkspace {
                name: "env-freshness".into(),
                version: "0.1.0".into(),
            },
            packages: vec![LockedPackage {
                name: "json".into(),
                version: "1.2.0".into(),
                source: LockedPackageSource {
                    kind: "registry".into(),
                    location: "agam".into(),
                    reference: Some("json@1.2.0".into()),
                },
                content_hash: "sha256-json-120".into(),
                dependencies: Vec::new(),
            }],
            environments,
        };

        assert!(is_lockfile_fresh(&manifest, &lockfile));
        assert!(lockfile_diagnostics(&manifest, &lockfile).is_empty());
    }

    #[test]
    fn lockfile_freshness_rejects_environment_drift() {
        let mut manifest = scaffold_workspace_manifest("env-drift");
        manifest.dependencies.insert(
            "json".into(),
            DependencySpec {
                version: Some("^1".into()),
                ..DependencySpec::default()
            },
        );
        manifest.environments.insert(
            "dev".into(),
            EnvironmentSpec {
                compiler: Some("0.2.0".into()),
                sdk: Some("host-native".into()),
                preferred_backend: Some(RuntimeBackend::Jit),
                ..EnvironmentSpec::default()
            },
        );

        let mut environments = BTreeMap::new();
        environments.insert(
            "dev".into(),
            LockedEnvironment {
                compiler: Some("0.2.0".into()),
                sdk: Some("legacy-sdk".into()),
                preferred_backend: Some(RuntimeBackend::Llvm),
                packages: vec!["json@1.2.0".into()],
                ..LockedEnvironment::default()
            },
        );

        let lockfile = WorkspaceLockfile {
            format_version: LOCKFILE_FORMAT_VERSION,
            workspace: LockedWorkspace {
                name: "env-drift".into(),
                version: "0.1.0".into(),
            },
            packages: vec![LockedPackage {
                name: "json".into(),
                version: "1.2.0".into(),
                source: LockedPackageSource {
                    kind: "registry".into(),
                    location: "agam".into(),
                    reference: Some("json@1.2.0".into()),
                },
                content_hash: "sha256-json-120".into(),
                dependencies: Vec::new(),
            }],
            environments,
        };

        assert!(!is_lockfile_fresh(&manifest, &lockfile));
        assert!(
            lockfile_diagnostics(&manifest, &lockfile)
                .iter()
                .any(|diagnostic| diagnostic.contains("locked environment `dev`"))
        );
    }

    #[test]
    fn deterministic_resolution_order() {
        let dir = temp_dir("deterministic_order");
        let entry = dir.join("src").join("main.agam");
        let lib_z = dir.join("libs").join("zeta");
        let lib_a = dir.join("libs").join("alpha");
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        fs::create_dir_all(&lib_z).expect("create zeta");
        fs::create_dir_all(&lib_a).expect("create alpha");
        fs::write(&entry, sample_source()).expect("write entry");
        fs::write(lib_z.join("lib.agam"), "fn z() -> i32:\n    return 0\n").expect("write zeta");
        fs::write(lib_a.join("lib.agam"), "fn a() -> i32:\n    return 1\n").expect("write alpha");

        let mut manifest = scaffold_workspace_manifest("order-test");
        // Insert in reverse alphabetical order — resolver should still sort.
        manifest.dependencies.insert(
            "zeta".into(),
            DependencySpec {
                path: Some("libs/zeta".into()),
                ..DependencySpec::default()
            },
        );
        manifest.dependencies.insert(
            "alpha".into(),
            DependencySpec {
                path: Some("libs/alpha".into()),
                ..DependencySpec::default()
            },
        );
        write_workspace_manifest_to_path(&default_manifest_path(&dir), &manifest)
            .expect("write manifest");

        let session =
            resolve_workspace_session(Some(dir.clone())).expect("resolve workspace session");

        let lockfile1 = resolve_dependencies(&session).expect("resolve first");
        let lockfile2 = resolve_dependencies(&session).expect("resolve second");

        // Both runs must produce byte-identical package order.
        assert_eq!(lockfile1.packages.len(), 2);
        assert_eq!(lockfile1.packages[0].name, "alpha");
        assert_eq!(lockfile1.packages[1].name, "zeta");
        assert_eq!(lockfile1, lockfile2);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn content_hash_directory_is_deterministic() {
        let dir = temp_dir("content_hash_det");
        let sub = dir.join("pkg");
        fs::create_dir_all(&sub).expect("create pkg dir");
        fs::write(sub.join("b.agam"), "fn b() -> i32:\n    return 2\n").expect("write b");
        fs::write(sub.join("a.agam"), "fn a() -> i32:\n    return 1\n").expect("write a");

        let hash1 = content_hash_directory(&sub).expect("hash first");
        let hash2 = content_hash_directory(&sub).expect("hash second");

        assert_eq!(hash1, hash2);
        assert!(!hash1.is_empty());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn resolves_transitive_path_dependencies() {
        let dir = temp_dir("transitive_deps");
        let entry = dir.join("src").join("main.agam");
        let lib_a = dir.join("libs").join("alpha");
        let lib_b = lib_a.join("beta"); // Beta is inside Alpha's workspace
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        fs::create_dir_all(lib_a.join("src")).expect("create alpha src");
        fs::create_dir_all(&lib_b).expect("create beta");
        fs::write(&entry, sample_source()).expect("write entry");
        fs::write(
            lib_a.join("src").join("main.agam"),
            "fn a() -> i32:\n    return 1\n",
        )
        .expect("write alpha source");
        fs::write(lib_b.join("lib.agam"), "fn b() -> i32:\n    return 2\n")
            .expect("write beta source");

        // Root depends on alpha.
        let mut root_manifest = scaffold_workspace_manifest("transitive-root");
        root_manifest.dependencies.insert(
            "alpha".into(),
            DependencySpec {
                path: Some("libs/alpha".into()),
                ..DependencySpec::default()
            },
        );
        write_workspace_manifest_to_path(&default_manifest_path(&dir), &root_manifest)
            .expect("write root manifest");

        // Alpha's manifest declares a dependency on beta (inside its own root).
        let mut alpha_manifest = scaffold_workspace_manifest("alpha");
        alpha_manifest.dependencies.insert(
            "beta".into(),
            DependencySpec {
                path: Some("beta".into()),
                ..DependencySpec::default()
            },
        );
        write_workspace_manifest_to_path(&default_manifest_path(&lib_a), &alpha_manifest)
            .expect("write alpha manifest");

        let session =
            resolve_workspace_session(Some(dir.clone())).expect("resolve workspace session");
        let lockfile = resolve_dependencies(&session).expect("resolve dependencies");

        // Should include both alpha and beta (transitive).
        assert_eq!(lockfile.packages.len(), 2);
        let names: Vec<&str> = lockfile.packages.iter().map(|p| p.name.as_str()).collect();
        assert!(
            names.contains(&"alpha"),
            "should contain alpha: {:?}",
            names
        );
        assert!(names.contains(&"beta"), "should contain beta: {:?}", names);

        // Alpha should list beta as a dependency.
        let alpha_pkg = lockfile
            .packages
            .iter()
            .find(|p| p.name == "alpha")
            .unwrap();
        assert!(
            alpha_pkg
                .dependencies
                .iter()
                .any(|d| d.starts_with("beta@")),
            "alpha should list beta as dep: {:?}",
            alpha_pkg.dependencies
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn resolves_transitive_path_dependencies_from_session_metadata() {
        let dir = temp_dir("transitive_deps_session_metadata");
        let entry = dir.join("src").join("main.agam");
        let lib_a = dir.join("libs").join("alpha");
        let lib_b = lib_a.join("beta");
        let alpha_manifest_path = default_manifest_path(&lib_a);
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        fs::create_dir_all(lib_a.join("src")).expect("create alpha src");
        fs::create_dir_all(&lib_b).expect("create beta");
        fs::write(&entry, sample_source()).expect("write entry");
        fs::write(
            lib_a.join("src").join("main.agam"),
            "fn a() -> i32:\n    return 1\n",
        )
        .expect("write alpha source");
        fs::write(lib_b.join("lib.agam"), "fn b() -> i32:\n    return 2\n")
            .expect("write beta source");

        let mut root_manifest = scaffold_workspace_manifest("transitive-root");
        root_manifest.dependencies.insert(
            "alpha".into(),
            DependencySpec {
                path: Some("libs/alpha".into()),
                ..DependencySpec::default()
            },
        );
        write_workspace_manifest_to_path(&default_manifest_path(&dir), &root_manifest)
            .expect("write root manifest");

        let mut alpha_manifest = scaffold_workspace_manifest("alpha");
        alpha_manifest.dependencies.insert(
            "beta".into(),
            DependencySpec {
                path: Some("beta".into()),
                ..DependencySpec::default()
            },
        );
        write_workspace_manifest_to_path(&alpha_manifest_path, &alpha_manifest)
            .expect("write alpha manifest");

        let session =
            resolve_workspace_session(Some(dir.clone())).expect("resolve workspace session");
        fs::remove_file(&alpha_manifest_path).expect("remove direct dependency manifest");

        let lockfile = resolve_dependencies(&session).expect("resolve dependencies from session");
        let names: Vec<&str> = lockfile.packages.iter().map(|p| p.name.as_str()).collect();
        assert!(
            names.contains(&"alpha"),
            "should contain alpha: {:?}",
            names
        );
        assert!(
            names.contains(&"beta"),
            "session metadata should preserve transitive dependency discovery: {:?}",
            names
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn resolves_nested_transitive_path_dependencies_from_session_metadata() {
        let dir = temp_dir("transitive_nested_deps_session_metadata");
        let entry = dir.join("src").join("main.agam");
        let alpha_root = dir.join("libs").join("alpha");
        let beta_root = alpha_root.join("beta");
        let gamma_root = beta_root.join("gamma");
        let alpha_manifest_path = default_manifest_path(&alpha_root);
        let beta_manifest_path = default_manifest_path(&beta_root);
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        fs::create_dir_all(alpha_root.join("src")).expect("create alpha src");
        fs::create_dir_all(beta_root.join("src")).expect("create beta src");
        fs::create_dir_all(&gamma_root).expect("create gamma");
        fs::write(&entry, sample_source()).expect("write entry");
        fs::write(
            alpha_root.join("src").join("main.agam"),
            "fn a() -> i32:\n    return 1\n",
        )
        .expect("write alpha source");
        fs::write(
            beta_root.join("src").join("main.agam"),
            "fn b() -> i32:\n    return 2\n",
        )
        .expect("write beta source");
        fs::write(
            gamma_root.join("lib.agam"),
            "fn g() -> i32:\n    return 3\n",
        )
        .expect("write gamma source");

        let mut root_manifest = scaffold_workspace_manifest("transitive-root");
        root_manifest.dependencies.insert(
            "alpha".into(),
            DependencySpec {
                path: Some("libs/alpha".into()),
                ..DependencySpec::default()
            },
        );
        write_workspace_manifest_to_path(&default_manifest_path(&dir), &root_manifest)
            .expect("write root manifest");

        let mut alpha_manifest = scaffold_workspace_manifest("alpha");
        alpha_manifest.dependencies.insert(
            "beta".into(),
            DependencySpec {
                path: Some("beta".into()),
                ..DependencySpec::default()
            },
        );
        write_workspace_manifest_to_path(&alpha_manifest_path, &alpha_manifest)
            .expect("write alpha manifest");

        let mut beta_manifest = scaffold_workspace_manifest("beta");
        beta_manifest.dependencies.insert(
            "gamma".into(),
            DependencySpec {
                path: Some("gamma".into()),
                ..DependencySpec::default()
            },
        );
        write_workspace_manifest_to_path(&beta_manifest_path, &beta_manifest)
            .expect("write beta manifest");

        let session =
            resolve_workspace_session(Some(dir.clone())).expect("resolve workspace session");
        fs::remove_file(&alpha_manifest_path).expect("remove alpha manifest");
        fs::remove_file(&beta_manifest_path).expect("remove beta manifest");

        let lockfile = resolve_dependencies(&session).expect("resolve dependencies from session");
        let names: Vec<&str> = lockfile.packages.iter().map(|p| p.name.as_str()).collect();
        assert!(
            names.contains(&"alpha"),
            "should contain alpha: {:?}",
            names
        );
        assert!(names.contains(&"beta"), "should contain beta: {:?}", names);
        assert!(
            names.contains(&"gamma"),
            "nested session metadata should preserve deeper transitive dependency discovery: {:?}",
            names
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn generate_or_refresh_lockfile_refreshes_on_path_dependency_content_drift() {
        let dir = temp_dir("lockfile_refresh_path_drift");
        let entry = dir.join("src").join("main.agam");
        let lib_dir = dir.join("libs").join("core");
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        fs::create_dir_all(&lib_dir).expect("create lib dir");
        fs::write(&entry, sample_source()).expect("write entry");
        fs::write(
            lib_dir.join("lib.agam"),
            "fn core() -> i32:\n    return 1\n",
        )
        .expect("write core lib");

        let mut manifest = scaffold_workspace_manifest("drift-refresh-project");
        manifest.dependencies.insert(
            "core".into(),
            DependencySpec {
                path: Some("libs/core".into()),
                ..DependencySpec::default()
            },
        );
        write_workspace_manifest_to_path(&default_manifest_path(&dir), &manifest)
            .expect("write manifest");

        let session =
            resolve_workspace_session(Some(dir.clone())).expect("resolve workspace session");
        let initial = generate_or_refresh_lockfile(&session).expect("generate lockfile");
        let initial_hash = initial
            .packages
            .iter()
            .find(|package| package.name == "core")
            .expect("locked core package")
            .content_hash
            .clone();

        fs::write(
            lib_dir.join("lib.agam"),
            "fn core() -> i32:\n    return 99\n",
        )
        .expect("modify core lib");

        let refreshed = generate_or_refresh_lockfile(&session).expect("refresh lockfile");
        let refreshed_hash = refreshed
            .packages
            .iter()
            .find(|package| package.name == "core")
            .expect("locked core package after refresh")
            .content_hash
            .clone();

        assert_ne!(initial_hash, refreshed_hash);
        assert!(
            lockfile_content_drift(&dir, &refreshed).is_empty(),
            "refreshed lockfile should match the live path dependency content"
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn lockfile_content_drift_detects_modified_path_dep() {
        let dir = temp_dir("drift_detection");
        let entry = dir.join("src").join("main.agam");
        let lib_dir = dir.join("libs").join("core");
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        fs::create_dir_all(&lib_dir).expect("create lib dir");
        fs::write(&entry, sample_source()).expect("write entry");
        fs::write(
            lib_dir.join("lib.agam"),
            "fn core() -> i32:\n    return 1\n",
        )
        .expect("write core lib");

        let mut manifest = scaffold_workspace_manifest("drift-project");
        manifest.dependencies.insert(
            "core".into(),
            DependencySpec {
                path: Some("libs/core".into()),
                ..DependencySpec::default()
            },
        );
        write_workspace_manifest_to_path(&default_manifest_path(&dir), &manifest)
            .expect("write manifest");

        let session =
            resolve_workspace_session(Some(dir.clone())).expect("resolve workspace session");
        let lockfile = resolve_dependencies(&session).expect("resolve dependencies");

        // No drift initially.
        let drift = lockfile_content_drift(&dir, &lockfile);
        assert!(drift.is_empty(), "no drift expected initially");

        // Modify the path dep.
        fs::write(
            lib_dir.join("lib.agam"),
            "fn core() -> i32:\n    return 999\n",
        )
        .expect("modify core lib");

        // Now there should be drift.
        let drift = lockfile_content_drift(&dir, &lockfile);
        assert_eq!(drift.len(), 1);
        assert_eq!(drift[0].0, "core");
        assert_ne!(drift[0].1, drift[0].2); // lockfile hash != live hash

        let _ = fs::remove_dir_all(dir);
    }

    // -----------------------------------------------------------------------
    // Phase 17C — Registry Index and Publish Protocol tests
    // -----------------------------------------------------------------------

    #[test]
    fn validate_package_name_accepts_valid() {
        let valid = [
            "io",
            "net",
            "json",
            "my-lib",
            "my_lib",
            "lib123",
            "a1b2c3",
            "some-longer-package-name",
        ];
        for name in valid {
            assert!(
                validate_package_name(name).is_ok(),
                "expected `{name}` to be valid"
            );
        }
    }

    #[test]
    fn validate_package_name_rejects_invalid() {
        // Too short.
        assert!(validate_package_name("a").is_err());
        // Too long.
        let long = "a".repeat(65);
        assert!(validate_package_name(&long).is_err());
        // Starts with hyphen.
        assert!(validate_package_name("-foo").is_err());
        // Ends with hyphen.
        assert!(validate_package_name("foo-").is_err());
        // Uppercase.
        assert!(validate_package_name("Foo").is_err());
        // Consecutive hyphens.
        assert!(validate_package_name("foo--bar").is_err());
        // Reserved prefix.
        assert!(validate_package_name("agam-core").is_err());
        // Contains dot.
        assert!(validate_package_name("foo.bar").is_err());
    }

    #[test]
    fn validate_official_package_name_accepts_reserved_prefix() {
        assert!(validate_official_package_name("agam-std").is_ok());
        assert!(validate_official_package_name("agam-dataframe").is_ok());
        assert!(validate_official_package_name("json").is_err());
    }

    #[test]
    fn registry_index_path_sharding() {
        assert_eq!(registry_index_path(""), "");
        assert_eq!(registry_index_path("a"), "1/a");
        assert_eq!(registry_index_path("io"), "2/io");
        assert_eq!(registry_index_path("net"), "3/n/net");
        assert_eq!(registry_index_path("json"), "js/on/json");
        assert_eq!(registry_index_path("my-lib"), "my/-l/my-lib");
        assert_eq!(
            registry_index_path("some-longer-name"),
            "so/me/some-longer-name"
        );
    }

    #[test]
    fn registry_config_roundtrip() {
        let dir = temp_dir("reg_config_rt");
        let config = RegistryConfig {
            format_version: REGISTRY_INDEX_FORMAT_VERSION,
            api_url: Some("https://registry.agam-lang.org/api/v1".into()),
            download_url: Some("https://registry.agam-lang.org/dl".into()),
            name: Some("agam".into()),
        };
        write_registry_config(&dir, &config).expect("write config");
        let loaded = read_registry_config(&dir).expect("read config");
        assert_eq!(config, loaded);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn registry_package_entry_roundtrip() {
        let dir = temp_dir("reg_entry_rt");
        let entry = RegistryPackageEntry {
            name: "json".to_string(),
            owners: vec!["alice".into()],
            description: Some("JSON parser".into()),
            keywords: vec!["json".into(), "parser".into()],
            homepage: None,
            repository: Some("https://github.com/agam-lang/json".into()),
            created_at: "2026-01-01T00:00:00Z".into(),
            releases: vec![RegistryRelease {
                version: "1.0.0".into(),
                checksum: "abc123".into(),
                agam_version: "0.1".into(),
                dependencies: vec![],
                features: vec![],
                download_url: None,
                provenance: None,
                published_at: "2026-01-01T00:00:00Z".into(),
                yanked: false,
            }],
        };
        write_registry_package_entry(&dir, &entry).expect("write entry");
        let loaded = read_registry_package_entry(&dir, "json").expect("read entry");
        assert_eq!(entry, loaded);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn append_release_immutability() {
        let dir = temp_dir("reg_immutable");
        let release1 = RegistryRelease {
            version: "1.0.0".into(),
            checksum: "hash1".into(),
            agam_version: "0.1".into(),
            dependencies: vec![],
            features: vec![],
            download_url: None,
            provenance: None,
            published_at: "2026-01-01T00:00:00Z".into(),
            yanked: false,
        };
        let receipt = append_release_to_index(&dir, "mylib", &release1).expect("first append");
        assert_eq!(receipt.name, "mylib");
        assert_eq!(receipt.version, "1.0.0");

        // Second append with the same version must fail.
        let result = append_release_to_index(&dir, "mylib", &release1);
        assert!(result.is_err());
        assert!(
            result.unwrap_err().contains("already exists"),
            "expected immutability error"
        );

        // A different version should succeed.
        let release2 = RegistryRelease {
            version: "1.1.0".into(),
            checksum: "hash2".into(),
            agam_version: "0.1".into(),
            dependencies: vec![],
            features: vec![],
            download_url: None,
            provenance: None,
            published_at: "2026-02-01T00:00:00Z".into(),
            yanked: false,
        };
        let receipt2 =
            append_release_to_index(&dir, "mylib", &release2).expect("second version append");
        assert_eq!(receipt2.version, "1.1.0");

        // Verify the entry has both releases.
        let entry = read_registry_package_entry(&dir, "mylib").expect("read entry");
        assert_eq!(entry.releases.len(), 2);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn publish_manifest_validation() {
        // Empty name.
        let m1 = PublishManifest {
            name: "".into(),
            version: "1.0.0".into(),
            agam_version: "0.1".into(),
            checksum: "abc".into(),
            manifest_checksum: "manifest-abc".into(),
            description: None,
            keywords: vec![],
            homepage: None,
            repository: None,
            download_url: None,
            dependencies: vec![],
            features: vec![],
        };
        assert!(validate_publish_manifest(&m1).is_err());

        // Empty version.
        let m2 = PublishManifest {
            name: "mylib".into(),
            version: "".into(),
            agam_version: "0.1".into(),
            checksum: "abc".into(),
            manifest_checksum: "manifest-abc".into(),
            description: None,
            keywords: vec![],
            homepage: None,
            repository: None,
            download_url: None,
            dependencies: vec![],
            features: vec![],
        };
        assert!(validate_publish_manifest(&m2).is_err());

        // Empty checksum.
        let m3 = PublishManifest {
            name: "mylib".into(),
            version: "1.0.0".into(),
            agam_version: "0.1".into(),
            checksum: "".into(),
            manifest_checksum: "manifest-abc".into(),
            description: None,
            keywords: vec![],
            homepage: None,
            repository: None,
            download_url: None,
            dependencies: vec![],
            features: vec![],
        };
        assert!(validate_publish_manifest(&m3).is_err());

        // Valid manifest.
        let m4 = PublishManifest {
            name: "mylib".into(),
            version: "1.0.0".into(),
            agam_version: "0.1".into(),
            checksum: "abc123".into(),
            manifest_checksum: "".into(),
            description: None,
            keywords: vec![],
            homepage: None,
            repository: None,
            download_url: None,
            dependencies: vec![],
            features: vec![],
        };
        assert!(validate_publish_manifest(&m4).is_err());

        // Valid manifest.
        let m5 = PublishManifest {
            name: "mylib".into(),
            version: "1.0.0".into(),
            agam_version: "0.1".into(),
            checksum: "abc123".into(),
            manifest_checksum: "manifest-abc".into(),
            description: None,
            keywords: vec![],
            homepage: None,
            repository: None,
            download_url: None,
            dependencies: vec![],
            features: vec![],
        };
        assert!(validate_publish_manifest(&m5).is_ok());
    }

    #[test]
    fn official_publish_manifest_requires_canonical_registry_owner_and_repository() {
        let manifest = PublishManifest {
            name: "agam-std".into(),
            version: "0.1.0".into(),
            agam_version: "0.1".into(),
            checksum: "official-checksum".into(),
            manifest_checksum: "official-manifest".into(),
            description: Some("official std".into()),
            keywords: vec!["std".into()],
            homepage: None,
            repository: Some("https://github.com/agam-lang/agam-std".into()),
            download_url: None,
            dependencies: vec![],
            features: vec![],
        };

        assert!(
            validate_official_publish_manifest(&manifest, "agam", &["agam-lang".into()]).is_ok()
        );
        assert!(
            validate_official_publish_manifest(&manifest, "mirror", &["agam-lang".into()]).is_err()
        );
        assert!(validate_official_publish_manifest(&manifest, "agam", &["alice".into()]).is_err());

        let mut wrong_repo = manifest.clone();
        wrong_repo.repository = Some("https://github.com/example/agam-std".into());
        assert!(
            validate_official_publish_manifest(&wrong_repo, "agam", &["agam-lang".into()]).is_err()
        );
    }

    #[test]
    fn publish_official_package_to_registry_index_persists_reserved_prefix_package() {
        let dir = temp_dir("official_publish_registry_index");
        write_registry_config(
            &dir,
            &RegistryConfig {
                format_version: REGISTRY_INDEX_FORMAT_VERSION,
                api_url: None,
                download_url: Some("https://registry.agam-lang.org/dl".into()),
                name: Some("agam".into()),
            },
        )
        .expect("write config");

        let manifest = PublishManifest {
            name: "agam-std".into(),
            version: "0.1.0".into(),
            agam_version: "0.1".into(),
            checksum: "official-checksum".into(),
            manifest_checksum: "official-manifest".into(),
            description: Some("official std".into()),
            keywords: vec!["std".into()],
            homepage: None,
            repository: Some("https://github.com/agam-lang/agam-std".into()),
            download_url: None,
            dependencies: vec![],
            features: vec!["io".into()],
        };

        let receipt = publish_official_package_to_registry_index(
            &dir,
            &manifest,
            &["agam-lang".into()],
            "2026-04-10T12:00:00Z",
            "agam",
        )
        .expect("publish official package");

        assert_eq!(receipt.name, "agam-std");
        let entry = read_registry_package_entry(&dir, "agam-std").expect("read official entry");
        assert_eq!(entry.name, "agam-std");
        assert_eq!(entry.owners, vec!["agam-lang".to_string()]);
        assert_eq!(entry.releases.len(), 1);
        assert_eq!(entry.releases[0].version, "0.1.0");

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn publish_to_registry_index_persists_package_metadata() {
        let dir = temp_dir("publish_registry_index");
        write_registry_config(
            &dir,
            &RegistryConfig {
                format_version: REGISTRY_INDEX_FORMAT_VERSION,
                api_url: None,
                download_url: Some("https://registry.agam-lang.org/dl".into()),
                name: Some("agam".into()),
            },
        )
        .expect("write config");

        let manifest = PublishManifest {
            name: "mylib".into(),
            version: "1.0.0".into(),
            agam_version: "0.1".into(),
            checksum: "abc123".into(),
            manifest_checksum: "manifest-abc".into(),
            description: Some("My sample package".into()),
            keywords: vec!["json".into(), "parser".into()],
            homepage: Some("https://example.com/mylib".into()),
            repository: Some("https://github.com/agam-lang/mylib".into()),
            download_url: None,
            dependencies: vec![RegistryReleaseDependency {
                name: "core".into(),
                version_req: "^1.0".into(),
                registry: None,
                optional: false,
                features: vec!["simd".into()],
            }],
            features: vec!["fast".into()],
        };

        let receipt = publish_to_registry_index(
            &dir,
            &manifest,
            &["alice".into(), "bob".into()],
            "2026-04-10T12:00:00Z",
        )
        .expect("publish manifest");

        assert_eq!(receipt.name, "mylib");
        assert_eq!(receipt.version, "1.0.0");

        let entry = read_registry_package_entry(&dir, "mylib").expect("read entry");
        assert_eq!(entry.owners, vec!["alice".to_string(), "bob".to_string()]);
        assert_eq!(entry.description.as_deref(), Some("My sample package"));
        assert_eq!(
            entry.repository.as_deref(),
            Some("https://github.com/agam-lang/mylib")
        );
        assert_eq!(
            entry.keywords,
            vec!["json".to_string(), "parser".to_string()]
        );
        assert_eq!(entry.releases.len(), 1);
        assert_eq!(entry.releases[0].features, vec!["fast".to_string()]);
        assert_eq!(
            entry.releases[0].download_url.as_deref(),
            Some("https://registry.agam-lang.org/dl/mylib/1.0.0/mylib-1.0.0.agam-src.tar.gz")
        );
        let provenance = entry.releases[0]
            .provenance
            .as_ref()
            .expect("publish should persist provenance");
        assert_eq!(provenance.source_checksum, "abc123");
        assert_eq!(provenance.manifest_checksum, "manifest-abc");
        assert_eq!(provenance.published_by.as_deref(), Some("alice"));
        assert_eq!(
            provenance.source_repository.as_deref(),
            Some("https://github.com/agam-lang/mylib")
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn resolve_registry_from_index() {
        let dir = temp_dir("reg_resolve");
        let release = RegistryRelease {
            version: "2.0.0".into(),
            checksum: "sha256-abc".into(),
            agam_version: "0.1".into(),
            dependencies: vec![RegistryReleaseDependency {
                name: "core".into(),
                version_req: "^1.0".into(),
                registry: None,
                optional: false,
                features: vec![],
            }],
            features: vec![],
            download_url: None,
            provenance: None,
            published_at: "2026-03-01T00:00:00Z".into(),
            yanked: false,
        };
        append_release_to_index(&dir, "json", &release).expect("publish json");

        let locked =
            resolve_registry_dependency_from_index(&dir, "json", "2.0.0").expect("resolve");
        assert_eq!(locked.name, "json");
        assert_eq!(locked.version, "2.0.0");
        assert_eq!(locked.content_hash, "sha256-abc");
        assert_eq!(locked.source.kind, "registry");
        assert_eq!(locked.dependencies.len(), 1);
        assert!(locked.dependencies[0].starts_with("core@"));

        // Wildcard match.
        let locked_wild =
            resolve_registry_dependency_from_index(&dir, "json", "*").expect("resolve wildcard");
        assert_eq!(locked_wild.version, "2.0.0");

        // Non-existent version.
        let result = resolve_registry_dependency_from_index(&dir, "json", "3.0.0");
        assert!(result.is_err());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn select_registry_release_prefers_latest_non_yanked_match() {
        let dir = temp_dir("reg_select_latest");
        append_release_to_index(
            &dir,
            "json",
            &RegistryRelease {
                version: "1.0.0".into(),
                checksum: "sha256-100".into(),
                agam_version: "0.1".into(),
                dependencies: vec![],
                features: vec![],
                download_url: None,
                provenance: None,
                published_at: "2026-01-01T00:00:00Z".into(),
                yanked: false,
            },
        )
        .expect("publish 1.0.0");
        append_release_to_index(
            &dir,
            "json",
            &RegistryRelease {
                version: "1.2.0".into(),
                checksum: "sha256-120".into(),
                agam_version: "0.1".into(),
                dependencies: vec![],
                features: vec![],
                download_url: None,
                provenance: None,
                published_at: "2026-02-01T00:00:00Z".into(),
                yanked: false,
            },
        )
        .expect("publish 1.2.0");
        append_release_to_index(
            &dir,
            "json",
            &RegistryRelease {
                version: "2.0.0".into(),
                checksum: "sha256-200".into(),
                agam_version: "0.1".into(),
                dependencies: vec![],
                features: vec![],
                download_url: None,
                provenance: None,
                published_at: "2026-03-01T00:00:00Z".into(),
                yanked: true,
            },
        )
        .expect("publish yanked 2.0.0");

        let latest = select_registry_release(&dir, "json", None).expect("select latest release");
        assert_eq!(latest.version, "1.2.0");

        let latest_matching =
            select_registry_release(&dir, "json", Some("^1")).expect("select matching release");
        assert_eq!(latest_matching.version, "1.2.0");

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn audit_registry_package_output() {
        let dir = temp_dir("reg_audit");
        let release = RegistryRelease {
            version: "1.0.0".into(),
            checksum: "deadbeef".into(),
            agam_version: "0.1".into(),
            dependencies: vec![RegistryReleaseDependency {
                name: "core".into(),
                version_req: "^1.0".into(),
                registry: None,
                optional: false,
                features: vec![],
            }],
            features: vec![],
            download_url: None,
            provenance: None,
            published_at: "2026-04-01T00:00:00Z".into(),
            yanked: false,
        };
        append_release_to_index(&dir, "mylib", &release).expect("publish");

        let lines = audit_registry_package(&dir, "mylib").expect("audit");
        assert!(lines.iter().any(|l| l.contains("package: mylib")));
        assert!(lines.iter().any(|l| l.contains("releases: 1")));
        assert!(lines.iter().any(|l| l.contains("1.0.0")));
        assert!(lines.iter().any(|l| l.contains("deadbeef")));
        assert!(lines.iter().any(|l| l.contains("dep: core")));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn set_registry_release_yanked_updates_index_entry() {
        let dir = temp_dir("reg_yank");
        append_release_to_index(
            &dir,
            "mylib",
            &RegistryRelease {
                version: "1.0.0".into(),
                checksum: "deadbeef".into(),
                agam_version: "0.1".into(),
                dependencies: vec![],
                features: vec![],
                download_url: Some("https://example.com/mylib-1.0.0.tar.gz".into()),
                provenance: Some(RegistryReleaseProvenance {
                    source_checksum: "deadbeef".into(),
                    manifest_checksum: "manifest-deadbeef".into(),
                    published_by: Some("alice".into()),
                    source_repository: Some("https://github.com/agam-lang/mylib".into()),
                }),
                published_at: "2026-04-11T00:00:00Z".into(),
                yanked: false,
            },
        )
        .expect("publish");

        let yanked = set_registry_release_yanked(&dir, "mylib", "1.0.0", true)
            .expect("yank release should succeed");
        assert!(yanked.yanked);

        let entry = read_registry_package_entry(&dir, "mylib").expect("read updated entry");
        assert!(entry.releases[0].yanked);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn first_party_distribution_profiles_include_expected_taxonomy() {
        let profiles = first_party_distribution_profiles();
        let names = profiles
            .iter()
            .map(|profile| profile.name.as_str())
            .collect::<Vec<_>>();
        assert!(names.contains(&"base"));
        assert!(names.contains(&"systems"));
        assert!(names.contains(&"data-ai"));
    }

    #[test]
    fn first_party_distribution_profile_lookup_returns_curated_packages() {
        let profile =
            first_party_distribution_profile("systems").expect("systems profile should exist");
        assert_eq!(profile.name, "systems");
        assert!(
            profile
                .packages
                .iter()
                .any(|package| package.name == "agam-ffi")
        );
        assert!(
            profile
                .notes
                .iter()
                .any(|note| note.contains("opt-in") || note.contains("explicit"))
        );
    }

    #[test]
    fn official_package_governance_uses_reserved_prefix_and_registry() {
        let governance = official_package_governance();
        assert_eq!(governance.registry, "agam");
        assert_eq!(governance.reserved_prefix, "agam-");
        assert!(governance.repository_namespace.contains("agam-lang"));
        assert!(
            governance
                .publication_rules
                .iter()
                .any(|rule| rule.contains("reserved `agam-` prefix"))
        );
    }
}
