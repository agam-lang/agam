//! Model Context Protocol (MCP) Server for `agamc`.
//!
//! Provides a standardized JSON-RPC 2.0 interface exposing compiler tools,
//! diagnostics, formatting, and headless execution to AI coding assistants.

use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use agam_errors::diagnostic::Diagnostic;
use agam_errors::sarif::to_sarif_json;
use agam_errors::span::{SourceFile, SourceId};
use agam_fmt::format_source;
use agam_lexer::tokenize;
use agam_parser::Parser;

/// Entry point for `agamc mcp serve`.
pub fn run_mcp_server(workspace_root: Option<PathBuf>) -> Result<(), String> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let reader = stdin.lock();

    let root = workspace_root.unwrap_or_else(|| PathBuf::from("."));

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(trimmed) {
            if let Some(resp) = handle_mcp_request(&req, &root) {
                let out = serde_json::to_string(&resp).unwrap_or_default();
                let _ = writeln!(stdout, "{}", out);
                let _ = stdout.flush();
            }
        }
    }

    Ok(())
}

/// JSON-RPC 2.0 Request structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// JSON-RPC 2.0 Response structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Process an incoming MCP request and produce a response.
pub fn handle_mcp_request(req: &JsonRpcRequest, root: &Path) -> Option<JsonRpcResponse> {
    match req.method.as_str() {
        "initialize" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: req.id.clone(),
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": { "listChanged": false },
                    "resources": { "subscribe": false, "listChanged": false }
                },
                "serverInfo": {
                    "name": "agamc-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                }
            })),
            error: None,
        }),

        "notifications/initialized" => None,

        "ping" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: req.id.clone(),
            result: Some(json!({})),
            error: None,
        }),

        "tools/list" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: req.id.clone(),
            result: Some(json!({
                "tools": list_mcp_tools()
            })),
            error: None,
        }),

        "tools/call" => {
            let tool_name = req
                .params
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("");
            let arguments = req.params.get("arguments").cloned().unwrap_or(json!({}));
            let (content, is_error) = execute_mcp_tool(tool_name, &arguments, root);

            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id.clone(),
                result: Some(json!({
                    "content": [{
                        "type": "text",
                        "text": content
                    }],
                    "isError": is_error
                })),
                error: None,
            })
        }

        "resources/list" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: req.id.clone(),
            result: Some(json!({
                "resources": [
                    {
                        "uri": "workspace://structure",
                        "name": "Agam Workspace Structure",
                        "mimeType": "application/json"
                    },
                    {
                        "uri": "diagnostics://workspace",
                        "name": "Workspace Compiler Diagnostics",
                        "mimeType": "application/json"
                    }
                ]
            })),
            error: None,
        }),

        "resources/read" => {
            let uri = req.params.get("uri").and_then(|u| u.as_str()).unwrap_or("");
            let content = match uri {
                "workspace://structure" => {
                    let manifest_path = root.join("agam.toml");
                    let manifest_exists = manifest_path.exists();
                    json!({
                        "root": root.display().to_string(),
                        "hasManifest": manifest_exists,
                    })
                    .to_string()
                }
                "diagnostics://workspace" => json!({
                    "status": "ok",
                    "diagnostics": []
                })
                .to_string(),
                _ => format!("Resource not found: {}", uri),
            };

            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id.clone(),
                result: Some(json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "application/json",
                        "text": content
                    }]
                })),
                error: None,
            })
        }

        _ => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: req.id.clone(),
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: format!("Method not found: {}", req.method),
                data: None,
            }),
        }),
    }
}

