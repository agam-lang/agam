//! Portable Agam package format and helpers.

use std::collections::BTreeMap;
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
    toml::from_str(&manifest).map_err(|e| {
        format!(
            "failed to parse workspace manifest `{}`: {e}",
            path.display()
        )
    })
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
    let hint = match path {
        Some(path) => path,
        None => std::env::current_dir()
            .map_err(|e| format!("failed to read current directory: {e}"))?,
    };

    resolve_workspace_layout_from_path(&hint)
}

/// Resolve the canonical workspace layout from an explicit filesystem path.
pub fn resolve_workspace_layout_from_path(path: &Path) -> Result<WorkspaceLayout, String> {
    if path.is_file() {
        if path.file_name().and_then(|name| name.to_str()) == Some("agam.toml") {
            let root = path
                .parent()
                .ok_or_else(|| format!("manifest `{}` has no parent directory", path.display()))?;
            return workspace_layout_from_root(root, Some(path.to_path_buf()), None);
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
        return workspace_layout_from_root(&root, manifest, Some(path.to_path_buf()));
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
    workspace_layout_from_root(&root, manifest, None)
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

fn find_workspace_manifest(start: &Path) -> Option<PathBuf> {
    for ancestor in start.ancestors() {
        let manifest = ancestor.join("agam.toml");
        if manifest.is_file() {
            return Some(manifest);
        }
    }
    None
}

fn workspace_layout_from_root(
    root: &Path,
    manifest_path: Option<PathBuf>,
    entry_override: Option<PathBuf>,
) -> Result<WorkspaceLayout, String> {
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

    let mut source_files = Vec::new();
    let src_dir = root.join("src");
    if src_dir.is_dir() {
        collect_agam_files(&src_dir, &mut source_files)?;
    }
    if !source_files.iter().any(|file| file == &entry_file) {
        source_files.push(entry_file.clone());
    }
    source_files.sort();
    source_files.dedup();

    let mut test_files = Vec::new();
    let tests_dir = root.join("tests");
    if tests_dir.is_dir() {
        collect_agam_files(&tests_dir, &mut test_files)?;
    }
    test_files.sort();
    test_files.dedup();

    Ok(WorkspaceLayout {
        root: root.to_path_buf(),
        manifest_path,
        project_name,
        entry_file,
        source_files,
        test_files,
    })
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
}
