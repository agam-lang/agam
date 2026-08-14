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

/// Register all builtin `Console` effect handlers for stdin/stdout/stderr.
pub fn register_console_handlers(table: &mut EffectHandlerTable) {
    table.register("Console", "print", console_print);
    table.register("Console", "println", console_println);
    table.register("Console", "read_line", console_read_line);
    table.register("Console", "eprint", console_eprint);
    table.register("Console", "eprintln", console_eprintln);
}

/// Register all builtin `Network` effect handlers.
pub fn register_network_handlers(table: &mut EffectHandlerTable) {
    table.register("Network", "connect", net_connect);
    table.register("Network", "listen", net_listen);
    table.register("Network", "accept", net_accept);
    table.register("Network", "send", net_send);
    table.register("Network", "recv", net_recv);
    table.register("Network", "close", net_close);
}

/// Register all builtin `Environment` effect handlers.
pub fn register_env_handlers(table: &mut EffectHandlerTable) {
    table.register("Environment", "get_var", env_get_var);
    table.register("Environment", "set_var", env_set_var);
    table.register("Environment", "remove_var", env_remove_var);
    table.register("Environment", "current_dir", env_current_dir);
    table.register("Environment", "args", env_args);
}

/// Register all builtin `Process` effect handlers.
pub fn register_process_handlers(table: &mut EffectHandlerTable) {
    table.register("Process", "run", process_run);
    table.register("Process", "pid", process_pid);
    table.register("Process", "exit", process_exit);
}

