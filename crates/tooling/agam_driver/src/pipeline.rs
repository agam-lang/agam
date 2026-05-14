//! Driver wrappers around the shared compiler interface.

use super::*;

pub(crate) use agam_interface::{
    CallCacheSelection, FeatureFlags, SourceFeatureFlags, WarmState, build_warm_state,
    collect_experimental_function_features, compile_file_with_warm_state,
    emit_experimental_feature_warnings, emit_resolve_error, emit_type_error,
    lower_module_to_hir_and_optimized_mir, lower_parsed_to_optimized_mir, lower_to_optimized_mir,
    merge_function_call_cache_annotations, parse_source_file, semantic_check_parsed_source,
    source_feature_flags_from_tokens,
};

pub(crate) fn run_check_request_locally(path: &PathBuf, verbose: bool) -> Result<(), String> {
    compile_file(path, verbose)?;
    if verbose {
        eprintln!("[agamc] {} â€” OK", path.display());
    }
    Ok(())
}

/// Read, parse, and run semantic checks without lowering or code generation.
pub(crate) fn compile_file(path: &PathBuf, verbose: bool) -> Result<(), String> {
    if load_daemon_prewarmed_warm_state(path, verbose).is_some() {
        return Ok(());
    }
    if load_daemon_warm_state_for_file(path, verbose).is_some() {
        return Ok(());
    }
    let parsed = parse_source_file(path, verbose)?;
    semantic_check_parsed_source(path, &parsed, verbose)?;
    Ok(())
}

/// Compile a file for `agamc dev`; only the runnable entry file needs warm lowered state.
pub(crate) fn compile_dev_source_file(
    path: &PathBuf,
    keep_warm_state: bool,
    verbose: bool,
) -> Result<Option<WarmState>, String> {
    if keep_warm_state {
        if let Some(warm_state) = load_daemon_prewarmed_warm_state(path, verbose) {
            return Ok(Some(warm_state));
        }
        if let Some(warm_state) = load_daemon_warm_state_for_file(path, verbose) {
            if warm_state_supports_runnable_reuse(&warm_state) {
                return Ok(Some(warm_state));
            }
            if verbose && warm_state.mir.is_some() {
                eprintln!(
                    "[agamc] warm state for `{}` is incomplete for runnable reuse; rebuilding locally",
                    path.display()
                );
            }
        }
        Ok(Some(compile_file_with_warm_state(path, verbose)?))
    } else {
        compile_file(path, verbose)?;
        Ok(None)
    }
}