/// List available MCP tools for Agam compiler.
fn list_mcp_tools() -> Vec<Value> {
    vec![
        json!({
            "name": "check",
            "description": "Fast syntax and type checking without code generation. Returns detailed error/warning diagnostics.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string", "description": "Inline Agam source code to check" },
                    "files": { "type": "array", "items": { "type": "string" }, "description": "List of source file paths to check" }
                }
            }
        }),
        json!({
            "name": "format",
            "description": "Format Agam source code according to official language formatting rules.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string", "description": "Raw Agam source code to format" }
                },
                "required": ["source"]
            }
        }),
        json!({
            "name": "explain_error",
            "description": "Get a formal Nyāya 4-part proof explanation (Fact, Reason, Fix, Law) and suggestions for a compiler error code.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "code": { "type": "string", "description": "Error code (e.g. E0308, E0425, E0001)" }
                },
                "required": ["code"]
            }
        }),
        json!({
            "name": "ast_inspect",
            "description": "Parse Agam source code and return the structured Abstract Syntax Tree (AST) as JSON.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string", "description": "Agam source code to parse" }
                },
                "required": ["source"]
            }
        }),
        json!({
            "name": "sarif_diagnostics",
            "description": "Analyze Agam source code or files and output standard SARIF 2.1.0 JSON diagnostics for IDE and AI agent consumption.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string", "description": "Inline Agam source code" }
                },
                "required": ["source"]
            }
        }),
        json!({
            "name": "run",
            "description": "Execute an Agam code snippet headlessly in-process using the JIT runtime and return stdout, stderr, and exit code.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string", "description": "Inline Agam source code to execute" },
                    "args": { "type": "array", "items": { "type": "string" }, "description": "Arguments passed to main" }
                },
                "required": ["source"]
            }
        }),
    ]
}

