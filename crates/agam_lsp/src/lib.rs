//! # agam_lsp
//!
//! Language Server Protocol implementation.

use std::io::{self, BufRead, BufReader, Write};

use serde_json::{Value, json};

pub fn run_stdio() -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = stdout.lock();

    loop {
        let Some(payload) = read_message(&mut reader)? else {
            break;
        };
        let message: Value =
            serde_json::from_slice(&payload).map_err(|e| format!("invalid LSP payload: {e}"))?;
        let should_exit = message.get("method").and_then(Value::as_str) == Some("exit");
        if let Some(response) = handle_message(&message)? {
            write_message(&mut writer, &response)?;
        }
        if should_exit {
            break;
        }
    }

    Ok(())
}

fn handle_message(message: &Value) -> Result<Option<Value>, String> {
    let Some(method) = message.get("method").and_then(Value::as_str) else {
        return Ok(None);
    };

    match method {
        "initialize" => {
            let id = message
                .get("id")
                .cloned()
                .ok_or_else(|| "initialize request is missing an id".to_string())?;
            Ok(Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "capabilities": {},
                    "serverInfo": {
                        "name": "agam_lsp",
                        "version": env!("CARGO_PKG_VERSION"),
                    }
                }
            })))
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
        "initialized" | "exit" | "textDocument/didOpen" | "textDocument/didChange"
        | "textDocument/didClose" => Ok(None),
        _ => Ok(message.get("id").cloned().map(|id| {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32601,
                    "message": format!("method `{method}` is not implemented"),
                }
            })
        })),
    }
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

    #[test]
    fn handle_initialize_returns_server_info() {
        let response = handle_message(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }))
        .expect("handle initialize")
        .expect("initialize response");

        assert_eq!(response["id"], 1);
        assert_eq!(response["result"]["serverInfo"]["name"], "agam_lsp");
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

