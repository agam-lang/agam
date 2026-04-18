//! Builtin effect handler registrations for `agam_std` modules.
//!
//! Registers concrete handler functions from `agam_std::io` into the
//! runtime's `EffectHandlerTable`, bridging the semantic effect definitions
//! to actual filesystem operations.

use agam_runtime::effects::{EffectError, EffectHandlerTable, EffectValue};

/// Register all builtin `FileSystem` effect handlers backed by `agam_std::io`.
pub fn register_filesystem_handlers(table: &mut EffectHandlerTable) {
    table.register("FileSystem", "exists", fs_exists);
    table.register("FileSystem", "is_file", fs_is_file);
    table.register("FileSystem", "is_dir", fs_is_dir);
    table.register("FileSystem", "create_dir_all", fs_create_dir_all);
    table.register("FileSystem", "read_to_string", fs_read_to_string);
    table.register("FileSystem", "read_lines", fs_read_lines);
    table.register("FileSystem", "write_string", fs_write_string);
    table.register("FileSystem", "append_string", fs_append_string);
    table.register("FileSystem", "list_dir", fs_list_dir);
}

/// Register all builtin effect handlers for all `agam_std` modules.
pub fn register_all_builtin_handlers(table: &mut EffectHandlerTable) {
    register_filesystem_handlers(table);
}

/// Create an `EffectHandlerTable` pre-populated with all builtin handlers.
pub fn builtin_handler_table() -> EffectHandlerTable {
    let mut table = EffectHandlerTable::new();
    register_all_builtin_handlers(&mut table);
    table
}

// ── Internal helpers ───────────────────────────────────────────────────

fn require_string_arg(
    effect: &str,
    op: &str,
    args: &[EffectValue],
    index: usize,
) -> Result<String, EffectError> {
    match args.get(index) {
        Some(EffectValue::String(s)) => Ok(s.clone()),
        _ => Err(EffectError {
            effect: effect.to_string(),
            operation: op.to_string(),
            message: format!("expected string argument at position {index}"),
        }),
    }
}

fn fs_exists(args: &[EffectValue]) -> Result<EffectValue, EffectError> {
    let path = require_string_arg("FileSystem", "exists", args, 0)?;
    Ok(EffectValue::Bool(crate::io::exists(&path)))
}

fn fs_is_file(args: &[EffectValue]) -> Result<EffectValue, EffectError> {
    let path = require_string_arg("FileSystem", "is_file", args, 0)?;
    Ok(EffectValue::Bool(crate::io::is_file(&path)))
}

fn fs_is_dir(args: &[EffectValue]) -> Result<EffectValue, EffectError> {
    let path = require_string_arg("FileSystem", "is_dir", args, 0)?;
    Ok(EffectValue::Bool(crate::io::is_dir(&path)))
}

fn fs_create_dir_all(args: &[EffectValue]) -> Result<EffectValue, EffectError> {
    let path = require_string_arg("FileSystem", "create_dir_all", args, 0)?;
    crate::io::create_dir_all(&path).map_err(|e| EffectError {
        effect: "FileSystem".into(),
        operation: "create_dir_all".into(),
        message: e.to_string(),
    })?;
    Ok(EffectValue::Unit)
}

fn fs_read_to_string(args: &[EffectValue]) -> Result<EffectValue, EffectError> {
    let path = require_string_arg("FileSystem", "read_to_string", args, 0)?;
    let contents = crate::io::read_to_string(&path).map_err(|e| EffectError {
        effect: "FileSystem".into(),
        operation: "read_to_string".into(),
        message: e.to_string(),
    })?;
    Ok(EffectValue::String(contents))
}

fn fs_read_lines(args: &[EffectValue]) -> Result<EffectValue, EffectError> {
    let path = require_string_arg("FileSystem", "read_lines", args, 0)?;
    let lines = crate::io::read_lines(&path).map_err(|e| EffectError {
        effect: "FileSystem".into(),
        operation: "read_lines".into(),
        message: e.to_string(),
    })?;
    Ok(EffectValue::List(
        lines.into_iter().map(EffectValue::String).collect(),
    ))
}

