//! # agam_lsp
//!
//! Language Server Protocol implementation.

use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;

use serde_json::{Value, json};

const TEXT_DOCUMENT_SYNC_FULL: u8 = 1;
const LSP_METHOD_NOT_FOUND: i64 = -32601;
const LSP_INVALID_PARAMS: i64 = -32602;
const LSP_SERVER_NOT_INITIALIZED: i64 = -32002;

#[derive(Default)]
struct ServerState {
    initialized: bool,
    workspace: Option<agam_pkg::WorkspaceSession>,
    open_documents: HashMap<String, String>,
}

pub fn run_stdio() -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = stdout.lock();
    let mut state = ServerState::default();

    loop {
        let Some(payload) = read_message(&mut reader)? else {
            break;
        };
        let message: Value =
            serde_json::from_slice(&payload).map_err(|e| format!("invalid LSP payload: {e}"))?;
        let should_exit = message.get("method").and_then(Value::as_str) == Some("exit");
        if let Some(response) = handle_message(&mut state, &message)? {
            write_message(&mut writer, &response)?;
        }
        if should_exit {
            break;
        }
    }

    Ok(())
}

fn handle_message(state: &mut ServerState, message: &Value) -> Result<Option<Value>, String> {
    let Some(method) = message.get("method").and_then(Value::as_str) else {
        return Ok(None);
    };

    match method {
        "initialize" => {
            let id = message
                .get("id")
                .cloned()
                .ok_or_else(|| "initialize request is missing an id".to_string())?;
            Ok(Some(handle_initialize(
                state,
                id,
                message.get("params").unwrap_or(&Value::Null),
            )))
        }
        "shutdown" => {
            let id = message
                .get("id")
                .cloned()
                .ok_or_else(|| "shutdown request is missing an id".to_string())?;
            Ok(Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": null,
            })))
        }
        "initialized" | "exit" => Ok(None),
        "textDocument/didOpen" => {
            apply_did_open(state, message.get("params").unwrap_or(&Value::Null));
            Ok(None)
        }
        "textDocument/didChange" => {
            apply_did_change(state, message.get("params").unwrap_or(&Value::Null));
            Ok(None)
        }
        "textDocument/didClose" => {
            apply_did_close(state, message.get("params").unwrap_or(&Value::Null));
            Ok(None)
        }
        "textDocument/formatting" => {
            let id = message
                .get("id")
                .cloned()
                .ok_or_else(|| "formatting request is missing an id".to_string())?;
            Ok(Some(handle_formatting(
                state,
                id,
                message.get("params").unwrap_or(&Value::Null),
            )))
        }
        _ => Ok(message.get("id").cloned().map(|id| {
            error_response(
                id,
                LSP_METHOD_NOT_FOUND,
                format!("method `{method}` is not implemented"),
            )
        })),
    }
}

fn handle_initialize(state: &mut ServerState, id: Value, params: &Value) -> Value {
    match initialize_workspace(params) {
        Ok(workspace) => {
            state.initialized = true;
            state.workspace = workspace;
            state.open_documents.clear();

            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "capabilities": {
                        "textDocumentSync": TEXT_DOCUMENT_SYNC_FULL,
                        "documentFormattingProvider": true,
                        "workspace": {
                            "workspaceFolders": {
                                "supported": true,
                                "changeNotifications": false,
                            }
                        },
                        "experimental": {
                            "workspace": workspace_metadata(state.workspace.as_ref()),
                        }
                    },
                    "serverInfo": {
                        "name": "agam_lsp",
                        "version": env!("CARGO_PKG_VERSION"),
                    }
                }
            })
        }
        Err(error) => error_response(id, LSP_INVALID_PARAMS, error),
    }
}

fn handle_formatting(state: &ServerState, id: Value, params: &Value) -> Value {
    if !state.initialized {
        return error_response(
            id,
            LSP_SERVER_NOT_INITIALIZED,
            "server must receive `initialize` before formatting requests",
        );
    }

    match format_document(state, params) {
        Ok(edits) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": edits,
        }),
        Err(error) => error_response(id, LSP_INVALID_PARAMS, error),
    }
}

