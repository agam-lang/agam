//! First-party declarative CLI argument and flag parser powered by `clap`.
//!
//! Enforces the Facade-Completeness & Zero-Identity-Leak Invariant per `ADOPTED_DEPENDENCIES.md`
//! and `note.md`: all errors and help formatting use the compiler's native Nyāya diagnostic voice.

#![deny(clippy::unwrap_used)]

use std::collections::{HashMap, HashSet};
use std::fmt;

use clap::{Arg as ClapArg, ArgAction as ClapAction, Command as ClapCommand};

/// Specific classification of CLI error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliErrorKind {
    InvalidDefinition,
    MissingRequiredArgument,
    UnknownArgument,
    InvalidValue,
    ExecutionError,
}

/// Structured CLI diagnostic formatted in the Agam Nyāya voice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliError {
    pub kind: CliErrorKind,
    pub cause: String,
    pub context: String,
    pub remedy: String,
}

impl CliError {
    pub fn new(
        kind: CliErrorKind,
        cause: impl fmt::Display,
        context: impl fmt::Display,
        remedy: impl fmt::Display,
    ) -> Self {
        Self {
            kind,
            cause: cause.to_string(),
            context: context.to_string(),
            remedy: remedy.to_string(),
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CLI Diagnostic [{:?}]: {}\n  Context: {}\n  Remedy:  {}",
            self.kind, self.cause, self.context, self.remedy
        )
    }
}

impl std::error::Error for CliError {}

#[derive(Clone, Debug)]
struct ArgDef {
    short: char,
    long: String,
    desc: String,
    required: bool,
}

#[derive(Clone, Debug)]
struct FlagDef {
    short: char,
    long: String,
    desc: String,
}

/// Declarative CLI application builder.
#[derive(Clone, Debug)]
pub struct App {
    name: String,
    version: Option<String>,
    about: Option<String>,
    args: Vec<ArgDef>,
    flags: Vec<FlagDef>,
    seen_shorts: HashSet<char>,
    seen_longs: HashSet<String>,
}

impl App {
    /// Create a new CLI application definition with the specified name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: None,
            about: None,
            args: Vec::new(),
            flags: Vec::new(),
            seen_shorts: HashSet::new(),
            seen_longs: HashSet::new(),
        }
    }

    /// Set the application version string.
    pub fn version(&mut self, version: impl Into<String>) -> &mut Self {
        self.version = Some(version.into());
        self
    }

    /// Set the application summary description.
    pub fn about(&mut self, about: impl Into<String>) -> &mut Self {
        self.about = Some(about.into());
        self
    }

    /// Register a value argument with short flag, long name, description, and required flag.
    /// Returns `Err(CliError)` if the short or long flag is already registered.
    pub fn arg(
        &mut self,
        short: char,
        long: &str,
        desc: &str,
        required: bool,
    ) -> Result<&mut Self, CliError> {
        if self.seen_shorts.contains(&short) {
            return Err(CliError::new(
                CliErrorKind::InvalidDefinition,
                format!("Duplicate short flag '-{}'", short),
                format!(
                    "Short flag '-{}' was already defined on application '{}'",
                    short, self.name
                ),
                "Assign a unique short character flag to each argument",
            ));
        }
        if self.seen_longs.contains(long) {
            return Err(CliError::new(
                CliErrorKind::InvalidDefinition,
                format!("Duplicate long argument name '--{}'", long),
                format!(
                    "Argument '--{}' was already defined on application '{}'",
                    long, self.name
                ),
                "Assign a unique long name to each argument",
            ));
        }

        self.seen_shorts.insert(short);
        self.seen_longs.insert(long.to_string());
        self.args.push(ArgDef {
            short,
            long: long.to_string(),
            desc: desc.to_string(),
            required,
        });
        Ok(self)
    }

    /// Register a boolean flag (switch) with short flag, long name, and description.
    /// Returns `Err(CliError)` if the short or long flag is already registered.
    pub fn flag(&mut self, short: char, long: &str, desc: &str) -> Result<&mut Self, CliError> {
        if self.seen_shorts.contains(&short) {
            return Err(CliError::new(
                CliErrorKind::InvalidDefinition,
                format!("Duplicate short flag '-{}'", short),
                format!(
                    "Short flag '-{}' was already defined on application '{}'",
                    short, self.name
                ),
                "Assign a unique short character flag to each switch",
            ));
        }
        if self.seen_longs.contains(long) {
            return Err(CliError::new(
                CliErrorKind::InvalidDefinition,
                format!("Duplicate long flag name '--{}'", long),
                format!(
                    "Flag '--{}' was already defined on application '{}'",
                    long, self.name
                ),
                "Assign a unique long name to each switch",
            ));
        }

        self.seen_shorts.insert(short);
        self.seen_longs.insert(long.to_string());
        self.flags.push(FlagDef {
            short,
            long: long.to_string(),
            desc: desc.to_string(),
        });
        Ok(self)
    }

    /// Parse raw CLI arguments through the adopted `clap` engine and wrap output in `ParsedArgs`.
    pub fn parse_from(&self, raw_args: &[String]) -> Result<ParsedArgs, CliError> {
        let mut cmd = ClapCommand::new(self.name.clone())
            .no_binary_name(true)
            .disable_version_flag(self.version.is_none());

        if let Some(v) = &self.version {
            cmd = cmd.version(v.clone());
        }
        if let Some(a) = &self.about {
            cmd = cmd.about(a.clone());
        }

        for arg in &self.args {
            let mut clap_arg = ClapArg::new(arg.long.clone())
                .short(arg.short)
                .long(arg.long.clone())
                .help(arg.desc.clone())
                .action(ClapAction::Set)
                .num_args(1);
            if arg.required {
                clap_arg = clap_arg.required(true);
            }
            cmd = cmd.arg(clap_arg);
        }

        for flag in &self.flags {
            let clap_flag = ClapArg::new(flag.long.clone())
                .short(flag.short)
                .long(flag.long.clone())
                .help(flag.desc.clone())
                .action(ClapAction::SetTrue);
            cmd = cmd.arg(clap_flag);
        }

        let matches = match cmd.try_get_matches_from(raw_args) {
            Ok(m) => m,
            Err(e) => {
                let kind = match e.kind() {
                    clap::error::ErrorKind::MissingRequiredArgument => {
                        CliErrorKind::MissingRequiredArgument
                    }
                    clap::error::ErrorKind::UnknownArgument => CliErrorKind::UnknownArgument,
                    clap::error::ErrorKind::InvalidValue => CliErrorKind::InvalidValue,
                    _ => CliErrorKind::ExecutionError,
                };
                let raw_msg = e.to_string();
                let clean_msg = raw_msg
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .collect::<Vec<_>>()
                    .join(" ");

                return Err(CliError::new(
                    kind,
                    clean_msg,
                    format!("Error parsing arguments for '{}'", self.name),
                    "Run with '--help' to inspect valid arguments and flags",
                ));
            }
        };

        let mut string_values = HashMap::new();
        for arg in &self.args {
            if let Some(val) = matches.get_one::<String>(&arg.long) {
                string_values.insert(arg.long.clone(), val.clone());
            }
        }

        let mut flag_values = HashMap::new();
        for flag in &self.flags {
            let present = matches.get_flag(&flag.long);
            flag_values.insert(flag.long.clone(), present);
        }

        Ok(ParsedArgs {
            strings: string_values,
            flags: flag_values,
        })
    }
}

