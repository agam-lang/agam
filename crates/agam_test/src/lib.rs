//! # agam_test
//!
//! Test framework, property testing, and fuzzing.

use std::fs;
use std::path::{Path, PathBuf};

use agam_ast::Module;
use agam_ast::decl::DeclKind;
use agam_errors::{SourceFile, SourceId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestCase {
    pub name: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestResult {
    pub case: TestCase,
    pub passed: bool,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TestSummary {
    pub results: Vec<TestResult>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileTestSummary {
    pub path: PathBuf,
    pub summary: TestSummary,
}

impl TestSummary {
    pub fn total(&self) -> usize {
        self.results.len()
    }

    pub fn passed(&self) -> usize {
        self.results.iter().filter(|result| result.passed).count()
    }

    pub fn failed(&self) -> usize {
        self.results.iter().filter(|result| !result.passed).count()
    }
}

pub fn run_file(path: &Path) -> Result<TestSummary, String> {
    let source = fs::read_to_string(path)
        .map_err(|e| format!("failed to read Agam test file `{}`: {e}", path.display()))?;
    run_source(&source, &path.to_string_lossy())
}

pub fn run_paths(paths: &[PathBuf]) -> Result<Vec<FileTestSummary>, String> {
    paths
        .iter()
        .map(|path| {
            run_file(path).map(|summary| FileTestSummary {
                path: path.clone(),
                summary,
            })
        })
        .collect()
}

pub fn run_inputs(inputs: Vec<PathBuf>) -> Result<Vec<FileTestSummary>, String> {
    let paths = agam_pkg::expand_agam_inputs(inputs)?;
    run_paths(&paths)
}

fn run_source(source: &str, label: &str) -> Result<TestSummary, String> {
    let source_id = SourceId(0);
    let tokens = agam_lexer::tokenize(source, source_id);
    let module = agam_parser::parse(tokens, source_id).map_err(|errors| {
        errors
            .iter()
            .map(|error| error.message.clone())
            .collect::<Vec<_>>()
            .join("; ")
    })?;
    let source_file = SourceFile::new(source_id, label.to_string(), source.to_string());
    let test_cases = collect_test_cases(&module, &source_file);
    if test_cases.is_empty() {
        return Ok(TestSummary::default());
    }

    let mut hir_lowering = agam_hir::lower::HirLowering::new();
    let hir = hir_lowering.lower_module(&module);
    let mut mir_lowering = agam_mir::lower::MirLowering::new();
    let mut mir = mir_lowering.lower_module(&hir);
    let _ = agam_mir::opt::optimize_module(&mut mir);

    let compiled = agam_jit::CompiledJitModule::compile(&mir, agam_jit::JitOptions::default())?;
    let results = test_cases
        .into_iter()
        .map(|case| {
            let evaluation = compiled.run_function(&case.name, &[]);
            match evaluation {
                Ok(value) => {
                    let (passed, message) = evaluate_test_value(value);
                    TestResult {
                        case,
                        passed,
                        message,
                    }
                }
                Err(error) => TestResult {
                    case,
                    passed: false,
                    message: Some(error),
                },
            }
        })
        .collect();

    Ok(TestSummary { results })
}

fn collect_test_cases(module: &Module, source_file: &SourceFile) -> Vec<TestCase> {
    module
        .declarations
        .iter()
        .filter_map(|decl| match &decl.kind {
            DeclKind::Function(function)
                if function
                    .annotations
                    .iter()
                    .any(|annotation| annotation.name.name == "test") =>
            {
                let (line, column) =
                    source_file.offset_to_line_col(function.name.span.start as usize);
                Some(TestCase {
                    name: function.name.name.clone(),
                    line: line + 1,
                    column: column + 1,
                })
            }
            _ => None,
        })
        .collect()
}

fn evaluate_test_value(value: agam_jit::JitValue) -> (bool, Option<String>) {
    match value {
        agam_jit::JitValue::Unit => (true, None),
        agam_jit::JitValue::Bool(true) => (true, None),
        agam_jit::JitValue::Bool(false) => (false, Some("returned false".into())),
        agam_jit::JitValue::Int(0) => (true, None),
        agam_jit::JitValue::Int(value) => (false, Some(format!("returned {value}"))),
        agam_jit::JitValue::UInt(0) => (true, None),
        agam_jit::JitValue::UInt(value) => (false, Some(format!("returned {value}"))),
        agam_jit::JitValue::Float32(value) if value == 0.0 => (true, None),
        agam_jit::JitValue::Float32(value) => (false, Some(format!("returned {value}"))),
        agam_jit::JitValue::Float64(value) if value == 0.0 => (true, None),
        agam_jit::JitValue::Float64(value) => (false, Some(format!("returned {value}"))),
        agam_jit::JitValue::Pointer(0) => (true, None),
        agam_jit::JitValue::Pointer(value) => (false, Some(format!("returned {value}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_source_executes_test_functions() {
        let summary = run_source(
            r#"
@test
fn passes() -> bool:
    return true

@test
fn fails() -> bool:
    return false
"#,
            "memory://tests.agam",
        )
        .expect("run source tests");

        assert_eq!(summary.total(), 2);
        assert_eq!(summary.passed(), 1);
        assert_eq!(summary.failed(), 1);
        assert_eq!(summary.results[0].case.name, "passes");
        assert!(summary.results[0].passed);
        assert_eq!(
            summary.results[1].message.as_deref(),
            Some("returned false")
        );
    }

    #[test]
    fn run_source_returns_empty_summary_without_test_annotations() {
        let summary = run_source(
            r#"
fn helper() -> bool:
    return true
"#,
            "memory://helpers.agam",
        )
        .expect("run source without tests");

        assert_eq!(summary.total(), 0);
    }

    #[test]
    fn run_paths_preserves_file_paths() {
        let dir = std::env::temp_dir().join(format!(
            "agam_test_paths_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time should move forward")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let file = dir.join("smoke.agam");
        std::fs::write(&file, "@test\nfn smoke() -> bool:\n    return true\n")
            .expect("write test file");

        let summaries = run_paths(std::slice::from_ref(&file)).expect("run test paths");

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].path, file);
        assert_eq!(summaries[0].summary.passed(), 1);

        let _ = std::fs::remove_dir_all(dir);
    }
}