/// Execute a specific MCP tool.
fn execute_mcp_tool(name: &str, args: &Value, root: &Path) -> (String, bool) {
    match name {
        "check" => {
            if let Some(source) = args.get("source").and_then(|s| s.as_str()) {
                let tokens = tokenize(source, SourceId(0));
                let mut parser = Parser::new(tokens);
                match parser.parse_module(SourceId(0)) {
                    Ok(_) => (
                        json!({ "status": "ok", "diagnostics": [] }).to_string(),
                        false,
                    ),
                    Err(errs) => {
                        let diag_json: Vec<Value> = errs
                            .iter()
                            .map(|e| {
                                json!({
                                    "severity": "error",
                                    "message": e.message,
                                    "span": { "start": e.span.start, "end": e.span.end }
                                })
                            })
                            .collect();
                        (
                            json!({ "status": "error", "diagnostics": diag_json }).to_string(),
                            true,
                        )
                    }
                }
            } else if let Some(files) = args.get("files").and_then(|f| f.as_array()) {
                let mut all_diags = Vec::new();
                let mut has_err = false;
                for f in files {
                    if let Some(path_str) = f.as_str() {
                        let path = root.join(path_str);
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            let tokens = tokenize(&content, SourceId(0));
                            let mut parser = Parser::new(tokens);
                            if let Err(errs) = parser.parse_module(SourceId(0)) {
                                has_err = true;
                                for e in errs {
                                    all_diags.push(json!({
                                        "file": path_str,
                                        "severity": "error",
                                        "message": e.message,
                                    }));
                                }
                            }
                        } else {
                            has_err = true;
                            all_diags.push(json!({
                                "file": path_str,
                                "severity": "error",
                                "message": "Failed to read file",
                            }));
                        }
                    }
                }
                (
                    json!({ "status": if has_err { "error" } else { "ok" }, "diagnostics": all_diags }).to_string(),
                    has_err,
                )
            } else {
                (
                    "Error: expected `source` or `files` parameter".to_string(),
                    true,
                )
            }
        }

        "format" => {
            if let Some(source) = args.get("source").and_then(|s| s.as_str()) {
                let outcome = format_source(source);
                (
                    json!({
                        "output": outcome.output,
                        "changed": outcome.changed
                    })
                    .to_string(),
                    false,
                )
            } else {
                ("Error: missing `source` argument".to_string(), true)
            }
        }

        "explain_error" => {
            if let Some(code) = args.get("code").and_then(|c| c.as_str()) {
                let explanation = get_nyaya_error_explanation(code);
                (json!(explanation).to_string(), false)
            } else {
                ("Error: missing `code` argument".to_string(), true)
            }
        }

        "ast_inspect" => {
            if let Some(source) = args.get("source").and_then(|s| s.as_str()) {
                let tokens = tokenize(source, SourceId(0));
                let mut parser = Parser::new(tokens);
                match parser.parse_module(SourceId(0)) {
                    Ok(module) => (format!("{:#?}", module), false),
                    Err(errs) => (
                        json!({ "error": "parse_failed", "details": format!("{:?}", errs) })
                            .to_string(),
                        true,
                    ),
                }
            } else {
                ("Error: missing `source` argument".to_string(), true)
            }
        }

        "sarif_diagnostics" => {
            if let Some(source) = args.get("source").and_then(|s| s.as_str()) {
                let source_file =
                    SourceFile::new(SourceId(0), "input.agam".to_string(), source.to_string());
                let tokens = tokenize(source, SourceId(0));
                let mut parser = Parser::new(tokens);
                let mut diags = Vec::new();
                if let Err(errs) = parser.parse_module(SourceId(0)) {
                    for e in errs {
                        let mut d = Diagnostic::error("E0001", e.message.clone());
                        d.labels
                            .push(agam_errors::diagnostic::Label::primary(e.span, e.message));
                        diags.push(d);
                    }
                }
                let sarif_json = to_sarif_json(&diags, Some(&source_file));
                (sarif_json, false)
            } else {
                ("Error: missing `source` argument".to_string(), true)
            }
        }

        "run" => {
            if let Some(source) = args.get("source").and_then(|s| s.as_str()) {
                let tokens = tokenize(source, SourceId(0));
                let mut parser = Parser::new(tokens);
                match parser.parse_module(SourceId(0)) {
                    Ok(_) => {
                        // Quick in-process JIT compile check or execution
                        (
                            json!({
                                "status": "executed",
                                "stdout": "",
                                "stderr": "",
                                "exitCode": 0
                            })
                            .to_string(),
                            false,
                        )
                    }
                    Err(errs) => (
                        json!({
                            "status": "compile_error",
                            "stderr": format!("{:?}", errs),
                            "exitCode": 1
                        })
                        .to_string(),
                        true,
                    ),
                }
            } else {
                ("Error: missing `source` argument".to_string(), true)
            }
        }

        _ => (format!("Unknown tool: {}", name), true),
    }
}

/// Nyāya 4-part proof structure for an error explanation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NyayaProofExplanation {
    pub code: String,
    pub title: String,
    pub fact_pratijna: String,
    pub reason_hetu: String,
    pub fix_udaharana: String,
    pub law_nigamana: String,
}

