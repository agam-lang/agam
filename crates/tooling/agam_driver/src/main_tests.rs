use super::*;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::time::Duration;

fn parse_source_features(source: &str) -> SourceFeatureFlags {
    let tokens = agam_lexer::tokenize(source, SourceId(0));
    let mut features = source_feature_flags_from_tokens(&tokens);
    let module = agam_parser::parse(tokens, SourceId(0)).expect("source should parse");
    merge_function_call_cache_annotations(&module, &mut features.call_cache);
    features
}

fn temp_dir(prefix: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "agam_driver_{prefix}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn with_clean_agam_registry_env<R>(f: impl FnOnce() -> R) -> R {
    let _guard = registry_env_lock()
        .lock()
        .expect("registry env lock should not be poisoned");
    let default_key = "AGAM_REGISTRY_INDEX";
    let agam_key = agam_pkg::registry_index_env_var("agam");
    let default_restore = RegistryIndexEnvRestore::capture(default_key);
    let agam_restore = RegistryIndexEnvRestore::capture(&agam_key);
    unsafe {
        std::env::remove_var(default_key);
        std::env::remove_var(&agam_key);
    }
    let result = f();
    drop(agam_restore);
    drop(default_restore);
    result
}

fn environment_report(
    name: &str,
    target: Option<&str>,
    backend: Option<agam_runtime::contract::RuntimeBackend>,
) -> EnvironmentInspectReport {
    EnvironmentInspectReport {
        workspace_root: PathBuf::from("C:/agam/workspace"),
        manifest_path: PathBuf::from("C:/agam/workspace/agam.toml"),
        selected_by_default: false,
        environment: agam_pkg::ResolvedEnvironment {
            name: name.into(),
            compiler: "0.2.0".into(),
            sdk: None,
            target: target.map(str::to_string),
            runtime_abi: Some(agam_runtime::contract::RUNTIME_ABI_VERSION),
            preferred_backend: backend,
            profiles: vec!["release".into()],
            packages: vec!["json@1.4.0".into()],
        },
    }
}

fn build_request(file: impl Into<PathBuf>, output: impl Into<PathBuf>) -> BuildRequest {
    BuildRequest {
        file: file.into(),
        output: output.into(),
    }
}

fn check_request(file: impl Into<PathBuf>) -> CheckRequest {
    CheckRequest { file: file.into() }
}

fn test_request(file: impl Into<PathBuf>) -> TestRequest {
    TestRequest { file: file.into() }
}

fn update_maximum(counter: &AtomicUsize, candidate: usize) {
    let mut current = counter.load(AtomicOrdering::SeqCst);
    while candidate > current {
        match counter.compare_exchange(
            current,
            candidate,
            AtomicOrdering::SeqCst,
            AtomicOrdering::SeqCst,
        ) {
            Ok(_) => break,
            Err(observed) => current = observed,
        }
    }
}

#[test]
fn test_sanitize_project_name_collapses_non_identifier_runs() {
    assert_eq!(sanitize_project_name("  Hello__Agam!!  "), "hello-agam");
    assert_eq!(sanitize_project_name("###"), "agam-app");
}