fn initialize_workspace(params: &Value) -> Result<Option<agam_pkg::WorkspaceSession>, String> {
    let Some(path) = workspace_path_from_initialize_params(params)? else {
        return Ok(None);
    };
    agam_pkg::resolve_workspace_session(Some(path)).map(Some)
}

fn workspace_path_from_initialize_params(params: &Value) -> Result<Option<PathBuf>, String> {
    if let Some(workspace_folders) = params.get("workspaceFolders").and_then(Value::as_array) {
        for folder in workspace_folders {
            if let Some(uri) = folder.get("uri").and_then(Value::as_str) {
                return path_from_lsp_file_uri(uri).map(Some);
            }
        }
    }

    if let Some(root_uri) = params.get("rootUri").and_then(Value::as_str) {
        if !root_uri.trim().is_empty() {
            return path_from_lsp_file_uri(root_uri).map(Some);
        }
    }

    if let Some(root_path) = params.get("rootPath").and_then(Value::as_str) {
        if !root_path.trim().is_empty() {
            return Ok(Some(PathBuf::from(root_path)));
        }
    }

    Ok(None)
}

fn workspace_metadata(session: Option<&agam_pkg::WorkspaceSession>) -> Value {
    let Some(session) = session else {
        return Value::Null;
    };

    let manifest = session.manifest.as_ref();
    let manifest_path = session
        .layout
        .manifest_path
        .as_ref()
        .map(|path| path.display().to_string());

    json!({
        "projectName": session.layout.project_name,
        "rootPath": session.layout.root.display().to_string(),
        "manifestPath": manifest_path,
        "entryFile": session.layout.entry_file.display().to_string(),
        "sourceFileCount": session.layout.source_files.len(),
        "testFileCount": session.layout.test_files.len(),
        "manifestFormatVersion": manifest.map(|manifest| manifest.format_version),
        "dependencyCount": manifest.map(|manifest| {
            manifest.dependencies.len()
                + manifest.dev_dependencies.len()
                + manifest.build_dependencies.len()
        }),
        "environmentCount": manifest.map(|manifest| manifest.environments.len()),
    })
}

fn apply_did_open(state: &mut ServerState, params: &Value) {
    let Some(document) = params.get("textDocument") else {
        return;
    };
    let Some(uri) = document.get("uri").and_then(Value::as_str) else {
        return;
    };
    let Some(text) = document.get("text").and_then(Value::as_str) else {
        return;
    };
    state.open_documents.insert(uri.to_string(), text.to_string());
}

fn apply_did_change(state: &mut ServerState, params: &Value) {
    let Some(uri) = params
        .get("textDocument")
        .and_then(|document| document.get("uri"))
        .and_then(Value::as_str)
    else {
        return;
    };
    let Some(content_changes) = params.get("contentChanges").and_then(Value::as_array) else {
        return;
    };
    let Some(text) = content_changes
        .iter()
        .rev()
        .filter_map(|change| change.get("text").and_then(Value::as_str))
        .next()
    else {
        return;
    };
    state.open_documents.insert(uri.to_string(), text.to_string());
}

fn apply_did_close(state: &mut ServerState, params: &Value) {
    let Some(uri) = params
        .get("textDocument")
        .and_then(|document| document.get("uri"))
        .and_then(Value::as_str)
    else {
        return;
    };
    state.open_documents.remove(uri);
}

fn format_document(state: &ServerState, params: &Value) -> Result<Vec<Value>, String> {
    let uri = params
        .get("textDocument")
        .and_then(|document| document.get("uri"))
        .and_then(Value::as_str)
        .ok_or_else(|| "formatting request is missing `textDocument.uri`".to_string())?;
    let path = path_from_lsp_file_uri(uri)?;

    let source = match state.open_documents.get(uri) {
        Some(source) => source.clone(),
        None => fs::read_to_string(&path)
            .map_err(|e| format!("failed to read `{}`: {e}", path.display()))?,
    };
    let formatted = agam_fmt::format_source(&source);
    if !formatted.changed {
        return Ok(Vec::new());
    }

    let (end_line, end_character) = document_end_position(&source);
    Ok(vec![json!({
        "range": {
            "start": {
                "line": 0,
                "character": 0,
            },
            "end": {
                "line": end_line,
                "character": end_character,
            }
        },
        "newText": formatted.output,
    })])
}

