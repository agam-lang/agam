//! # agam_ffi
//!
//! Foreign function interface bridges (Python, C++, Java).

use agam_notebook::{
    HeadlessExecutionBackend, HeadlessExecutionPolicy, HeadlessExecutionRequest,
    HeadlessExecutionResponse, default_headless_filename, default_headless_opt_level,
};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Default environment variable used to override the `agamc` executable path.
pub const AGAMC_EXECUTABLE_ENV: &str = "AGAMC_EXECUTABLE";

/// Thin client for invoking the `agamc exec --json` execution surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgamExecClient {
    executable: PathBuf,
}

impl Default for AgamExecClient {
    fn default() -> Self {
        Self::from_env_or_default()
    }
}

impl AgamExecClient {
    /// Create a client that invokes the provided `agamc` executable.
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
        }
    }

    /// Resolve the executable from `AGAMC_EXECUTABLE`, falling back to `agamc`.
    pub fn from_env_or_default() -> Self {
        let executable = std::env::var_os(AGAMC_EXECUTABLE_ENV)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("agamc"));
        Self::new(executable)
    }

    /// Return the configured executable path.
    pub fn executable(&self) -> &Path {
        &self.executable
    }

    /// Execute one strict JSON request through `agamc exec --json`.
    pub fn run_request(
        &self,
        request: &HeadlessExecutionRequest,
    ) -> Result<HeadlessExecutionResponse, AgamExecError> {
        let payload = serde_json::to_vec(request).map_err(AgamExecError::SerializeRequest)?;

        let mut command = Command::new(&self.executable);
        command.arg("exec").arg("--json");
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let mut child = command.spawn().map_err(AgamExecError::Spawn)?;
        if let Some(stdin) = child.stdin.as_mut() {
            stdin
                .write_all(&payload)
                .map_err(AgamExecError::WriteStdin)?;
        }
        let output = child.wait_with_output().map_err(AgamExecError::Wait)?;

        match serde_json::from_slice::<HeadlessExecutionResponse>(&output.stdout) {
            Ok(response) => Ok(response),
            Err(error) => Err(AgamExecError::ParseResponse {
                error,
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                status_code: output.status.code(),
            }),
        }
    }

    /// Execute one source string through the default request contract.
    pub fn run_source(
        &self,
        source: impl Into<String>,
    ) -> Result<HeadlessExecutionResponse, AgamExecError> {
        self.run_request(&HeadlessExecutionRequest {
            source: source.into(),
            filename: default_headless_filename(),
            args: Vec::new(),
            backend: HeadlessExecutionBackend::Jit,
            opt_level: default_headless_opt_level(),
            fast: false,
            policy: HeadlessExecutionPolicy::default(),
        })
    }
}

/// Small reusable execution tool abstraction for later Python/agent wrappers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgamReplTool {
    client: AgamExecClient,
    filename: String,
    backend: HeadlessExecutionBackend,
    opt_level: u8,
    fast: bool,
    args: Vec<String>,
}

impl Default for AgamReplTool {
    fn default() -> Self {
        Self::new(AgamExecClient::default())
    }
}

impl AgamReplTool {
    /// Create a tool backed by the provided execution client.
    pub fn new(client: AgamExecClient) -> Self {
        Self {
            client,
            filename: default_headless_filename(),
            backend: HeadlessExecutionBackend::Jit,
            opt_level: default_headless_opt_level(),
            fast: false,
            args: Vec::new(),
        }
    }

    /// Set the logical filename reported in diagnostics.
    pub fn with_filename(mut self, filename: impl Into<String>) -> Self {
        self.filename = filename.into();
        self
    }

    /// Set the execution backend.
    pub fn with_backend(mut self, backend: HeadlessExecutionBackend) -> Self {
        self.backend = backend;
        self
    }

    /// Set the optimization level.
    pub fn with_opt_level(mut self, opt_level: u8) -> Self {
        self.opt_level = opt_level;
        self
    }

    /// Enable or disable fast mode.
    pub fn with_fast(mut self, fast: bool) -> Self {
        self.fast = fast;
        self
    }

    /// Set process arguments passed into the executed Agam program.
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// Build a strict headless request for one source string.
    pub fn build_request(&self, source: impl Into<String>) -> HeadlessExecutionRequest {
        HeadlessExecutionRequest {
            source: source.into(),
            filename: self.filename.clone(),
            args: self.args.clone(),
            backend: self.backend,
            opt_level: self.opt_level,
            fast: self.fast,
            policy: HeadlessExecutionPolicy {
                allow_native_backends: !matches!(self.backend, HeadlessExecutionBackend::Jit),
                ..HeadlessExecutionPolicy::default()
            },
        }
    }

