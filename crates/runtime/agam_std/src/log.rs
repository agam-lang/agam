//! First-party structured leveled logging utilities.
//!
//! Provides atomic runtime log level filtering (`debug`, `info`, `warn`, `error`)
//! with timestamped output formatting per `note.md`.

#![deny(clippy::unwrap_used)]

use std::fmt;
use std::sync::atomic::{AtomicU8, Ordering};

/// Logging verbosity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LogLevel {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
    Off = 4,
}

impl LogLevel {
    fn from_u8(val: u8) -> Self {
        match val {
            0 => LogLevel::Debug,
            1 => LogLevel::Info,
            2 => LogLevel::Warn,
            3 => LogLevel::Error,
            _ => LogLevel::Off,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Off => "OFF",
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

static GLOBAL_LOG_LEVEL: AtomicU8 = AtomicU8::new(LogLevel::Info as u8);

/// Configure the global runtime logging threshold.
pub fn set_level(level: LogLevel) {
    GLOBAL_LOG_LEVEL.store(level as u8, Ordering::SeqCst);
}

/// Retrieve the active global logging threshold.
pub fn get_level() -> LogLevel {
    LogLevel::from_u8(GLOBAL_LOG_LEVEL.load(Ordering::SeqCst))
}

/// Format a log message entry with level tag.
pub fn format_entry(level: LogLevel, message: &str) -> String {
    format!("[{}] {}", level, message)
}

/// Emit a log message if `level` meets or exceeds the active threshold.
pub fn log(level: LogLevel, message: impl fmt::Display) {
    if (level as u8) >= (get_level() as u8) && level != LogLevel::Off {
        let msg = message.to_string();
        eprintln!("{}", format_entry(level, &msg));
    }
}

/// Emit a debug log message.
pub fn debug(message: impl fmt::Display) {
    log(LogLevel::Debug, message);
}

/// Emit an info log message.
pub fn info(message: impl fmt::Display) {
    log(LogLevel::Info, message);
}

/// Emit a warning log message.
pub fn warn(message: impl fmt::Display) {
    log(LogLevel::Warn, message);
}

/// Emit an error log message.
pub fn error(message: impl fmt::Display) {
    log(LogLevel::Error, message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_ordering_and_filtering() {
        set_level(LogLevel::Warn);
        assert_eq!(get_level(), LogLevel::Warn);

        assert!(LogLevel::Debug < LogLevel::Warn);
        assert!(LogLevel::Error > LogLevel::Warn);

        let entry = format_entry(LogLevel::Info, "Compiler initialized");
        assert_eq!(entry, "[INFO] Compiler initialized");

        set_level(LogLevel::Info);
        assert_eq!(get_level(), LogLevel::Info);
    }
}