fn fs_write_string(args: &[EffectValue]) -> Result<EffectValue, EffectError> {
    let path = require_string_arg("FileSystem", "write_string", args, 0)?;
    let contents = require_string_arg("FileSystem", "write_string", args, 1)?;
    crate::io::write_string(&path, &contents).map_err(|e| EffectError {
        effect: "FileSystem".into(),
        operation: "write_string".into(),
        message: e.to_string(),
    })?;
    Ok(EffectValue::Unit)
}

fn fs_append_string(args: &[EffectValue]) -> Result<EffectValue, EffectError> {
    let path = require_string_arg("FileSystem", "append_string", args, 0)?;
    let contents = require_string_arg("FileSystem", "append_string", args, 1)?;
    crate::io::append_string(&path, &contents).map_err(|e| EffectError {
        effect: "FileSystem".into(),
        operation: "append_string".into(),
        message: e.to_string(),
    })?;
    Ok(EffectValue::Unit)
}

fn fs_list_dir(args: &[EffectValue]) -> Result<EffectValue, EffectError> {
    let path = require_string_arg("FileSystem", "list_dir", args, 0)?;
    let entries = crate::io::list_dir(&path).map_err(|e| EffectError {
        effect: "FileSystem".into(),
        operation: "list_dir".into(),
        message: e.to_string(),
    })?;
    Ok(EffectValue::List(
        entries
            .into_iter()
            .map(|p| EffectValue::String(p.to_string_lossy().to_string()))
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("agam_std_effects_{label}_{stamp}"));
        std::fs::create_dir_all(&path).expect("temp dir should be created");
        path
    }

    #[test]
    fn builtin_table_has_all_filesystem_ops() {
        let table = builtin_handler_table();
        assert_eq!(table.len(), 9);
        assert!(table.get("FileSystem", "exists").is_some());
        assert!(table.get("FileSystem", "read_to_string").is_some());
        assert!(table.get("FileSystem", "write_string").is_some());
        assert!(table.get("FileSystem", "list_dir").is_some());
    }

    #[test]
    fn dispatch_exists_returns_bool() {
        let table = builtin_handler_table();
        let result = table
            .dispatch("FileSystem", "exists", &[EffectValue::String(".".into())])
            .expect("exists should succeed");
        assert_eq!(result, EffectValue::Bool(true));
    }

    #[test]
    fn dispatch_write_and_read_round_trip() {
        let root = temp_dir("round_trip");
        let file_path = root.join("test.txt");
        let table = builtin_handler_table();

        table
            .dispatch(
                "FileSystem",
                "write_string",
                &[
                    EffectValue::String(file_path.to_string_lossy().to_string()),
                    EffectValue::String("hello effects\n".into()),
                ],
            )
            .expect("write should succeed");

        let result = table
            .dispatch(
                "FileSystem",
                "read_to_string",
                &[EffectValue::String(file_path.to_string_lossy().to_string())],
            )
            .expect("read should succeed");
        assert_eq!(result, EffectValue::String("hello effects\n".into()));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn dispatch_list_dir_returns_sorted_entries() {
        let root = temp_dir("list_dir");
        let table = builtin_handler_table();

        for name in &["zz.txt", "aa.txt", "mm.txt"] {
            table
                .dispatch(
                    "FileSystem",
                    "write_string",
                    &[
                        EffectValue::String(root.join(name).to_string_lossy().to_string()),
                        EffectValue::String("x".into()),
                    ],
                )
                .expect("write should succeed");
        }

        let result = table
            .dispatch(
                "FileSystem",
                "list_dir",
                &[EffectValue::String(root.to_string_lossy().to_string())],
            )
            .expect("list_dir should succeed");

        if let EffectValue::List(entries) = &result {
            let names: Vec<_> = entries
                .iter()
                .filter_map(|v| match v {
                    EffectValue::String(s) => std::path::Path::new(s)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string()),
                    _ => None,
                })
                .collect();
            assert_eq!(names, vec!["aa.txt", "mm.txt", "zz.txt"]);
        } else {
            panic!("expected list result");
        }

        let _ = std::fs::remove_dir_all(root);
    }
}
