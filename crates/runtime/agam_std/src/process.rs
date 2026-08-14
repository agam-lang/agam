//! Native Process execution module for `agam_std`.
//!
//! Provides deterministic process spawning, exit status inspection,
//! and PID querying.

use std::process::Command;

/// Error type for process execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessError {
    pub command: String,
    pub message: String,
}

impl std::fmt::Display for ProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ProcessError for command '{}': {}", self.command, self.message)
    }
}

impl std::error::Error for ProcessError {}

/// Output result of running a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessOutput {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

pub fn run(cmd: &str, args: &[String]) -> Result<ProcessOutput, ProcessError> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| ProcessError {
            command: cmd.to_string(),
            message: e.to_string(),
        })?;

    Ok(ProcessOutput {
        status: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

pub fn pid() -> u32 {
    std::process::id()
}

pub fn exit(code: i32) -> ! {
    std::process::exit(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_pid() {
        assert!(pid() > 0);
    }
}