    /// Execute one source string through the configured client.
    pub fn execute(
        &self,
        source: impl Into<String>,
    ) -> Result<HeadlessExecutionResponse, AgamExecError> {
        self.client.run_request(&self.build_request(source))
    }
}

/// Errors surfaced while invoking the `agamc exec` bridge.
#[derive(Debug)]
pub enum AgamExecError {
    SerializeRequest(serde_json::Error),
    Spawn(std::io::Error),
    WriteStdin(std::io::Error),
    Wait(std::io::Error),
    ParseResponse {
        error: serde_json::Error,
        stdout: String,
        stderr: String,
        status_code: Option<i32>,
    },
}

impl std::fmt::Display for AgamExecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SerializeRequest(error) => {
                write!(f, "failed to serialize Agam execution request: {error}")
            }
            Self::Spawn(error) => write!(f, "failed to spawn `agamc exec --json`: {error}"),
            Self::WriteStdin(error) => {
                write!(
                    f,
                    "failed to write Agam execution request to stdin: {error}"
                )
            }
            Self::Wait(error) => write!(f, "failed while waiting for `agamc exec`: {error}"),
            Self::ParseResponse {
                error,
                stdout,
                stderr,
                status_code,
            } => write!(
                f,
                "failed to parse `agamc exec` response: {error} (status: {:?}, stdout: {:?}, stderr: {:?})",
                status_code, stdout, stderr
            ),
        }
    }
}

impl std::error::Error for AgamExecError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn repl_tool_build_request_applies_tool_configuration() {
        let tool = AgamReplTool::new(AgamExecClient::new("agamc"))
            .with_filename("agent.agam")
            .with_backend(HeadlessExecutionBackend::Llvm)
            .with_opt_level(3)
            .with_fast(true)
            .with_args(vec!["alpha".into(), "beta".into()]);

        let request = tool.build_request("fn main() -> i32 { return 0; }\n");
        assert_eq!(request.filename, "agent.agam");
        assert_eq!(request.backend, HeadlessExecutionBackend::Llvm);
        assert_eq!(request.opt_level, 3);
        assert!(request.fast);
        assert_eq!(request.args, vec!["alpha".to_string(), "beta".to_string()]);
        assert!(request.policy.allow_native_backends);
    }

    #[test]
    fn exec_client_invokes_mock_exec_tool_and_parses_response() {
        let root = temp_dir("agam_exec_client");
        let executable = write_mock_exec_tool(&root);
        let client = AgamExecClient::new(executable);

        let response = client
            .run_request(&HeadlessExecutionRequest {
                source: "fn main() -> i32 { return 0; }\n".into(),
                ..HeadlessExecutionRequest::default()
            })
            .expect("mock exec tool should return a structured response");

        assert!(response.success);
        assert_eq!(response.exit_code, Some(0));
        assert_eq!(response.stdout, "mock\n");
        assert_eq!(response.backend, HeadlessExecutionBackend::Jit);

        let _ = fs::remove_dir_all(root);
    }

    fn temp_dir(label: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("agam_ffi_{label}_{now}"));
        fs::create_dir_all(&path).expect("temp dir should be created");
        path
    }

    #[cfg(windows)]
    fn write_mock_exec_tool(root: &Path) -> PathBuf {
        let path = root.join("mock_exec.cmd");
        fs::write(
            &path,
            "@echo off\r\nmore >nul\r\necho {\"success\":true,\"filename\":\"snippet.agam\",\"backend\":\"jit\",\"exit_code\":0,\"stdout\":\"mock\\n\",\"stderr\":\"\"}\r\n",
        )
        .expect("mock exec script should be written");
        path
    }

    #[cfg(not(windows))]
    fn write_mock_exec_tool(root: &Path) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let path = root.join("mock_exec.sh");
        fs::write(
            &path,
            "#!/bin/sh\ncat >/dev/null\nprintf '%s\\n' '{\"success\":true,\"filename\":\"snippet.agam\",\"backend\":\"jit\",\"exit_code\":0,\"stdout\":\"mock\\\\n\",\"stderr\":\"\"}'\n",
        )
        .expect("mock exec script should be written");
        let mut permissions = fs::metadata(&path)
            .expect("mock exec script should exist")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).expect("mock exec script should be executable");
        path
    }
}
