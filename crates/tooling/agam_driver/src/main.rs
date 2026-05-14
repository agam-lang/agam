//! # agamc â€” The Agam Compiler
#![allow(
    clippy::collapsible_else_if,
    clippy::collapsible_if,
    clippy::default_constructed_unit_structs,
    clippy::derivable_impls,
    clippy::if_same_then_else,
    clippy::large_enum_variant,
    clippy::manual_flatten,
    clippy::needless_as_bytes,
    clippy::needless_borrow,
    clippy::needless_lifetimes,
    clippy::ptr_arg,
    clippy::question_mark,
    clippy::redundant_closure,
    clippy::redundant_pattern_matching,
    clippy::single_match,
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::unnecessary_cast
)]

mod build;
mod cli;
mod daemon;
mod dispatch;
mod packaging;
mod pipeline;
mod repl;

pub(crate) use build::*;
pub(crate) use cli::*;
pub(crate) use daemon::*;
pub(crate) use dispatch::*;
pub(crate) use packaging::*;
pub(crate) use pipeline::*;
pub(crate) use repl::*;

use clap::Parser;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::{BTreeMap, BTreeSet, HashSet};
pub(crate) use std::ffi::c_void;
pub(crate) use std::io::{BufRead, Read, Write};
pub(crate) use std::path::{Path, PathBuf};
pub(crate) use std::process::{self, Stdio};
pub(crate) use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc,
};
pub(crate) use std::time::Duration;

pub(crate) use agam_errors::{DiagnosticEmitter, SourceFile, SourceId};
pub(crate) use agam_notebook::{
    HeadlessExecutionBackend, HeadlessExecutionPolicy, HeadlessExecutionRequest,
    HeadlessExecutionResponse,
};
pub(crate) use agam_pkg::{self, WorkspaceLayout};
pub(crate) use agam_profile;
pub(crate) use agam_runtime;
pub(crate) use agam_test;

fn main() {
    run_cli();
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
