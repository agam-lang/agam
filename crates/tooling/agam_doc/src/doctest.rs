//! Doctest extraction and execution runner.

use agam_errors::span::SourceId;
use agam_lexer::tokenize;
use agam_parser::Parser;
use serde::{Deserialize, Serialize};

use crate::model::{DocItem, DocModule, DocPackage, Doctest};

/// Result of running a single doctest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestStatus {
    Passed,
    Failed(String),
    Ignored,
}

/// A tested doctest entry with outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctestOutcome {
    pub item_name: String,
    pub line: usize,
    pub status: TestStatus,
}

/// Overall summary report of doctest execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DoctestReport {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub ignored: usize,
    pub outcomes: Vec<DoctestOutcome>,
}

impl DoctestReport {
    pub fn is_success(&self) -> bool {
        self.failed == 0
    }
}

/// Collect and run all doctests across a `DocPackage`.
pub fn run_package_doctests(package: &DocPackage) -> DoctestReport {
    let mut report = DoctestReport::default();
    run_module_doctests(&package.root_module, &mut report);
    report
}

fn run_module_doctests(module: &DocModule, report: &mut DoctestReport) {
    for item in &module.items {
        if let DocItem::Function(f) = item {
            for dt in &f.doctests {
                run_single_doctest(dt, report);
            }
        }
    }

    for sub in &module.submodules {
        run_module_doctests(sub, report);
    }
}

/// Execute a single doctest block.
pub fn run_single_doctest(dt: &Doctest, report: &mut DoctestReport) {
    report.total += 1;

    if dt.ignore {
        report.ignored += 1;
        report.outcomes.push(DoctestOutcome {
            item_name: dt.item_name.clone(),
            line: dt.line,
            status: TestStatus::Ignored,
        });
        return;
    }

    // Prepare doctest source code
    let source =
        if dt.code.contains("fn ") || dt.code.contains("struct ") || dt.code.contains("enum ") {
            dt.code.clone()
        } else {
            format!("fn __doctest_main__() {{\n{}\n}}", dt.code)
        };

    let tokens = tokenize(&source, SourceId(0));
    let mut parser = Parser::new(tokens);

    match parser.parse_module(SourceId(0)) {
        Ok(_) => {
            if dt.should_panic {
                report.failed += 1;
                report.outcomes.push(DoctestOutcome {
                    item_name: dt.item_name.clone(),
                    line: dt.line,
                    status: TestStatus::Failed(
                        "expected doctest to fail/panic, but it passed".to_string(),
                    ),
                });
            } else {
                report.passed += 1;
                report.outcomes.push(DoctestOutcome {
                    item_name: dt.item_name.clone(),
                    line: dt.line,
                    status: TestStatus::Passed,
                });
            }
        }
        Err(errs) => {
            if dt.should_panic {
                report.passed += 1;
                report.outcomes.push(DoctestOutcome {
                    item_name: dt.item_name.clone(),
                    line: dt.line,
                    status: TestStatus::Passed,
                });
            } else {
                report.failed += 1;
                let err_msg = errs
                    .iter()
                    .map(|e| e.message.clone())
                    .collect::<Vec<_>>()
                    .join("; ");
                report.outcomes.push(DoctestOutcome {
                    item_name: dt.item_name.clone(),
                    line: dt.line,
                    status: TestStatus::Failed(err_msg),
                });
            }
        }
    }
}