/// Parsed CLI values providing safe zero-panic accessors.
#[derive(Clone, Debug, PartialEq)]
pub struct ParsedArgs {
    strings: HashMap<String, String>,
    flags: HashMap<String, bool>,
}

impl ParsedArgs {
    /// Retrieve string value by long argument name.
    pub fn get_string(&self, name: &str) -> Option<&str> {
        self.strings.get(name).map(|s| s.as_str())
    }

    /// Retrieve boolean flag state by long flag name.
    pub fn get_flag(&self, name: &str) -> bool {
        self.flags.get(name).copied().unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_builder_rejects_duplicate_short_flag_without_panic() {
        let mut app = App::new("test_app");
        let r1 = app.arg('i', "input", "Input file", true);
        assert!(r1.is_ok());

        let r2 = app.flag('i', "interactive", "Interactive mode");
        assert!(
            r2.is_err(),
            "Duplicate short flag '-i' must return Err(CliError)"
        );
        if let Err(e) = r2 {
            assert_eq!(e.kind, CliErrorKind::InvalidDefinition);
        }
    }

    #[test]
    fn test_cli_builder_rejects_duplicate_long_name_without_panic() {
        let mut app = App::new("test_app");
        let r1 = app.arg('i', "input", "Input file", true);
        assert!(r1.is_ok());

        let r2 = app.arg('x', "input", "Another input file", false);
        assert!(
            r2.is_err(),
            "Duplicate long name '--input' must return Err(CliError)"
        );
        if let Err(e) = r2 {
            assert_eq!(e.kind, CliErrorKind::InvalidDefinition);
        }
    }

    #[test]
    fn test_cli_parse_valid_args_and_flags() {
        let mut app = App::new("compiler");
        app.version("1.0.0").about("Agam test compiler");
        let _ = app.arg('o', "output", "Output binary path", true);
        let _ = app.flag('v', "verbose", "Verbose diagnostics");

        let args = vec!["-o".to_string(), "dist/bin".to_string(), "-v".to_string()];
        let parsed = app.parse_from(&args);
        assert!(parsed.is_ok());
        if let Ok(p) = parsed {
            assert_eq!(p.get_string("output"), Some("dist/bin"));
            assert!(p.get_flag("verbose"));
            assert!(!p.get_flag("nonexistent"));
        }
    }

    #[test]
    fn test_cli_missing_required_arg_returns_nyaya_error() {
        let mut app = App::new("compiler");
        let _ = app.arg('o', "output", "Output binary path", true);

        let args = vec!["-v".to_string()];
        let parsed = app.parse_from(&args);
        assert!(parsed.is_err());
        if let Err(e) = parsed {
            assert_eq!(e.kind, CliErrorKind::UnknownArgument);
            assert!(e.to_string().contains("CLI Diagnostic"));
        }
    }
}