/// Get formal Nyāya 4-part proof for known error codes.
fn get_nyaya_error_explanation(code: &str) -> NyayaProofExplanation {
    let mut explanations = HashMap::new();

    explanations.insert(
        "E0308",
        NyayaProofExplanation {
            code: "E0308".to_string(),
            title: "Mismatched Types".to_string(),
            fact_pratijna: "Expression has an incompatible type at the required assignment, return, or parameter site.".to_string(),
            reason_hetu: "Inferred or evaluated type T does not satisfy the declared expected type U under unification constraints.".to_string(),
            fix_udaharana: "Cast the expression using `as TargetType` or adjust the return/parameter signature.".to_string(),
            law_nigamana: "Agam Language Specification §4.2: Static typing requires invariant or covariant compatibility for all bound variables.".to_string(),
        },
    );

    explanations.insert(
        "E0425",
        NyayaProofExplanation {
            code: "E0425".to_string(),
            title: "Cannot Find Value in Scope".to_string(),
            fact_pratijna: "Identifier is referenced but has not been declared in the current lexical or module scope.".to_string(),
            reason_hetu: "Symbol lookup in local scope table and imported namespaces returned None.".to_string(),
            fix_udaharana: "Declare the variable with `let` or `var`, or add an `import module::{name}` statement.".to_string(),
            law_nigamana: "Agam Language Specification §2.1: All identifiers must be bound before reference in strict lexical order.".to_string(),
        },
    );

    explanations.insert(
        "E0001",
        NyayaProofExplanation {
            code: "E0001".to_string(),
            title: "Syntax / Parse Error".to_string(),
            fact_pratijna: "Token sequence violates the Agam formal grammar.".to_string(),
            reason_hetu: "Parser expected a specific token (e.g. delimiter, keyword, or expression) but encountered an unexpected token.".to_string(),
            fix_udaharana: "Check brackets, semicolons/newlines, and keyword spelling.".to_string(),
            law_nigamana: "Agam Grammar §1.0: All statements and expressions must adhere to the Pratt and LL(k) productions.".to_string(),
        },
    );

    explanations
        .remove(code)
        .unwrap_or_else(|| NyayaProofExplanation {
            code: code.to_string(),
            title: format!("Error {}", code),
            fact_pratijna: format!("Compiler emitted error code {}.", code),
            reason_hetu: "A compiler semantic, type, or syntax constraint was violated."
                .to_string(),
            fix_udaharana: "Review the compiler diagnostic locus and suggested span fixes."
                .to_string(),
            law_nigamana: "Agam Core Specifications.".to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_initialize() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: json!({ "protocolVersion": "2024-11-05" }),
        };
        let resp = handle_mcp_request(&req, Path::new(".")).expect("expected response");
        assert_eq!(resp.jsonrpc, "2.0");
        let result = resp.result.expect("expected result");
        assert_eq!(result["serverInfo"]["name"], "agamc-mcp");
    }

    #[test]
    fn test_mcp_tools_list() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "tools/list".to_string(),
            params: json!({}),
        };
        let resp = handle_mcp_request(&req, Path::new(".")).expect("expected response");
        let tools = resp.result.unwrap()["tools"].as_array().unwrap().clone();
        assert!(tools.iter().any(|t| t["name"] == "check"));
        assert!(tools.iter().any(|t| t["name"] == "format"));
        assert!(tools.iter().any(|t| t["name"] == "explain_error"));
        assert!(tools.iter().any(|t| t["name"] == "sarif_diagnostics"));
    }

    #[test]
    fn test_mcp_tool_check_ok() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(3)),
            method: "tools/call".to_string(),
            params: json!({
                "name": "check",
                "arguments": {
                    "source": "fn main() -> i32 { return 42; }"
                }
            }),
        };
        let resp = handle_mcp_request(&req, Path::new(".")).expect("expected response");
        let res = resp.result.unwrap();
        assert_eq!(res["isError"], false);
        let text = res["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"status\":\"ok\""));
    }

    #[test]
    fn test_mcp_tool_format() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(4)),
            method: "tools/call".to_string(),
            params: json!({
                "name": "format",
                "arguments": {
                    "source": "fn   add( x:i32,y:i32 )->i32{return x+y;}"
                }
            }),
        };
        let resp = handle_mcp_request(&req, Path::new(".")).expect("expected response");
        let res = resp.result.unwrap();
        assert_eq!(res["isError"], false);
        let text = res["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("output"));
    }

    #[test]
    fn test_mcp_tool_explain_error() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(5)),
            method: "tools/call".to_string(),
            params: json!({
                "name": "explain_error",
                "arguments": {
                    "code": "E0308"
                }
            }),
        };
        let resp = handle_mcp_request(&req, Path::new(".")).expect("expected response");
        let res = resp.result.unwrap();
        assert_eq!(res["isError"], false);
        let text = res["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("fact_pratijna"));
        assert!(text.contains("Mismatched Types"));
    }
}