/// Register all builtin effect handlers for all `agam_std` modules.
pub fn register_all_builtin_handlers(table: &mut EffectHandlerTable) {
    register_filesystem_handlers(table);
    register_console_handlers(table);
    register_network_handlers(table);
    register_env_handlers(table);
    register_process_handlers(table);
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

fn console_print(args: &[EffectValue]) -> Result<EffectValue, EffectError> {
    let msg = require_string_arg("Console", "print", args, 0)?;
    print!("{}", msg);
    Ok(EffectValue::Unit)
}

fn console_println(args: &[EffectValue]) -> Result<EffectValue, EffectError> {
    let msg = require_string_arg("Console", "println", args, 0)?;
    println!("{}", msg);
    Ok(EffectValue::Unit)
}

fn console_read_line(_args: &[EffectValue]) -> Result<EffectValue, EffectError> {
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .map_err(|e| EffectError {
            effect: "Console".into(),
            operation: "read_line".into(),
            message: e.to_string(),
        })?;
    // Strip trailing newline
    if line.ends_with('\n') {
        line.pop();
        if line.ends_with('\r') {
            line.pop();
        }
    }
    Ok(EffectValue::String(line))
}

fn console_eprint(args: &[EffectValue]) -> Result<EffectValue, EffectError> {
    let msg = require_string_arg("Console", "eprint", args, 0)?;
    eprint!("{}", msg);
    Ok(EffectValue::Unit)
}

fn console_eprintln(args: &[EffectValue]) -> Result<EffectValue, EffectError> {
    let msg = require_string_arg("Console", "eprintln", args, 0)?;
    eprintln!("{}", msg);
    Ok(EffectValue::Unit)
}

fn require_int_arg(
    effect: &str,
    op: &str,
    args: &[EffectValue],
    index: usize,
) -> Result<i64, EffectError> {
    match args.get(index) {
        Some(EffectValue::Int(i)) => Ok(*i),
        _ => Err(EffectError {
            effect: effect.to_string(),
            operation: op.to_string(),
            message: format!("expected integer argument at position {index}"),
        }),
    }
}

// ── Network effect handlers ──────────────────────────────────────────

fn net_connect(args: &[EffectValue]) -> Result<EffectValue, EffectError> {
    let addr = require_string_arg("Network", "connect", args, 0)?;
    let id = crate::net::global_net_manager()
        .lock()
        .unwrap()
        .connect(&addr)
        .map_err(|e| EffectError {
            effect: "Network".into(),
            operation: "connect".into(),
            message: e.to_string(),
        })?;
    Ok(EffectValue::Int(id))
}

fn net_listen(args: &[EffectValue]) -> Result<EffectValue, EffectError> {
    let addr = require_string_arg("Network", "listen", args, 0)?;
    let id = crate::net::global_net_manager()
        .lock()
        .unwrap()
        .listen(&addr)
        .map_err(|e| EffectError {
            effect: "Network".into(),
            operation: "listen".into(),
            message: e.to_string(),
        })?;
    Ok(EffectValue::Int(id))
}

fn net_accept(args: &[EffectValue]) -> Result<EffectValue, EffectError> {
    let listener_id = require_int_arg("Network", "accept", args, 0)?;
    let id = crate::net::global_net_manager()
        .lock()
        .unwrap()
        .accept(listener_id)
        .map_err(|e| EffectError {
            effect: "Network".into(),
            operation: "accept".into(),
            message: e.to_string(),
        })?;
    Ok(EffectValue::Int(id))
}

fn net_send(args: &[EffectValue]) -> Result<EffectValue, EffectError> {
    let stream_id = require_int_arg("Network", "send", args, 0)?;
    let data = require_string_arg("Network", "send", args, 1)?;
    let bytes = crate::net::global_net_manager()
        .lock()
        .unwrap()
        .send(stream_id, data.as_bytes())
        .map_err(|e| EffectError {
            effect: "Network".into(),
            operation: "send".into(),
            message: e.to_string(),
        })?;
    Ok(EffectValue::Int(bytes as i64))
}

fn net_recv(args: &[EffectValue]) -> Result<EffectValue, EffectError> {
    let stream_id = require_int_arg("Network", "recv", args, 0)?;
    let max_bytes = require_int_arg("Network", "recv", args, 1)? as usize;
    let bytes = crate::net::global_net_manager()
        .lock()
        .unwrap()
        .recv(stream_id, max_bytes)
        .map_err(|e| EffectError {
            effect: "Network".into(),
            operation: "recv".into(),
            message: e.to_string(),
        })?;
    Ok(EffectValue::String(String::from_utf8_lossy(&bytes).to_string()))
}

fn net_close(args: &[EffectValue]) -> Result<EffectValue, EffectError> {
    let id = require_int_arg("Network", "close", args, 0)?;
    let closed = crate::net::global_net_manager()
        .lock()
        .unwrap()
        .close(id);
    Ok(EffectValue::Bool(closed))
}

// ── Environment effect handlers ──────────────────────────────────────

fn env_get_var(args: &[EffectValue]) -> Result<EffectValue, EffectError> {
    let key = require_string_arg("Environment", "get_var", args, 0)?;
    let val = crate::env::get_var(&key).map_err(|e| EffectError {
        effect: "Environment".into(),
        operation: "get_var".into(),
        message: e.to_string(),
    })?;
    Ok(EffectValue::String(val))
}

fn env_set_var(args: &[EffectValue]) -> Result<EffectValue, EffectError> {
    let key = require_string_arg("Environment", "set_var", args, 0)?;
    let val = require_string_arg("Environment", "set_var", args, 1)?;
    crate::env::set_var(&key, &val);
    Ok(EffectValue::Unit)
}

fn env_remove_var(args: &[EffectValue]) -> Result<EffectValue, EffectError> {
    let key = require_string_arg("Environment", "remove_var", args, 0)?;
    crate::env::remove_var(&key);
    Ok(EffectValue::Unit)
}

fn env_current_dir(_args: &[EffectValue]) -> Result<EffectValue, EffectError> {
    let p = crate::env::current_dir().map_err(|e| EffectError {
        effect: "Environment".into(),
        operation: "current_dir".into(),
        message: e.to_string(),
    })?;
    Ok(EffectValue::String(p.to_string_lossy().to_string()))
}

fn env_args(_args: &[EffectValue]) -> Result<EffectValue, EffectError> {
    let list = crate::env::args()
        .into_iter()
        .map(EffectValue::String)
        .collect();
    Ok(EffectValue::List(list))
}

// ── Process effect handlers ──────────────────────────────────────────

fn process_run(args: &[EffectValue]) -> Result<EffectValue, EffectError> {
    let cmd = require_string_arg("Process", "run", args, 0)?;
    let raw_args = match args.get(1) {
        Some(EffectValue::List(list)) => list
            .iter()
            .filter_map(|v| match v {
                EffectValue::String(s) => Some(s.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    let output = crate::process::run(&cmd, &raw_args).map_err(|e| EffectError {
        effect: "Process".into(),
        operation: "run".into(),
        message: e.to_string(),
    })?;
    Ok(EffectValue::Int(output.status as i64))
}

fn process_pid(_args: &[EffectValue]) -> Result<EffectValue, EffectError> {
    Ok(EffectValue::Int(crate::process::pid() as i64))
}

fn process_exit(args: &[EffectValue]) -> Result<EffectValue, EffectError> {
    let code = require_int_arg("Process", "exit", args, 0)? as i32;
    crate::process::exit(code);
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
    fn builtin_table_has_all_ops() {
        let table = builtin_handler_table();
        assert_eq!(table.len(), 28); // 9 FileSystem + 5 Console + 6 Network + 5 Environment + 3 Process
        assert!(table.get("FileSystem", "exists").is_some());
        assert!(table.get("FileSystem", "read_to_string").is_some());
        assert!(table.get("FileSystem", "write_string").is_some());
        assert!(table.get("FileSystem", "list_dir").is_some());
        assert!(table.get("Console", "print").is_some());
        assert!(table.get("Console", "println").is_some());
        assert!(table.get("Console", "read_line").is_some());
        assert!(table.get("Console", "eprint").is_some());
        assert!(table.get("Console", "eprintln").is_some());
        assert!(table.get("Network", "connect").is_some());
        assert!(table.get("Network", "listen").is_some());
        assert!(table.get("Environment", "get_var").is_some());
        assert!(table.get("Environment", "current_dir").is_some());
        assert!(table.get("Process", "pid").is_some());
    }

    #[test]
    fn dispatch_env_and_process_effects() {
        let table = builtin_handler_table();
        let pid_res = table
            .dispatch("Process", "pid", &[])
            .expect("pid should succeed");
        if let EffectValue::Int(pid) = pid_res {
            assert!(pid > 0);
        } else {
            panic!("expected int pid");
        }

        table
            .dispatch(
                "Environment",
                "set_var",
                &[
                    EffectValue::String("AGAM_EFFECT_TEST".into()),
                    EffectValue::String("12345".into()),
                ],
            )
            .expect("set_var should succeed");

        let var_res = table
            .dispatch(
                "Environment",
                "get_var",
                &[EffectValue::String("AGAM_EFFECT_TEST".into())],
            )
            .expect("get_var should succeed");
        assert_eq!(var_res, EffectValue::String("12345".into()));
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
