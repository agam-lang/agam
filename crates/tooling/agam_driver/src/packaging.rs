//! Package, publish, registry, SDK distribution, and doctor.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PublishReport {
    pub dry_run: bool,
    pub official: bool,
    pub workspace_root: PathBuf,
    pub manifest_path: PathBuf,
    pub index_root: PathBuf,
    pub index_name: String,
    pub index_path: String,
    pub owners: Vec<String>,
    pub manifest: agam_pkg::PublishManifest,
    pub receipt: Option<agam_pkg::PublishReceipt>,
    pub bootstrapped_config: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegistryInspectReport {
    pub index_root: PathBuf,
    pub index_name: String,
    pub index_path: String,
    pub entry: agam_pkg::RegistryPackageEntry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegistryAuditReport {
    pub index_root: PathBuf,
    pub index_name: String,
    pub index_path: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegistryProfileListReport {
    pub profiles: Vec<agam_pkg::FirstPartyDistributionProfile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegistryProfileInspectReport {
    pub profile: agam_pkg::FirstPartyDistributionProfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegistryGovernanceReport {
    pub governance: agam_pkg::OfficialPackageGovernance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EnvironmentListReport {
    pub workspace_root: PathBuf,
    pub manifest_path: PathBuf,
    pub default_environment: Option<String>,
    pub environments: Vec<agam_pkg::ResolvedEnvironment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EnvironmentInspectReport {
    pub workspace_root: PathBuf,
    pub manifest_path: PathBuf,
    pub selected_by_default: bool,
    pub environment: agam_pkg::ResolvedEnvironment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegistryYankReport {
    pub index_root: PathBuf,
    pub index_name: String,
    pub package_name: String,
    pub version: String,
    pub yanked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegistryInstallReport {
    pub workspace_root: PathBuf,
    pub manifest_path: PathBuf,
    pub index_root: PathBuf,
    pub index_name: String,
    pub dependency_table: DependencyTable,
    pub dependency_key: String,
    pub package_name: String,
    pub requested_version: Option<String>,
    pub selected_version: String,
    pub added_new_entry: bool,
    pub changed_manifest: bool,
    pub lockfile_package_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegistryProfileInstallItem {
    pub package_name: String,
    pub requested_version: String,
    pub selected_version: String,
    pub added_new_entry: bool,
    pub changed_manifest: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegistryProfileInstallReport {
    pub workspace_root: PathBuf,
    pub manifest_path: PathBuf,
    pub index_root: PathBuf,
    pub index_name: String,
    pub dependency_table: DependencyTable,
    pub profile: agam_pkg::FirstPartyDistributionProfile,
    pub items: Vec<RegistryProfileInstallItem>,
    pub lockfile_package_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegistryUpdateItem {
    pub dependency_key: String,
    pub package_name: String,
    pub previous_version: Option<String>,
    pub selected_version: String,
    pub updated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegistryUpdateReport {
    pub workspace_root: PathBuf,
    pub manifest_path: PathBuf,
    pub index_root: PathBuf,
    pub index_name: String,
    pub dependency_table: DependencyTable,
    pub items: Vec<RegistryUpdateItem>,
    pub lockfile_package_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegistryDependencyTarget {
    pub dependency_key: String,
    pub package_name: String,
}

pub(crate) struct RegistryIndexEnvRestore {
    pub key: String,
    pub previous: Option<std::ffi::OsString>,
}

impl RegistryIndexEnvRestore {
    pub(crate) fn capture(key: &str) -> Self {
        Self {
            key: key.to_string(),
            previous: std::env::var_os(key),
        }
    }
}

impl Drop for RegistryIndexEnvRestore {
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

pub(crate) fn registry_env_lock() -> &'static Mutex<()> {
    pub(crate) static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub(crate) fn with_registry_index_env<R>(
    registry: &str,
    index_root: &Path,
    f: impl FnOnce() -> R,
) -> R {
    let _guard = registry_env_lock()
        .lock()
        .expect("registry env lock should not be poisoned");
    let key = agam_pkg::registry_index_env_var(registry);
    let restore = RegistryIndexEnvRestore::capture(&key);
    unsafe {
        std::env::set_var(&key, index_root);
    }
    let result = f();
    drop(restore);
    result
}

pub(crate) fn registry_field_for_index(index_name: &str) -> Option<String> {
    (index_name != "agam").then(|| index_name.to_string())
}

pub(crate) fn dependency_registry_name(spec: &agam_pkg::DependencySpec) -> &str {
    spec.registry.as_deref().unwrap_or("agam")
}

pub(crate) fn workspace_member_names(session: &agam_pkg::WorkspaceSession) -> BTreeSet<String> {
    session
        .members
        .iter()
        .map(|member| member.layout.project_name.clone())
        .collect()
}

pub(crate) fn dependency_table(
    manifest: &agam_pkg::WorkspaceManifest,
    table: DependencyTable,
) -> &BTreeMap<String, agam_pkg::DependencySpec> {
    match table {
        DependencyTable::Main => &manifest.dependencies,
        DependencyTable::Dev => &manifest.dev_dependencies,
        DependencyTable::Build => &manifest.build_dependencies,
    }
}

pub(crate) fn dependency_table_mut(
    manifest: &mut agam_pkg::WorkspaceManifest,
    table: DependencyTable,
) -> &mut BTreeMap<String, agam_pkg::DependencySpec> {
    match table {
        DependencyTable::Main => &mut manifest.dependencies,
        DependencyTable::Dev => &mut manifest.dev_dependencies,
        DependencyTable::Build => &mut manifest.build_dependencies,
    }
}

pub(crate) fn is_registry_dependency(
    dependency_key: &str,
    spec: &agam_pkg::DependencySpec,
    workspace_members: &BTreeSet<String>,
) -> bool {
    !workspace_members.contains(dependency_key)
        && spec.path.is_none()
        && spec.git.is_none()
        && (spec.version.is_some() || spec.registry.is_some())
}

pub(crate) fn ensure_registry_dependency_slot(
    dependency_key: &str,
    package_name: &str,
    spec: &agam_pkg::DependencySpec,
    index_name: &str,
    workspace_members: &BTreeSet<String>,
    table: DependencyTable,
) -> Result<(), String> {
    if workspace_members.contains(dependency_key) {
        return Err(format!(
            "cannot modify `{dependency_key}` in `{}` because it resolves to a workspace member",
            table.manifest_label()
        ));
    }
    if spec.path.is_some() || spec.git.is_some() {
        return Err(format!(
            "cannot modify `{dependency_key}` in `{}` because it already uses a non-registry source",
            table.manifest_label()
        ));
    }

    let existing_package = spec.package.as_deref().unwrap_or(dependency_key);
    if existing_package != package_name {
        return Err(format!(
            "cannot modify `{dependency_key}` in `{}` because it already targets package `{existing_package}`",
            table.manifest_label()
        ));
    }

    let existing_registry = dependency_registry_name(spec);
    if existing_registry != index_name {
        return Err(format!(
            "cannot modify `{dependency_key}` in `{}` because it targets registry `{existing_registry}` instead of `{index_name}`",
            table.manifest_label()
        ));
    }

    Ok(())
}

pub(crate) fn collect_registry_update_targets(
    manifest: &agam_pkg::WorkspaceManifest,
    table: DependencyTable,
    names: &[String],
    index_name: &str,
    workspace_members: &BTreeSet<String>,
) -> Result<Vec<RegistryDependencyTarget>, String> {
    let dependencies = dependency_table(manifest, table);
    if names.is_empty() {
        let targets = dependencies
            .iter()
            .filter(|(dependency_key, spec)| {
                is_registry_dependency(dependency_key, spec, workspace_members)
                    && dependency_registry_name(spec) == index_name
            })
            .map(|(dependency_key, spec)| RegistryDependencyTarget {
                dependency_key: dependency_key.clone(),
                package_name: spec
                    .package
                    .clone()
                    .unwrap_or_else(|| dependency_key.clone()),
            })
            .collect::<Vec<_>>();

        if targets.is_empty() {
            return Err(format!(
                "no registry dependencies in `{}` target registry `{index_name}`",
                table.manifest_label()
            ));
        }

        return Ok(targets);
    }

    let mut targets = Vec::new();
    let mut seen = BTreeSet::new();
    for raw_name in names {
        let name = raw_name.trim();
        if name.is_empty() {
            return Err("registry update names cannot be empty".into());
        }

        if let Some(spec) = dependencies.get(name) {
            ensure_registry_dependency_slot(
                name,
                spec.package.as_deref().unwrap_or(name),
                spec,
                index_name,
                workspace_members,
                table,
            )?;
            if seen.insert(name.to_string()) {
                targets.push(RegistryDependencyTarget {
                    dependency_key: name.to_string(),
                    package_name: spec.package.clone().unwrap_or_else(|| name.to_string()),
                });
            }
            continue;
        }

        let matches = dependencies
            .iter()
            .filter(|(dependency_key, spec)| {
                spec.package.as_deref() == Some(name)
                    && is_registry_dependency(dependency_key, spec, workspace_members)
                    && dependency_registry_name(spec) == index_name
            })
            .map(|(dependency_key, _)| dependency_key.clone())
            .collect::<Vec<_>>();

        match matches.len() {
            0 => {
                return Err(format!(
                    "dependency or package `{name}` was not found in `{}` for registry `{index_name}`",
                    table.manifest_label()
                ));
            }
            1 => {
                let dependency_key = matches[0].clone();
                if seen.insert(dependency_key.clone()) {
                    targets.push(RegistryDependencyTarget {
                        dependency_key,
                        package_name: name.to_string(),
                    });
                }
            }
            _ => {
                return Err(format!(
                    "package `{name}` maps to multiple dependency keys in `{}`; update by dependency key instead",
                    table.manifest_label()
                ));
            }
        }
    }

    Ok(targets)
}

pub(crate) fn refresh_lockfile_with_registry_index(
    workspace_root: &Path,
    index_name: &str,
    index_root: &Path,
    verbose: bool,
) -> Result<usize, String> {
    with_registry_index_env(index_name, index_root, || {
        let session = resolve_workspace_session_for_driver(Some(workspace_root.to_path_buf()))?;
        Ok(try_lockfile_refresh(&session, verbose)?
            .map(|lockfile| lockfile.packages.len())
            .unwrap_or(0))
    })
}

pub(crate) fn persist_manifest_and_refresh_lockfile(
    original_manifest: &agam_pkg::WorkspaceManifest,
    updated_manifest: &agam_pkg::WorkspaceManifest,
    manifest_path: &Path,
    workspace_root: &Path,
    index_name: &str,
    index_root: &Path,
    verbose: bool,
) -> Result<usize, String> {
    agam_pkg::validate_workspace_manifest(workspace_root, updated_manifest)?;
    agam_pkg::write_workspace_manifest_to_path(manifest_path, updated_manifest)?;

    match refresh_lockfile_with_registry_index(workspace_root, index_name, index_root, verbose) {
        Ok(lockfile_package_count) => Ok(lockfile_package_count),
        Err(error) => {
            let restore =
                agam_pkg::write_workspace_manifest_to_path(manifest_path, original_manifest);
            match restore {
                Ok(()) => Err(error),
                Err(restore_error) => Err(format!(
                    "{error}; failed to restore manifest `{}` after the lockfile refresh failed: {restore_error}",
                    manifest_path.display()
                )),
            }
        }
    }
}

pub(crate) fn install_registry_dependency(
    path: Option<PathBuf>,
    index_root: &Path,
    table: DependencyTable,
    package_name: &str,
    version_req: Option<&str>,
    verbose: bool,
) -> Result<RegistryInstallReport, String> {
    let session = resolve_workspace_session_for_driver(path)?;
    let manifest_path = session.layout.manifest_path.clone().ok_or_else(|| {
        "registry install requires a workspace rooted by `agam.toml`; single-file sessions are not installable"
            .to_string()
    })?;
    let index_name = resolve_registry_index_name(index_root)?;
    let selected_release =
        agam_pkg::select_registry_release(index_root, package_name, version_req)?;
    let workspace_members = workspace_member_names(&session);
    if workspace_members.contains(package_name) {
        return Err(format!(
            "cannot install `{package_name}` into `{}` because it already resolves to a workspace member",
            table.manifest_label()
        ));
    }

    let original_manifest = session.manifest.clone().ok_or_else(|| {
        format!(
            "registry install requires a manifest at `{}`",
            manifest_path.display()
        )
    })?;
    let mut updated_manifest = original_manifest.clone();
    let dependency_key = package_name.to_string();
    let dependencies = dependency_table_mut(&mut updated_manifest, table);

    let mut next_spec = dependencies
        .get(&dependency_key)
        .cloned()
        .unwrap_or_else(agam_pkg::DependencySpec::default);
    let mut added_new_entry = true;
    if let Some(existing) = dependencies.get(&dependency_key) {
        ensure_registry_dependency_slot(
            &dependency_key,
            package_name,
            existing,
            &index_name,
            &workspace_members,
            table,
        )?;
        added_new_entry = false;
    }

    let previous_version = next_spec.version.clone();
    let previous_registry = next_spec.registry.clone();
    next_spec.version = Some(selected_release.version.clone());
    next_spec.registry = registry_field_for_index(&index_name);
    next_spec.path = None;
    next_spec.git = None;
    next_spec.rev = None;
    next_spec.branch = None;
    next_spec.package = None;
    let changed_manifest =
        previous_version != next_spec.version || previous_registry != next_spec.registry;
    dependencies.insert(dependency_key.clone(), next_spec);

    let lockfile_package_count = if changed_manifest {
        persist_manifest_and_refresh_lockfile(
            &original_manifest,
            &updated_manifest,
            &manifest_path,
            &session.layout.root,
            &index_name,
            index_root,
            verbose,
        )?
    } else {
        refresh_lockfile_with_registry_index(
            &session.layout.root,
            &index_name,
            index_root,
            verbose,
        )?
    };

    Ok(RegistryInstallReport {
        workspace_root: session.layout.root,
        manifest_path,
        index_root: index_root.to_path_buf(),
        index_name,
        dependency_table: table,
        dependency_key,
        package_name: package_name.to_string(),
        requested_version: version_req.map(str::to_string),
        selected_version: selected_release.version,
        added_new_entry,
        changed_manifest,
        lockfile_package_count,
    })
}

pub(crate) fn update_registry_dependencies(
    path: Option<PathBuf>,
    index_root: &Path,
    table: DependencyTable,
    names: &[String],
    verbose: bool,
) -> Result<RegistryUpdateReport, String> {
    let session = resolve_workspace_session_for_driver(path)?;
    let manifest_path = session.layout.manifest_path.clone().ok_or_else(|| {
        "registry update requires a workspace rooted by `agam.toml`; single-file sessions are not installable"
            .to_string()
    })?;
    let index_name = resolve_registry_index_name(index_root)?;
    let workspace_members = workspace_member_names(&session);
    let original_manifest = session.manifest.clone().ok_or_else(|| {
        format!(
            "registry update requires a manifest at `{}`",
            manifest_path.display()
        )
    })?;
    let targets = collect_registry_update_targets(
        &original_manifest,
        table,
        names,
        &index_name,
        &workspace_members,
    )?;

    let mut updated_manifest = original_manifest.clone();
    let dependencies = dependency_table_mut(&mut updated_manifest, table);
    let mut items = Vec::new();
    let mut any_manifest_change = false;

    for target in targets {
        let selected_release =
            agam_pkg::select_registry_release(index_root, &target.package_name, None)?;
        let spec = dependencies
            .get_mut(&target.dependency_key)
            .ok_or_else(|| {
                format!(
                    "dependency `{}` disappeared from `{}` while preparing the update",
                    target.dependency_key,
                    table.manifest_label()
                )
            })?;

        let previous_version = spec.version.clone();
        let previous_registry = spec.registry.clone();
        spec.version = Some(selected_release.version.clone());
        spec.registry = registry_field_for_index(&index_name);
        spec.path = None;
        spec.git = None;
        spec.rev = None;
        spec.branch = None;
        if target.dependency_key == target.package_name {
            spec.package = None;
        } else {
            spec.package = Some(target.package_name.clone());
        }

        let updated = previous_version != spec.version || previous_registry != spec.registry;
        any_manifest_change |= updated;
        items.push(RegistryUpdateItem {
            dependency_key: target.dependency_key,
            package_name: target.package_name,
            previous_version,
            selected_version: selected_release.version,
            updated,
        });
    }

    let lockfile_package_count = if any_manifest_change {
        persist_manifest_and_refresh_lockfile(
            &original_manifest,
            &updated_manifest,
            &manifest_path,
            &session.layout.root,
            &index_name,
            index_root,
            verbose,
        )?
    } else {
        refresh_lockfile_with_registry_index(
            &session.layout.root,
            &index_name,
            index_root,
            verbose,
        )?
    };

    Ok(RegistryUpdateReport {
        workspace_root: session.layout.root,
        manifest_path,
        index_root: index_root.to_path_buf(),
        index_name,
        dependency_table: table,
        items,
        lockfile_package_count,
    })
}

pub(crate) fn yank_registry_release(
    index_root: &Path,
    package_name: &str,
    version: &str,
    undo: bool,
) -> Result<RegistryYankReport, String> {
    let index_name = resolve_registry_index_name(index_root)?;
    let release = agam_pkg::set_registry_release_yanked(index_root, package_name, version, !undo)?;
    Ok(RegistryYankReport {
        index_root: index_root.to_path_buf(),
        index_name,
        package_name: package_name.to_string(),
        version: release.version,
        yanked: release.yanked,
    })
}

pub(crate) fn list_registry_profiles() -> RegistryProfileListReport {
    RegistryProfileListReport {
        profiles: agam_pkg::first_party_distribution_profiles(),
    }
}

pub(crate) fn inspect_registry_profile(name: &str) -> Result<RegistryProfileInspectReport, String> {
    let profile = agam_pkg::first_party_distribution_profile(name)
        .ok_or_else(|| format!("unknown curated first-party profile `{name}`"))?;
    Ok(RegistryProfileInspectReport { profile })
}

pub(crate) fn registry_governance_report() -> RegistryGovernanceReport {
    RegistryGovernanceReport {
        governance: agam_pkg::official_package_governance(),
    }
}

pub(crate) fn install_registry_profile(
    path: Option<PathBuf>,
    index_root: &Path,
    table: DependencyTable,
    profile_name: &str,
    verbose: bool,
) -> Result<RegistryProfileInstallReport, String> {
    let session = resolve_workspace_session_for_driver(path)?;
    let manifest_path = session.layout.manifest_path.clone().ok_or_else(|| {
        "registry profile install requires a workspace rooted by `agam.toml`; single-file sessions are not installable"
            .to_string()
    })?;
    let profile = agam_pkg::first_party_distribution_profile(profile_name)
        .ok_or_else(|| format!("unknown curated first-party profile `{profile_name}`"))?;
    let index_name = resolve_registry_index_name(index_root)?;
    let workspace_members = workspace_member_names(&session);
    let original_manifest = session.manifest.clone().ok_or_else(|| {
        format!(
            "registry profile install requires a manifest at `{}`",
            manifest_path.display()
        )
    })?;

    let mut updated_manifest = original_manifest.clone();
    let dependencies = dependency_table_mut(&mut updated_manifest, table);
    let mut items = Vec::new();
    let mut any_manifest_change = false;

    for recommendation in &profile.packages {
        if workspace_members.contains(&recommendation.name) {
            return Err(format!(
                "cannot install profile `{}` because `{}` already resolves to a workspace member",
                profile.name, recommendation.name
            ));
        }

        let selected_release = agam_pkg::select_registry_release(
            index_root,
            &recommendation.name,
            Some(&recommendation.version_req),
        )?;
        let dependency_key = recommendation.name.clone();
        let mut next_spec = dependencies
            .get(&dependency_key)
            .cloned()
            .unwrap_or_else(agam_pkg::DependencySpec::default);
        let added_new_entry = !dependencies.contains_key(&dependency_key);
        if let Some(existing) = dependencies.get(&dependency_key) {
            ensure_registry_dependency_slot(
                &dependency_key,
                &recommendation.name,
                existing,
                &index_name,
                &workspace_members,
                table,
            )?;
        }

        let previous_version = next_spec.version.clone();
        let previous_registry = next_spec.registry.clone();
        next_spec.version = Some(selected_release.version.clone());
        next_spec.registry = registry_field_for_index(&index_name);
        next_spec.path = None;
        next_spec.git = None;
        next_spec.rev = None;
        next_spec.branch = None;
        next_spec.package = None;
        let changed_manifest =
            previous_version != next_spec.version || previous_registry != next_spec.registry;
        any_manifest_change |= changed_manifest;
        dependencies.insert(dependency_key, next_spec);

        items.push(RegistryProfileInstallItem {
            package_name: recommendation.name.clone(),
            requested_version: recommendation.version_req.clone(),
            selected_version: selected_release.version,
            added_new_entry,
            changed_manifest,
        });
    }

    let lockfile_package_count = if any_manifest_change {
        persist_manifest_and_refresh_lockfile(
            &original_manifest,
            &updated_manifest,
            &manifest_path,
            &session.layout.root,
            &index_name,
            index_root,
            verbose,
        )?
    } else {
        refresh_lockfile_with_registry_index(
            &session.layout.root,
            &index_name,
            index_root,
            verbose,
        )?
    };

    Ok(RegistryProfileInstallReport {
        workspace_root: session.layout.root,
        manifest_path,
        index_root: index_root.to_path_buf(),
        index_name,
        dependency_table: table,
        profile,
        items,
        lockfile_package_count,
    })
}

pub(crate) fn resolve_environment_session_and_lockfile(
    path: Option<PathBuf>,
) -> Result<
    (
        agam_pkg::WorkspaceSession,
        PathBuf,
        agam_pkg::WorkspaceManifest,
        agam_pkg::WorkspaceLockfile,
    ),
    String,
> {
    let session = resolve_workspace_session_for_driver(path)?;
    let manifest_path = session.layout.manifest_path.clone().ok_or_else(|| {
        "environment commands require a workspace rooted by `agam.toml`; single-file sessions do not define environments"
            .to_string()
    })?;
    let manifest = session.manifest.clone().ok_or_else(|| {
        format!(
            "environment commands require a manifest at `{}`",
            manifest_path.display()
        )
    })?;
    let lockfile = agam_pkg::resolve_dependencies(&session)?;
    Ok((session, manifest_path, manifest, lockfile))
}

pub(crate) fn list_workspace_environments(
    path: Option<PathBuf>,
) -> Result<EnvironmentListReport, String> {
    let (session, manifest_path, manifest, lockfile) =
        resolve_environment_session_and_lockfile(path)?;
    let default_environment = agam_pkg::default_environment_name(&manifest);
    let environments = agam_pkg::resolve_environment_catalog(&manifest, &lockfile)
        .into_values()
        .collect();

    Ok(EnvironmentListReport {
        workspace_root: session.layout.root,
        manifest_path,
        default_environment,
        environments,
    })
}

pub(crate) fn inspect_workspace_environment(
    path: Option<PathBuf>,
    name: Option<&str>,
) -> Result<EnvironmentInspectReport, String> {
    let (session, manifest_path, manifest, lockfile) =
        resolve_environment_session_and_lockfile(path)?;
    let selected_by_default = name.is_none();
    let environment =
        agam_pkg::resolve_environment(&manifest, &lockfile, name)?.ok_or_else(|| {
            if manifest.environments.is_empty() {
                "workspace defines no named environments".to_string()
            } else {
                "no environment selected".to_string()
            }
        })?;

    Ok(EnvironmentInspectReport {
        workspace_root: session.layout.root,
        manifest_path,
        selected_by_default,
        environment,
    })
}

pub(crate) fn publish_workspace_to_registry(
    path: Option<PathBuf>,
    index_root: &Path,
    owners: &[String],
    description: Option<&String>,
    homepage: Option<&String>,
    repository: Option<&String>,
    download_url: Option<&String>,
    official: bool,
    dry_run: bool,
    _verbose: bool,
) -> Result<PublishReport, String> {
    let session = resolve_workspace_session_for_driver(path)?;
    let manifest_path = session.layout.manifest_path.clone().ok_or_else(|| {
        "publish requires a workspace rooted by `agam.toml`; single-file sessions are not publishable"
            .to_string()
    })?;

    let mut manifest = agam_pkg::build_publish_manifest(&session)?;
    if let Some(description) = normalize_publish_text(description.map(String::as_str)) {
        manifest.description = Some(description);
    }
    if let Some(homepage) = normalize_publish_text(homepage.map(String::as_str)) {
        manifest.homepage = Some(homepage);
    }
    if let Some(repository) = normalize_publish_text(repository.map(String::as_str)) {
        manifest.repository = Some(repository);
    }
    if let Some(download_url) = normalize_publish_text(download_url.map(String::as_str)) {
        manifest.download_url = Some(download_url);
    }

    let owners = normalize_publish_owners(owners);
    let (index_name, bootstrapped_config) = ensure_registry_index_ready(index_root, dry_run)?;
    let index_path = agam_pkg::registry_index_path(&manifest.name);

    if official {
        agam_pkg::validate_official_publish_manifest(&manifest, &index_name, &owners)?;
    } else {
        agam_pkg::validate_publish_manifest(&manifest)?;
    }

    let receipt = if dry_run {
        None
    } else if official {
        Some(agam_pkg::publish_official_package_to_registry_index(
            index_root,
            &manifest,
            &owners,
            &publish_timestamp(),
            &index_name,
        )?)
    } else {
        Some(agam_pkg::publish_to_registry_index(
            index_root,
            &manifest,
            &owners,
            &publish_timestamp(),
        )?)
    };

    Ok(PublishReport {
        dry_run,
        official,
        workspace_root: session.layout.root,
        manifest_path,
        index_root: index_root.to_path_buf(),
        index_name,
        index_path,
        owners,
        manifest,
        receipt,
        bootstrapped_config,
    })
}

pub(crate) fn normalize_publish_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub(crate) fn normalize_publish_owners(owners: &[String]) -> Vec<String> {
    owners
        .iter()
        .map(|owner| owner.trim())
        .filter(|owner| !owner.is_empty())
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(crate) fn inferred_registry_index_name(index_root: &Path) -> String {
    index_root
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or("agam")
        .to_string()
}

pub(crate) fn resolve_registry_index_name(index_root: &Path) -> Result<String, String> {
    if !index_root.exists() {
        return Err(format!(
            "registry index root `{}` does not exist",
            index_root.display()
        ));
    }
    if !index_root.is_dir() {
        return Err(format!(
            "registry index root `{}` is not a directory",
            index_root.display()
        ));
    }

    let config_path = index_root.join("config.json");
    if config_path.exists() && !config_path.is_file() {
        return Err(format!(
            "registry config path `{}` is not a file",
            config_path.display()
        ));
    }

    if config_path.is_file() {
        let config = agam_pkg::read_registry_config(index_root)?;
        if config.format_version != agam_pkg::REGISTRY_INDEX_FORMAT_VERSION {
            return Err(format!(
                "registry index `{}` uses unsupported format version {}; expected {}",
                index_root.display(),
                config.format_version,
                agam_pkg::REGISTRY_INDEX_FORMAT_VERSION
            ));
        }
        Ok(config
            .name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| inferred_registry_index_name(index_root)))
    } else {
        Ok(inferred_registry_index_name(index_root))
    }
}

pub(crate) fn ensure_registry_index_ready(
    index_root: &Path,
    dry_run: bool,
) -> Result<(String, bool), String> {
    if index_root.exists() && !index_root.is_dir() {
        return Err(format!(
            "registry index root `{}` is not a directory",
            index_root.display()
        ));
    }

    let config_path = index_root.join("config.json");
    if config_path.exists() && !config_path.is_file() {
        return Err(format!(
            "registry config path `{}` is not a file",
            config_path.display()
        ));
    }

    if config_path.is_file() {
        let config = agam_pkg::read_registry_config(index_root)?;
        if config.format_version != agam_pkg::REGISTRY_INDEX_FORMAT_VERSION {
            return Err(format!(
                "registry index `{}` uses unsupported format version {}; expected {}",
                index_root.display(),
                config.format_version,
                agam_pkg::REGISTRY_INDEX_FORMAT_VERSION
            ));
        }
        let index_name = config
            .name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| inferred_registry_index_name(index_root));
        return Ok((index_name, false));
    }

    let index_name = inferred_registry_index_name(index_root);
    if dry_run {
        return Ok((index_name, false));
    }

    agam_pkg::write_registry_config(
        index_root,
        &agam_pkg::RegistryConfig {
            format_version: agam_pkg::REGISTRY_INDEX_FORMAT_VERSION,
            api_url: None,
            download_url: None,
            name: Some(index_name.clone()),
        },
    )?;

    Ok((index_name, true))
}

pub(crate) fn publish_timestamp() -> String {
    now_unix_ms().to_string()
}

pub(crate) fn inspect_registry_package(
    index_root: &Path,
    name: &str,
) -> Result<RegistryInspectReport, String> {
    let index_name = resolve_registry_index_name(index_root)?;
    let entry = agam_pkg::read_registry_package_entry(index_root, name)?;
    Ok(RegistryInspectReport {
        index_root: index_root.to_path_buf(),
        index_name,
        index_path: agam_pkg::registry_index_path(name),
        entry,
    })
}

pub(crate) fn audit_registry_index_package(
    index_root: &Path,
    name: &str,
) -> Result<RegistryAuditReport, String> {
    let index_name = resolve_registry_index_name(index_root)?;
    let lines = agam_pkg::audit_registry_package(index_root, name)?;
    Ok(RegistryAuditReport {
        index_root: index_root.to_path_buf(),
        index_name,
        index_path: agam_pkg::registry_index_path(name),
        lines,
    })
}

pub(crate) fn print_publish_report(report: &PublishReport, verbose: bool) {
    println!("publish: {}", if report.dry_run { "dry-run" } else { "ok" });
    println!(
        "package: {}@{}",
        report.manifest.name, report.manifest.version
    );
    println!("workspace: {}", report.workspace_root.display());
    println!("manifest: {}", report.manifest_path.display());
    println!(
        "registry: {} ({})",
        report.index_name,
        report.index_root.display()
    );
    println!("index path: {}", report.index_path);
    println!("checksum: {}", report.manifest.checksum);
    println!("manifest checksum: {}", report.manifest.manifest_checksum);
    println!("agam version: {}", report.manifest.agam_version);
    println!("official: {}", if report.official { "yes" } else { "no" });
    println!(
        "owners: {}",
        if report.owners.is_empty() {
            "none".to_string()
        } else {
            report.owners.join(", ")
        }
    );
    println!("dependencies: {}", report.manifest.dependencies.len());

    if let Some(description) = report.manifest.description.as_deref() {
        println!("description: {description}");
    } else if verbose {
        println!("description: none");
    }
    if let Some(homepage) = report.manifest.homepage.as_deref() {
        println!("homepage: {homepage}");
    } else if verbose {
        println!("homepage: none");
    }
    if let Some(repository) = report.manifest.repository.as_deref() {
        println!("repository: {repository}");
    } else if verbose {
        println!("repository: none");
    }
    if let Some(download_url) = report.manifest.download_url.as_deref() {
        println!("download: {download_url}");
    } else if verbose {
        println!("download: registry default or none");
    }
    if !report.manifest.keywords.is_empty() {
        println!("keywords: {}", report.manifest.keywords.join(", "));
    } else if verbose {
        println!("keywords: none");
    }

    if verbose && !report.manifest.dependencies.is_empty() {
        println!("dependency detail:");
        for dependency in &report.manifest.dependencies {
            let mut line = format!("  {} {}", dependency.name, dependency.version_req);
            if let Some(registry) = dependency.registry.as_deref() {
                line.push_str(&format!(" [registry: {registry}]"));
            }
            if dependency.optional {
                line.push_str(" [optional]");
            }
            if !dependency.features.is_empty() {
                line.push_str(&format!(" [features: {}]", dependency.features.join(", ")));
            }
            println!("{line}");
        }
    }

    if report.bootstrapped_config {
        println!("registry config: initialized config.json");
    } else if verbose {
        println!("registry config: existing or skipped");
    }

    if let Some(receipt) = report.receipt.as_ref() {
        println!("published at: {}", receipt.published_at);
    } else if verbose {
        println!("published at: pending (dry-run)");
    }
}

pub(crate) fn print_registry_inspect_report(report: &RegistryInspectReport, verbose: bool) {
    println!("package: {}", report.entry.name);
    println!(
        "registry: {} ({})",
        report.index_name,
        report.index_root.display()
    );
    println!("index path: {}", report.index_path);
    println!("created: {}", report.entry.created_at);
    println!(
        "owners: {}",
        if report.entry.owners.is_empty() {
            "none".to_string()
        } else {
            report.entry.owners.join(", ")
        }
    );
    println!("releases: {}", report.entry.releases.len());

    if let Some(description) = report.entry.description.as_deref() {
        println!("description: {description}");
    } else if verbose {
        println!("description: none");
    }
    if let Some(homepage) = report.entry.homepage.as_deref() {
        println!("homepage: {homepage}");
    } else if verbose {
        println!("homepage: none");
    }
    if let Some(repository) = report.entry.repository.as_deref() {
        println!("repository: {repository}");
    } else if verbose {
        println!("repository: none");
    }
    if !report.entry.keywords.is_empty() {
        println!("keywords: {}", report.entry.keywords.join(", "));
    } else if verbose {
        println!("keywords: none");
    }

    if verbose && !report.entry.releases.is_empty() {
        println!("release detail:");
        for release in &report.entry.releases {
            let yanked = if release.yanked { " [yanked]" } else { "" };
            println!(
                "  {} (checksum: {}, agam: {}, published: {}{}, deps: {}, features: {})",
                release.version,
                release.checksum,
                release.agam_version,
                release.published_at,
                yanked,
                release.dependencies.len(),
                release.features.len()
            );
            if let Some(download_url) = release.download_url.as_deref() {
                println!("    download: {download_url}");
            }
            if let Some(provenance) = release.provenance.as_ref() {
                println!(
                    "    provenance: source={}, manifest={}",
                    provenance.source_checksum, provenance.manifest_checksum
                );
                if let Some(published_by) = provenance.published_by.as_deref() {
                    println!("    published by: {published_by}");
                }
                if let Some(source_repository) = provenance.source_repository.as_deref() {
                    println!("    source repository: {source_repository}");
                }
            }
        }
    }
}

pub(crate) fn print_registry_audit_report(report: &RegistryAuditReport) {
    println!(
        "registry: {} ({})",
        report.index_name,
        report.index_root.display()
    );
    println!("index path: {}", report.index_path);
    for line in &report.lines {
        println!("{line}");
    }
}

pub(crate) fn print_registry_profile_list_report(report: &RegistryProfileListReport) {
    println!("profiles: {}", report.profiles.len());
    for profile in &report.profiles {
        println!(
            "{} | packages={} | {}",
            profile.name,
            profile.packages.len(),
            profile.summary
        );
    }
}

pub(crate) fn print_registry_profile_inspect_report(report: &RegistryProfileInspectReport) {
    println!("profile: {}", report.profile.name);
    println!("summary: {}", report.profile.summary);
    println!("description: {}", report.profile.description);
    println!("packages: {}", report.profile.packages.len());
    for package in &report.profile.packages {
        println!(
            "  {} {} | {}",
            package.name, package.version_req, package.rationale
        );
    }
    if !report.profile.notes.is_empty() {
        println!("notes:");
        for note in &report.profile.notes {
            println!("  {note}");
        }
    }
}

pub(crate) fn print_registry_governance_report(report: &RegistryGovernanceReport) {
    println!("registry: {}", report.governance.registry);
    println!("reserved prefix: {}", report.governance.reserved_prefix);
    println!(
        "repository namespace: {}",
        report.governance.repository_namespace
    );
    println!(
        "owners: {}",
        if report.governance.owner_handles.is_empty() {
            "none".to_string()
        } else {
            report.governance.owner_handles.join(", ")
        }
    );
    println!("rules: {}", report.governance.publication_rules.len());
    for rule in &report.governance.publication_rules {
        println!("  {rule}");
    }
}

pub(crate) fn print_registry_install_report(report: &RegistryInstallReport, verbose: bool) {
    println!(
        "install: {}",
        if report.changed_manifest {
            "ok"
        } else {
            "up-to-date"
        }
    );
    println!(
        "package: {}@{}",
        report.package_name, report.selected_version
    );
    println!("workspace: {}", report.workspace_root.display());
    println!("manifest: {}", report.manifest_path.display());
    println!(
        "registry: {} ({})",
        report.index_name,
        report.index_root.display()
    );
    println!("table: {}", report.dependency_table.manifest_label());
    println!("dependency: {}", report.dependency_key);
    println!(
        "manifest change: {}",
        if report.added_new_entry {
            "added dependency"
        } else if report.changed_manifest {
            "updated existing dependency"
        } else {
            "unchanged"
        }
    );
    println!("lockfile packages: {}", report.lockfile_package_count);

    if let Some(requested_version) = report.requested_version.as_deref() {
        println!("requested: {requested_version}");
    } else if verbose {
        println!("requested: latest");
    }
}

pub(crate) fn print_registry_profile_install_report(
    report: &RegistryProfileInstallReport,
    verbose: bool,
) {
    let changed = report
        .items
        .iter()
        .filter(|item| item.changed_manifest)
        .count();
    let unchanged = report.items.len().saturating_sub(changed);

    println!(
        "profile install: {}",
        if changed > 0 { "ok" } else { "up-to-date" }
    );
    println!("profile: {}", report.profile.name);
    println!("workspace: {}", report.workspace_root.display());
    println!("manifest: {}", report.manifest_path.display());
    println!(
        "registry: {} ({})",
        report.index_name,
        report.index_root.display()
    );
    println!("table: {}", report.dependency_table.manifest_label());
    println!("packages: {}", report.items.len());
    println!("updated: {changed}");
    println!("unchanged: {unchanged}");
    println!("lockfile packages: {}", report.lockfile_package_count);

    for item in &report.items {
        if item.changed_manifest || verbose {
            let status = if item.added_new_entry {
                "added"
            } else if item.changed_manifest {
                "updated"
            } else {
                "unchanged"
            };
            println!(
                "{}: {} -> {} ({status})",
                item.package_name, item.requested_version, item.selected_version
            );
        }
    }
}

pub(crate) fn print_registry_update_report(report: &RegistryUpdateReport, verbose: bool) {
    let updated = report.items.iter().filter(|item| item.updated).count();
    let unchanged = report.items.len().saturating_sub(updated);

    println!("update: ok");
    println!("workspace: {}", report.workspace_root.display());
    println!("manifest: {}", report.manifest_path.display());
    println!(
        "registry: {} ({})",
        report.index_name,
        report.index_root.display()
    );
    println!("table: {}", report.dependency_table.manifest_label());
    println!("updated: {updated}");
    println!("unchanged: {unchanged}");
    println!("lockfile packages: {}", report.lockfile_package_count);

    for item in &report.items {
        if item.updated || verbose {
            let label = if item.dependency_key == item.package_name {
                item.dependency_key.clone()
            } else {
                format!("{} ({})", item.dependency_key, item.package_name)
            };
            let previous = item.previous_version.as_deref().unwrap_or("*");
            println!("{}: {} -> {}", label, previous, item.selected_version);
        }
    }
}

pub(crate) fn print_registry_yank_report(report: &RegistryYankReport) {
    println!(
        "yank: {}",
        if report.yanked { "yanked" } else { "available" }
    );
    println!("package: {}@{}", report.package_name, report.version);
    println!(
        "registry: {} ({})",
        report.index_name,
        report.index_root.display()
    );
}

pub(crate) fn print_environment_list_report(report: &EnvironmentListReport) {
    println!("workspace: {}", report.workspace_root.display());
    println!("manifest: {}", report.manifest_path.display());
    println!("environments: {}", report.environments.len());
    println!(
        "default: {}",
        report.default_environment.as_deref().unwrap_or("none")
    );

    for environment in &report.environments {
        let default_marker =
            if report.default_environment.as_deref() == Some(environment.name.as_str()) {
                " [default]"
            } else {
                ""
            };
        println!(
            "{}{} | compiler={} | sdk={} | target={} | backend={} | profiles={} | packages={}",
            environment.name,
            default_marker,
            environment.compiler,
            environment.sdk.as_deref().unwrap_or("none"),
            environment.target.as_deref().unwrap_or("none"),
            environment
                .preferred_backend
                .map(|backend| format!("{backend:?}").to_lowercase())
                .unwrap_or_else(|| "none".to_string()),
            if environment.profiles.is_empty() {
                "none".to_string()
            } else {
                environment.profiles.join(", ")
            },
            environment.packages.len()
        );
    }
}

pub(crate) fn print_environment_inspect_report(report: &EnvironmentInspectReport) {
    println!("workspace: {}", report.workspace_root.display());
    println!("manifest: {}", report.manifest_path.display());
    println!("environment: {}", report.environment.name);
    println!(
        "selected by: {}",
        if report.selected_by_default {
            "implicit default rules"
        } else {
            "explicit request"
        }
    );
    println!("compiler: {}", report.environment.compiler);
    println!(
        "sdk: {}",
        report.environment.sdk.as_deref().unwrap_or("none")
    );
    println!(
        "target: {}",
        report.environment.target.as_deref().unwrap_or("none")
    );
    println!(
        "runtime abi: {}",
        report
            .environment
            .runtime_abi
            .map(|abi| abi.to_string())
            .unwrap_or_else(|| "none".to_string())
    );
    println!(
        "backend: {}",
        report
            .environment
            .preferred_backend
            .map(|backend| format!("{backend:?}").to_lowercase())
            .unwrap_or_else(|| "none".to_string())
    );
    println!(
        "profiles: {}",
        if report.environment.profiles.is_empty() {
            "none".to_string()
        } else {
            report.environment.profiles.join(", ")
        }
    );
    println!("packages: {}", report.environment.packages.len());
    for package in &report.environment.packages {
        println!("  {package}");
    }
}

pub(crate) fn build_portable_package_file(
    path: &PathBuf,
    verbose: bool,
) -> Result<agam_pkg::PortablePackage, String> {
    if let Some(prewarmed) = load_daemon_prewarmed_entry(path, verbose) {
        return Ok(prewarmed.package);
    }

    let parsed = parse_source_file(path, verbose)?;
    semantic_check_parsed_source(path, &parsed, verbose)?;
    let mir = lower_parsed_to_optimized_mir(&parsed, verbose);
    Ok(agam_pkg::build_portable_package(
        path,
        &parsed.source,
        &parsed.module,
        &mir,
        agam_runtime::contract::RuntimeBackend::Jit,
    ))
}

pub(crate) fn write_portable_package_with_cache(
    source_path: &PathBuf,
    output: &PathBuf,
    package: &agam_pkg::PortablePackage,
    verbose: bool,
) -> Result<agam_runtime::cache::CacheHit, String> {
    let cache = agam_runtime::cache::CacheStore::for_path(source_path)?;
    let source = std::fs::read(source_path).map_err(|e| {
        format!(
            "failed to read `{}` for cache key generation: {}",
            source_path.display(),
            e
        )
    })?;
    let package_hash = agam_runtime::cache::hash_bytes(&source);
    let semantic_hash = agam_runtime::cache::hash_serializable(&package.manifest)?;
    let key = agam_runtime::cache::default_cache_key(
        package_hash,
        semantic_hash,
        agam_runtime::contract::RuntimeBackend::Jit,
        0,
        "package".to_string(),
    );

    if let Some(hit) = cache.lookup(&key)? {
        if verbose {
            eprintln!("[agamc] Package cache hit: {}", hit.id);
        }
        cache.restore_to_path(&hit, output)?;
        return Ok(hit);
    }

    let bytes = serde_json::to_vec_pretty(package)
        .map_err(|e| format!("failed to serialize package for cache: {}", e))?;
    let hit = cache.store_bytes(
        &key,
        agam_runtime::cache::CacheArtifactKind::PortablePackage,
        source_path,
        output
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("package.agpkg.json"),
        &bytes,
    )?;
    if verbose {
        eprintln!("[agamc] Package cache miss; stored {}", hit.id);
    }
    cache.restore_to_path(&hit, output)?;
    Ok(hit)
}

pub(crate) fn run_portable_package_file(
    path: &PathBuf,
    args: &[String],
    verbose: bool,
) -> Result<i32, String> {
    let package = agam_pkg::read_package_from_path(path)?;
    let host = agam_runtime::contract::host_runtime();
    let plan = agam_runtime::contract::plan_package_load(
        &package.runtime,
        agam_runtime::contract::RuntimeBackend::Auto,
        &host,
    )?;

    if verbose {
        eprintln!(
            "[agamc] Loaded portable package `{}` via {:?} on {} / {} / {}-bit",
            package.manifest.name,
            plan.backend,
            plan.host.os,
            plan.host.arch,
            plan.host.pointer_width
        );
    }

    match plan.backend {
        agam_runtime::contract::RuntimeBackend::Jit => {
            let mut runtime_args = Vec::with_capacity(args.len() + 1);
            runtime_args.push(path.to_string_lossy().to_string());
            runtime_args.extend(args.iter().cloned());
            agam_jit::run_main(&package.mir, &runtime_args)
        }
        backend => Err(format!(
            "portable package execution is currently implemented only through the JIT runtime; package requested {:?}",
            backend
        )),
    }
}

pub(crate) fn print_package_summary(package: &agam_pkg::PortablePackage) {
    println!("package: {}", package.manifest.name);
    println!("source: {}", package.manifest.source_path);
    println!("entry: {}", package.manifest.entry_function);
    println!(
        "runtime ABI: v{} ({:?})",
        package.runtime.abi.version, package.runtime.requirements.preferred_backend
    );
    println!(
        "build host: {} / {} / {}-bit",
        package.runtime.build_host.os,
        package.runtime.build_host.arch,
        package.runtime.build_host.pointer_width
    );
    println!(
        "verified functions: {}",
        package.manifest.verified_ir.function_count
    );
    println!("source map entries: {}", package.manifest.source_map.len());
    println!(
        "declared effects: {}",
        package.manifest.effects.declared_effects.len()
    );
}

pub(crate) fn print_doctor_status(label: &str, status: &str, detail: &str) {
    println!("{label}: {status}");
    println!("  {detail}");
}

pub(crate) fn run_doctor(
    environment: Option<&EnvironmentInspectReport>,
    verbose: bool,
) -> Result<bool, String> {
    let host = current_host_sdk_platform();
    let bundled_root = detect_packaged_llvm_bundle_root();
    let bundled_driver = discover_bundled_llvm_clang();
    let override_driver = configured_llvm_clang_override();
    let native_driver = resolve_native_llvm_command();
    let vs_install = discover_visual_studio_installation_path();
    let vs_driver = discover_visual_studio_llvm_clang();
    let wsl_clang = wsl_command_exists("clang");
    let c_driver = command_exists(default_c_compiler());
    let android_sysroot = resolve_android_sysroot_for_target(None);

    let current_exe = std::env::current_exe()
        .map_err(|e| format!("failed to locate current compiler executable: {}", e))?;

    println!("Agam Doctor");
    println!("host: {host}");
    println!("core compiler: {}", current_exe.display());
    if let Some(environment) = environment {
        println!("environment: {}", environment_selection_label(environment));
        println!(
            "environment manifest: {}",
            environment.manifest_path.display()
        );
    }

    match native_driver.as_ref() {
        Some(driver) => {
            print_doctor_status("native llvm", "ok", &format!("using `{driver}`"));
        }
        None => {
            let hint = if cfg!(windows) {
                windows_native_llvm_install_hint().unwrap_or_else(|| {
                    format!(
                        "install a native LLVM/Clang toolchain, bundle one next to agamc, or set `{LLVM_CLANG_ENV}`"
                    )
                })
            } else {
                format!(
                    "install a native LLVM/Clang toolchain, bundle one next to agamc, or set `{LLVM_CLANG_ENV}`"
                )
            };
            print_doctor_status("native llvm", "missing", &hint);
        }
    }

    match bundled_root.as_ref() {
        Some(root) => print_doctor_status(
            "bundled llvm",
            "ok",
            &format!("bundle root `{}`", root.display()),
        ),
        None => print_doctor_status(
            "bundled llvm",
            "missing",
            &format!(
                "no bundled LLVM found; expected `toolchains/llvm/{}/bin` near `agamc` or set `{}`",
                bundled_llvm_platform_dir(),
                LLVM_BUNDLE_DIR_ENV
            ),
        ),
    }

    if let Some(driver) = bundled_driver.as_ref() {
        print_doctor_status("bundled driver", "ok", &format!("driver `{driver}`"));
    } else if verbose {
        print_doctor_status(
            "bundled driver",
            "missing",
            "no bundled clang/clang++ executable resolved from the bundle search paths",
        );
    }

    if cfg!(windows) {
        match vs_install.as_ref() {
            Some(path) => print_doctor_status(
                "visual studio",
                "ok",
                &format!("installation `{}`", path.display()),
            ),
            None => print_doctor_status(
                "visual studio",
                "missing",
                "Visual Studio installation not detected via vswhere",
            ),
        }
        match vs_driver.as_ref() {
            Some(path) => {
                print_doctor_status("visual studio llvm", "ok", &format!("driver `{path}`"))
            }
            None => print_doctor_status(
                "visual studio llvm",
                "missing",
                "LLVM/Clang component is not currently installed in Visual Studio",
            ),
        }
        if wsl_clang {
            print_doctor_status(
                "wsl llvm",
                "available",
                &format!(
                    "development-only fallback; enable with `{DEV_WSL_LLVM_ENV}=1` for `agamc run --backend llvm`"
                ),
            );
        } else if verbose {
            print_doctor_status("wsl llvm", "missing", "WSL clang was not detected");
        }
    }

    if let Some(driver) = override_driver.as_ref() {
        print_doctor_status(
            "llvm override",
            "configured",
            &format!("`{LLVM_CLANG_ENV}` -> `{driver}`"),
        );
    } else if verbose {
        print_doctor_status(
            "llvm override",
            "unset",
            &format!("set `{LLVM_CLANG_ENV}` to pin `clang` or `clang++`"),
        );
    }

    if c_driver {
        print_doctor_status(
            "c fallback",
            "ok",
            &format!("`{}` detected", default_c_compiler()),
        );
    } else {
        print_doctor_status(
            "c fallback",
            "missing",
            &format!("`{}` was not detected on PATH", default_c_compiler()),
        );
    }

    match android_sysroot.as_ref() {
        Some(path) => print_doctor_status(
            "android sysroot",
            "ok",
            &format!("resolved `{}`", path.display()),
        ),
        None => print_doctor_status(
            "android sysroot",
            "missing",
            &format!(
                "set `{LLVM_SYSROOT_ENV}` or `ANDROID_NDK_HOME`/`ANDROID_NDK_ROOT` for Android LLVM builds"
            ),
        ),
    }

    let mut healthy = native_driver.is_some();
    if let Some(environment) = environment {
        let resolved = &environment.environment;
        print_doctor_status(
            "env compiler",
            "selected",
            &format!("compiler requirement `{}`", resolved.compiler),
        );
        if let Some(sdk) = resolved.sdk.as_deref() {
            print_doctor_status("env sdk", "selected", &format!("sdk `{sdk}`"));
        } else if verbose {
            print_doctor_status("env sdk", "inherit", "no environment-specific SDK override");
        }
        if let Some(target) = resolved.target.as_deref() {
            let sdk_root = env_path(LLVM_SDKROOT_ENV).or_else(|| env_path("SDKROOT"));
            let (status, detail, target_ok) = match classify_llvm_target_platform(Some(target)) {
                LlvmTargetPlatform::Android => match android_sysroot.as_ref() {
                    Some(path) => (
                        "ok",
                        format!("target `{target}` via sysroot `{}`", path.display()),
                        true,
                    ),
                    None => (
                        "missing",
                        format!(
                            "target `{target}` needs `{LLVM_SYSROOT_ENV}` or `ANDROID_NDK_HOME`/`ANDROID_NDK_ROOT`"
                        ),
                        false,
                    ),
                },
                LlvmTargetPlatform::Ios | LlvmTargetPlatform::MacOs => match sdk_root.as_ref() {
                    Some(path) => (
                        "ok",
                        format!("target `{target}` via SDK root `{}`", path.display()),
                        true,
                    ),
                    None => (
                        "missing",
                        format!("target `{target}` needs `{LLVM_SDKROOT_ENV}` or `SDKROOT`"),
                        false,
                    ),
                },
                _ => ("ok", format!("target `{target}`"), true),
            };
            print_doctor_status("env target", status, &detail);
            healthy &= target_ok;
        } else if verbose {
            print_doctor_status("env target", "inherit", "host-native target");
        }
        if let Some(backend) = resolved.preferred_backend {
            let (status, detail, backend_ok) = match backend {
                agam_runtime::contract::RuntimeBackend::Llvm => (
                    if native_driver.is_some() {
                        "ok"
                    } else {
                        "missing"
                    },
                    if native_driver.is_some() {
                        "environment can use the native LLVM backend".to_string()
                    } else {
                        "environment requests LLVM but no native LLVM toolchain was detected"
                            .to_string()
                    },
                    native_driver.is_some(),
                ),
                agam_runtime::contract::RuntimeBackend::C => (
                    if c_driver { "ok" } else { "missing" },
                    if c_driver {
                        format!("environment can use `{}`", default_c_compiler())
                    } else {
                        format!(
                            "environment requests the C backend but `{}` was not detected",
                            default_c_compiler()
                        )
                    },
                    c_driver,
                ),
                agam_runtime::contract::RuntimeBackend::Jit => (
                    "ok",
                    "environment prefers the in-memory JIT backend".to_string(),
                    true,
                ),
                agam_runtime::contract::RuntimeBackend::Auto => (
                    "selected",
                    "environment defers backend choice to normal auto-resolution".to_string(),
                    true,
                ),
            };
            print_doctor_status("env backend", status, &detail);
            healthy &= backend_ok;
        } else if verbose {
            print_doctor_status(
                "env backend",
                "inherit",
                "no environment-specific backend override",
            );
        }
        if let Some(runtime_abi) = resolved.runtime_abi {
            let abi_ok = runtime_abi == agam_runtime::contract::RUNTIME_ABI_VERSION;
            print_doctor_status(
                "env runtime abi",
                if abi_ok { "ok" } else { "mismatch" },
                &format!(
                    "environment expects v{}; host runtime exports v{}",
                    runtime_abi,
                    agam_runtime::contract::RUNTIME_ABI_VERSION
                ),
            );
            healthy &= abi_ok;
        }
        if !resolved.profiles.is_empty() {
            print_doctor_status(
                "env profiles",
                "selected",
                &format!("profiles `{}`", resolved.profiles.join(", ")),
            );
        }
    }

    println!(
        "recommended sdk command: agamc package sdk{} --output {}",
        environment
            .map(|report| format!(" --env {}", report.environment.name))
            .unwrap_or_default(),
        default_sdk_distribution_output_dir().display()
    );

    Ok(healthy)
}

#[derive(Debug)]
pub(crate) struct SdkDistributionOutcome {
    pub root: PathBuf,
    pub compiler_binary: PathBuf,
    pub manifest_path: PathBuf,
    pub llvm_bundle_root: Option<PathBuf>,
    pub android_sysroot_root: Option<PathBuf>,
}

pub(crate) fn current_host_sdk_platform() -> String {
    bundled_llvm_platform_dir().to_string()
}

pub(crate) fn default_sdk_distribution_output_dir() -> PathBuf {
    PathBuf::from("dist").join(current_host_sdk_platform())
}

pub(crate) fn relative_path_string(root: &Path, path: &Path) -> Result<String, String> {
    path.strip_prefix(root)
        .map_err(|_| {
            format!(
                "failed to compute relative path for `{}` under `{}`",
                path.display(),
                root.display()
            )
        })
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
}

pub(crate) fn default_host_target_triple() -> String {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => "x86_64-pc-windows-msvc".into(),
        ("windows", "aarch64") => "aarch64-pc-windows-msvc".into(),
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu".into(),
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu".into(),
        ("macos", "x86_64") => "x86_64-apple-darwin".into(),
        ("macos", "aarch64") => "aarch64-apple-darwin".into(),
        _ => format!(
            "{}-unknown-{}",
            std::env::consts::ARCH,
            std::env::consts::OS
        ),
    }
}

pub(crate) fn sdk_supported_targets(
    environment: Option<&EnvironmentInspectReport>,
    packaged_android_sysroot: Option<&str>,
) -> Vec<agam_pkg::SdkTargetProfile> {
    let mut targets = vec![agam_pkg::SdkTargetProfile {
        name: "host-native".into(),
        target_triple: default_host_target_triple(),
        backend: agam_runtime::contract::RuntimeBackend::Llvm,
        sysroot_env: None,
        sdk_env: None,
        packaged_sysroot: None,
    }];

    if matches!(
        host_llvm_target_platform(),
        LlvmTargetPlatform::Windows | LlvmTargetPlatform::Linux
    ) {
        targets.push(agam_pkg::SdkTargetProfile {
            name: "android-arm64".into(),
            target_triple: "aarch64-linux-android21".into(),
            backend: agam_runtime::contract::RuntimeBackend::Llvm,
            sysroot_env: Some(LLVM_SYSROOT_ENV.into()),
            sdk_env: None,
            packaged_sysroot: packaged_android_sysroot.map(str::to_string),
        });
    }

    if let Some(environment) = environment {
        if let Some(target) = environment.environment.target.as_deref() {
            let platform = classify_llvm_target_platform(Some(target));
            let sysroot_env = match platform {
                LlvmTargetPlatform::Android => Some(LLVM_SYSROOT_ENV.into()),
                _ => None,
            };
            let sdk_env = match platform {
                LlvmTargetPlatform::Ios | LlvmTargetPlatform::MacOs => {
                    Some(LLVM_SDKROOT_ENV.into())
                }
                _ => None,
            };
            let packaged_sysroot = match platform {
                LlvmTargetPlatform::Android => packaged_android_sysroot.map(str::to_string),
                _ => None,
            };
            let backend = match environment.environment.preferred_backend {
                Some(agam_runtime::contract::RuntimeBackend::Auto) | None => {
                    agam_runtime::contract::RuntimeBackend::Llvm
                }
                Some(backend) => backend,
            };

            if let Some(existing) = targets
                .iter_mut()
                .find(|profile| profile.target_triple == target)
            {
                existing.backend = backend;
                if existing.sysroot_env.is_none() {
                    existing.sysroot_env = sysroot_env;
                }
                if existing.sdk_env.is_none() {
                    existing.sdk_env = sdk_env;
                }
                if existing.packaged_sysroot.is_none() {
                    existing.packaged_sysroot = packaged_sysroot;
                }
            } else {
                targets.insert(
                    0,
                    agam_pkg::SdkTargetProfile {
                        name: environment.environment.name.clone(),
                        target_triple: target.to_string(),
                        backend,
                        sysroot_env,
                        sdk_env,
                        packaged_sysroot,
                    },
                );
            }
        }
    }

    targets
}

pub(crate) fn detect_packaged_llvm_bundle_root() -> Option<PathBuf> {
    if let Some(explicit_root) = env_path(LLVM_BUNDLE_DIR_ENV) {
        if explicit_root.is_dir() {
            return Some(explicit_root);
        }
    }
    let current_exe = std::env::current_exe().ok()?;
    let exe_dir = current_exe.parent()?;
    for base in [Some(exe_dir), exe_dir.parent()].into_iter().flatten() {
        let candidate = base.join("toolchains").join("llvm");
        if candidate.is_dir() {
            return Some(candidate);
        }
    }
    None
}

pub(crate) fn resolve_sdk_llvm_bundle_source(explicit: Option<&PathBuf>) -> Option<PathBuf> {
    explicit.cloned().or_else(detect_packaged_llvm_bundle_root)
}

pub(crate) fn copy_directory_recursive(source: &Path, destination: &Path) -> Result<(), String> {
    if !source.is_dir() {
        return Err(format!(
            "directory copy source `{}` does not exist or is not a directory",
            source.display()
        ));
    }
    std::fs::create_dir_all(destination).map_err(|e| {
        format!(
            "failed to create directory `{}`: {}",
            destination.display(),
            e
        )
    })?;
    for entry in std::fs::read_dir(source)
        .map_err(|e| format!("failed to read directory `{}`: {}", source.display(), e))?
    {
        let entry = entry.map_err(|e| {
            format!(
                "failed to read directory entry in `{}`: {}",
                source.display(),
                e
            )
        })?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type().map_err(|e| {
            format!(
                "failed to read file type for `{}`: {}",
                source_path.display(),
                e
            )
        })?;
        if file_type.is_dir() {
            copy_directory_recursive(&source_path, &destination_path)?;
        } else {
            if let Some(parent) = destination_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    format!("failed to create directory `{}`: {}", parent.display(), e)
                })?;
            }
            std::fs::copy(&source_path, &destination_path).map_err(|e| {
                format!(
                    "failed to copy `{}` to `{}`: {}",
                    source_path.display(),
                    destination_path.display(),
                    e
                )
            })?;
        }
    }
    Ok(())
}

pub(crate) fn stage_llvm_bundle_into_sdk(
    source: &Path,
    output_root: &Path,
) -> Result<PathBuf, String> {
    let host_platform = bundled_llvm_platform_dir();
    let dest_root = output_root.join("toolchains").join("llvm");
    if source.join(host_platform).is_dir() || source.join("bin").is_dir() {
        copy_directory_recursive(source, &dest_root)?;
        return Ok(dest_root);
    }
    if source
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name == host_platform)
        .unwrap_or(false)
    {
        let destination = dest_root.join(host_platform);
        copy_directory_recursive(source, &destination)?;
        return Ok(dest_root);
    }
    Err(format!(
        "LLVM bundle source `{}` must be a bundle root or `{}` platform directory",
        source.display(),
        host_platform
    ))
}

pub(crate) fn validate_android_sysroot_layout(source: &Path) -> Result<(), String> {
    if !source.is_dir() {
        return Err(format!(
            "Android sysroot source `{}` does not exist or is not a directory",
            source.display()
        ));
    }
    if !source.join("usr").is_dir() {
        return Err(format!(
            "Android sysroot `{}` must include a `usr/` directory",
            source.display()
        ));
    }
    Ok(())
}

pub(crate) fn stage_android_sysroot_into_sdk(
    source: &Path,
    output_root: &Path,
) -> Result<PathBuf, String> {
    validate_android_sysroot_layout(source)?;
    let destination = output_root
        .join("target-packs")
        .join("android-arm64")
        .join("sysroot");
    copy_directory_recursive(source, &destination)?;
    Ok(destination)
}

pub(crate) fn package_sdk_distribution(
    output_root: &Path,
    llvm_bundle: Option<&PathBuf>,
    android_sysroot: Option<&PathBuf>,
    environment: Option<&EnvironmentInspectReport>,
    verbose: bool,
) -> Result<SdkDistributionOutcome, String> {
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("failed to locate current compiler executable: {}", e))?;
    let compiler_name = current_exe.file_name().ok_or_else(|| {
        format!(
            "failed to determine compiler filename from `{}`",
            current_exe.display()
        )
    })?;
    let compiler_destination = output_root.join("bin").join(compiler_name);
    if let Some(parent) = compiler_destination.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create directory `{}`: {}", parent.display(), e))?;
    }
    std::fs::copy(&current_exe, &compiler_destination).map_err(|e| {
        format!(
            "failed to copy compiler binary `{}` to `{}`: {}",
            current_exe.display(),
            compiler_destination.display(),
            e
        )
    })?;

    let llvm_bundle_root = match resolve_sdk_llvm_bundle_source(llvm_bundle) {
        Some(source) => {
            let staged = stage_llvm_bundle_into_sdk(&source, output_root)?;
            if verbose {
                eprintln!("[agamc] staged bundled LLVM from {}", source.display());
            }
            Some(staged)
        }
        None => None,
    };
    let staged_android_sysroot = match resolve_sdk_android_sysroot_source(android_sysroot) {
        Some(source) => {
            let staged = stage_android_sysroot_into_sdk(&source, output_root)?;
            if verbose {
                eprintln!(
                    "[agamc] staged Android sysroot target pack from {}",
                    source.display()
                );
            }
            Some(staged)
        }
        None => None,
    };
    let android_sysroot_relative = staged_android_sysroot
        .as_ref()
        .map(|path| relative_path_string(output_root, path))
        .transpose()?;

    let preferred_llvm_driver = llvm_bundle_root.as_ref().and_then(|root| {
        bundled_llvm_candidate_paths(root)
            .into_iter()
            .find(|path| path.is_file())
    });
    let mut notes = vec![
        "native llvm is the preferred production backend".into(),
        "wsl remains a development-only fallback and is not part of the shipped sdk contract"
            .into(),
    ];
    if let Some(environment) = environment {
        let resolved = &environment.environment;
        let mut note = format!(
            "selected environment `{}` pins compiler `{}`",
            resolved.name, resolved.compiler
        );
        if let Some(sdk) = resolved.sdk.as_deref() {
            note.push_str(&format!(", sdk `{sdk}`"));
        }
        if let Some(target) = resolved.target.as_deref() {
            note.push_str(&format!(", target `{target}`"));
        }
        if let Some(backend) = resolved.preferred_backend {
            note.push_str(&format!(", backend `{}`", runtime_backend_label(backend)));
        }
        if !resolved.profiles.is_empty() {
            note.push_str(&format!(", profiles `{}`", resolved.profiles.join(", ")));
        }
        notes.push(note);
    }
    if let Some(relative) = android_sysroot_relative.as_deref() {
        notes.push(format!(
            "bundled Android target pack `android-arm64` at `{relative}`"
        ));
    }

    let manifest = agam_pkg::SdkDistributionManifest {
        format_version: agam_pkg::SDK_DISTRIBUTION_FORMAT_VERSION,
        sdk_name: format!("agam-sdk-{}", current_host_sdk_platform()),
        host_platform: current_host_sdk_platform(),
        compiler_binary: relative_path_string(output_root, &compiler_destination)?,
        llvm_bundle_root: llvm_bundle_root
            .as_ref()
            .map(|path| relative_path_string(output_root, path))
            .transpose()?,
        preferred_llvm_driver: preferred_llvm_driver
            .as_ref()
            .map(|path| relative_path_string(output_root, path))
            .transpose()?,
        supported_targets: sdk_supported_targets(environment, android_sysroot_relative.as_deref()),
        notes,
    };
    let manifest_path = output_root.join("sdk-manifest.json");
    if let Some(parent) = manifest_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create directory `{}`: {}", parent.display(), e))?;
    }
    agam_pkg::write_sdk_distribution_manifest_to_path(&manifest_path, &manifest)?;

    Ok(SdkDistributionOutcome {
        root: output_root.to_path_buf(),
        compiler_binary: compiler_destination,
        manifest_path,
        llvm_bundle_root,
        android_sysroot_root: staged_android_sysroot,
    })
}