#[test]
fn test_scaffold_project_layout_creates_first_party_files() {
    let root = temp_dir("scaffold");
    let project_root = root.join("hello-app");

    let scaffold =
        scaffold_project_layout(&project_root, false, false).expect("scaffold should work");

    assert_eq!(scaffold.manifest_path, project_root.join("agam.toml"));
    assert_eq!(
        scaffold.entry_file,
        project_root.join("src").join("main.agam")
    );
    assert!(project_root.join("tests").join("smoke.agam").is_file());
    assert!(
        agam_pkg::read_workspace_manifest_from_path(&project_root.join("agam.toml"))
            .expect("read manifest")
            .project
            .name
            == "hello-app"
    );
    let gitignore = fs::read_to_string(project_root.join(".gitignore")).expect("read gitignore");
    assert!(gitignore.contains("dist/"));
    assert!(!gitignore.contains("src/main"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_resolve_workspace_layout_uses_manifest_root_entry_and_tests() {
    let root = temp_dir("workspace");
    let manifest = root.join("agam.toml");
    let entry = root.join("src").join("main.agam");
    let test_file = root.join("tests").join("smoke.agam");
    fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
    fs::create_dir_all(test_file.parent().expect("test parent")).expect("create tests");
    agam_pkg::write_workspace_manifest_to_path(
        &manifest,
        &agam_pkg::scaffold_workspace_manifest("workspace"),
    )
    .expect("write manifest");
    fs::write(&entry, render_project_entry("workspace")).expect("write entry");
    fs::write(&test_file, render_project_smoke_test()).expect("write test");

    let layout =
        resolve_workspace_layout(Some(root.clone())).expect("workspace layout should resolve");

    assert_eq!(layout.root, root);
    assert_eq!(layout.manifest_path.as_ref(), Some(&manifest));
    assert_eq!(layout.project_name, "workspace");
    assert_eq!(layout.entry_file, entry);
    assert_eq!(layout.test_files, vec![test_file]);

    let _ = fs::remove_dir_all(layout.root);
}

#[test]
fn test_resolve_workspace_layout_uses_manifest_declared_entry_path() {
    let root = temp_dir("workspace_entry");
    let manifest = root.join("agam.toml");
    let entry = root.join("app").join("main.agam");
    fs::create_dir_all(entry.parent().expect("entry parent")).expect("create app");

    let mut workspace_manifest = agam_pkg::scaffold_workspace_manifest("workspace-entry");
    workspace_manifest.project.entry = Some("app/main.agam".into());
    agam_pkg::write_workspace_manifest_to_path(&manifest, &workspace_manifest)
        .expect("write manifest");
    fs::write(&entry, render_project_entry("workspace-entry")).expect("write entry");

    let layout =
        resolve_workspace_layout(Some(root.clone())).expect("workspace layout should resolve");

    assert_eq!(layout.manifest_path.as_ref(), Some(&manifest));
    assert_eq!(layout.project_name, "workspace-entry");
    assert_eq!(layout.entry_file, entry);
    assert_eq!(layout.source_files, vec![layout.entry_file.clone()]);

    let _ = fs::remove_dir_all(layout.root);
}

#[test]
fn test_resolve_workspace_layout_rejects_manifest_entry_outside_workspace() {
    let root = temp_dir("workspace_invalid_entry");
    let manifest = root.join("agam.toml");

    let mut workspace_manifest = agam_pkg::scaffold_workspace_manifest("workspace-invalid");
    workspace_manifest.project.entry = Some("../escape.agam".into());
    agam_pkg::write_workspace_manifest_to_path(&manifest, &workspace_manifest)
        .expect("write manifest");

    let error = resolve_workspace_layout(Some(root.clone())).expect_err("manifest should fail");
    assert!(error.contains("must stay inside the workspace root"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_resolve_workspace_layout_supports_single_source_file_without_manifest() {
    let root = temp_dir("single_file");
    let file = root.join("script.agam");
    fs::write(&file, "fn main() -> i32 { return 0; }\n").expect("write source");

    let layout =
        resolve_workspace_layout(Some(file.clone())).expect("single source should resolve");

    assert!(layout.manifest_path.is_none());
    assert_eq!(layout.entry_file, file);
    assert_eq!(layout.source_files, vec![layout.entry_file.clone()]);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_resolve_entry_source_path_uses_workspace_root_entry_file() {
    let root = temp_dir("entry_source_root");
    let manifest = root.join("agam.toml");
    let entry = root.join("src").join("main.agam");
    fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
    agam_pkg::write_workspace_manifest_to_path(
        &manifest,
        &agam_pkg::scaffold_workspace_manifest("entry-source-root"),
    )
    .expect("write manifest");
    fs::write(&entry, render_project_entry("entry-source-root")).expect("write entry");

    let resolved =
        resolve_entry_source_path(&root).expect("workspace root should resolve to entry file");
    assert_eq!(resolved, entry);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_default_package_output_path_uses_dist_for_manifest_workspace() {
    let root = temp_dir("package_output_workspace");
    let manifest = root.join("agam.toml");
    let entry = root.join("src").join("main.agam");
    fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
    agam_pkg::write_workspace_manifest_to_path(
        &manifest,
        &agam_pkg::scaffold_workspace_manifest("package-output-workspace"),
    )
    .expect("write manifest");
    fs::write(&entry, render_project_entry("package-output-workspace")).expect("write entry");

    let output =
        default_package_output_path(&root).expect("workspace root should resolve package output");
    assert_eq!(
        output,
        root.join("dist")
            .join("package-output-workspace.agpkg.json")
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_default_package_output_path_keeps_single_file_neighbor() {
    let root = temp_dir("package_output_single_file");
    let file = root.join("script.agam");
    fs::write(&file, "fn main() -> i32 { return 0; }\n").expect("write source");

    let output =
        default_package_output_path(&file).expect("single-file package output should resolve");
    assert_eq!(output, root.join("script.agpkg.json"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_default_build_output_path_uses_dist_for_manifest_workspace() {
    let root = temp_dir("build_output_workspace");
    let manifest = root.join("agam.toml");
    let entry = root.join("src").join("main.agam");
    fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
    agam_pkg::write_workspace_manifest_to_path(
        &manifest,
        &agam_pkg::scaffold_workspace_manifest("build-output-workspace"),
    )
    .expect("write manifest");
    fs::write(&entry, render_project_entry("build-output-workspace")).expect("write entry");

    let output = default_build_output_path(&root, Some("x86_64-pc-windows-msvc"))
        .expect("workspace root should resolve build output");
    assert_eq!(output, root.join("dist").join("build-output-workspace.exe"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_default_build_output_path_keeps_single_file_neighbor() {
    let root = temp_dir("build_output_single_file");
    let file = root.join("script.agam");
    fs::write(&file, "fn main() -> i32 { return 0; }\n").expect("write source");

    let output =
        default_build_output_path(&file, None).expect("single-file build output should resolve");
    assert_eq!(output, default_native_binary_output_path(&file, None));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_resolve_build_requests_uses_workspace_manifest_entry_output() {
    let root = temp_dir("build_requests_manifest");
    let manifest = root.join("agam.toml");
    let entry = root.join("src").join("main.agam");
    fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
    agam_pkg::write_workspace_manifest_to_path(
        &manifest,
        &agam_pkg::scaffold_workspace_manifest("build-requests-manifest"),
    )
    .expect("write manifest");
    fs::write(&entry, render_project_entry("build-requests-manifest")).expect("write entry");

    let requests = resolve_build_requests(
        std::slice::from_ref(&manifest),
        None,
        Some("x86_64-pc-windows-msvc"),
    )
    .expect("manifest input should resolve to entry file");
    assert_eq!(
        requests,
        vec![BuildRequest {
            file: entry.clone(),
            output: entry.with_extension("exe"),
        }]
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_compile_file_rejects_undeclared_identifier() {
    let root = temp_dir("compile_semantic_error");
    let file = root.join("broken.agam");
    fs::write(&file, "fn main(): y\n").expect("write source");

    let error = compile_file(&file, false).expect_err("compile should fail");
    assert!(error.contains("semantic error"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_compile_file_accepts_builtin_println() {
    let root = temp_dir("compile_builtin");
    let file = root.join("builtin.agam");
    fs::write(&file, "fn main(): println(\"hi\")\n").expect("write source");

    compile_file(&file, false).expect("compile should succeed");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_resolve_build_requests_rejects_explicit_output_for_multiple_inputs() {
    let files = vec![PathBuf::from("a.agam"), PathBuf::from("b.agam")];
    let error = resolve_build_requests(&files, Some(PathBuf::from("out.exe")), None)
        .expect_err("multiple inputs should reject one explicit output");
    assert!(error.contains("`--output` only supports a single input file"));
}

#[test]
fn test_resolve_build_requests_uses_default_output_per_file() {
    let root = temp_dir("build_requests_defaults");
    let alpha = root.join("alpha.agam");
    let beta = root.join("beta.agam");
    fs::write(&alpha, "fn main() -> i32 { return 0; }\n").expect("write alpha source");
    fs::write(&beta, "fn main() -> i32 { return 0; }\n").expect("write beta source");

    let files = vec![alpha.clone(), beta.clone()];
    let requests = resolve_build_requests(&files, None, Some("x86_64-pc-windows-msvc"))
        .expect("build requests should resolve");
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[0],
        BuildRequest {
            file: alpha.clone(),
            output: alpha.with_extension("exe"),
        }
    );
    assert_eq!(
        requests[1],
        BuildRequest {
            file: beta.clone(),
            output: beta.with_extension("exe"),
        }
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_resolve_build_requests_keeps_explicit_output_for_single_input() {
    let root = temp_dir("build_requests_single_output");
    let file = root.join("main.agam");
    fs::write(&file, "fn main() -> i32 { return 0; }\n").expect("write source");

    let files = vec![file.clone()];
    let output = root.join("program.exe");
    let requests = resolve_build_requests(&files, Some(output.clone()), None)
        .expect("single input should allow explicit output");
    assert_eq!(requests, vec![BuildRequest { file, output }]);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_resolve_build_requests_resolves_workspace_root_before_explicit_output() {
    let root = temp_dir("build_requests_root_output");
    let manifest = root.join("agam.toml");
    let entry = root.join("src").join("main.agam");
    fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
    agam_pkg::write_workspace_manifest_to_path(
        &manifest,
        &agam_pkg::scaffold_workspace_manifest("build-requests-root-output"),
    )
    .expect("write manifest");
    fs::write(&entry, render_project_entry("build-requests-root-output")).expect("write entry");

    let output = root.join("dist").join("program.exe");
    let requests = resolve_build_requests(std::slice::from_ref(&root), Some(output.clone()), None)
        .expect("workspace root should resolve to entry before output is applied");
    assert_eq!(
        requests,
        vec![BuildRequest {
            file: entry.clone(),
            output,
        }]
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_resolve_build_requests_deduplicates_overlapping_workspace_inputs() {
    let root = temp_dir("build_requests_dedup");
    let manifest = root.join("agam.toml");
    let entry = root.join("src").join("main.agam");
    fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
    agam_pkg::write_workspace_manifest_to_path(
        &manifest,
        &agam_pkg::scaffold_workspace_manifest("build-requests-dedup"),
    )
    .expect("write manifest");
    fs::write(&entry, render_project_entry("build-requests-dedup")).expect("write entry");

    let requests = resolve_build_requests(
        &[root.clone(), manifest.clone(), entry.clone()],
        None,
        Some("x86_64-pc-windows-msvc"),
    )
    .expect("overlapping workspace inputs should resolve");
    assert_eq!(
        requests,
        vec![BuildRequest {
            file: entry.clone(),
            output: entry.with_extension("exe"),
        }]
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_execute_build_requests_with_runner_preserves_request_order() {
    let requests = vec![
        build_request("alpha.agam", "alpha.exe"),
        build_request("beta.agam", "beta.exe"),
        build_request("gamma.agam", "gamma.exe"),
    ];

    let results = execute_build_requests_with_runner(&requests, 3, |request| {
        let delay_ms = match request.file.file_stem().and_then(|name| name.to_str()) {
            Some("alpha") => 40,
            Some("beta") => 5,
            Some("gamma") => 20,
            _ => 1,
        };
        std::thread::sleep(Duration::from_millis(delay_ms));
        BuildRequestResult {
            request: request.clone(),
            stdout: request.file.to_string_lossy().as_bytes().to_vec(),
            stderr: Vec::new(),
            succeeded: true,
            launch_error: None,
        }
    });

    let result_requests = results
        .iter()
        .map(|result| result.request.clone())
        .collect::<Vec<_>>();
    assert_eq!(result_requests, requests);
}

#[test]
fn test_execute_build_requests_with_runner_respects_parallelism_limit() {
    let requests = vec![
        build_request("one.agam", "one.exe"),
        build_request("two.agam", "two.exe"),
        build_request("three.agam", "three.exe"),
        build_request("four.agam", "four.exe"),
    ];
    let active = AtomicUsize::new(0);
    let observed_max = AtomicUsize::new(0);

    let results = execute_build_requests_with_runner(&requests, 2, |request| {
        let now_active = active.fetch_add(1, AtomicOrdering::SeqCst) + 1;
        update_maximum(&observed_max, now_active);
        std::thread::sleep(Duration::from_millis(20));
        active.fetch_sub(1, AtomicOrdering::SeqCst);

        BuildRequestResult {
            request: request.clone(),
            stdout: Vec::new(),
            stderr: Vec::new(),
            succeeded: true,
            launch_error: None,
        }
    });

    assert_eq!(results.len(), requests.len());
    assert!(observed_max.load(AtomicOrdering::SeqCst) <= 2);
    assert!(observed_max.load(AtomicOrdering::SeqCst) >= 2);
}

#[test]
fn test_execute_check_requests_with_runner_preserves_request_order() {
    let requests = vec![
        check_request("alpha.agam"),
        check_request("beta.agam"),
        check_request("gamma.agam"),
    ];

    let results = execute_check_requests_with_runner(&requests, 3, |request| {
        let delay_ms = match request.file.file_stem().and_then(|name| name.to_str()) {
            Some("alpha") => 40,
            Some("beta") => 5,
            Some("gamma") => 20,
            _ => 1,
        };
        std::thread::sleep(Duration::from_millis(delay_ms));
        CheckRequestResult {
            request: request.clone(),
            stdout: request.file.to_string_lossy().as_bytes().to_vec(),
            stderr: Vec::new(),
            succeeded: true,
            launch_error: None,
        }
    });

    let result_requests = results
        .iter()
        .map(|result| result.request.clone())
        .collect::<Vec<_>>();
    assert_eq!(result_requests, requests);
}

#[test]
fn test_execute_check_requests_with_runner_respects_parallelism_limit() {
    let requests = vec![
        check_request("one.agam"),
        check_request("two.agam"),
        check_request("three.agam"),
        check_request("four.agam"),
    ];
    let active = AtomicUsize::new(0);
    let observed_max = AtomicUsize::new(0);

    let results = execute_check_requests_with_runner(&requests, 2, |request| {
        let now_active = active.fetch_add(1, AtomicOrdering::SeqCst) + 1;
        update_maximum(&observed_max, now_active);
        std::thread::sleep(Duration::from_millis(20));
        active.fetch_sub(1, AtomicOrdering::SeqCst);

        CheckRequestResult {
            request: request.clone(),
            stdout: Vec::new(),
            stderr: Vec::new(),
            succeeded: true,
            launch_error: None,
        }
    });

    assert_eq!(results.len(), requests.len());
    assert!(observed_max.load(AtomicOrdering::SeqCst) <= 2);
    assert!(observed_max.load(AtomicOrdering::SeqCst) >= 2);
}

#[test]
fn test_execute_test_requests_with_runner_preserves_request_order() {
    let requests = vec![
        test_request("alpha.agam"),
        test_request("beta.agam"),
        test_request("gamma.agam"),
    ];

    let results = execute_test_requests_with_runner(&requests, 3, |request| {
        let delay_ms = match request.file.file_stem().and_then(|name| name.to_str()) {
            Some("alpha") => 40,
            Some("beta") => 5,
            Some("gamma") => 20,
            _ => 1,
        };
        std::thread::sleep(Duration::from_millis(delay_ms));
        TestRequestResult {
            request: request.clone(),
            summary: Some(agam_test::FileTestSummary {
                path: request.file.clone(),
                summary: agam_test::TestSummary::default(),
            }),
            error: None,
        }
    });

    let result_requests = results
        .iter()
        .map(|result| result.request.clone())
        .collect::<Vec<_>>();
    assert_eq!(result_requests, requests);
}

#[test]
fn test_execute_test_requests_with_runner_respects_parallelism_limit() {
    let requests = vec![
        test_request("one.agam"),
        test_request("two.agam"),
        test_request("three.agam"),
        test_request("four.agam"),
    ];
    let active = AtomicUsize::new(0);
    let observed_max = AtomicUsize::new(0);

    let results = execute_test_requests_with_runner(&requests, 2, |request| {
        let now_active = active.fetch_add(1, AtomicOrdering::SeqCst) + 1;
        update_maximum(&observed_max, now_active);
        std::thread::sleep(Duration::from_millis(20));
        active.fetch_sub(1, AtomicOrdering::SeqCst);

        TestRequestResult {
            request: request.clone(),
            summary: Some(agam_test::FileTestSummary {
                path: request.file.clone(),
                summary: agam_test::TestSummary::default(),
            }),
            error: None,
        }
    });

    assert_eq!(results.len(), requests.len());
    assert!(observed_max.load(AtomicOrdering::SeqCst) <= 2);
    assert!(observed_max.load(AtomicOrdering::SeqCst) >= 2);
}

#[test]
fn test_ensure_build_output_parent_dir_creates_missing_directory() {
    let root = temp_dir("build_output_parent_dir");
    let output = root.join("nested").join("program.exe");

    ensure_build_output_parent_dir(&output).expect("missing output parent should be created");

    assert!(output.parent().expect("parent").is_dir());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_compile_dev_source_file_skips_warm_state_when_not_running() {
    let root = temp_dir("compile_dev_no_run");
    let file = root.join("main.agam");
    fs::write(&file, "@lang.advance\nfn main() -> i32 { return 0; }\n").expect("write source");

    let warm = compile_dev_source_file(&file, false, false).expect("dev compile should work");
    assert!(warm.is_none());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_compile_dev_source_file_keeps_warm_state_for_run() {
    let root = temp_dir("compile_dev_run");
    let file = root.join("main.agam");
    fs::write(&file, "@lang.advance\nfn main() -> i32 { return 0; }\n").expect("write source");

    let warm = compile_dev_source_file(&file, true, false).expect("warm dev compile should work");
    let warm = warm.expect("warm state should be retained for runnable entry file");
    assert!(warm.source_features.is_some());
    assert_eq!(warm.mir.as_ref().expect("mir").functions.len(), 1);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_compile_file_with_warm_state_captures_mir_and_source_features() {
    let root = temp_dir("compile_warm_state");
    let file = root.join("warm.agam");
    fs::write(&file, "@lang.advance\nfn main() -> i32 { return 0; }\n").expect("write source");

    let warm = compile_file_with_warm_state(&file, false).expect("warm compile should succeed");
    assert!(warm.source_features.is_some());
    assert_eq!(warm.mir.as_ref().expect("mir").functions.len(), 1);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_lower_to_optimized_mir_rejects_type_errors() {
    let root = temp_dir("lower_type_error");
    let file = root.join("broken_type.agam");
    fs::write(&file, "fn main(): while 42: let x = 1\n").expect("write source");

    let error = lower_to_optimized_mir(&file, false).expect_err("lowering should fail");
    assert!(error.contains("type error"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_lower_to_optimized_mir_accepts_valid_source() {
    let root = temp_dir("lower_valid");
    let file = root.join("ok.agam");
    fs::write(&file, "fn main(): let x = 42\n").expect("write source");

    let (mir, _) = lower_to_optimized_mir(&file, false).expect("lowering should succeed");
    assert_eq!(mir.functions.len(), 1);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_incremental_pipeline_clears_all_warm_state_when_manifest_changes() {
    let root = temp_dir("daemon_manifest_invalidation");
    let manifest = root.join("agam.toml");
    let entry = root.join("src").join("main.agam");
    fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
    agam_pkg::write_workspace_manifest_to_path(
        &manifest,
        &agam_pkg::scaffold_workspace_manifest("daemon-manifest"),
    )
    .expect("write manifest");
    fs::write(&entry, render_project_entry("daemon-manifest")).expect("write entry");

    let previous = agam_pkg::snapshot_workspace(Some(root.clone())).expect("snapshot");
    let source_hash = previous.source_files[0].content_hash.clone();
    let mut session = DaemonSession {
        snapshot: Some(previous.clone()),
        cache: BTreeMap::new(),
        last_prewarm: DaemonPrewarmSummary::default(),
    };
    session.cache.entry(entry.clone()).or_default().insert(
        source_hash,
        WarmState {
            source_features: None,
            module: None,
            hir: None,
            mir: None,
        },
    );

    let mut next_manifest = agam_pkg::scaffold_workspace_manifest("daemon-manifest");
    next_manifest.project.version = "0.2.0".into();
    agam_pkg::write_workspace_manifest_to_path(&manifest, &next_manifest)
        .expect("rewrite manifest");

    let next = agam_pkg::snapshot_workspace(Some(root.clone())).expect("snapshot");
    let diff = agam_pkg::diff_workspace_snapshots(&previous, &next);
    let mut pipeline = IncrementalPipeline::new(&mut session);
    pipeline.apply_diff(next, &diff);

    assert!(session.cache.is_empty());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_incremental_pipeline_clears_all_warm_state_when_member_manifest_changes() {
    let root = temp_dir("daemon_member_manifest_invalidation");
    let manifest = root.join("agam.toml");
    let entry = root.join("src").join("main.agam");
    let member_root = root.join("packages").join("core");
    let member_manifest = member_root.join("agam.toml");
    let member_entry = member_root.join("src").join("main.agam");
    fs::create_dir_all(entry.parent().expect("entry parent")).expect("create root src");
    fs::create_dir_all(member_entry.parent().expect("member entry parent"))
        .expect("create member src");

    let mut workspace_manifest = agam_pkg::scaffold_workspace_manifest("daemon-manifest");
    workspace_manifest.workspace.members = vec!["packages/core".into()];
    agam_pkg::write_workspace_manifest_to_path(&manifest, &workspace_manifest)
        .expect("write root manifest");
    agam_pkg::write_workspace_manifest_to_path(
        &member_manifest,
        &agam_pkg::scaffold_workspace_manifest("daemon-member"),
    )
    .expect("write member manifest");
    fs::write(&entry, render_project_entry("daemon-manifest")).expect("write root entry");
    fs::write(&member_entry, render_project_entry("daemon-member")).expect("write member entry");

    let previous = agam_pkg::snapshot_workspace(Some(root.clone())).expect("snapshot");
    let source_hash = previous
        .source_files
        .iter()
        .find(|file| file.path == member_entry)
        .expect("member entry should be tracked")
        .content_hash
        .clone();
    let mut session = DaemonSession {
        snapshot: Some(previous.clone()),
        cache: BTreeMap::new(),
        last_prewarm: DaemonPrewarmSummary::default(),
    };
    session
        .cache
        .entry(member_entry.clone())
        .or_default()
        .insert(
            source_hash,
            WarmState {
                source_features: None,
                module: None,
                hir: None,
                mir: None,
            },
        );

    let mut next_member_manifest = agam_pkg::scaffold_workspace_manifest("daemon-member");
    next_member_manifest.project.version = "0.2.0".into();
    agam_pkg::write_workspace_manifest_to_path(&member_manifest, &next_member_manifest)
        .expect("rewrite member manifest");

    let next = agam_pkg::snapshot_workspace(Some(root.clone())).expect("snapshot");
    let diff = agam_pkg::diff_workspace_snapshots(&previous, &next);
    let mut pipeline = IncrementalPipeline::new(&mut session);
    pipeline.apply_diff(next, &diff);

    assert!(session.cache.is_empty());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_refresh_daemon_session_reuses_unchanged_files_and_rewarms_changed_ones() {
    let root = temp_dir("daemon_refresh");
    let manifest = root.join("agam.toml");
    let entry = root.join("src").join("main.agam");
    fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
    agam_pkg::write_workspace_manifest_to_path(
        &manifest,
        &agam_pkg::scaffold_workspace_manifest("daemon-refresh"),
    )
    .expect("write manifest");
    fs::write(&entry, render_project_entry("daemon-refresh")).expect("write entry");

    let mut session = DaemonSession::default();
    let first_snapshot = agam_pkg::snapshot_workspace(Some(root.clone())).expect("snapshot");
    let first_hash = first_snapshot.source_files[0].content_hash.clone();
    let (first_warm, first_diff) =
        refresh_daemon_session(&mut session, first_snapshot.clone(), false)
            .expect("warm first snapshot");
    assert_eq!(first_warm.warmed_files, 1);
    assert_eq!(first_warm.reused_files, 0);
    assert_eq!(first_diff.added_files, 2);
    assert!(
        session
            .cache
            .get(&entry)
            .expect("entry cache")
            .contains_key(&first_hash)
    );

    let repeat_snapshot = agam_pkg::snapshot_workspace(Some(root.clone())).expect("snapshot");
    let (repeat_warm, repeat_diff) = refresh_daemon_session(&mut session, repeat_snapshot, false)
        .expect("warm repeated snapshot");
    assert_eq!(repeat_warm.warmed_files, 0);
    assert_eq!(repeat_warm.reused_files, 1);
    assert_eq!(repeat_diff.changed_files, 0);
    assert_eq!(repeat_diff.removed_files, 0);

    fs::write(
        &entry,
        "@lang.advance\n\nfn main() -> i32 {\n    return 1;\n}\n",
    )
    .expect("rewrite entry");
    let changed_snapshot = agam_pkg::snapshot_workspace(Some(root.clone())).expect("snapshot");
    let changed_hash = changed_snapshot.source_files[0].content_hash.clone();
    let (changed_warm, changed_diff) =
        refresh_daemon_session(&mut session, changed_snapshot, false)
            .expect("warm changed snapshot");
    assert_eq!(changed_diff.changed_files, 1);
    assert_eq!(changed_warm.warmed_files, 1);
    assert_eq!(changed_warm.reused_files, 0);
    let entry_versions = session.cache.get(&entry).expect("entry cache");
    assert!(entry_versions.contains_key(&changed_hash));
    assert!(!entry_versions.contains_key(&first_hash));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_active_daemon_status_requires_fresh_heartbeat() {
    let root = temp_dir("daemon_status");
    let mut status = DaemonStatusRecord {
        schema_version: DAEMON_STATUS_SCHEMA_VERSION,
        run_mode: DaemonRunMode::ForegroundLoop,
        workspace_root: root.display().to_string(),
        project_name: "daemon-status".into(),
        pid: process::id(),
        session_started_unix_ms: now_unix_ms(),
        last_heartbeat_unix_ms: now_unix_ms(),
        snapshot_file_count: 2,
        warmed_file_count: 1,
        warmed_version_count: 1,
        ast_decl_count: 1,
        hir_function_count: 1,
        mir_function_count: 1,
        last_error: None,
        prewarm: DaemonPrewarmSummary::default(),
        last_diff: DaemonDiffSummary::default(),
    };
    write_daemon_status(&root, &status).expect("write fresh status");
    assert!(active_daemon_status(&root).expect("read status").is_some());

    status.last_heartbeat_unix_ms = now_unix_ms().saturating_sub(DAEMON_HEARTBEAT_STALE_MS + 1);
    write_daemon_status(&root, &status).expect("write stale status");
    assert!(
        read_daemon_status(&root)
            .expect("read stale status")
            .is_some()
    );
    assert!(
        active_daemon_status(&root)
            .expect("read active status")
            .is_none()
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_active_daemon_status_ignores_one_shot_snapshots() {
    let root = temp_dir("daemon_snapshot_status");
    let status = DaemonStatusRecord {
        schema_version: DAEMON_STATUS_SCHEMA_VERSION,
        run_mode: DaemonRunMode::OneShot,
        workspace_root: root.display().to_string(),
        project_name: "daemon-snapshot".into(),
        pid: process::id(),
        session_started_unix_ms: now_unix_ms(),
        last_heartbeat_unix_ms: now_unix_ms(),
        snapshot_file_count: 1,
        warmed_file_count: 1,
        warmed_version_count: 1,
        ast_decl_count: 1,
        hir_function_count: 1,
        mir_function_count: 1,
        last_error: None,
        prewarm: DaemonPrewarmSummary::default(),
        last_diff: DaemonDiffSummary::default(),
    };
    write_daemon_status(&root, &status).expect("write snapshot status");
    assert!(
        active_daemon_status(&root)
            .expect("read active status")
            .is_none()
    );
    assert_eq!(
        daemon_liveness(&status, now_unix_ms()),
        DaemonLiveness::Snapshot
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_active_daemon_status_ignores_error_status() {
    let root = temp_dir("daemon_error_status");
    let status = DaemonStatusRecord {
        schema_version: DAEMON_STATUS_SCHEMA_VERSION,
        run_mode: DaemonRunMode::ForegroundLoop,
        workspace_root: root.display().to_string(),
        project_name: "daemon-error".into(),
        pid: process::id(),
        session_started_unix_ms: now_unix_ms(),
        last_heartbeat_unix_ms: now_unix_ms(),
        snapshot_file_count: 1,
        warmed_file_count: 0,
        warmed_version_count: 0,
        ast_decl_count: 0,
        hir_function_count: 0,
        mir_function_count: 0,
        last_error: Some("parse error".into()),
        prewarm: DaemonPrewarmSummary::default(),
        last_diff: DaemonDiffSummary::default(),
    };
    write_daemon_status(&root, &status).expect("write error status");
    assert!(
        active_daemon_status(&root)
            .expect("read active status")
            .is_none()
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_dev_daemon_status_message_reports_snapshot_separately() {
    let root = temp_dir("dev_daemon_snapshot");
    let status = DaemonStatusRecord {
        schema_version: DAEMON_STATUS_SCHEMA_VERSION,
        run_mode: DaemonRunMode::OneShot,
        workspace_root: root.display().to_string(),
        project_name: "dev-daemon-snapshot".into(),
        pid: process::id(),
        session_started_unix_ms: now_unix_ms(),
        last_heartbeat_unix_ms: now_unix_ms(),
        snapshot_file_count: 1,
        warmed_file_count: 1,
        warmed_version_count: 1,
        ast_decl_count: 1,
        hir_function_count: 1,
        mir_function_count: 1,
        last_error: None,
        prewarm: DaemonPrewarmSummary::default(),
        last_diff: DaemonDiffSummary::default(),
    };
    write_daemon_status(&root, &status).expect("write snapshot status");

    let message = dev_daemon_status_message(&root).expect("format dev daemon message");
    assert!(message.contains("snapshot available"));
    assert!(!message.contains("stale"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_dev_daemon_status_message_reports_failure() {
    let root = temp_dir("dev_daemon_error");
    let status = DaemonStatusRecord {
        schema_version: DAEMON_STATUS_SCHEMA_VERSION,
        run_mode: DaemonRunMode::ForegroundLoop,
        workspace_root: root.display().to_string(),
        project_name: "dev-daemon-error".into(),
        pid: process::id(),
        session_started_unix_ms: now_unix_ms(),
        last_heartbeat_unix_ms: now_unix_ms(),
        snapshot_file_count: 1,
        warmed_file_count: 0,
        warmed_version_count: 0,
        ast_decl_count: 0,
        hir_function_count: 0,
        mir_function_count: 0,
        last_error: Some("semantic error".into()),
        prewarm: DaemonPrewarmSummary::default(),
        last_diff: DaemonDiffSummary::default(),
    };
    write_daemon_status(&root, &status).expect("write daemon error status");

    let message = dev_daemon_status_message(&root).expect("format dev daemon message");
    assert!(message.contains("last warm refresh failed"));
    assert!(message.contains("semantic error"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_read_daemon_status_accepts_legacy_status_without_new_fields() {
    let root = temp_dir("daemon_legacy_status");
    ensure_daemon_status_dir(&root).expect("create daemon status dir");
    let path = daemon_status_path(&root);
    fs::write(
        &path,
        format!(
            concat!(
                "{{",
                "\"schema_version\":{},",
                "\"workspace_root\":\"{}\",",
                "\"project_name\":\"legacy\",",
                "\"pid\":{},",
                "\"session_started_unix_ms\":1,",
                "\"last_heartbeat_unix_ms\":2,",
                "\"snapshot_file_count\":3,",
                "\"warmed_file_count\":2,",
                "\"warmed_version_count\":2,",
                "\"ast_decl_count\":4,",
                "\"hir_function_count\":5,",
                "\"mir_function_count\":6,",
                "\"last_diff\":{{",
                "\"added_files\":1,",
                "\"changed_files\":0,",
                "\"removed_files\":0,",
                "\"unchanged_files\":2,",
                "\"manifest_changed\":false",
                "}}",
                "}}"
            ),
            DAEMON_STATUS_SCHEMA_VERSION,
            root.display().to_string().replace('\\', "\\\\"),
            process::id()
        ),
    )
    .expect("write legacy status");

    let status = read_daemon_status(&root)
        .expect("read legacy daemon status")
        .expect("status should parse");
    assert_eq!(status.run_mode, DaemonRunMode::ForegroundLoop);
    assert_eq!(status.last_error, None);
    assert_eq!(status.prewarm, DaemonPrewarmSummary::default());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_run_daemon_foreground_once_persists_status_file() {
    let root = temp_dir("daemon_once_status");
    let file = root.join("main.agam");
    fs::write(&file, "fn main() -> i32 { return 0; }\n").expect("write source");

    run_daemon_foreground(
        Some(file.clone()),
        true,
        DAEMON_DEFAULT_POLL_MS,
        false,
        false,
    )
    .expect("one-shot daemon run should succeed");

    let status = read_daemon_status(&root)
        .expect("read daemon status")
        .expect("status file should exist after one-shot refresh");
    assert_eq!(status.run_mode, DaemonRunMode::OneShot);
    assert_eq!(status.last_error, None);
    assert_eq!(status.workspace_root, root.display().to_string());
    assert_eq!(status.warmed_file_count, 1);
    assert_eq!(status.snapshot_file_count, 1);
    assert!(status.prewarm.package_ready);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_prewarm_daemon_entry_artifacts_populates_cache() {
    let root = temp_dir("daemon_prewarm_cache");
    let file = root.join("main.agam");
    fs::write(&file, "fn main() -> i32 { return 0; }\n").expect("write source");

    let snapshot = agam_pkg::snapshot_workspace(Some(file.clone())).expect("snapshot");
    let mut session = DaemonSession::default();
    refresh_daemon_session(&mut session, snapshot.clone(), false).expect("warm snapshot");

    let summary = prewarm_daemon_entry_artifacts(&session, &snapshot, false);
    let cache = agam_runtime::cache::CacheStore::for_path(&root).expect("cache store");
    let status = cache.status(10).expect("cache status");
    let expected_backend = resolve_backend(Backend::Auto, true);

    assert!(summary.package_ready);
    assert_eq!(
        summary.build_backend.as_deref(),
        Some(render_backend_cli_value(expected_backend))
    );
    assert_eq!(summary.last_error, None);
    assert!(
        status
            .by_kind
            .iter()
            .any(|kind| { kind.kind == agam_runtime::cache::CacheArtifactKind::PortablePackage })
    );
    if expected_backend == Backend::Jit {
        assert!(!summary.build_ready);
    } else {
        assert!(summary.build_ready);
        assert!(status.by_kind.iter().any(|kind| {
            matches!(
                kind.kind,
                agam_runtime::cache::CacheArtifactKind::NativeBinary
                    | agam_runtime::cache::CacheArtifactKind::LlvmIr
                    | agam_runtime::cache::CacheArtifactKind::CSource
            )
        }));
    }

    // Multi-file warm index should have been written
    assert!(summary.prewarmed_file_count > 0);
    assert_eq!(summary.prewarmed_total_files, 1); // single-file workspace

    let warm_index =
        agam_pkg::read_daemon_warm_index(&root).expect("reading warm index should succeed");
    assert!(
        warm_index.is_some(),
        "warm index should exist after prewarm"
    );
    let warm_index = warm_index.unwrap();
    assert_eq!(warm_index.files.len(), 1);

    // MIR artifacts are persisted in the prewarm directory for cross-process reuse
    let prewarm_dir = daemon_prewarm_stage_dir(&root);
    assert!(prewarm_dir.is_dir(), "prewarm directory should exist");
    let prewarm_entries: Vec<_> = fs::read_dir(&prewarm_dir)
        .expect("read prewarm dir")
        .collect();
    assert!(
        prewarm_entries.iter().any(|e| {
            e.as_ref()
                .map(|e| e.file_name().to_string_lossy().contains("_mir_"))
                .unwrap_or(false)
        }),
        "prewarm directory should contain MIR artifact(s)"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_daemon_prewarm_status_message_reports_missing_package_artifact() {
    let root = temp_dir("daemon_prewarm_status_missing_package");
    let missing_package = root.join("missing.agpkg.json");
    let summary = DaemonPrewarmSummary {
        package_ready: true,
        package_artifact_path: Some(missing_package.display().to_string()),
        build_backend: Some("jit".into()),
        ..DaemonPrewarmSummary::default()
    };

    let message =
        daemon_prewarm_status_message(&summary).expect("prewarm status message should exist");
    assert!(message.contains("package stale (artifact missing)"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_load_daemon_prewarmed_entry_reuses_matching_snapshot() {
    let root = temp_dir("daemon_prewarm_reuse");
    let file = root.join("main.agam");
    fs::write(&file, "fn main() -> i32 { return 0; }\n").expect("write source");

    run_daemon_foreground(
        Some(file.clone()),
        true,
        DAEMON_DEFAULT_POLL_MS,
        false,
        false,
    )
    .expect("one-shot daemon run should succeed");

    let prewarmed = load_daemon_prewarmed_entry(&file, false).expect("prewarmed entry should load");
    assert_eq!(prewarmed.package.mir.functions.len(), 1);
    assert_eq!(prewarmed.call_cache, CallCacheSelection::default());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_load_daemon_prewarmed_warm_state_reuses_matching_snapshot() {
    let root = temp_dir("daemon_prewarm_warm_state");
    let file = root.join("main.agam");
    fs::write(&file, "fn main() -> i32 { return 0; }\n").expect("write source");

    run_daemon_foreground(
        Some(file.clone()),
        true,
        DAEMON_DEFAULT_POLL_MS,
        false,
        false,
    )
    .expect("one-shot daemon run should succeed");

    let warm_state =
        load_daemon_prewarmed_warm_state(&file, false).expect("warm state should load");
    assert!(warm_state.module.is_none());
    assert!(warm_state.hir.is_none());
    assert_eq!(warm_state.mir.as_ref().expect("mir").functions.len(), 1);
    assert!(warm_state.source_features.is_some());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_load_daemon_warm_state_for_file_reuses_disk_call_cache_metadata() {
    let root = temp_dir("daemon_warm_disk_features");
    let file = root.join("main.agam");
    fs::write(
        &file,
        "@lang.advance\n@lang.feat.call_cache\nfn main() -> i32 { return 0; }\n",
    )
    .expect("write source");

    run_daemon_foreground(
        Some(file.clone()),
        true,
        DAEMON_DEFAULT_POLL_MS,
        false,
        false,
    )
    .expect("one-shot daemon run should succeed");

    let warm_state = load_daemon_warm_state_for_file(&file, false).expect("warm state should load");
    assert_eq!(warm_state.mir.as_ref().expect("mir").functions.len(), 1);
    let source_features = warm_state
        .source_features
        .as_ref()
        .expect("disk warm state should carry source features");
    assert!(source_features.call_cache.enable_all);
    assert!(!source_features.call_cache.disable_all);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_load_daemon_prewarmed_entry_rejects_hash_mismatch() {
    let root = temp_dir("daemon_prewarm_hash_mismatch");
    let file = root.join("main.agam");
    fs::write(&file, "fn main() -> i32 { return 0; }\n").expect("write source");

    run_daemon_foreground(
        Some(file.clone()),
        true,
        DAEMON_DEFAULT_POLL_MS,
        false,
        false,
    )
    .expect("one-shot daemon run should succeed");
    fs::write(&file, "fn main() -> i32 { return 1; }\n").expect("rewrite source");

    assert!(load_daemon_prewarmed_entry(&file, false).is_none());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_compile_dev_source_file_prefers_daemon_prewarm_for_run() {
    let root = temp_dir("compile_dev_daemon_prewarm");
    let file = root.join("main.agam");
    fs::write(&file, "fn main() -> i32 { return 0; }\n").expect("write source");

    run_daemon_foreground(
        Some(file.clone()),
        true,
        DAEMON_DEFAULT_POLL_MS,
        false,
        false,
    )
    .expect("one-shot daemon run should succeed");

    let warm = compile_dev_source_file(&file, true, false).expect("warm dev compile should work");
    let warm = warm.expect("warm state should be retained for runnable entry file");
    assert!(warm.module.is_none());
    assert!(warm.hir.is_none());
    assert_eq!(warm.mir.as_ref().expect("mir").functions.len(), 1);
    assert!(warm.source_features.is_some());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_compile_dev_source_file_rebuilds_when_disk_warm_state_lacks_source_features() {
    let root = temp_dir("compile_dev_incomplete_disk_warm_state");
    let file = root.join("main.agam");
    fs::write(
        &file,
        "@lang.advance\n@lang.feat.call_cache\nfn main() -> i32 { return 0; }\n",
    )
    .expect("write source");

    let warm_state =
        compile_file_with_warm_state(&file, false).expect("warm compile should succeed");
    let mir = warm_state.mir.as_ref().expect("mir should exist");
    let content_hash =
        agam_runtime::cache::hash_bytes(&fs::read(&file).expect("read source for content hash"));
    let artifact_path =
        daemon_prewarm_mir_artifact_path(&root, &file).expect("artifact path should resolve");
    if let Some(parent) = artifact_path.parent() {
        fs::create_dir_all(parent).expect("create artifact dir");
    }
    let raw_mir_json = serde_json::to_vec(mir).expect("serialize legacy raw MIR artifact");
    fs::write(&artifact_path, raw_mir_json).expect("write legacy raw MIR artifact");
    agam_pkg::write_daemon_warm_index(
        &root,
        &agam_pkg::DaemonWarmIndex {
            format_version: agam_pkg::DAEMON_WARM_INDEX_FORMAT_VERSION,
            files: BTreeMap::from([(
                file.display().to_string(),
                agam_pkg::DaemonWarmFileEntry {
                    content_hash,
                    mir_hash: Some(
                        agam_runtime::cache::hash_serializable(mir)
                            .expect("hash legacy raw MIR artifact"),
                    ),
                    artifact_path: Some(artifact_path.display().to_string()),
                    warm_level: agam_pkg::DaemonWarmLevel::Lowered,
                },
            )]),
        },
    )
    .expect("write daemon warm index");

    let warm = compile_dev_source_file(&file, true, false).expect("dev compile should succeed");
    let warm = warm.expect("warm state should be rebuilt locally");
    assert!(
        warm.module.is_some(),
        "incomplete disk warm state should not be reused for runnable dev flows"
    );
    assert!(warm.hir.is_some());
    assert!(warm.source_features.is_some());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_run_daemon_foreground_once_persists_error_status_on_failure() {
    let root = temp_dir("daemon_once_error");
    let file = root.join("broken.agam");
    fs::write(&file, "fn main(): missing_name\n").expect("write invalid source");

    let error = run_daemon_foreground(
        Some(file.clone()),
        true,
        DAEMON_DEFAULT_POLL_MS,
        false,
        false,
    )
    .expect_err("one-shot daemon run should fail");
    assert!(error.contains("semantic error"));

    let status = read_daemon_status(&root)
        .expect("read daemon error status")
        .expect("status file should exist after one-shot failure");
    assert_eq!(status.run_mode, DaemonRunMode::OneShot);
    assert_eq!(status.warmed_file_count, 0);
    assert!(
        status
            .last_error
            .as_ref()
            .expect("last error should exist")
            .contains("semantic error")
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_run_daemon_cycle_recovers_after_transient_semantic_error() {
    let root = temp_dir("daemon_cycle_recovery");
    let file = root.join("main.agam");
    fs::write(&file, "fn main(): println(\"hi\")\n").expect("write source");

    let initial_snapshot = agam_pkg::snapshot_workspace(Some(file.clone())).expect("snapshot");
    let workspace = initial_snapshot.session.layout.clone();
    let session_started_unix_ms = now_unix_ms();
    let mut session = DaemonSession::default();

    let first = run_daemon_cycle(
        &mut session,
        &daemon_refresh_snapshot_hint(&workspace),
        &initial_snapshot,
        session_started_unix_ms,
        DaemonRunMode::ForegroundLoop,
        false,
        true,
    )
    .expect("first daemon cycle should succeed");
    let (first_status, first_diff) = match first {
        DaemonCycleOutcome::Success {
            status,
            diff_summary,
            ..
        } => (status, diff_summary),
        DaemonCycleOutcome::Error { error, .. } => {
            panic!("unexpected daemon error on first cycle: {error}")
        }
    };
    assert_eq!(first_status.last_error, None);
    assert_eq!(first_status.warmed_file_count, 1);
    assert_eq!(first_diff.added_files, 1);

    fs::write(&file, "fn main(): missing_name\n").expect("write broken source");
    let second = run_daemon_cycle(
        &mut session,
        &daemon_refresh_snapshot_hint(&workspace),
        &initial_snapshot,
        session_started_unix_ms,
        DaemonRunMode::ForegroundLoop,
        false,
        false,
    )
    .expect("second daemon cycle should return an error status");
    let (second_status, second_error) = match second {
        DaemonCycleOutcome::Error { status, error } => (status, error),
        DaemonCycleOutcome::Success { .. } => {
            panic!("second daemon cycle should have failed");
        }
    };
    assert!(!second_error.is_empty());
    assert_eq!(
        second_status.last_error.as_deref(),
        Some(second_error.as_str())
    );
    assert_eq!(second_status.warmed_file_count, 1);
    assert_eq!(second_status.warmed_version_count, 1);

    fs::write(&file, "fn main(): println(\"recovered\")\n").expect("rewrite fixed source");
    let third = run_daemon_cycle(
        &mut session,
        &daemon_refresh_snapshot_hint(&workspace),
        &initial_snapshot,
        session_started_unix_ms,
        DaemonRunMode::ForegroundLoop,
        false,
        false,
    )
    .expect("third daemon cycle should recover");
    let (third_status, third_diff) = match third {
        DaemonCycleOutcome::Success {
            status,
            diff_summary,
            ..
        } => (status, diff_summary),
        DaemonCycleOutcome::Error { error, .. } => {
            panic!("daemon cycle should have recovered: {error}")
        }
    };
    assert_eq!(third_status.last_error, None);
    assert_eq!(third_status.warmed_file_count, 1);
    assert_eq!(third_diff.changed_files, 1);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_run_daemon_cycle_rewarms_missing_package_artifact() {
    let root = temp_dir("daemon_cycle_missing_prewarm");
    let file = root.join("main.agam");
    fs::write(&file, "fn main() -> i32 { return 0; }\n").expect("write source");

    let initial_snapshot = agam_pkg::snapshot_workspace(Some(file.clone())).expect("snapshot");
    let workspace = initial_snapshot.session.layout.clone();
    let session_started_unix_ms = now_unix_ms();
    let mut session = DaemonSession::default();

    let first = run_daemon_cycle(
        &mut session,
        &daemon_refresh_snapshot_hint(&workspace),
        &initial_snapshot,
        session_started_unix_ms,
        DaemonRunMode::ForegroundLoop,
        false,
        true,
    )
    .expect("first daemon cycle should succeed");
    let first_status = match first {
        DaemonCycleOutcome::Success { status, .. } => status,
        DaemonCycleOutcome::Error { error, .. } => {
            panic!("unexpected daemon error on first cycle: {error}")
        }
    };
    let package_artifact = PathBuf::from(
        first_status
            .prewarm
            .package_artifact_path
            .clone()
            .expect("package artifact path should exist"),
    );
    assert!(package_artifact.is_file());

    fs::remove_file(&package_artifact).expect("remove daemon prewarm package artifact");
    assert!(!package_artifact.exists());

    let second = run_daemon_cycle(
        &mut session,
        &daemon_refresh_snapshot_hint(&workspace),
        &initial_snapshot,
        session_started_unix_ms,
        DaemonRunMode::ForegroundLoop,
        false,
        false,
    )
    .expect("second daemon cycle should succeed");
    let (second_status, second_diff, prewarm_ran) = match second {
        DaemonCycleOutcome::Success {
            status,
            diff_summary,
            prewarm_ran,
        } => (status, diff_summary, prewarm_ran),
        DaemonCycleOutcome::Error { error, .. } => {
            panic!("daemon cycle should have rewarmed missing package artifact: {error}")
        }
    };
    assert!(prewarm_ran);
    assert_eq!(second_diff.changed_files, 0);
    assert!(second_status.prewarm.package_ready);
    let rerwarmed_artifact = PathBuf::from(
        second_status
            .prewarm
            .package_artifact_path
            .expect("package artifact path should be restored"),
    );
    assert!(rerwarmed_artifact.is_file());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_clear_daemon_status_removes_persisted_status_file() {
    let root = temp_dir("daemon_clear_status");
    let file = root.join("main.agam");
    fs::write(&file, "fn main(): println(\"hi\")\n").expect("write source");

    run_daemon_foreground(
        Some(file.clone()),
        true,
        DAEMON_DEFAULT_POLL_MS,
        false,
        false,
    )
    .expect("one-shot daemon run should succeed");
    assert!(daemon_status_path(&root).is_file());

    clear_daemon_status(Some(file), false).expect("clear daemon status should succeed");
    assert!(!daemon_status_path(&root).exists());
    assert!(
        read_daemon_status(&root)
            .expect("read cleared daemon status")
            .is_none()
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_resolve_daemon_workspace_target_allows_missing_source_hint() {
    let root = temp_dir("daemon_missing_source_hint");
    let file = root.join("main.agam");
    fs::write(&file, "fn main(): println(\"hi\")\n").expect("write source");

    let layout =
        resolve_workspace_layout(Some(file.clone())).expect("existing source should resolve");
    fs::remove_file(&file).expect("remove source");

    let daemon_target = resolve_daemon_workspace_target(Some(file))
        .expect("daemon target should resolve from missing source parent");
    assert_eq!(daemon_target.root, layout.root);
    assert_eq!(daemon_target.project_name, layout.project_name);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_resolve_daemon_workspace_target_allows_root_dir_with_status_but_no_entry() {
    let root = temp_dir("daemon_root_status_hint");
    let file = root.join("main.agam");
    fs::write(&file, "fn main(): println(\"hi\")\n").expect("write source");

    run_daemon_foreground(
        Some(file.clone()),
        true,
        DAEMON_DEFAULT_POLL_MS,
        false,
        false,
    )
    .expect("one-shot daemon run should succeed");
    fs::remove_file(&file).expect("remove source");

    let daemon_target = resolve_daemon_workspace_target(Some(root.clone()))
        .expect("daemon target should resolve from root with persisted status");
    assert_eq!(daemon_target.root, root);

    let _ = fs::remove_dir_all(daemon_target.root);
}

#[test]
fn test_resolve_daemon_workspace_target_allows_existing_directory_without_workspace_layout() {
    let root = temp_dir("daemon_existing_dir_hint");

    let daemon_target = resolve_daemon_workspace_target(Some(root.clone()))
        .expect("daemon target should resolve from an existing directory");
    assert_eq!(daemon_target.root, root);

    let _ = fs::remove_dir_all(daemon_target.root);
}

#[test]
fn test_daemon_refresh_snapshot_hint_uses_entry_file_for_single_file_workspace() {
    let root = temp_dir("daemon_refresh_hint_single_file");
    let file = root.join("main.agam");
    fs::write(&file, "fn main(): println(\"hi\")\n").expect("write source");

    let layout =
        resolve_workspace_layout(Some(file.clone())).expect("single-file workspace should resolve");
    assert_eq!(daemon_refresh_snapshot_hint(&layout), file);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_source_call_cache_can_enable_whole_module_and_opt_out_function() {
    let features = parse_source_features(
        r#"
@lang.advance
@lang.feat.call_cache

fn hot(n: i64) -> i64 { return n + 1; }

@lang.feat.no_call_cache
fn cold(n: i64) -> i64 { return n * 2; }
"#,
    );

    assert!(features.call_cache.enable_all);
    assert!(!features.call_cache.disable_all);
    assert!(features.call_cache.exclude_functions.contains("cold"));
    assert!(!features.call_cache.include_functions.contains("cold"));
}

#[test]
fn test_source_call_cache_can_target_specific_function_without_global_enable() {
    let features = parse_source_features(
        r#"
@lang.advance
fn main() -> i32 { if hot(1) > 0 { return 0; } return 1; }

@lang.feat.call_cache
fn hot(n: i64) -> i64 { return n + 1; }
"#,
    );

    assert!(!features.call_cache.enable_all);
    assert!(!features.call_cache.disable_all);
    assert!(features.call_cache.include_functions.contains("hot"));
    assert!(features.call_cache.exclude_functions.is_empty());
}

#[test]
fn test_source_no_call_cache_disables_automatic_service() {
    let features = parse_source_features(
        r#"
@lang.advance
@lang.feat.no_call_cache

fn main() -> i32 { return 0; }
"#,
    );

    assert!(features.call_cache.disable_all);
    assert!(!features.call_cache.enable_all);
    assert!(!features.call_cache.optimize_all);
}

#[test]
fn test_source_call_cache_optimize_marks_experimental_usage() {
    let features = parse_source_features(
        r#"
@lang.advance
@experimental.call_cache.optimize

@experimental.call_cache.optimize
fn hot(n: i64) -> i64 { return n + 1; }
"#,
    );

    assert!(features.call_cache.enable_all);
    assert!(!features.call_cache.disable_all);
    assert!(features.call_cache.optimize_all);
    assert!(features.call_cache.optimize_functions.contains("hot"));
    assert_eq!(features.experimental_usages.len(), 2);
}

#[test]
fn test_persisted_profile_prepromotes_selectable_hot_functions() {
    let profile = agam_profile::PersistentCallCacheProfile {
        schema_version: agam_profile::CALL_CACHE_PROFILE_SCHEMA_VERSION,
        backend: "jit".into(),
        runs: 2,
        total_calls: 64,
        total_hits: 48,
        total_stores: 2,
        functions: vec![agam_profile::PersistentCallCacheFunctionProfile {
            name: "hot".into(),
            runs: 2,
            total_calls: 32,
            total_hits: 24,
            total_stores: 1,
            last_entries: 1,
            profile: agam_profile::CallCacheFunctionProfile {
                unique_keys: 1,
                hottest_key_hits: 32,
                avg_reuse_distance: Some(1),
                max_reuse_distance: Some(1),
                stable_values: vec![agam_profile::StableScalarValueProfile {
                    index: 0,
                    raw_bits: 33,
                    matches: 32,
                }],
                specialization_hint:
                    agam_profile::CallCacheSpecializationHint::StableArgumentsAndHotKey {
                        slots: vec![0],
                        hits: 32,
                        unique_keys: 1,
                    },
                ..Default::default()
            },
        }],
    };

    let (selection, promoted) =
        apply_persisted_optimize_profile(&CallCacheSelection::default(), Some(&profile));

    assert_eq!(promoted, vec!["hot".to_string()]);
    assert!(selection.optimize_functions.contains("hot"));
}

#[test]
fn test_persisted_profile_respects_disabled_automatic_service_and_exclusions() {
    let profile = agam_profile::PersistentCallCacheProfile {
        schema_version: agam_profile::CALL_CACHE_PROFILE_SCHEMA_VERSION,
        backend: "jit".into(),
        runs: 1,
        total_calls: 32,
        total_hits: 24,
        total_stores: 1,
        functions: vec![agam_profile::PersistentCallCacheFunctionProfile {
            name: "hot".into(),
            runs: 1,
            total_calls: 32,
            total_hits: 24,
            total_stores: 1,
            last_entries: 1,
            profile: agam_profile::CallCacheFunctionProfile {
                unique_keys: 1,
                hottest_key_hits: 32,
                avg_reuse_distance: Some(1),
                max_reuse_distance: Some(1),
                stable_values: vec![],
                specialization_hint: agam_profile::CallCacheSpecializationHint::HotKey {
                    hits: 32,
                    unique_keys: 1,
                },
                ..Default::default()
            },
        }],
    };

    let selection = CallCacheSelection {
        disable_all: true,
        exclude_functions: ["hot".to_string()].into_iter().collect(),
        ..Default::default()
    };

    let (selection, promoted) = apply_persisted_optimize_profile(&selection, Some(&profile));

    assert!(promoted.is_empty());
    assert!(selection.optimize_functions.is_empty());
}

#[test]
fn test_persisted_profile_builds_specialization_plans_for_cache_enabled_functions() {
    let profile = agam_profile::PersistentCallCacheProfile {
        schema_version: agam_profile::CALL_CACHE_PROFILE_SCHEMA_VERSION,
        backend: "jit".into(),
        runs: 2,
        total_calls: 64,
        total_hits: 0,
        total_stores: 2,
        functions: vec![agam_profile::PersistentCallCacheFunctionProfile {
            name: "hot".into(),
            runs: 2,
            total_calls: 32,
            total_hits: 0,
            total_stores: 1,
            last_entries: 1,
            profile: agam_profile::CallCacheFunctionProfile {
                unique_keys: 1,
                hottest_key_hits: 32,
                avg_reuse_distance: None,
                max_reuse_distance: None,
                stable_values: vec![
                    agam_profile::StableScalarValueProfile {
                        index: 0,
                        raw_bits: 33,
                        matches: 24,
                    },
                    agam_profile::StableScalarValueProfile {
                        index: 1,
                        raw_bits: 7,
                        matches: 18,
                    },
                ],
                specialization_hint:
                    agam_profile::CallCacheSpecializationHint::StableArgumentsAndHotKey {
                        slots: vec![0, 1],
                        hits: 32,
                        unique_keys: 1,
                    },
                ..Default::default()
            },
        }],
    };

    let (selection, promoted) =
        apply_persisted_optimize_profile(&CallCacheSelection::default(), Some(&profile));
    let plans = apply_persisted_specialization_profile(&selection, Some(&profile));

    assert!(promoted.is_empty());
    assert!(selection.optimize_functions.is_empty());
    assert_eq!(plans.len(), 2);
    assert_eq!(plans[0].name, "hot");
    assert_eq!(plans[0].stable_values.len(), 2);
    assert_eq!(plans[1].stable_values.len(), 1);
    assert_eq!(plans[1].stable_values[0].raw_bits, 33);
}

#[test]
fn test_persisted_profile_builds_specialization_plans_for_explicit_basic_selection() {
    let profile = agam_profile::PersistentCallCacheProfile {
        schema_version: agam_profile::CALL_CACHE_PROFILE_SCHEMA_VERSION,
        backend: "jit".into(),
        runs: 2,
        total_calls: 64,
        total_hits: 0,
        total_stores: 2,
        functions: vec![agam_profile::PersistentCallCacheFunctionProfile {
            name: "hot".into(),
            runs: 2,
            total_calls: 32,
            total_hits: 0,
            total_stores: 1,
            last_entries: 1,
            profile: agam_profile::CallCacheFunctionProfile {
                unique_keys: 1,
                hottest_key_hits: 32,
                avg_reuse_distance: None,
                max_reuse_distance: None,
                stable_values: vec![agam_profile::StableScalarValueProfile {
                    index: 0,
                    raw_bits: 33,
                    matches: 24,
                }],
                specialization_hint:
                    agam_profile::CallCacheSpecializationHint::StableArgumentsAndHotKey {
                        slots: vec![0],
                        hits: 32,
                        unique_keys: 1,
                    },
                ..Default::default()
            },
        }],
    };

    let selection = CallCacheSelection {
        disable_all: true,
        include_functions: ["hot".to_string()].into_iter().collect(),
        ..Default::default()
    };
    let (selection, promoted) = apply_persisted_optimize_profile(&selection, Some(&profile));
    let plans = apply_persisted_specialization_profile(&selection, Some(&profile));

    assert!(promoted.is_empty());
    assert!(selection.optimize_functions.is_empty());
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].name, "hot");
    assert_eq!(plans[0].stable_values[0].raw_bits, 33);
}

#[test]
fn test_persisted_profile_skips_specialization_plans_when_cache_disabled() {
    let profile = agam_profile::PersistentCallCacheProfile {
        schema_version: agam_profile::CALL_CACHE_PROFILE_SCHEMA_VERSION,
        backend: "jit".into(),
        runs: 2,
        total_calls: 64,
        total_hits: 48,
        total_stores: 2,
        functions: vec![agam_profile::PersistentCallCacheFunctionProfile {
            name: "hot".into(),
            runs: 2,
            total_calls: 32,
            total_hits: 24,
            total_stores: 1,
            last_entries: 1,
            profile: agam_profile::CallCacheFunctionProfile {
                unique_keys: 1,
                hottest_key_hits: 32,
                avg_reuse_distance: Some(1),
                max_reuse_distance: Some(1),
                stable_values: vec![agam_profile::StableScalarValueProfile {
                    index: 0,
                    raw_bits: 33,
                    matches: 24,
                }],
                specialization_hint:
                    agam_profile::CallCacheSpecializationHint::StableArgumentsAndHotKey {
                        slots: vec![0],
                        hits: 32,
                        unique_keys: 1,
                    },
                ..Default::default()
            },
        }],
    };

    let selection = CallCacheSelection {
        disable_all: true,
        ..Default::default()
    };
    let plans = apply_persisted_specialization_profile(&selection, Some(&profile));

    assert!(plans.is_empty());
}

#[test]
fn test_parse_llvm_call_cache_run_profile() {
    let profile = parse_llvm_call_cache_run_profile(
            "AGAM_LLVM_CALL_CACHE_PROFILE_V6\nFN\thot\t32\t24\t2\t1\t3\t24\nSP\thot\t12\t4\nSC\thot\t0=33\t12\t0\nSC\thot\t0=33,1=7\t0\t4\nSV\thot\t0\t33\t24\nRD\thot\t1\t3\t24\nFN\twarm\t8\t0\t0\t0\t0\t0\nSP\twarm\t0\t0\nSV\twarm\t0\t7\t0\nRD\twarm\t0\t0\t0\n",
        )
        .expect("profile should parse");

    assert_eq!(profile.backend, "llvm");
    assert_eq!(profile.total_calls, 40);
    assert_eq!(profile.total_hits, 24);
    assert_eq!(profile.total_stores, 2);
    assert_eq!(profile.functions.len(), 2);
    assert_eq!(profile.functions[0].name, "hot");
    assert_eq!(profile.functions[0].entries, 1);
    assert_eq!(profile.functions[0].profile.unique_keys, 3);
    assert_eq!(profile.functions[0].profile.hottest_key_hits, 24);
    assert_eq!(profile.functions[0].profile.stable_values.len(), 1);
    assert_eq!(profile.functions[0].profile.stable_values[0].raw_bits, 33);
    assert_eq!(profile.functions[0].profile.avg_reuse_distance, Some(1));
    assert_eq!(profile.functions[0].profile.max_reuse_distance, Some(3));
    assert_eq!(profile.functions[0].profile.specialization_guard_hits, 12);
    assert_eq!(
        profile.functions[0].profile.specialization_guard_fallbacks,
        4
    );
    assert_eq!(
        profile.functions[0].profile.specialization_profiles.len(),
        2
    );
    assert_eq!(
        profile.functions[0].profile.specialization_profiles[0].stable_values[0].index,
        0
    );
    assert_eq!(
        profile.functions[0].profile.specialization_profiles[0].stable_values[0].raw_bits,
        33
    );
}

#[test]
fn test_parse_llvm_call_cache_run_profile_v4_compatibility() {
    let profile = parse_llvm_call_cache_run_profile(
            "AGAM_LLVM_CALL_CACHE_PROFILE_V4\nFN\thot\t32\t24\t2\t1\nSP\thot\t12\t4\nSV\thot\t0\t33\t24\nRD\thot\t1\t3\t24\n",
        )
        .expect("legacy profile should parse");

    assert_eq!(profile.functions.len(), 1);
    assert_eq!(profile.functions[0].profile.unique_keys, 2);
    assert_eq!(profile.functions[0].profile.hottest_key_hits, 0);
    assert_eq!(profile.functions[0].profile.avg_reuse_distance, Some(1));
}

#[test]
fn test_persisted_profile_skips_unfavorable_specialization_feedback() {
    let profile = agam_profile::PersistentCallCacheProfile {
        schema_version: agam_profile::CALL_CACHE_PROFILE_SCHEMA_VERSION,
        backend: "jit".into(),
        runs: 2,
        total_calls: 64,
        total_hits: 48,
        total_stores: 2,
        functions: vec![agam_profile::PersistentCallCacheFunctionProfile {
            name: "hot".into(),
            runs: 2,
            total_calls: 32,
            total_hits: 24,
            total_stores: 1,
            last_entries: 1,
            profile: agam_profile::CallCacheFunctionProfile {
                unique_keys: 1,
                hottest_key_hits: 32,
                avg_reuse_distance: Some(1),
                max_reuse_distance: Some(1),
                stable_values: vec![agam_profile::StableScalarValueProfile {
                    index: 0,
                    raw_bits: 33,
                    matches: 24,
                }],
                specialization_guard_hits: 1,
                specialization_guard_fallbacks: 15,
                specialization_profiles: Vec::new(),
                specialization_hint:
                    agam_profile::CallCacheSpecializationHint::StableArgumentsAndHotKey {
                        slots: vec![0],
                        hits: 32,
                        unique_keys: 1,
                    },
            },
        }],
    };

    let (selection, _) =
        apply_persisted_optimize_profile(&CallCacheSelection::default(), Some(&profile));
    let plans = apply_persisted_specialization_profile(&selection, Some(&profile));

    assert!(plans.is_empty());
}

#[test]
fn test_persisted_profile_does_not_prepromote_unfavorable_specialization_only_signal() {
    let profile = agam_profile::PersistentCallCacheProfile {
        schema_version: agam_profile::CALL_CACHE_PROFILE_SCHEMA_VERSION,
        backend: "jit".into(),
        runs: 2,
        total_calls: 64,
        total_hits: 6,
        total_stores: 2,
        functions: vec![agam_profile::PersistentCallCacheFunctionProfile {
            name: "thrashy".into(),
            runs: 2,
            total_calls: 32,
            total_hits: 3,
            total_stores: 1,
            last_entries: 8,
            profile: agam_profile::CallCacheFunctionProfile {
                unique_keys: 8,
                hottest_key_hits: 6,
                avg_reuse_distance: None,
                max_reuse_distance: None,
                stable_values: vec![agam_profile::StableScalarValueProfile {
                    index: 0,
                    raw_bits: 33,
                    matches: 12,
                }],
                specialization_guard_hits: 1,
                specialization_guard_fallbacks: 15,
                specialization_profiles: Vec::new(),
                specialization_hint: agam_profile::CallCacheSpecializationHint::StableArguments {
                    slots: vec![0],
                },
            },
        }],
    };

    let (selection, promoted) =
        apply_persisted_optimize_profile(&CallCacheSelection::default(), Some(&profile));

    assert!(promoted.is_empty());
    assert!(!selection.optimize_functions.contains("thrashy"));
}

#[test]
fn test_persisted_profile_retains_specialization_from_favorable_feedback() {
    let profile = agam_profile::PersistentCallCacheProfile {
        schema_version: agam_profile::CALL_CACHE_PROFILE_SCHEMA_VERSION,
        backend: "jit".into(),
        runs: 2,
        total_calls: 64,
        total_hits: 48,
        total_stores: 2,
        functions: vec![agam_profile::PersistentCallCacheFunctionProfile {
            name: "retained".into(),
            runs: 2,
            total_calls: 32,
            total_hits: 24,
            total_stores: 1,
            last_entries: 1,
            profile: agam_profile::CallCacheFunctionProfile {
                unique_keys: 1,
                hottest_key_hits: 24,
                avg_reuse_distance: Some(1),
                max_reuse_distance: Some(1),
                stable_values: vec![
                    agam_profile::StableScalarValueProfile {
                        index: 0,
                        raw_bits: 33,
                        matches: 4,
                    },
                    agam_profile::StableScalarValueProfile {
                        index: 1,
                        raw_bits: 7,
                        matches: 3,
                    },
                ],
                specialization_guard_hits: 12,
                specialization_guard_fallbacks: 4,
                specialization_profiles: Vec::new(),
                specialization_hint: agam_profile::CallCacheSpecializationHint::StableArguments {
                    slots: vec![0, 1],
                },
            },
        }],
    };

    let (selection, promoted) =
        apply_persisted_optimize_profile(&CallCacheSelection::default(), Some(&profile));
    let plans = apply_persisted_specialization_profile(&selection, Some(&profile));

    assert_eq!(promoted, vec!["retained".to_string()]);
    assert_eq!(plans.len(), 2);
    assert_eq!(plans[0].name, "retained");
    assert_eq!(plans[0].stable_values.len(), 2);
    assert_eq!(plans[1].stable_values[0].raw_bits, 33);
}

#[test]
fn test_build_feature_signature_includes_cache_generation() {
    let signature = build_feature_signature(
        Backend::Llvm,
        &CallCacheSelection::default(),
        false,
        &ReleaseTuning::default(),
    );

    assert!(signature.contains("build_cache=compiler-build-v2"));
}

#[test]
fn test_auto_run_backend_falls_back_to_jit_without_external_toolchains() {
    let resolved = resolve_backend_with_toolchains(Backend::Auto, true, false, false, false, false);
    assert_eq!(resolved, Backend::Jit);
}

#[test]
fn test_auto_build_backend_keeps_aot_fallback_without_external_toolchains() {
    let resolved =
        resolve_backend_with_toolchains(Backend::Auto, false, false, false, false, false);
    assert_eq!(resolved, Backend::C);
}

#[test]
fn test_auto_run_backend_ignores_wsl_llvm_without_dev_opt_in() {
    let resolved = resolve_backend_with_toolchains(Backend::Auto, true, false, true, false, false);
    assert_eq!(resolved, Backend::Jit);
}

#[test]
fn test_auto_run_backend_accepts_wsl_llvm_with_dev_opt_in() {
    let resolved = resolve_backend_with_toolchains(Backend::Auto, true, false, true, true, false);
    assert_eq!(resolved, Backend::Llvm);
}

#[test]
fn test_auto_build_backend_does_not_treat_wsl_llvm_as_native_aot_toolchain() {
    let resolved = resolve_backend_with_toolchains(Backend::Auto, false, false, true, true, false);
    assert_eq!(resolved, Backend::C);
}

#[test]
fn test_default_native_binary_output_path_uses_target_platform_extension() {
    let windows = default_native_binary_output_path(
        Path::new("examples/hello.agam"),
        Some("x86_64-pc-windows-msvc"),
    );
    let linux = default_native_binary_output_path(
        Path::new("examples/hello.agam"),
        Some("x86_64-unknown-linux-gnu"),
    );

    assert_eq!(
        windows.file_name().and_then(|name| name.to_str()),
        Some("hello.exe")
    );
    assert_eq!(
        linux.file_name().and_then(|name| name.to_str()),
        Some("hello")
    );
}

#[test]
fn test_default_sdk_distribution_output_dir_uses_host_platform() {
    let output = default_sdk_distribution_output_dir();
    assert_eq!(
        output,
        PathBuf::from("dist").join(current_host_sdk_platform())
    );
}

#[test]
fn test_sdk_supported_targets_begin_with_host_native() {
    let targets = sdk_supported_targets(None, None);
    assert!(!targets.is_empty());
    assert_eq!(targets[0].name, "host-native");
    assert_eq!(
        targets[0].backend,
        agam_runtime::contract::RuntimeBackend::Llvm
    );
}

#[test]
fn test_sdk_supported_targets_record_packaged_android_sysroot() {
    let targets = sdk_supported_targets(None, Some("target-packs/android-arm64/sysroot"));
    let android = targets
        .iter()
        .find(|target| target.target_triple == "aarch64-linux-android21")
        .expect("default SDK target list should include android");
    assert_eq!(
        android.packaged_sysroot.as_deref(),
        Some("target-packs/android-arm64/sysroot")
    );
}

#[test]
fn test_requested_backend_for_command_uses_llvm_when_target_is_selected() {
    let requested =
        requested_backend_for_command(Backend::Auto, None, false, Some("aarch64-linux-android21"));
    assert_eq!(requested, Backend::Llvm);
}

#[test]
fn test_requested_backend_from_environment_ignores_jit_for_build() {
    let environment = environment_report(
        "dev",
        None,
        Some(agam_runtime::contract::RuntimeBackend::Jit),
    );
    assert_eq!(
        requested_backend_from_environment(&environment.environment, false),
        None
    );
    assert_eq!(
        requested_backend_from_environment(&environment.environment, true),
        Some(Backend::Jit)
    );
}

#[test]
fn test_sdk_supported_targets_include_selected_environment_target() {
    let environment = environment_report(
        "release-linux",
        Some("x86_64-unknown-linux-musl"),
        Some(agam_runtime::contract::RuntimeBackend::Llvm),
    );

    let targets = sdk_supported_targets(Some(&environment), None);
    assert!(targets.iter().any(|target| {
        target.name == "release-linux"
            && target.target_triple == "x86_64-unknown-linux-musl"
            && target.backend == agam_runtime::contract::RuntimeBackend::Llvm
    }));
}

#[test]
fn test_package_sdk_distribution_records_selected_environment_metadata() {
    let root = temp_dir("sdk_env_metadata");
    let output = root.join("dist");
    let environment = environment_report(
        "release-linux",
        Some("x86_64-unknown-linux-musl"),
        Some(agam_runtime::contract::RuntimeBackend::Llvm),
    );

    let outcome = package_sdk_distribution(&output, None, None, Some(&environment), false)
        .expect("package sdk should succeed");
    let manifest = agam_pkg::read_sdk_distribution_manifest_from_path(&outcome.manifest_path)
        .expect("read sdk manifest");

    assert!(manifest.notes.iter().any(|note| {
        note.contains("selected environment `release-linux`")
            && note.contains("target `x86_64-unknown-linux-musl`")
            && note.contains("backend `llvm`")
    }));
    assert!(manifest.supported_targets.iter().any(|target| {
        target.name == "release-linux"
            && target.target_triple == "x86_64-unknown-linux-musl"
            && target.backend == agam_runtime::contract::RuntimeBackend::Llvm
    }));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_package_sdk_distribution_stages_android_target_pack() {
    let root = temp_dir("sdk_android_target_pack");
    let output = root.join("dist");
    let sysroot = root.join("android-sysroot");
    fs::create_dir_all(sysroot.join("usr").join("include"))
        .expect("create synthetic android sysroot");

    let outcome = package_sdk_distribution(&output, None, Some(&sysroot), None, false)
        .expect("package sdk should accept an explicit android sysroot");
    let manifest = agam_pkg::read_sdk_distribution_manifest_from_path(&outcome.manifest_path)
        .expect("read sdk manifest");
    let expected_sysroot = output
        .join("target-packs")
        .join("android-arm64")
        .join("sysroot");
    let android = manifest
        .supported_targets
        .iter()
        .find(|target| target.target_triple == "aarch64-linux-android21")
        .expect("manifest should include android target support");
    assert_eq!(
        android.packaged_sysroot.as_deref(),
        Some("target-packs/android-arm64/sysroot")
    );
    assert!(
        expected_sysroot.join("usr").is_dir(),
        "staged SDK should include the Android sysroot target pack"
    );
    assert_eq!(
        outcome.android_sysroot_root.as_deref(),
        Some(expected_sysroot.as_path())
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_sanitize_headless_filename_keeps_single_file_name() {
    assert_eq!(
        sanitize_headless_filename("../tmp\\demo script"),
        "demo_script.agam"
    );
    assert_eq!(sanitize_headless_filename("session.agam"), "session.agam");
}

#[test]
fn test_repl_execution_cache_reuses_warm_state_for_unchanged_source() {
    let request = HeadlessExecutionRequest {
        source: "fn main() -> i32 { return 0; }\n".into(),
        filename: "demo.agam".into(),
        backend: HeadlessExecutionBackend::Jit,
        ..HeadlessExecutionRequest::default()
    };
    let mut cache =
        ReplExecutionCache::new(&request.filename).expect("repl execution cache should initialize");

    let first_ptr = {
        cache
            .materialize_request(&request)
            .expect("request materialization should succeed");
        let warm = cache
            .ensure_materialized_warm_state(false)
            .expect("first warm state build should succeed");
        warm as *const WarmState as usize
    };
    let second_ptr = {
        cache
            .materialize_request(&request)
            .expect("second request materialization should succeed");
        let warm = cache
            .ensure_materialized_warm_state(false)
            .expect("second warm state lookup should succeed");
        warm as *const WarmState as usize
    };

    assert_eq!(
        first_ptr, second_ptr,
        "unchanged REPL buffers should reuse the cached warm state"
    );
}

#[test]
fn test_repl_execution_cache_invalidates_warm_state_when_source_changes() {
    let mut request = HeadlessExecutionRequest {
        source: "fn main() -> i32 { return 0; }\n".into(),
        filename: "demo.agam".into(),
        backend: HeadlessExecutionBackend::Jit,
        ..HeadlessExecutionRequest::default()
    };
    let mut cache =
        ReplExecutionCache::new(&request.filename).expect("repl execution cache should initialize");

    cache
        .materialize_request(&request)
        .expect("first request materialization should succeed");
    let first_hash = cache
        .source_hash
        .clone()
        .expect("materialized request should record a source hash");
    cache
        .ensure_materialized_warm_state(false)
        .expect("first warm state build should succeed");
    assert!(
        cache
            .daemon_session
            .cache
            .get(cache.source_path())
            .expect("daemon cache for REPL entry")
            .contains_key(&first_hash)
    );

    request.source = "fn main() -> i32 { return 1; }\n".into();

    cache
        .materialize_request(&request)
        .expect("changed request materialization should succeed");
    let second_hash = cache
        .source_hash
        .clone()
        .expect("updated request should record a source hash");

    assert_ne!(
        first_hash, second_hash,
        "changed REPL buffers should update the cached source hash"
    );
    assert_eq!(
        std::fs::read_to_string(cache.source_path()).expect("read materialized REPL source"),
        request.source
    );
    cache
        .ensure_materialized_warm_state(false)
        .expect("changed warm state build should succeed");
    let versions = cache
        .daemon_session
        .cache
        .get(cache.source_path())
        .expect("daemon cache for changed REPL entry");
    assert!(
        versions.contains_key(&second_hash),
        "changed REPL buffers should warm the new source hash"
    );
    assert!(
        !versions.contains_key(&first_hash),
        "changed REPL buffers should invalidate the previous daemon warm state version"
    );
}

#[test]
fn test_repl_execution_cache_updates_manifest_when_filename_changes() {
    let mut cache =
        ReplExecutionCache::new("demo.agam").expect("repl execution cache should initialize");
    let previous_source_path = cache.source_path().clone();

    let request = HeadlessExecutionRequest {
        source: "fn main() -> i32 { return 0; }\n".into(),
        filename: "renamed.agam".into(),
        backend: HeadlessExecutionBackend::Jit,
        ..HeadlessExecutionRequest::default()
    };

    cache
        .materialize_request(&request)
        .expect("renamed request materialization should succeed");

    let manifest = agam_pkg::read_workspace_manifest_from_path(&cache.manifest_path)
        .expect("read REPL workspace manifest");
    assert_eq!(
        manifest.project.entry.as_deref(),
        Some("src/renamed.agam"),
        "REPL manifest should track the current buffer filename"
    );
    assert_eq!(
        cache.source_path(),
        &cache.root.join("src").join("renamed.agam")
    );
    assert!(
        !previous_source_path.exists(),
        "renaming the REPL buffer should remove the stale source path"
    );
}

#[test]
fn test_execute_repl_request_runs_in_process() {
    let request = HeadlessExecutionRequest {
        source: "fn main() -> i32 { return 0; }\n".into(),
        filename: "demo.agam".into(),
        backend: HeadlessExecutionBackend::Jit,
        ..HeadlessExecutionRequest::default()
    };
    let mut cache =
        ReplExecutionCache::new(&request.filename).expect("repl execution cache should initialize");

    let exit_code =
        execute_repl_request(&request, &mut cache, false).expect("REPL request should run");
    assert_eq!(exit_code, 0);
}

#[test]
fn test_execute_headless_request_runs_jit_in_process_and_captures_stdout() {
    let request = HeadlessExecutionRequest {
        source: "fn main(): println(\"hi\")\n".into(),
        filename: "snippet.agam".into(),
        backend: HeadlessExecutionBackend::Jit,
        ..HeadlessExecutionRequest::default()
    };

    let response = execute_headless_request(&request, false);
    assert!(
        response.success,
        "expected successful headless response: {response:?}"
    );
    assert_eq!(response.exit_code, Some(0));
    assert_eq!(response.stdout, "hi\n");
    assert!(response.stderr.is_empty());
    assert!(response.error.is_none());
}

#[test]
fn test_execute_headless_request_buffers_jit_parse_errors_into_stderr() {
    let request = HeadlessExecutionRequest {
        source: "fn main(".into(),
        filename: "broken.agam".into(),
        backend: HeadlessExecutionBackend::Jit,
        ..HeadlessExecutionRequest::default()
    };

    let response = execute_headless_request(&request, false);
    assert!(!response.success);
    assert!(response.exit_code.is_none());
    assert!(response.stdout.is_empty());
    assert!(
        response.stderr.contains("error"),
        "expected rendered parse diagnostics in stderr: {response:?}"
    );
    assert!(response.error.is_some());
}

#[test]
fn test_execute_headless_request_runs_available_non_jit_backend_in_process() {
    let backend = if resolve_llvm_run_toolchain().is_some() {
        HeadlessExecutionBackend::Llvm
    } else if command_exists(default_c_compiler()) {
        HeadlessExecutionBackend::C
    } else {
        return;
    };
    let request = HeadlessExecutionRequest {
        source: "fn main() -> i32 { return 0; }\n".into(),
        filename: "native.agam".into(),
        backend,
        policy: HeadlessExecutionPolicy {
            allow_native_backends: true,
            ..HeadlessExecutionPolicy::default()
        },
        ..HeadlessExecutionRequest::default()
    };

    let response = execute_headless_request(&request, false);
    assert!(
        response.success,
        "expected successful non-JIT headless response: {response:?}"
    );
    assert_eq!(response.exit_code, Some(0));
    assert!(response.stdout.is_empty());
    assert!(response.stderr.is_empty());
    assert!(response.error.is_none());
}

#[test]
fn test_build_exec_request_from_inline_source_uses_cli_options() {
    let request = build_exec_request(
        None,
        Some("fn main() -> i32 { return 0; }\n".into()),
        Some("agent.agam".into()),
        Backend::Llvm,
        3,
        true,
        vec!["hello".into(), "world".into()],
        "process".into(),
        false,
        false,
    )
    .expect("inline exec request should build");

    assert_eq!(request.filename, "agent.agam");
    assert_eq!(request.source, "fn main() -> i32 { return 0; }\n");
    assert_eq!(request.backend, HeadlessExecutionBackend::Llvm);
    assert_eq!(request.opt_level, 3);
    assert!(request.fast);
    assert_eq!(request.args, vec!["hello".to_string(), "world".to_string()]);
    assert!(request.policy.allow_native_backends);
}

#[test]
fn test_build_exec_request_from_file_reads_source_and_defaults_filename() {
    let root = temp_dir("exec_request_file");
    let file = root.join("demo.agam");
    fs::write(&file, "fn main() -> i32 { return 0; }\n").expect("write source file");

    let request = build_exec_request(
        Some(file.clone()),
        None,
        None,
        Backend::Jit,
        2,
        false,
        Vec::new(),
        "process".into(),
        false,
        false,
    )
    .expect("file exec request should build");

    assert_eq!(request.filename, "demo.agam");
    assert_eq!(request.source, "fn main() -> i32 { return 0; }\n");
    assert_eq!(request.backend, HeadlessExecutionBackend::Jit);
    assert!(!request.policy.allow_native_backends);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_normalize_headless_request_rejects_source_over_policy_limit() {
    let request = HeadlessExecutionRequest {
        source: "fn main() -> i32 { return 0; }\n".into(),
        policy: HeadlessExecutionPolicy {
            max_source_bytes: 8,
            ..HeadlessExecutionPolicy::default()
        },
        ..HeadlessExecutionRequest::default()
    };

    let error =
        normalize_headless_request(&request).expect_err("oversized source should be rejected");
    assert!(error.contains("exceeding the policy limit"));
}

#[test]
fn test_normalize_headless_request_rejects_too_many_args() {
    let request = HeadlessExecutionRequest {
        source: "fn main() -> i32 { return 0; }\n".into(),
        args: vec!["alpha".into(), "beta".into()],
        policy: HeadlessExecutionPolicy {
            max_arg_count: 1,
            ..HeadlessExecutionPolicy::default()
        },
        ..HeadlessExecutionRequest::default()
    };

    let error = normalize_headless_request(&request)
        .expect_err("requests exceeding the arg-count policy should be rejected");
    assert!(error.contains("exceeding the policy limit"));
}

#[test]
fn test_normalize_headless_request_rejects_native_backend_without_policy_opt_in() {
    let request = HeadlessExecutionRequest {
        source: "fn main() -> i32 { return 0; }\n".into(),
        backend: HeadlessExecutionBackend::Llvm,
        ..HeadlessExecutionRequest::default()
    };

    let error = normalize_headless_request(&request)
        .expect_err("native backend should require explicit policy opt-in");
    assert!(error.contains("policy.allow_native_backends=true"));
}

#[test]
fn test_normalize_headless_request_rejects_zero_runtime_limit() {
    let request = HeadlessExecutionRequest {
        source: "fn main() -> i32 { return 0; }\n".into(),
        policy: HeadlessExecutionPolicy {
            max_runtime_ms: 0,
            ..HeadlessExecutionPolicy::default()
        },
        ..HeadlessExecutionRequest::default()
    };

    let error =
        normalize_headless_request(&request).expect_err("zero runtime limit should be rejected");
    assert!(error.contains("max_runtime_ms"));
}

#[test]
fn test_normalize_headless_request_rejects_zero_memory_limit() {
    let request = HeadlessExecutionRequest {
        source: "fn main() -> i32 { return 0; }\n".into(),
        policy: HeadlessExecutionPolicy {
            max_memory_bytes: 0,
            ..HeadlessExecutionPolicy::default()
        },
        ..HeadlessExecutionRequest::default()
    };

    let error =
        normalize_headless_request(&request).expect_err("zero memory limit should be rejected");
    assert!(error.contains("max_memory_bytes"));
}

#[test]
fn test_cli_parses_exec_command_with_inline_source() {
    let cli = Cli::try_parse_from([
        "agamc",
        "exec",
        "--source",
        "fn main() -> i32 { return 0; }",
        "--backend",
        "jit",
        "--arg",
        "alpha",
    ])
    .expect("exec command should parse");

    match cli.command {
        Command::Exec {
            json,
            pretty,
            source,
            backend,
            args,
            ..
        } => {
            assert!(!json);
            assert!(!pretty);
            assert_eq!(source.as_deref(), Some("fn main() -> i32 { return 0; }"));
            assert_eq!(backend, Backend::Jit);
            assert_eq!(args, vec!["alpha".to_string()]);
        }
        other => panic!("expected exec command, got {other:?}"),
    }
}

#[test]
fn test_parse_repl_command_understands_backend_and_fast_commands() {
    assert_eq!(
        parse_repl_command(":backend llvm").expect("parse backend"),
        Some(ReplCommandKind::Backend(HeadlessExecutionBackend::Llvm))
    );
    assert_eq!(
        parse_repl_command(":fast on").expect("parse fast"),
        Some(ReplCommandKind::Fast(true))
    );
    assert_eq!(
        parse_repl_command(":opt 2").expect("parse opt"),
        Some(ReplCommandKind::Opt(2))
    );
}

#[test]
fn test_parse_repl_command_rejects_unknown_commands() {
    let error = parse_repl_command(":wat").expect_err("unknown repl commands should fail");
    assert!(error.contains("unknown repl command"));
}

#[test]
fn test_optional_workspace_environment_allows_missing_path_without_env() {
    let resolved = maybe_resolve_optional_workspace_environment(None, None)
        .expect("missing workspace path should be allowed when no env was requested");
    assert!(resolved.is_none());
}

#[test]
fn test_optional_workspace_environment_allows_non_workspace_path_without_env() {
    let root = temp_dir("sdk_optional_env_free");
    let resolved = maybe_resolve_optional_workspace_environment(Some(root.clone()), None)
        .expect("non-workspace path should be allowed when no env was requested");
    assert!(resolved.is_none());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_optional_workspace_environment_requires_workspace_when_env_is_requested() {
    let error = maybe_resolve_optional_workspace_environment(None, Some("release"))
        .expect_err("selecting an environment should require a workspace");
    assert!(error.contains("`--env` requires a workspace"));
}

#[test]
fn test_visual_studio_llvm_candidate_paths_include_expected_clang_locations() {
    let candidates = visual_studio_llvm_candidate_paths(Path::new("C:/VS"));

    assert_eq!(
        candidates[0],
        PathBuf::from("C:/VS/VC/Tools/Llvm/x64/bin/clang.exe")
    );
    assert_eq!(
        candidates[1],
        PathBuf::from("C:/VS/VC/Tools/Llvm/bin/clang.exe")
    );
}

#[test]
fn test_standalone_windows_llvm_candidate_paths_include_program_files_layout() {
    let candidates = standalone_windows_llvm_candidate_paths(Path::new("C:/Program Files/LLVM"));

    let primary = if cfg!(windows) {
        PathBuf::from("C:/Program Files/LLVM/bin/clang.exe")
    } else {
        PathBuf::from("C:/Program Files/LLVM/bin/clang")
    };
    let secondary = if cfg!(windows) {
        PathBuf::from("C:/Program Files/LLVM/bin/clang++.exe")
    } else {
        PathBuf::from("C:/Program Files/LLVM/bin/clang++")
    };

    assert!(candidates.iter().any(|candidate| candidate == &primary));
    assert!(candidates.iter().any(|candidate| candidate == &secondary));
}

#[test]
fn test_bundled_llvm_candidate_paths_include_packaged_toolchain_layout() {
    let root = Path::new("C:/agam");
    let candidates = bundled_llvm_candidate_paths(root);
    let expected = root
        .join("toolchains")
        .join("llvm")
        .join(bundled_llvm_platform_dir())
        .join("bin")
        .join(if cfg!(windows) { "clang.exe" } else { "clang" });

    assert!(candidates.iter().any(|candidate| candidate == &expected));
}

#[test]
fn test_stage_llvm_bundle_into_sdk_accepts_bundle_root_layout() {
    let unique = format!(
        "agam_sdk_test_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time should be monotonic enough")
            .as_nanos()
    );
    let temp_root = std::env::temp_dir().join(unique);
    let bundle_root = temp_root.join("bundle");
    let output_root = temp_root.join("out");
    let driver = bundle_root
        .join(bundled_llvm_platform_dir())
        .join("bin")
        .join(if cfg!(windows) { "clang.exe" } else { "clang" });
    std::fs::create_dir_all(driver.parent().expect("driver should have parent"))
        .expect("create bundle layout");
    std::fs::write(&driver, b"clang").expect("write fake driver");

    let staged = stage_llvm_bundle_into_sdk(&bundle_root, &output_root)
        .expect("bundle root layout should stage");

    assert_eq!(staged, output_root.join("toolchains").join("llvm"));
    assert!(
        staged
            .join(bundled_llvm_platform_dir())
            .join("bin")
            .join(if cfg!(windows) { "clang.exe" } else { "clang" })
            .is_file()
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn test_bundled_llvm_candidate_paths_support_bundle_root_override_layout() {
    let root = Path::new("C:/agam/toolchains/llvm");
    let candidates = bundled_llvm_candidate_paths(root);
    let expected = root
        .join(bundled_llvm_platform_dir())
        .join("bin")
        .join(if cfg!(windows) { "clang.exe" } else { "clang" });

    assert!(candidates.iter().any(|candidate| candidate == &expected));
}

#[test]
fn test_native_llvm_clang_args_include_cross_target_and_sysroot() {
    let tuning = ReleaseTuning {
        target: Some("aarch64-linux-android21".into()),
        native_cpu: false,
        lto: Some(LtoMode::Thin),
        pgo_generate: None,
        pgo_use: None,
    };
    let target_config = LlvmTargetConfig {
        target_triple: tuning.target.clone(),
        platform: LlvmTargetPlatform::Android,
        sysroot: Some(PathBuf::from("/ndk/sysroot")),
        sdk_root: None,
    };

    let args = build_native_llvm_clang_args(
        Path::new("hello.ll"),
        Path::new("hello"),
        3,
        &tuning,
        &target_config,
    );

    assert!(
        args.iter()
            .any(|arg| arg == "--target=aarch64-linux-android21")
    );
    assert!(args.iter().any(|arg| arg == "--sysroot=/ndk/sysroot"));
    assert!(args.iter().any(|arg| arg == "-flto=thin"));
    assert!(args.iter().any(|arg| arg == "-lm"));
}

#[test]
fn test_native_llvm_clang_args_omit_math_library_on_windows() {
    let tuning = ReleaseTuning {
        target: Some("x86_64-pc-windows-msvc".into()),
        native_cpu: false,
        lto: None,
        pgo_generate: None,
        pgo_use: None,
    };
    let target_config = LlvmTargetConfig {
        target_triple: tuning.target.clone(),
        platform: LlvmTargetPlatform::Windows,
        sysroot: None,
        sdk_root: None,
    };

    let args = build_native_llvm_clang_args(
        Path::new("hello.ll"),
        Path::new("hello.exe"),
        2,
        &tuning,
        &target_config,
    );

    assert!(!args.iter().any(|arg| arg == "-lm"));
}

#[test]
fn test_validate_release_tuning_rejects_target_for_non_llvm_backend() {
    let tuning = ReleaseTuning {
        target: Some("x86_64-unknown-linux-gnu".into()),
        native_cpu: false,
        lto: None,
        pgo_generate: None,
        pgo_use: None,
    };

    let error =
        validate_release_tuning(Backend::C, &tuning).expect_err("target should require llvm");
    assert!(error.contains("--target"));
}

#[test]
fn test_validate_release_tuning_rejects_native_cpu_for_cross_target() {
    let tuning = ReleaseTuning {
        target: Some("x86_64-unknown-linux-gnu".into()),
        native_cpu: true,
        lto: None,
        pgo_generate: None,
        pgo_use: None,
    };

    let error = validate_release_tuning(Backend::Llvm, &tuning)
        .expect_err("cross target should reject native cpu");
    assert!(error.contains("--fast"));
}

#[test]
fn test_publish_workspace_to_registry_writes_local_index_entry() {
    let root = temp_dir("publish_workspace");
    let workspace = root.join("workspace");
    let src = workspace.join("src");
    let entry = src.join("main.agam");
    fs::create_dir_all(&src).expect("create source directory");
    fs::write(&entry, render_project_entry("publish-demo")).expect("write entry source");

    let mut manifest = agam_pkg::scaffold_workspace_manifest("publish-demo");
    manifest.project.keywords = vec!["math".into(), "ml".into()];
    manifest.dependencies.insert(
        "core".into(),
        agam_pkg::DependencySpec {
            version: Some("^1.0".into()),
            optional: true,
            features: vec!["simd".into()],
            ..agam_pkg::DependencySpec::default()
        },
    );
    agam_pkg::write_workspace_manifest_to_path(
        &agam_pkg::default_manifest_path(&workspace),
        &manifest,
    )
    .expect("write manifest");

    let index_root = root.join("registry-index");
    let report = publish_workspace_to_registry(
        Some(workspace.clone()),
        &index_root,
        &["alice".into(), " bob ".into(), "alice".into()],
        Some(&"Sample package".to_string()),
        Some(&"https://example.com/publish-demo".to_string()),
        Some(&"https://github.com/agam-lang/publish-demo".to_string()),
        Some(&"https://cdn.example.com/publish-demo-0.1.0.agam-src.tar.gz".to_string()),
        false,
        false,
        false,
    )
    .expect("publish should succeed");

    assert!(!report.dry_run);
    assert!(!report.official);
    assert!(report.bootstrapped_config);
    assert_eq!(report.owners, vec!["alice".to_string(), "bob".to_string()]);
    assert!(report.receipt.is_some());
    assert!(index_root.join("config.json").is_file());

    let config = agam_pkg::read_registry_config(&index_root).expect("read registry config");
    assert_eq!(
        config.format_version,
        agam_pkg::REGISTRY_INDEX_FORMAT_VERSION
    );

    let entry =
        agam_pkg::read_registry_package_entry(&index_root, "publish-demo").expect("read entry");
    assert_eq!(entry.owners, vec!["alice".to_string(), "bob".to_string()]);
    assert_eq!(entry.description.as_deref(), Some("Sample package"));
    assert_eq!(
        entry.homepage.as_deref(),
        Some("https://example.com/publish-demo")
    );
    assert_eq!(
        entry.repository.as_deref(),
        Some("https://github.com/agam-lang/publish-demo")
    );
    assert_eq!(entry.keywords, vec!["math".to_string(), "ml".to_string()]);
    assert_eq!(entry.releases.len(), 1);
    assert_eq!(entry.releases[0].dependencies.len(), 1);
    assert_eq!(entry.releases[0].dependencies[0].name, "core");
    assert_eq!(entry.releases[0].dependencies[0].version_req, "^1.0");
    assert!(entry.releases[0].dependencies[0].optional);
    assert_eq!(
        entry.releases[0].dependencies[0].features,
        vec!["simd".to_string()]
    );
    assert_eq!(
        entry.releases[0].download_url.as_deref(),
        Some("https://cdn.example.com/publish-demo-0.1.0.agam-src.tar.gz")
    );
    let provenance = entry.releases[0]
        .provenance
        .as_ref()
        .expect("publish should record provenance");
    assert_eq!(provenance.published_by.as_deref(), Some("alice"));
    assert_eq!(
        provenance.source_repository.as_deref(),
        Some("https://github.com/agam-lang/publish-demo")
    );
    assert_eq!(provenance.source_checksum, report.manifest.checksum);
    assert_eq!(
        provenance.manifest_checksum,
        report.manifest.manifest_checksum
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_publish_workspace_to_registry_dry_run_keeps_index_clean() {
    let root = temp_dir("publish_dry_run");
    let workspace = root.join("workspace");
    let src = workspace.join("src");
    let entry = src.join("main.agam");
    fs::create_dir_all(&src).expect("create source directory");
    fs::write(&entry, render_project_entry("dry-run-demo")).expect("write entry source");
    agam_pkg::write_workspace_manifest_to_path(
        &agam_pkg::default_manifest_path(&workspace),
        &agam_pkg::scaffold_workspace_manifest("dry-run-demo"),
    )
    .expect("write manifest");

    let index_root = root.join("registry-index");
    let report = publish_workspace_to_registry(
        Some(workspace.clone()),
        &index_root,
        &[],
        None,
        None,
        None,
        None,
        false,
        true,
        false,
    )
    .expect("dry run should succeed");

    assert!(report.dry_run);
    assert!(report.receipt.is_none());
    assert!(!report.bootstrapped_config);
    assert_eq!(
        report.index_path,
        agam_pkg::registry_index_path(&report.manifest.name)
    );
    assert!(!index_root.exists());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_publish_workspace_to_registry_supports_official_packages() {
    let root = temp_dir("publish_official_workspace");
    let workspace = root.join("workspace");
    let src = workspace.join("src");
    let entry = src.join("main.agam");
    fs::create_dir_all(&src).expect("create source directory");
    fs::write(&entry, render_project_entry("official-demo")).expect("write entry source");

    let mut manifest = agam_pkg::scaffold_workspace_manifest("official-demo");
    manifest.project.name = "agam-std".into();
    agam_pkg::write_workspace_manifest_to_path(
        &agam_pkg::default_manifest_path(&workspace),
        &manifest,
    )
    .expect("write manifest");

    let index_root = root.join("registry-index");
    agam_pkg::write_registry_config(
        &index_root,
        &agam_pkg::RegistryConfig {
            format_version: agam_pkg::REGISTRY_INDEX_FORMAT_VERSION,
            api_url: None,
            download_url: None,
            name: Some("agam".into()),
        },
    )
    .expect("write registry config");

    let report = publish_workspace_to_registry(
        Some(workspace.clone()),
        &index_root,
        &["agam-lang".into()],
        None,
        None,
        Some(&"https://github.com/agam-lang/agam-std".to_string()),
        None,
        true,
        false,
        false,
    )
    .expect("official publish should succeed");

    assert!(report.official);
    let entry = agam_pkg::read_registry_package_entry(&index_root, "agam-std").expect("read entry");
    assert_eq!(entry.owners, vec!["agam-lang".to_string()]);
    assert_eq!(entry.releases.len(), 1);
    assert_eq!(entry.releases[0].version, manifest.project.version);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_inspect_registry_package_reads_entry_metadata() {
    let root = temp_dir("registry_inspect");
    let index_root = root.join("registry-index");
    agam_pkg::write_registry_config(
        &index_root,
        &agam_pkg::RegistryConfig {
            format_version: agam_pkg::REGISTRY_INDEX_FORMAT_VERSION,
            api_url: None,
            download_url: None,
            name: Some("agam".into()),
        },
    )
    .expect("write registry config");

    agam_pkg::publish_to_registry_index(
        &index_root,
        &agam_pkg::PublishManifest {
            name: "json".into(),
            version: "1.2.0".into(),
            agam_version: "0.1".into(),
            checksum: "sha256-json".into(),
            manifest_checksum: "manifest-json".into(),
            description: Some("JSON support".into()),
            keywords: vec!["json".into(), "parser".into()],
            homepage: Some("https://example.com/json".into()),
            repository: Some("https://github.com/agam-lang/json".into()),
            download_url: None,
            dependencies: vec![agam_pkg::RegistryReleaseDependency {
                name: "core".into(),
                version_req: "^1.0".into(),
                registry: None,
                optional: false,
                features: vec![],
            }],
            features: vec!["simd".into()],
        },
        &["alice".into()],
        "2026-04-10T12:00:00Z",
    )
    .expect("publish package");

    let report = inspect_registry_package(&index_root, "json").expect("inspect package");
    assert_eq!(report.index_name, "agam");
    assert_eq!(report.index_path, "js/on/json");
    assert_eq!(report.entry.name, "json");
    assert_eq!(report.entry.owners, vec!["alice".to_string()]);
    assert_eq!(report.entry.description.as_deref(), Some("JSON support"));
    assert_eq!(
        report.entry.repository.as_deref(),
        Some("https://github.com/agam-lang/json")
    );
    assert_eq!(report.entry.releases.len(), 1);
    assert_eq!(report.entry.releases[0].version, "1.2.0");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_audit_registry_index_package_reports_release_history() {
    let root = temp_dir("registry_audit");
    let index_root = root.join("registry-index");
    agam_pkg::write_registry_config(
        &index_root,
        &agam_pkg::RegistryConfig {
            format_version: agam_pkg::REGISTRY_INDEX_FORMAT_VERSION,
            api_url: None,
            download_url: None,
            name: Some("agam".into()),
        },
    )
    .expect("write registry config");

    agam_pkg::append_release_to_index(
        &index_root,
        "tensor",
        &agam_pkg::RegistryRelease {
            version: "0.3.0".into(),
            checksum: "sha256-tensor".into(),
            agam_version: "0.1".into(),
            dependencies: vec![agam_pkg::RegistryReleaseDependency {
                name: "core".into(),
                version_req: "^1.0".into(),
                registry: None,
                optional: false,
                features: vec!["simd".into()],
            }],
            features: vec!["cuda".into()],
            download_url: None,
            provenance: None,
            published_at: "2026-04-10T12:30:00Z".into(),
            yanked: false,
        },
    )
    .expect("append release");

    let report =
        audit_registry_index_package(&index_root, "tensor").expect("audit package history");
    assert_eq!(report.index_name, "agam");
    assert_eq!(report.index_path, "te/ns/tensor");
    assert!(
        report
            .lines
            .iter()
            .any(|line| line.contains("package: tensor"))
    );
    assert!(report.lines.iter().any(|line| line.contains("releases: 1")));
    assert!(
        report
            .lines
            .iter()
            .any(|line| line.contains("sha256-tensor"))
    );
    assert!(
        report
            .lines
            .iter()
            .any(|line| line.contains("dep: core ^1.0"))
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_install_registry_dependency_pins_latest_release_and_refreshes_lockfile() {
    let root = temp_dir("registry_install");
    let workspace = root.join("workspace");
    let src = workspace.join("src");
    let entry = src.join("main.agam");
    fs::create_dir_all(&src).expect("create source directory");
    fs::write(&entry, render_project_entry("install-demo")).expect("write entry source");
    agam_pkg::write_workspace_manifest_to_path(
        &agam_pkg::default_manifest_path(&workspace),
        &agam_pkg::scaffold_workspace_manifest("install-demo"),
    )
    .expect("write manifest");

    let index_root = root.join("mirror-index");
    agam_pkg::write_registry_config(
        &index_root,
        &agam_pkg::RegistryConfig {
            format_version: agam_pkg::REGISTRY_INDEX_FORMAT_VERSION,
            api_url: None,
            download_url: None,
            name: Some("mirror".into()),
        },
    )
    .expect("write registry config");
    agam_pkg::append_release_to_index(
        &index_root,
        "json",
        &agam_pkg::RegistryRelease {
            version: "1.0.0".into(),
            checksum: "sha256-json-100".into(),
            agam_version: "0.1".into(),
            dependencies: vec![],
            features: vec![],
            download_url: None,
            provenance: None,
            published_at: "2026-04-10T10:00:00Z".into(),
            yanked: false,
        },
    )
    .expect("append 1.0.0");
    agam_pkg::append_release_to_index(
        &index_root,
        "json",
        &agam_pkg::RegistryRelease {
            version: "1.2.0".into(),
            checksum: "sha256-json-120".into(),
            agam_version: "0.1".into(),
            dependencies: vec![],
            features: vec![],
            download_url: None,
            provenance: None,
            published_at: "2026-04-10T11:00:00Z".into(),
            yanked: false,
        },
    )
    .expect("append 1.2.0");

    let report = install_registry_dependency(
        Some(workspace.clone()),
        &index_root,
        DependencyTable::Main,
        "json",
        None,
        false,
    )
    .expect("install dependency");

    assert_eq!(report.index_name, "mirror");
    assert_eq!(report.selected_version, "1.2.0");
    assert!(report.added_new_entry);
    assert!(report.changed_manifest);

    let manifest = agam_pkg::read_workspace_manifest_from_path(&workspace.join("agam.toml"))
        .expect("read updated manifest");
    let spec = manifest.dependencies.get("json").expect("json dependency");
    assert_eq!(spec.version.as_deref(), Some("1.2.0"));
    assert_eq!(spec.registry.as_deref(), Some("mirror"));

    let lockfile = agam_pkg::read_lockfile_from_path(&workspace.join("agam.lock"))
        .expect("read refreshed lockfile");
    assert_eq!(lockfile.packages.len(), 1);
    assert_eq!(lockfile.packages[0].name, "json");
    assert_eq!(lockfile.packages[0].version, "1.2.0");
    assert_eq!(lockfile.packages[0].content_hash, "sha256-json-120");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_install_registry_profile_pins_curated_packages_and_refreshes_lockfile() {
    let root = temp_dir("registry_profile_install");
    let workspace = root.join("workspace");
    let src = workspace.join("src");
    let entry = src.join("main.agam");
    fs::create_dir_all(&src).expect("create source directory");
    fs::write(&entry, render_project_entry("profile-demo")).expect("write entry source");
    agam_pkg::write_workspace_manifest_to_path(
        &agam_pkg::default_manifest_path(&workspace),
        &agam_pkg::scaffold_workspace_manifest("profile-demo"),
    )
    .expect("write manifest");

    let index_root = root.join("registry-index");
    agam_pkg::write_registry_config(
        &index_root,
        &agam_pkg::RegistryConfig {
            format_version: agam_pkg::REGISTRY_INDEX_FORMAT_VERSION,
            api_url: None,
            download_url: None,
            name: Some("agam".into()),
        },
    )
    .expect("write registry config");
    agam_pkg::append_release_to_index(
        &index_root,
        "agam-std",
        &agam_pkg::RegistryRelease {
            version: "0.1.0".into(),
            checksum: "sha256-agam-std-010".into(),
            agam_version: "0.1".into(),
            dependencies: vec![],
            features: vec![],
            download_url: None,
            provenance: None,
            published_at: "2026-04-10T10:00:00Z".into(),
            yanked: false,
        },
    )
    .expect("append agam-std");
    agam_pkg::append_release_to_index(
        &index_root,
        "agam-test",
        &agam_pkg::RegistryRelease {
            version: "0.1.3".into(),
            checksum: "sha256-agam-test-013".into(),
            agam_version: "0.1".into(),
            dependencies: vec![],
            features: vec![],
            download_url: None,
            provenance: None,
            published_at: "2026-04-10T11:00:00Z".into(),
            yanked: false,
        },
    )
    .expect("append agam-test");

    let report = install_registry_profile(
        Some(workspace.clone()),
        &index_root,
        DependencyTable::Main,
        "base",
        false,
    )
    .expect("install curated profile");

    assert_eq!(report.profile.name, "base");
    assert_eq!(report.items.len(), 2);
    assert!(report.items.iter().all(|item| item.added_new_entry));

    let manifest = agam_pkg::read_workspace_manifest_from_path(&workspace.join("agam.toml"))
        .expect("read updated manifest");
    assert_eq!(
        manifest
            .dependencies
            .get("agam-std")
            .and_then(|spec| spec.version.as_deref()),
        Some("0.1.0")
    );
    assert_eq!(
        manifest
            .dependencies
            .get("agam-test")
            .and_then(|spec| spec.version.as_deref()),
        Some("0.1.3")
    );

    let lockfile = agam_pkg::read_lockfile_from_path(&workspace.join("agam.lock"))
        .expect("read refreshed lockfile");
    assert_eq!(lockfile.packages.len(), 2);
    assert!(
        lockfile
            .packages
            .iter()
            .any(|package| package.name == "agam-std")
    );
    assert!(
        lockfile
            .packages
            .iter()
            .any(|package| package.name == "agam-test")
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_update_registry_dependencies_advances_matching_manifest_entries() {
    let root = temp_dir("registry_update");
    let workspace = root.join("workspace");
    let src = workspace.join("src");
    let entry = src.join("main.agam");
    fs::create_dir_all(&src).expect("create source directory");
    fs::write(&entry, render_project_entry("update-demo")).expect("write entry source");

    let mut manifest = agam_pkg::scaffold_workspace_manifest("update-demo");
    manifest.dependencies.insert(
        "json".into(),
        agam_pkg::DependencySpec {
            version: Some("1.0.0".into()),
            features: vec!["simd".into()],
            optional: true,
            ..agam_pkg::DependencySpec::default()
        },
    );
    agam_pkg::write_workspace_manifest_to_path(&workspace.join("agam.toml"), &manifest)
        .expect("write manifest");

    let index_root = root.join("registry-index");
    agam_pkg::write_registry_config(
        &index_root,
        &agam_pkg::RegistryConfig {
            format_version: agam_pkg::REGISTRY_INDEX_FORMAT_VERSION,
            api_url: None,
            download_url: None,
            name: Some("agam".into()),
        },
    )
    .expect("write registry config");
    agam_pkg::append_release_to_index(
        &index_root,
        "json",
        &agam_pkg::RegistryRelease {
            version: "1.0.0".into(),
            checksum: "sha256-json-100".into(),
            agam_version: "0.1".into(),
            dependencies: vec![],
            features: vec![],
            download_url: None,
            provenance: None,
            published_at: "2026-04-10T10:00:00Z".into(),
            yanked: false,
        },
    )
    .expect("append 1.0.0");
    agam_pkg::append_release_to_index(
        &index_root,
        "json",
        &agam_pkg::RegistryRelease {
            version: "1.4.0".into(),
            checksum: "sha256-json-140".into(),
            agam_version: "0.1".into(),
            dependencies: vec![],
            features: vec![],
            download_url: None,
            provenance: None,
            published_at: "2026-04-10T12:00:00Z".into(),
            yanked: false,
        },
    )
    .expect("append 1.4.0");

    let report = update_registry_dependencies(
        Some(workspace.clone()),
        &index_root,
        DependencyTable::Main,
        &[],
        false,
    )
    .expect("update dependency");

    assert_eq!(report.index_name, "agam");
    assert_eq!(report.items.len(), 1);
    assert!(report.items[0].updated);
    assert_eq!(report.items[0].previous_version.as_deref(), Some("1.0.0"));
    assert_eq!(report.items[0].selected_version, "1.4.0");

    let manifest = agam_pkg::read_workspace_manifest_from_path(&workspace.join("agam.toml"))
        .expect("read updated manifest");
    let spec = manifest.dependencies.get("json").expect("json dependency");
    assert_eq!(spec.version.as_deref(), Some("1.4.0"));
    assert_eq!(spec.registry, None);
    assert_eq!(spec.features, vec!["simd".to_string()]);
    assert!(spec.optional);

    let lockfile = agam_pkg::read_lockfile_from_path(&workspace.join("agam.lock"))
        .expect("read refreshed lockfile");
    assert_eq!(lockfile.packages.len(), 1);
    assert_eq!(lockfile.packages[0].name, "json");
    assert_eq!(lockfile.packages[0].version, "1.4.0");
    assert_eq!(lockfile.packages[0].content_hash, "sha256-json-140");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_yank_registry_release_marks_release_unavailable() {
    let root = temp_dir("registry_yank");
    let index_root = root.join("registry-index");
    agam_pkg::write_registry_config(
        &index_root,
        &agam_pkg::RegistryConfig {
            format_version: agam_pkg::REGISTRY_INDEX_FORMAT_VERSION,
            api_url: None,
            download_url: Some("https://registry.example.com/dl".into()),
            name: Some("agam".into()),
        },
    )
    .expect("write registry config");
    agam_pkg::append_release_to_index(
        &index_root,
        "json",
        &agam_pkg::RegistryRelease {
            version: "1.0.0".into(),
            checksum: "sha256-json-100".into(),
            agam_version: "0.1".into(),
            dependencies: vec![],
            features: vec![],
            download_url: Some(
                "https://registry.example.com/dl/json/1.0.0/json-1.0.0.agam-src.tar.gz".into(),
            ),
            provenance: Some(agam_pkg::RegistryReleaseProvenance {
                source_checksum: "sha256-json-100".into(),
                manifest_checksum: "manifest-json-100".into(),
                published_by: Some("alice".into()),
                source_repository: Some("https://github.com/agam-lang/json".into()),
            }),
            published_at: "2026-04-10T12:00:00Z".into(),
            yanked: false,
        },
    )
    .expect("append release");

    let report = yank_registry_release(&index_root, "json", "1.0.0", false).expect("yank release");
    assert_eq!(report.index_name, "agam");
    assert!(report.yanked);

    let entry = agam_pkg::read_registry_package_entry(&index_root, "json").expect("read package");
    assert!(entry.releases[0].yanked);

    let unyank = yank_registry_release(&index_root, "json", "1.0.0", true).expect("unyank release");
    assert!(!unyank.yanked);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_list_workspace_environments_reports_default_dev_environment() {
    let root = temp_dir("env_list");
    let workspace = root.join("workspace");
    let src = workspace.join("src");
    let entry = src.join("main.agam");
    fs::create_dir_all(&src).expect("create source directory");
    fs::write(&entry, render_project_entry("env-list-demo")).expect("write entry source");

    let mut manifest = agam_pkg::scaffold_workspace_manifest("env-list-demo");
    manifest.toolchain = Some(agam_pkg::ToolchainRequirement {
        agam: "0.2.0".into(),
        sdk: Some("host-native".into()),
        target: Some("x86_64-pc-windows-msvc".into()),
        runtime_abi: Some(agam_runtime::contract::RUNTIME_ABI_VERSION),
        preferred_backend: Some(agam_runtime::contract::RuntimeBackend::Llvm),
    });
    manifest.environments.insert(
        "dev".into(),
        agam_pkg::EnvironmentSpec {
            preferred_backend: Some(agam_runtime::contract::RuntimeBackend::Jit),
            profiles: vec!["debug".into()],
            ..agam_pkg::EnvironmentSpec::default()
        },
    );
    manifest.environments.insert(
        "release".into(),
        agam_pkg::EnvironmentSpec {
            target: Some("x86_64-unknown-linux-gnu".into()),
            profiles: vec!["release".into()],
            ..agam_pkg::EnvironmentSpec::default()
        },
    );
    agam_pkg::write_workspace_manifest_to_path(&workspace.join("agam.toml"), &manifest)
        .expect("write manifest");

    let report = list_workspace_environments(Some(workspace.clone())).expect("list environments");
    assert_eq!(report.default_environment.as_deref(), Some("dev"));
    assert_eq!(report.environments.len(), 2);
    assert!(report.environments.iter().any(|env| env.name == "dev"));
    assert!(report.environments.iter().any(|env| env.name == "release"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_inspect_workspace_environment_uses_selection_rules() {
    with_clean_agam_registry_env(|| {
        let root = temp_dir("env_inspect");
        let workspace = root.join("workspace");
        let src = workspace.join("src");
        let entry = src.join("main.agam");
        fs::create_dir_all(&src).expect("create source directory");
        fs::write(&entry, render_project_entry("env-inspect-demo")).expect("write entry source");

        let mut manifest = agam_pkg::scaffold_workspace_manifest("env-inspect-demo");
        manifest.toolchain = Some(agam_pkg::ToolchainRequirement {
            agam: "0.2.0".into(),
            sdk: Some("host-native".into()),
            target: Some("x86_64-pc-windows-msvc".into()),
            runtime_abi: Some(agam_runtime::contract::RUNTIME_ABI_VERSION),
            preferred_backend: Some(agam_runtime::contract::RuntimeBackend::Llvm),
        });
        manifest.dependencies.insert(
            "json".into(),
            agam_pkg::DependencySpec {
                version: Some("1.4.0".into()),
                ..agam_pkg::DependencySpec::default()
            },
        );
        manifest.environments.insert(
            "dev".into(),
            agam_pkg::EnvironmentSpec {
                preferred_backend: Some(agam_runtime::contract::RuntimeBackend::Jit),
                profiles: vec!["debug".into()],
                ..agam_pkg::EnvironmentSpec::default()
            },
        );
        agam_pkg::write_workspace_manifest_to_path(&workspace.join("agam.toml"), &manifest)
            .expect("write manifest");

        let report =
            inspect_workspace_environment(Some(workspace.clone()), None).expect("inspect env");
        assert!(report.selected_by_default);
        assert_eq!(report.environment.name, "dev");
        assert_eq!(report.environment.compiler, "0.2.0");
        assert_eq!(report.environment.sdk.as_deref(), Some("host-native"));
        assert_eq!(
            report.environment.target.as_deref(),
            Some("x86_64-pc-windows-msvc")
        );
        assert_eq!(
            report.environment.preferred_backend,
            Some(agam_runtime::contract::RuntimeBackend::Jit)
        );
        assert_eq!(report.environment.profiles, vec!["debug".to_string()]);
        assert_eq!(report.environment.packages, vec!["json@1.4.0".to_string()]);

        let _ = fs::remove_dir_all(root);
    });
}

#[test]
fn test_self_hosting_stage0_modules_exist_and_validate() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir.join("../..");

    let lexer_agm = root.join("crates/core/agam_lexer/self_host/lexer.agm");
    let parser_agm = root.join("crates/core/agam_parser/self_host/parser.agm");
    let sema_agm = root.join("crates/middle/agam_sema/self_host/type_check.agm");

    assert!(lexer_agm.exists(), "Self-hosting stage-0 Lexer must exist");
    assert!(parser_agm.exists(), "Self-hosting stage-0 Parser must exist");
    assert!(sema_agm.exists(), "Self-hosting stage-0 Type Checker must exist");

    let lexer_src = fs::read_to_string(&lexer_agm).expect("read lexer.agm");
    let parser_src = fs::read_to_string(&parser_agm).expect("read parser.agm");
    let sema_src = fs::read_to_string(&sema_agm).expect("read type_check.agm");

    assert!(lexer_src.contains("struct Lexer"));
    assert!(parser_src.contains("struct Parser"));
    assert!(sema_src.contains("struct TypeChecker"));
}