fn document_end_position(source: &str) -> (u64, u64) {
    let mut line = 0u64;
    let mut character = 0u64;

    for ch in source.chars() {
        if ch == '\n' {
            line += 1;
            character = 0;
        } else if ch != '\r' {
            character += 1;
        }
    }

    (line, character)
}

fn path_from_lsp_file_uri(uri: &str) -> Result<PathBuf, String> {
    let encoded_path = uri
        .strip_prefix("file://")
        .ok_or_else(|| format!("unsupported LSP URI `{uri}`; only `file://` is supported"))?;
    let encoded_path = encoded_path.strip_prefix("localhost/").unwrap_or(encoded_path);
    let decoded_path = percent_decode(encoded_path)?;

    if cfg!(windows) {
        let without_drive_slash = match decoded_path.strip_prefix('/') {
            Some(path) if path.as_bytes().get(1) == Some(&b':') => path,
            _ => decoded_path.as_str(),
        };
        return Ok(PathBuf::from(without_drive_slash.replace('/', "\\")));
    }

    Ok(PathBuf::from(decoded_path))
}

fn percent_decode(input: &str) -> Result<String, String> {
    let bytes = input.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err(format!("invalid percent-encoding in LSP URI `{input}`"));
            }
            let high = decode_hex_digit(bytes[index + 1], input)?;
            let low = decode_hex_digit(bytes[index + 2], input)?;
            output.push((high << 4) | low);
            index += 3;
            continue;
        }

        output.push(bytes[index]);
        index += 1;
    }

    String::from_utf8(output).map_err(|e| format!("invalid UTF-8 in LSP URI `{input}`: {e}"))
}

fn decode_hex_digit(byte: u8, input: &str) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(10 + (byte - b'a')),
        b'A'..=b'F' => Ok(10 + (byte - b'A')),
        _ => Err(format!(
            "invalid percent-encoding in LSP URI `{input}`"
        )),
    }
}

fn error_response(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message.into(),
        }
    })
}

fn read_message<R: BufRead>(reader: &mut R) -> Result<Option<Vec<u8>>, String> {
    let mut content_length = None;

    loop {
        let mut line = String::new();
        let read = reader
            .read_line(&mut line)
            .map_err(|e| format!("failed to read LSP header: {e}"))?;
        if read == 0 {
            return if content_length.is_some() {
                Err("unexpected EOF while reading LSP headers".into())
            } else {
                Ok(None)
            };
        }

        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            let parsed = value
                .trim()
                .parse::<usize>()
                .map_err(|e| format!("invalid LSP Content-Length header `{trimmed}`: {e}"))?;
            content_length = Some(parsed);
        }
    }

    let length = content_length.ok_or_else(|| "missing LSP Content-Length header".to_string())?;
    let mut payload = vec![0u8; length];
    reader
        .read_exact(&mut payload)
        .map_err(|e| format!("failed to read LSP payload: {e}"))?;
    Ok(Some(payload))
}

