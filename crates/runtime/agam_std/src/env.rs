//! Native Environment module for `agam_std`.
//!
//! Provides deterministic access to environment variables, current directory,
//! and command line arguments.

use std::env;
use std::path::PathBuf;

/// Error type for environment operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvError {
    pub variable: String,
    pub message: String,
}

impl std::fmt::Display for EnvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EnvError for variable '{}': {}",
            self.variable, self.message
        )
    }
}

impl std::error::Error for EnvError {}

pub fn get_var(key: &str) -> Result<String, EnvError> {
    env::var(key).map_err(|e| EnvError {
        variable: key.to_string(),
        message: e.to_string(),
    })
}

pub fn set_var(key: &str, value: &str) {
    unsafe {
        env::set_var(key, value);
    }
}

pub fn remove_var(key: &str) {
    unsafe {
        env::remove_var(key);
    }
}

pub fn current_dir() -> Result<PathBuf, EnvError> {
    env::current_dir().map_err(|e| EnvError {
        variable: "PWD".to_string(),
        message: e.to_string(),
    })
}

pub fn args() -> Vec<String> {
    env::args().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_get_set() {
        set_var("AGAM_TEST_ENV_KEY", "AGAM_TEST_VAL");
        assert_eq!(get_var("AGAM_TEST_ENV_KEY").unwrap(), "AGAM_TEST_VAL");
        remove_var("AGAM_TEST_ENV_KEY");
        assert!(get_var("AGAM_TEST_ENV_KEY").is_err());
    }

    #[test]
    fn test_current_dir() {
        assert!(current_dir().is_ok());
    }
}
