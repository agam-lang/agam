//! # agam_notebook
//!
//! Jupyter-like notebook, REPL, and headless execution contracts.

use serde::{Deserialize, Serialize};

/// Backend selection for REPL and headless execution requests.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum HeadlessExecutionBackend {
    Auto,
    C,
    Llvm,
    #[default]
    Jit,
}

/// Explicit limits and capability gates for one headless execution request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HeadlessExecutionPolicy {
    #[serde(default = "default_headless_max_source_bytes")]
    pub max_source_bytes: usize,
    #[serde(default = "default_headless_max_arg_count")]
    pub max_arg_count: usize,
    #[serde(default = "default_headless_max_total_arg_bytes")]
    pub max_total_arg_bytes: usize,
    #[serde(default = "default_headless_max_runtime_ms")]
    pub max_runtime_ms: u64,
    #[serde(default = "default_headless_max_memory_bytes")]
    pub max_memory_bytes: u64,
    #[serde(default)]
    pub inherit_environment: bool,
    #[serde(default)]
    pub allow_native_backends: bool,
    /// Deny outbound network access in the sandbox.
    #[serde(default = "default_true")]
    pub deny_network: bool,
    /// Deny child process spawning in the sandbox.
    #[serde(default = "default_true")]
    pub deny_process_spawn: bool,
    /// Sandbox enforcement level: "none", "process", or "full".
    #[serde(default = "default_headless_sandbox_level")]
    pub sandbox_level: String,
}

impl Default for HeadlessExecutionPolicy {
    fn default() -> Self {
        Self {
            max_source_bytes: default_headless_max_source_bytes(),
            max_arg_count: default_headless_max_arg_count(),
            max_total_arg_bytes: default_headless_max_total_arg_bytes(),
            max_runtime_ms: default_headless_max_runtime_ms(),
            max_memory_bytes: default_headless_max_memory_bytes(),
            inherit_environment: false,
            allow_native_backends: false,
            deny_network: true,
            deny_process_spawn: true,
            sandbox_level: default_headless_sandbox_level(),
        }
    }
}

/// Strict JSON contract for one headless Agam execution request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HeadlessExecutionRequest {
    pub source: String,
    #[serde(default = "default_headless_filename")]
    pub filename: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub backend: HeadlessExecutionBackend,
    #[serde(default = "default_headless_opt_level")]
    pub opt_level: u8,
    #[serde(default)]
    pub fast: bool,
    #[serde(default)]
    pub policy: HeadlessExecutionPolicy,
}

impl Default for HeadlessExecutionRequest {
    fn default() -> Self {
        Self {
            source: String::new(),
            filename: default_headless_filename(),
            args: Vec::new(),
            backend: HeadlessExecutionBackend::default(),
            opt_level: default_headless_opt_level(),
            fast: false,
            policy: HeadlessExecutionPolicy::default(),
        }
    }
}

/// Structured response returned by the headless execution surface.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HeadlessExecutionResponse {
    pub success: bool,
    pub filename: String,
    pub backend: HeadlessExecutionBackend,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl HeadlessExecutionResponse {
    pub fn process_result(
        request: &HeadlessExecutionRequest,
        exit_code: i32,
        stdout: String,
        stderr: String,
    ) -> Self {
        Self {
            success: exit_code == 0,
            filename: request.filename.clone(),
            backend: request.backend,
            exit_code: Some(exit_code),
            stdout,
            stderr,
            error: None,
        }
    }

    pub fn execution_error(
        request: &HeadlessExecutionRequest,
        error: impl Into<String>,
        stderr: String,
    ) -> Self {
        Self {
            success: false,
            filename: request.filename.clone(),
            backend: request.backend,
            exit_code: None,
            stdout: String::new(),
            stderr,
            error: Some(error.into()),
        }
    }
}

pub fn default_headless_filename() -> String {
    "snippet.agam".into()
}

pub fn default_headless_opt_level() -> u8 {
    2
}

pub fn default_headless_max_source_bytes() -> usize {
    1024 * 1024
}

pub fn default_headless_max_arg_count() -> usize {
    64
}

pub fn default_headless_max_total_arg_bytes() -> usize {
    16 * 1024
}

pub fn default_headless_max_runtime_ms() -> u64 {
    30_000
}

pub fn default_headless_max_memory_bytes() -> u64 {
    1024 * 1024 * 1024
}

pub fn default_headless_sandbox_level() -> String {
    "process".into()
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_defaults_to_jit_filename_and_opt_level() {
        let request = HeadlessExecutionRequest::default();
        assert_eq!(request.filename, "snippet.agam");
        assert_eq!(request.backend, HeadlessExecutionBackend::Jit);
        assert_eq!(request.opt_level, 2);
        assert!(request.args.is_empty());
        assert!(!request.fast);
        assert_eq!(request.policy.max_source_bytes, 1024 * 1024);
        assert_eq!(request.policy.max_arg_count, 64);
        assert_eq!(request.policy.max_total_arg_bytes, 16 * 1024);
        assert_eq!(request.policy.max_runtime_ms, 30_000);
        assert_eq!(request.policy.max_memory_bytes, 1024 * 1024 * 1024);
        assert!(!request.policy.inherit_environment);
        assert!(!request.policy.allow_native_backends);
        assert!(request.policy.deny_network);
        assert!(request.policy.deny_process_spawn);
        assert_eq!(request.policy.sandbox_level, "process");
    }

    #[test]
    fn request_json_rejects_unknown_fields() {
        let error = serde_json::from_str::<HeadlessExecutionRequest>(
            r#"{"source":"fn main(): return 0","unknown":true}"#,
        )
        .expect_err("unknown fields should be rejected");
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn response_process_result_marks_exit_code_zero_as_success() {
        let request = HeadlessExecutionRequest {
            source: "fn main(): return 0".into(),
            ..HeadlessExecutionRequest::default()
        };
        let response =
            HeadlessExecutionResponse::process_result(&request, 0, "ok".into(), String::new());
        assert!(response.success);
        assert_eq!(response.exit_code, Some(0));
        assert_eq!(response.stdout, "ok");
        assert!(response.error.is_none());
    }

    #[test]
    fn policy_json_rejects_unknown_fields() {
        let error = serde_json::from_str::<HeadlessExecutionRequest>(
            r#"{"source":"fn main(): return 0","policy":{"unknown":true}}"#,
        )
        .expect_err("unknown policy fields should be rejected");
        assert!(error.to_string().contains("unknown field"));
    }
}