fn write_message<W: Write>(writer: &mut W, value: &Value) -> Result<(), String> {
    let payload =
        serde_json::to_vec(value).map_err(|e| format!("failed to encode LSP response: {e}"))?;
    write!(writer, "Content-Length: {}\r\n\r\n", payload.len())
        .map_err(|e| format!("failed to write LSP header: {e}"))?;
    writer
        .write_all(&payload)
        .map_err(|e| format!("failed to write LSP payload: {e}"))?;
    writer
        .flush()
        .map_err(|e| format!("failed to flush LSP response: {e}"))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    fn temp_dir(prefix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "agam_lsp_{prefix}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time should move forward")
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn file_uri(path: &std::path::Path) -> String {
        let raw = path.to_string_lossy().replace('\\', "/").replace(' ', "%20");
        if raw.starts_with('/') {
            format!("file://{raw}")
        } else {
            format!("file:///{raw}")
        }
    }

    #[test]
    fn handle_initialize_returns_server_info_and_workspace_metadata() {
        let dir = temp_dir("initialize");
        let entry = dir.join("src").join("main.agam");
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        fs::write(
            &entry,
            "@lang.advance\nfn main() -> i32 {\n    return 0;\n}\n",
        )
        .expect("write entry");
        let manifest = agam_pkg::scaffold_workspace_manifest("lsp-workspace");
        agam_pkg::write_workspace_manifest_to_path(&dir.join("agam.toml"), &manifest)
            .expect("write manifest");

        let mut state = ServerState::default();
        let response = handle_message(
            &mut state,
            &json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "rootPath": dir.display().to_string(),
                }
            }),
        )
        .expect("handle initialize")
        .expect("initialize response");

        assert_eq!(response["id"], 1);
        assert_eq!(response["result"]["serverInfo"]["name"], "agam_lsp");
        assert_eq!(
            response["result"]["capabilities"]["experimental"]["workspace"]["projectName"],
            "lsp-workspace"
        );
        assert_eq!(
            state
                .workspace
                .as_ref()
                .expect("workspace should resolve")
                .manifest
                .as_ref()
                .expect("manifest should exist")
                .project
                .name,
            "lsp-workspace"
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn handle_initialize_rejects_invalid_manifest_metadata() {
        let dir = temp_dir("invalid_initialize");
        fs::write(
            dir.join("agam.toml"),
            r#"
[project]
name = "broken"
version = "0.1.0"
agam = "0.1"

[dependencies.tensor-core]
rev = "abc123"
"#,
        )
        .expect("write invalid manifest");

        let mut state = ServerState::default();
        let response = handle_message(
            &mut state,
            &json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "rootPath": dir.display().to_string(),
                }
            }),
        )
        .expect("handle initialize")
        .expect("initialize response");

        assert_eq!(response["error"]["code"], LSP_INVALID_PARAMS);
        assert!(
            response["error"]["message"]
                .as_str()
                .expect("error message")
                .contains("requires `git`")
        );
        assert!(!state.initialized);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn formatting_request_uses_latest_open_document_text() {
        let dir = temp_dir("formatting");
        let entry = dir.join("src").join("main.agam");
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        fs::write(&entry, "fn main() {\n    return 0;\n}\n").expect("write entry");
        let manifest = agam_pkg::scaffold_workspace_manifest("formatting-workspace");
        agam_pkg::write_workspace_manifest_to_path(&dir.join("agam.toml"), &manifest)
            .expect("write manifest");

        let uri = file_uri(&entry);
        let mut state = ServerState::default();
        handle_message(
            &mut state,
            &json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "rootPath": dir.display().to_string(),
                }
            }),
        )
        .expect("initialize should succeed");
        handle_message(
            &mut state,
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "languageId": "agam",
                        "version": 1,
                        "text": "fn main() {\n    return 0;\n}\n",
                    }
                }
            }),
        )
        .expect("didOpen should succeed");
        handle_message(
            &mut state,
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "version": 2,
                    },
                    "contentChanges": [
                        {
                            "text": "fn main() {   \n    return 0; \t\n}",
                        }
                    ]
                }
            }),
        )
        .expect("didChange should succeed");

        let response = handle_message(
            &mut state,
            &json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/formatting",
                "params": {
                    "textDocument": {
                        "uri": uri,
                    },
                    "options": {
                        "tabSize": 4,
                        "insertSpaces": true,
                    }
                }
            }),
        )
        .expect("formatting should succeed")
        .expect("formatting response");

        assert_eq!(
            response["result"][0]["newText"],
            "fn main() {\n    return 0;\n}\n"
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn read_message_parses_content_length_framing() {
        let payload = br#"{"method":"exit"}"#;
        let raw = format!(
            "Content-Length: {}\r\n\r\n{}",
            payload.len(),
            std::str::from_utf8(payload).expect("utf8 payload")
        );
        let mut reader = Cursor::new(raw.into_bytes());
        let decoded = read_message(&mut reader)
            .expect("read message")
            .expect("message payload");
        assert_eq!(decoded, payload);
    }
}
