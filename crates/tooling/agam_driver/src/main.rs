//! # agamc — The Agam Compiler CLI Driver
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

pub(crate) mod build;
pub(crate) mod cli;
pub(crate) mod daemon;
pub(crate) mod dispatch;
pub(crate) mod mcp;
pub(crate) mod packaging;
pub(crate) mod pipeline;
pub(crate) mod repl;

#[cfg(test)]
mod main_tests;

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::io::{BufRead, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Stdio};
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc,
};
use std::time::Duration;

use clap::Parser;
use serde::{Deserialize, Serialize};

use agam_errors::{DiagnosticEmitter, SourceFile, SourceId};
use agam_notebook::{
    HeadlessExecutionBackend, HeadlessExecutionPolicy, HeadlessExecutionRequest,
    HeadlessExecutionResponse,
};
pub(crate) use agam_pkg::WorkspaceLayout;

pub(crate) use build::*;
pub(crate) use cli::*;
pub(crate) use daemon::*;
pub(crate) use packaging::*;
pub(crate) use pipeline::*;
pub(crate) use repl::*;

fn main() {
    const STACK_SIZE: usize = 16 * 1024 * 1024;
    let builder = std::thread::Builder::new()
        .name("agamc-main".into())
        .stack_size(STACK_SIZE);

    let handler = builder
        .spawn(dispatch::run_cli)
        .expect("failed to spawn main compiler thread");

    if let Err(panic) = handler.join() {
        std::panic::resume_unwind(panic);
    }
}
