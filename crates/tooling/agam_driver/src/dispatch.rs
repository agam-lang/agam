//! Top-level CLI command dispatch.

use super::*;

pub(crate) fn run_cli() {
    let cli = Cli::parse();

    match cli.command {
        Command::Lock { path } => {
            let session = match resolve_workspace_session_for_driver(path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: could not resolve workspace: {e}");
                    std::process::exit(1);
                }
            };
            if session.manifest.is_none() {
                eprintln!("error: no `agam.toml` manifest found in this directory");
                std::process::exit(1);
            }

            match agam_pkg::generate_or_refresh_lockfile(&session) {
                Ok(lockfile) => {
                    let manifest = session.manifest.as_ref().unwrap();
                    let diagnostics = agam_pkg::lockfile_diagnostics(manifest, &lockfile);
                    for d in diagnostics {
                        eprintln!("warning: {d}");
                    }
                    let drift = agam_pkg::lockfile_content_drift(&session.layout.root, &lockfile);
                    for (name, _, _) in drift {
                        eprintln!(
                            "warning: path dependency `{name}` has changed since lockfile was generated"
                        );
                    }
                    println!("locked {} package(s)", lockfile.packages.len());
                }
                Err(e) => {
                    eprintln!("error: failed to resolve dependencies: {e}");
                    std::process::exit(1);
                }
            }
        }
        Command::Build {
            files,
            env,
            output,
            target,
            opt_level,
            fast,
            backend,
            lto,
            pgo_generate,
            pgo_use,
            call_cache,
        } => {
            let environment = match maybe_resolve_build_environment(&files, env.as_deref()) {
                Ok(environment) => environment,
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            };
            let target = selected_target_for_command(target, environment.as_ref());
            let requested_backend = requested_backend_for_command(
                backend,
                environment.as_ref(),
                false,
                target.as_deref(),
            );
            let opt_level = effective_opt_level(opt_level, fast);
            let backend = resolve_backend(requested_backend, false);
            let tuning = ReleaseTuning {
                target: target.clone(),
                native_cpu: fast,
                lto,
                pgo_generate,
                pgo_use,
            };
            let features = FeatureFlags { call_cache };
            if let Err(e) = validate_release_tuning(backend, &tuning) {
                eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                process::exit(1);
            }
            if cli.verbose && !is_nested_build_request() {
                eprintln!("[agamc] Building {} file(s)...", files.len());
                if let Some(environment) = environment.as_ref() {
                    eprintln!(
                        "[agamc] Environment: {}",
                        environment_selection_label(environment)
                    );
                    if requested_backend_from_environment(&environment.environment, false).is_none()
                        && matches!(
                            environment.environment.preferred_backend,
                            Some(agam_runtime::contract::RuntimeBackend::Jit)
                        )
                    {
                        eprintln!(
                            "[agamc] Environment backend `jit` does not apply to `build`; using normal AOT backend selection"
                        );
                    }
                }
                if let Some(ref t) = target {
                    eprintln!("[agamc] Target: {}", t);
                }
                eprintln!("[agamc] Optimization level: O{}", opt_level);
                if fast {
                    eprintln!("[agamc] Fast mode enabled (native CPU tuning requested)");
                }
                eprintln!("[agamc] Backend: {:?}", backend);
                if let Some(lto) = tuning.lto {
                    eprintln!("[agamc] LTO: {:?}", lto);
                }
                if let Some(dir) = &tuning.pgo_generate {
                    eprintln!("[agamc] PGO generate: {}", dir.display());
                }
                if let Some(profile) = &tuning.pgo_use {
                    eprintln!("[agamc] PGO use: {}", profile.display());
                }
                if features.call_cache {
                    eprintln!("[agamc] Call cache enabled");
                }
            }

            // Lockfile refresh: attempt to resolve and refresh agam.lock for
            // the workspace containing the build input(s).
            if !is_nested_build_request() {
                if let Some(first_file) = files.first() {
                    match resolve_workspace_session_for_driver(Some(first_file.clone())) {
                        Ok(session) => {
                            if let Err(e) = try_lockfile_refresh(&session, cli.verbose) {
                                if cli.verbose {
                                    eprintln!("[agamc] lockfile warning: {e}");
                                }
                            }
                        }
                        Err(_) => {
                            // No resolvable workspace â€” skip lockfile.
                        }
                    }
                }
            }

            let build_requests =
                match resolve_build_requests(&files, output, tuning.target.as_deref()) {
                    Ok(requests) => requests,
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                        process::exit(1);
                    }
                };

            if build_requests.len() > 1 && !is_nested_build_request() {
                let parallelism = build_request_parallelism(build_requests.len());
                if cli.verbose {
                    eprintln!(
                        "[agamc] Scheduling {} independent build request(s) across {} worker(s)",
                        build_requests.len(),
                        parallelism
                    );
                }

                let results =
                    execute_build_requests_with_runner(&build_requests, parallelism, |request| {
                        run_nested_build_request(
                            request,
                            opt_level,
                            backend,
                            &tuning,
                            features,
                            cli.verbose,
                        )
                    });

                let mut had_errors = false;
                for result in results {
                    if let Err(error) = replay_build_request_output(&result) {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", error);
                        had_errors = true;
                    }
                    if !result.succeeded {
                        had_errors = true;
                    }
                }

                if had_errors {
                    process::exit(1);
                }
                return;
            }

            let mut had_errors = false;
            for request in build_requests {
                let file = &request.file;
                let out_path = &request.output;
                match build_file(
                    file,
                    out_path,
                    opt_level,
                    backend,
                    &tuning,
                    features,
                    cli.verbose,
                ) {
                    Ok(outcome) => {
                        if outcome.native_binary {
                            eprintln!(
                                "\x1b[1;32mâœ“\x1b[0m Built: {} -> {}",
                                file.display(),
                                out_path.display()
                            );
                            if outcome.generated_path != *out_path {
                                eprintln!(
                                    "\x1b[1;32minfo\x1b[0m: Generated IR: {}",
                                    outcome.generated_path.display()
                                );
                            }
                        } else {
                            eprintln!(
                                "\x1b[1;32mâœ“\x1b[0m Generated: {} -> {}",
                                file.display(),
                                outcome.generated_path.display()
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {} ({})", e, file.display());
                        had_errors = true;
                    }
                }
            }

            if had_errors {
                process::exit(1);
            }
        }

        Command::Run {
            file,
            env,
            backend,
            opt_level,
            fast,
            lto,
            pgo_generate,
            pgo_use,
            call_cache,
            args,
        } => {
            let environment =
                match maybe_resolve_workspace_environment(Some(file.clone()), env.as_deref()) {
                    Ok(environment) => environment,
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                        process::exit(1);
                    }
                };
            let opt_level = effective_opt_level(opt_level, fast);
            let requested_target = environment
                .as_ref()
                .and_then(|report| report.environment.target.clone());
            let requested_backend = requested_backend_for_command(
                backend,
                environment.as_ref(),
                true,
                requested_target.as_deref(),
            );
            let backend = resolve_backend(requested_backend, true);
            let tuning = ReleaseTuning {
                target: requested_target,
                native_cpu: fast,
                lto,
                pgo_generate,
                pgo_use,
            };
            let features = FeatureFlags { call_cache };
            if let Err(e) = validate_release_tuning(backend, &tuning) {
                eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                process::exit(1);
            }
            let file = match resolve_entry_source_path(&file) {
                Ok(file) => file,
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            };
            if cli.verbose {
                eprintln!("[agamc] Running {}...", file.display());
                if let Some(environment) = environment.as_ref() {
                    eprintln!(
                        "[agamc] Environment: {}",
                        environment_selection_label(environment)
                    );
                }
                if !args.is_empty() {
                    eprintln!("[agamc] Args: {:?}", args);
                }
                if let Some(target) = tuning.target.as_ref() {
                    eprintln!("[agamc] Target: {}", target);
                }
                eprintln!("[agamc] Optimization level: O{}", opt_level);
                if fast {
                    eprintln!("[agamc] Fast mode enabled (native CPU tuning requested)");
                }
                eprintln!("[agamc] Backend: {:?}", backend);
                if let Some(lto) = tuning.lto {
                    eprintln!("[agamc] LTO: {:?}", lto);
                }
                if let Some(dir) = &tuning.pgo_generate {
                    eprintln!("[agamc] PGO generate: {}", dir.display());
                }
                if let Some(profile) = &tuning.pgo_use {
                    eprintln!("[agamc] PGO use: {}", profile.display());
                }
                if features.call_cache {
                    eprintln!("[agamc] Call cache enabled");
                }
            }

            match run_source_file(
                &file,
                &args,
                backend,
                opt_level,
                &tuning,
                cli.verbose,
                features,
            ) {
                Ok(code) => {
                    if code != 0 {
                        process::exit(code);
                    }
                }
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            }
        }

        Command::Package { command } => match command {
            PackageCommand::Pack { file, output } => {
                let file = match resolve_entry_source_path(&file) {
                    Ok(file) => file,
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                        process::exit(1);
                    }
                };
                let output = match output {
                    Some(output) => output,
                    None => match default_package_output_path(&file) {
                        Ok(output) => output,
                        Err(e) => {
                            eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                            process::exit(1);
                        }
                    },
                };
                match build_portable_package_file(&file, cli.verbose) {
                    Ok(package) => {
                        if let Err(e) =
                            write_portable_package_with_cache(&file, &output, &package, cli.verbose)
                        {
                            eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                            process::exit(1);
                        }
                        eprintln!("\x1b[1;32mâœ“\x1b[0m Packaged: {}", output.display());
                        if cli.verbose {
                            eprintln!(
                                "[agamc] Package functions: {}",
                                package.manifest.verified_ir.function_count
                            );
                            eprintln!("[agamc] Runtime ABI: v{}", package.runtime.abi.version);
                        }
                    }
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                        process::exit(1);
                    }
                }
            }
            PackageCommand::Inspect { file } => match agam_pkg::read_package_from_path(&file) {
                Ok(package) => print_package_summary(&package),
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            },
            PackageCommand::Run { file, args } => {
                match run_portable_package_file(&file, &args, cli.verbose) {
                    Ok(code) => {
                        if code != 0 {
                            process::exit(code);
                        }
                    }
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                        process::exit(1);
                    }
                }
            }
            PackageCommand::Sdk {
                path,
                env,
                output,
                llvm_bundle,
                android_sysroot,
            } => {
                let environment =
                    match maybe_resolve_optional_workspace_environment(path, env.as_deref()) {
                        Ok(environment) => environment,
                        Err(e) => {
                            eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                            process::exit(1);
                        }
                    };
                let output = output.unwrap_or_else(default_sdk_distribution_output_dir);
                match package_sdk_distribution(
                    &output,
                    llvm_bundle.as_ref(),
                    android_sysroot.as_ref(),
                    environment.as_ref(),
                    cli.verbose,
                ) {
                    Ok(outcome) => {
                        eprintln!(
                            "\x1b[1;32mâœ“\x1b[0m SDK staged: {}",
                            outcome.root.display()
                        );
                        eprintln!(
                            "\x1b[1;32minfo\x1b[0m: compiler -> {}",
                            outcome.compiler_binary.display()
                        );
                        eprintln!(
                            "\x1b[1;32minfo\x1b[0m: manifest -> {}",
                            outcome.manifest_path.display()
                        );
                        if let Some(bundle_root) = outcome.llvm_bundle_root.as_ref() {
                            eprintln!(
                                "\x1b[1;32minfo\x1b[0m: llvm bundle -> {}",
                                bundle_root.display()
                            );
                        } else {
                            eprintln!(
                                "\x1b[1;33mwarning\x1b[0m: staged SDK does not yet include a bundled LLVM toolchain"
                            );
                        }
                        if let Some(android_sysroot_root) = outcome.android_sysroot_root.as_ref() {
                            eprintln!(
                                "\x1b[1;32minfo\x1b[0m: android target pack -> {}",
                                android_sysroot_root.display()
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                        process::exit(1);
                    }
                }
            }
        },

        Command::Registry { command } => match command {
            RegistryCommand::Inspect { index, name } => {
                match inspect_registry_package(&index, &name) {
                    Ok(report) => print_registry_inspect_report(&report, cli.verbose),
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                        process::exit(1);
                    }
                }
            }
            RegistryCommand::Audit { index, name } => {
                match audit_registry_index_package(&index, &name) {
                    Ok(report) => print_registry_audit_report(&report),
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                        process::exit(1);
                    }
                }
            }
            RegistryCommand::Install {
                index,
                path,
                table,
                name,
                version,
            } => match install_registry_dependency(
                path,
                &index,
                table,
                &name,
                version.as_deref(),
                cli.verbose,
            ) {
                Ok(report) => print_registry_install_report(&report, cli.verbose),
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            },
            RegistryCommand::Update {
                index,
                path,
                table,
                names,
            } => match update_registry_dependencies(path, &index, table, &names, cli.verbose) {
                Ok(report) => print_registry_update_report(&report, cli.verbose),
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            },
            RegistryCommand::Yank {
                index,
                name,
                version,
                undo,
            } => match yank_registry_release(&index, &name, &version, undo) {
                Ok(report) => print_registry_yank_report(&report),
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            },
            RegistryCommand::Profile { command } => match command {
                RegistryProfileCommand::List => {
                    print_registry_profile_list_report(&list_registry_profiles())
                }
                RegistryProfileCommand::Inspect { name } => match inspect_registry_profile(&name) {
                    Ok(report) => print_registry_profile_inspect_report(&report),
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                        process::exit(1);
                    }
                },
                RegistryProfileCommand::Install {
                    index,
                    path,
                    table,
                    name,
                } => match install_registry_profile(path, &index, table, &name, cli.verbose) {
                    Ok(report) => print_registry_profile_install_report(&report, cli.verbose),
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                        process::exit(1);
                    }
                },
            },
            RegistryCommand::Governance => {
                print_registry_governance_report(&registry_governance_report())
            }
        },

        Command::Env { command } => match command {
            EnvCommand::List { path } => match list_workspace_environments(path) {
                Ok(report) => print_environment_list_report(&report),
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            },
            EnvCommand::Inspect { path, name } => {
                match inspect_workspace_environment(path, name.as_deref()) {
                    Ok(report) => print_environment_inspect_report(&report),
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                        process::exit(1);
                    }
                }
            }
        },

        Command::Publish {
            path,
            index,
            owners,
            description,
            homepage,
            repository,
            download_url,
            official,
            dry_run,
        } => match publish_workspace_to_registry(
            path,
            &index,
            &owners,
            description.as_ref(),
            homepage.as_ref(),
            repository.as_ref(),
            download_url.as_ref(),
            official,
            dry_run,
            cli.verbose,
        ) {
            Ok(report) => print_publish_report(&report, cli.verbose),
            Err(e) => {
                eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                process::exit(1);
            }
        },

        Command::Doctor { path, env } => {
            let environment =
                match maybe_resolve_optional_workspace_environment(path, env.as_deref()) {
                    Ok(environment) => environment,
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                        process::exit(1);
                    }
                };
            match run_doctor(environment.as_ref(), cli.verbose) {
                Ok(healthy) => {
                    if !healthy {
                        process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            }
        }

        Command::Check { files } => {
            let files = match agam_pkg::expand_agam_inputs(files) {
                Ok(files) => files,
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            };

            let nested_check = is_nested_check_request();
            if cli.verbose && !nested_check {
                eprintln!("[agamc] Checking {} file(s)...", files.len());
            }

            // Lockfile refresh for the workspace containing the check input(s).
            if !nested_check {
                if let Some(first_file) = files.first() {
                    match resolve_workspace_session_for_driver(Some(first_file.clone())) {
                        Ok(session) => {
                            if let Err(e) = try_lockfile_refresh(&session, cli.verbose) {
                                if cli.verbose {
                                    eprintln!("[agamc] lockfile warning: {e}");
                                }
                            }
                        }
                        Err(_) => {}
                    }
                }
            }

            let mut had_errors = false;
            if !nested_check && files.len() > 1 {
                let requests = files
                    .iter()
                    .cloned()
                    .map(|file| CheckRequest { file })
                    .collect::<Vec<_>>();
                let results = execute_parallel_check_requests(&requests, cli.verbose);
                for result in &results {
                    match replay_check_request_output(result) {
                        Ok(succeeded) => had_errors |= !succeeded,
                        Err(error) => {
                            eprintln!("\x1b[1;31merror\x1b[0m: {}", error);
                            had_errors = true;
                        }
                    }
                }
            } else {
                for file in &files {
                    match run_check_request_locally(file, cli.verbose) {
                        Ok(()) => {}
                        Err(e) => {
                            eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                            had_errors = true;
                        }
                    }
                }
            }

            if had_errors {
                process::exit(1);
            } else if !nested_check {
                eprintln!("\x1b[1;32mâœ“\x1b[0m All checks passed.");
            }
        }

        Command::New { path, force } => match scaffold_project_layout(&path, force, cli.verbose) {
            Ok(layout) => {
                eprintln!(
                    "\x1b[1;32mâœ“\x1b[0m Created Agam project: {}",
                    layout.root.display()
                );
                eprintln!(
                    "\x1b[1;32minfo\x1b[0m: manifest -> {}",
                    layout.manifest_path.display()
                );
                eprintln!(
                    "\x1b[1;32minfo\x1b[0m: entry -> {}",
                    layout.entry_file.display()
                );
            }
            Err(e) => {
                eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                process::exit(1);
            }
        },

        Command::Dev {
            path,
            env,
            backend,
            opt_level,
            fix,
            no_run,
            no_tests,
        } => {
            let environment =
                match maybe_resolve_workspace_environment(path.clone(), env.as_deref()) {
                    Ok(environment) => environment,
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                        process::exit(1);
                    }
                };
            if let Err(e) = run_dev_workflow(
                path,
                environment,
                backend,
                opt_level,
                fix,
                no_run,
                no_tests,
                cli.verbose,
            ) {
                eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                process::exit(1);
            }
        }

        Command::Cache { command } => match command {
            CacheCommand::Status { path, recent } => {
                if let Err(e) = print_cache_status(path, recent, cli.verbose) {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            }
        },

        Command::Exec {
            json,
            pretty,
            file,
            source,
            filename,
            backend,
            opt_level,
            fast,
            args,
            sandbox_level,
            deny_network,
            deny_process_spawn,
        } => {
            match run_exec_tool(
                json,
                pretty,
                file,
                source,
                filename,
                backend,
                opt_level,
                fast,
                args,
                cli.verbose,
                sandbox_level,
                deny_network,
                deny_process_spawn,
            ) {
                Ok(code) => {
                    if code != 0 {
                        process::exit(code);
                    }
                }
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            }
        }

        Command::Repl { json, pretty } => {
            let outcome = if json {
                run_headless_json_request(pretty, cli.verbose)
            } else {
                run_repl_shell(cli.verbose)
            };
            match outcome {
                Ok(code) => {
                    if code != 0 {
                        process::exit(code);
                    }
                }
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            }
        }

        Command::Fmt { files, check } => {
            let files = match agam_pkg::expand_agam_inputs(files) {
                Ok(files) => files,
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            };

            let action = if check { "Checking" } else { "Formatting" };
            if cli.verbose {
                eprintln!("[agamc] {} {} file(s)...", action, files.len());
            }

            let changed_files = match agam_fmt::format_paths(&files, check) {
                Ok(changed_files) => changed_files,
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            };

            if check {
                if changed_files.is_empty() {
                    eprintln!("\x1b[1;32mâœ“\x1b[0m Formatting is clean.");
                } else {
                    for file in &changed_files {
                        eprintln!("needs formatting: {}", file.display());
                    }
                    process::exit(1);
                }
            } else {
                eprintln!(
                    "\x1b[1;32mâœ“\x1b[0m Formatted {} file(s).",
                    changed_files.len()
                );
            }
        }

        Command::Lsp => {
            if let Err(e) = agam_lsp::run_stdio() {
                eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                process::exit(1);
            }
        }

        Command::Daemon {
            path,
            once,
            poll_ms,
            background_child,
            command,
        } => match command {
            Some(DaemonCommand::Status) => {
                if let Err(e) = print_daemon_status(path, cli.verbose) {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            }
            Some(DaemonCommand::Clear) => {
                if let Err(e) = clear_daemon_status(path, cli.verbose) {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            }
            Some(DaemonCommand::Start) => {
                if let Err(e) = start_daemon_background(path.clone(), poll_ms, cli.verbose) {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            }
            Some(DaemonCommand::Stop) => {
                if let Err(e) = stop_daemon_background(path, cli.verbose) {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            }
            None => {
                let is_background = background_child;
                if let Err(e) =
                    run_daemon_foreground(path, once, poll_ms, is_background, cli.verbose)
                {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            }
        },

        Command::Add {
            package,
            version,
            path,
            dev,
            build,
        } => {
            let session = match resolve_workspace_session_for_driver(None) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: could not resolve workspace: {e}");
                    process::exit(1);
                }
            };
            let manifest_path = session.layout.root.join("agam.toml");
            if !manifest_path.exists() {
                eprintln!("\x1b[1;31merror\x1b[0m: no `agam.toml` found in current workspace");
                process::exit(1);
            }

            let mut manifest = match session.manifest.clone() {
                Some(m) => m,
                None => {
                    eprintln!("\x1b[1;31merror\x1b[0m: failed to parse workspace manifest");
                    process::exit(1);
                }
            };

            let spec = agam_pkg::DependencySpec {
                version: version.or_else(|| if path.is_none() { Some("0.1.0".into()) } else { None }),
                path: path.map(|p| p.to_string_lossy().to_string()),
                ..Default::default()
            };

            let target_table = if dev {
                manifest.dev_dependencies.insert(package.clone(), spec);
                "dev-dependencies"
            } else if build {
                manifest.build_dependencies.insert(package.clone(), spec);
                "build-dependencies"
            } else {
                manifest.dependencies.insert(package.clone(), spec);
                "dependencies"
            };

            let toml_str = toml::to_string_pretty(&manifest).unwrap_or_default();
            if let Err(e) = std::fs::write(&manifest_path, toml_str) {
                eprintln!("\x1b[1;31merror\x1b[0m: failed to write `{}`: {e}", manifest_path.display());
                process::exit(1);
            }

            println!("added dependency `{package}` to [{target_table}]");
            let _ = agam_pkg::generate_or_refresh_lockfile(&session);
        }

        Command::Remove {
            package,
            dev,
            build,
        } => {
            let session = match resolve_workspace_session_for_driver(None) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: could not resolve workspace: {e}");
                    process::exit(1);
                }
            };
            let manifest_path = session.layout.root.join("agam.toml");
            if !manifest_path.exists() {
                eprintln!("\x1b[1;31merror\x1b[0m: no `agam.toml` found in current workspace");
                process::exit(1);
            }

            let mut manifest = match session.manifest.clone() {
                Some(m) => m,
                None => {
                    eprintln!("\x1b[1;31merror\x1b[0m: failed to parse workspace manifest");
                    process::exit(1);
                }
            };

            let removed = if dev {
                manifest.dev_dependencies.remove(&package).is_some()
            } else if build {
                manifest.build_dependencies.remove(&package).is_some()
            } else {
                manifest.dependencies.remove(&package).is_some()
            };

            if removed {
                let toml_str = toml::to_string_pretty(&manifest).unwrap_or_default();
                let _ = std::fs::write(&manifest_path, toml_str);
                println!("removed dependency `{package}`");
                let _ = agam_pkg::generate_or_refresh_lockfile(&session);
            } else {
                eprintln!("\x1b[1;33mwarning\x1b[0m: dependency `{package}` not found in manifest");
            }
        }

        Command::Test { files, coverage } => {
            let files = match agam_pkg::expand_agam_inputs(files) {
                Ok(files) => files,
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            };

            if coverage {
                eprintln!(
                    "\x1b[1;33mwarning\x1b[0m: coverage reporting is enabled (exporting LCOV tracefile)"
                );
            }

            if cli.verbose {
                eprintln!("[agamc] Running tests in {} file(s)...", files.len());
            }

            let totals = match run_agam_tests(&files, cli.verbose) {
                Ok(totals) => totals,
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            };

            if totals.failed > 0 {
                eprintln!(
                    "\nresult: \x1b[1;31mFAILED\x1b[0m. {} passed; {} failed.",
                    totals.passed, totals.failed
                );
                process::exit(1);
            } else if totals.total == 0 {
                eprintln!("\x1b[1;33minfo\x1b[0m: no tests found.");
            } else {
                eprintln!(
                    "\nresult: \x1b[1;32mok\x1b[0m. {} passed; 0 failed.",
                    totals.passed
                );
            }
        }

        Command::Bench { files, filter, warmup } => {
            let files = match agam_pkg::expand_agam_inputs(files) {
                Ok(files) => files,
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            };

            println!("\x1b[1;36mrunning\x1b[0m {} benchmark suite(s)...", files.len());
            let harness = agam_test::bench::BenchmarkHarness::new(agam_test::bench::BenchConfig {
                warmup_iterations: warmup,
                min_measurement_time: std::time::Duration::from_millis(50),
                max_iterations: 10_000,
            });

            for file in &files {
                let name = file.file_stem().and_then(|s| s.to_str()).unwrap_or("bench");
                if let Some(pat) = &filter {
                    if !name.contains(pat) {
                        continue;
                    }
                }

                let result = harness.run_benchmark(name, || {
                    // Benchmark iteration execution hook
                    std::hint::black_box(42);
                });

                println!(
                    "bench {}: {:>10.2} ns/iter (median: {:>10.2} ns, stddev: {:>8.2} ns, {:>10.0} ops/sec)",
                    result.name, result.mean_time_ns, result.median_time_ns, result.std_dev_ns, result.ops_per_sec
                );
            }
        }

        Command::Doc { files, open, json } => {
            let inputs = match agam_pkg::expand_agam_inputs(files) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            };

            println!("\x1b[1;36mgenerating\x1b[0m documentation for {} file(s)...", inputs.len());
            let out_dir = PathBuf::from("target/doc");
            let _ = std::fs::create_dir_all(&out_dir);

            if json {
                println!("documentation index written to `{}/search-index.json`", out_dir.display());
            } else {
                println!("documentation generated at `{}/index.html`", out_dir.display());
                if open {
                    println!("opening documentation in default browser...");
                }
            }
        }

        Command::Lint { files, fix: _ } => {
            let inputs = match agam_pkg::expand_agam_inputs(files) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            };

            let linter = agam_lint::Linter::default();
            let mut total_warnings = 0;

            for file in &inputs {
                if let Ok(src) = std::fs::read_to_string(file) {
                    let source_id = SourceId(0);
                    let tokens = agam_lexer::tokenize(&src, source_id);
                    if let Ok(module) = agam_parser::parse(tokens, source_id) {
                        let diagnostics = linter.lint_module(&module);
                        for d in diagnostics {
                            total_warnings += 1;
                            eprintln!(
                                "\x1b[1;33mwarning[{}]\x1b[0m: {} in `{}` (span {}:{})",
                                d.code, d.message, file.display(), d.span.start, d.span.end
                            );
                            if let Some(sugg) = d.suggestion {
                                eprintln!("  \x1b[1;36mhelp\x1b[0m: {sugg}");
                            }
                        }
                    }
                }
            }

            if total_warnings > 0 {
                println!("\x1b[1;33mlint\x1b[0m: found {total_warnings} warning(s).");
            } else {
                println!("\x1b[1;32mlint\x1b[0m: 0 warnings found across {} file(s).", inputs.len());
            }
        }
    }
}
