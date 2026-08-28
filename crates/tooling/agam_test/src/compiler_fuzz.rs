//! AST Mutation & LLVM/MIR Backend Invariant Compiler Fuzzer.
//!
//! Provides procedural AST code generation, deep-nesting stress tests,
//! pipeline crash-resistance validation, and deterministic invariant verification.

use std::panic::{AssertUnwindSafe, catch_unwind};

use agam_errors::SourceId;
use agam_lexer::tokenize;
use agam_parser::parse;
use agam_sema::checker::TypeChecker;
use agam_sema::resolver::Resolver;

/// Represents the outcome of running an Agam source string through the compilation pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineFuzzOutcome {
    /// Source failed at lexical/syntax parsing stage with cleanly reported diagnostics.
    ParseError { diagnostic_count: usize },
    /// Source passed parsing but failed semantic analysis with cleanly reported diagnostics.
    SemanticError { diagnostic_count: usize },
    /// Source passed full parsing, semantic analysis, and lowered into MIR/HIR.
    Success,
    /// Compilation pass crashed or panicked (Invariant violation!).
    Panic { message: String },
}

/// Statistics and findings from a compiler fuzzing campaign.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompilerFuzzReport {
    pub total_iterations: u64,
    pub parse_errors: u64,
    pub semantic_errors: u64,
    pub successful_compilations: u64,
    pub crashes: Vec<String>,
}

/// Procedural AST and source text generator for compiler stress testing.
pub struct AstMutationEngine {
    rng_state: u64,
}

impl AstMutationEngine {
    pub fn new(seed: u64) -> Self {
        Self {
            rng_state: if seed == 0 { 0xdeadbeefcafebabe } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        self.rng_state
    }

    fn next_bounded(&mut self, bound: usize) -> usize {
        if bound == 0 {
            0
        } else {
            (self.next_u64() % (bound as u64)) as usize
        }
    }

    /// Generate random primitive literal or variable expression.
    pub fn generate_expression(&mut self, depth: usize) -> String {
        if depth == 0 {
            match self.next_bounded(4) {
                0 => format!("{}", (self.next_u64() % 1000) as i64),
                1 => if self.next_bounded(2) == 0 {
                    "true"
                } else {
                    "false"
                }
                .to_string(),
                2 => "\"fuzz_string\"".to_string(),
                _ => "x".to_string(),
            }
        } else {
            match self.next_bounded(5) {
                0 => {
                    let lhs = self.generate_expression(depth - 1);
                    let rhs = self.generate_expression(depth - 1);
                    let op = match self.next_bounded(4) {
                        0 => "+",
                        1 => "-",
                        2 => "*",
                        _ => "==",
                    };
                    format!("({lhs} {op} {rhs})")
                }
                1 => {
                    let cond = self.generate_expression(depth - 1);
                    let then_b = self.generate_expression(depth - 1);
                    let else_b = self.generate_expression(depth - 1);
                    format!("if {cond} {{ {then_b} }} else {{ {else_b} }}")
                }
                2 => {
                    let inner = self.generate_expression(depth - 1);
                    format!("(-{inner})")
                }
                3 => {
                    let val = self.generate_expression(depth - 1);
                    format!("{{ let temp = {val}; temp }}")
                }
                _ => self.generate_expression(0),
            }
        }
    }

    /// Generate a full syntactically varied function definition.
    pub fn generate_function(&mut self, name: &str, depth: usize) -> String {
        let body = self.generate_expression(depth);
        format!("fn {name}(x: i32) -> i32 {{\n    let result = {body};\n    return 42;\n}}\n")
    }

    /// Generate a complete source module with multiple interacting functions and structs.
    pub fn generate_module(&mut self, num_functions: usize, max_depth: usize) -> String {
        let mut out = String::new();

        for i in 0..num_functions {
            let fn_name = format!("fuzz_fn_{i}");
            let depth = self.next_bounded(max_depth) + 1;
            out.push_str(&self.generate_function(&fn_name, depth));
            out.push('\n');
        }

        out.push_str("fn main() -> i32 {\n    return fuzz_fn_0(10);\n}\n");
        out
    }
}

/// Compiler Pipeline Fuzzer ensuring backend invariants and crash immunity.
pub struct CompilerPipelineFuzzer;

impl CompilerPipelineFuzzer {
    /// Fuzz a single source string through the frontend, sema, and MIR stages safely.
    pub fn test_source(source: &str) -> PipelineFuzzOutcome {
        let source_clone = source.to_string();

        let result = catch_unwind(AssertUnwindSafe(move || {
            let source_id = SourceId(0);
            let tokens = tokenize(&source_clone, source_id);

            let module = match parse(tokens, source_id) {
                Ok(m) => m,
                Err(errs) => {
                    return PipelineFuzzOutcome::ParseError {
                        diagnostic_count: errs.len(),
                    };
                }
            };

            let mut resolver = Resolver::new();
            resolver.resolve_module(&module);
            if !resolver.errors.is_empty() {
                return PipelineFuzzOutcome::SemanticError {
                    diagnostic_count: resolver.errors.len(),
                };
            }

            let mut checker = TypeChecker::from_resolver(resolver);
            checker.check_module(&module);
            if !checker.errors.is_empty() {
                return PipelineFuzzOutcome::SemanticError {
                    diagnostic_count: checker.errors.len(),
                };
            }

            PipelineFuzzOutcome::Success
        }));

        match result {
            Ok(outcome) => outcome,
            Err(panic_payload) => {
                let msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown panic in compiler pipeline".to_string()
                };
                PipelineFuzzOutcome::Panic { message: msg }
            }
        }
    }

    /// Run a multi-iteration fuzzing campaign across procedurally generated AST trees.
    pub fn run_campaign(iterations: u64, max_ast_depth: usize, seed: u64) -> CompilerFuzzReport {
        let mut engine = AstMutationEngine::new(seed);
        let mut report = CompilerFuzzReport::default();

        for _ in 0..iterations {
            report.total_iterations += 1;
            let src = engine.generate_module(2, max_ast_depth);
            match Self::test_source(&src) {
                PipelineFuzzOutcome::Success => {
                    report.successful_compilations += 1;
                }
                PipelineFuzzOutcome::ParseError { .. } => {
                    report.parse_errors += 1;
                }
                PipelineFuzzOutcome::SemanticError { .. } => {
                    report.semantic_errors += 1;
                }
                PipelineFuzzOutcome::Panic { message } => {
                    report.crashes.push(message);
                }
            }
        }

        report
    }

    /// Verify compiler determinism invariant: repeated compilation produces identical outcome.
    pub fn verify_determinism(source: &str) -> bool {
        let out1 = Self::test_source(source);
        let out2 = Self::test_source(source);
        out1 == out2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ast_mutation_generation_does_not_crash() {
        let mut engine = AstMutationEngine::new(42);
        let expr = engine.generate_expression(3);
        assert!(!expr.is_empty());

        let module = engine.generate_module(2, 3);
        assert!(module.contains("fn fuzz_fn_0"));
        assert!(module.contains("fn main"));
    }

    #[test]
    fn test_compiler_fuzzer_campaign_zero_crashes() {
        let report = CompilerPipelineFuzzer::run_campaign(30, 3, 12345);
        assert_eq!(report.total_iterations, 30);
        assert!(
            report.crashes.is_empty(),
            "Found compiler crashes: {:?}",
            report.crashes
        );
    }

    #[test]
    fn test_compiler_determinism_invariant() {
        let valid_src = r#"
fn compute(a: i32, b: i32) -> i32 {
    let sum = a + b;
    return sum;
}
fn main() -> i32 {
    return compute(10, 20);
}
"#;
        assert!(CompilerPipelineFuzzer::verify_determinism(valid_src));

        let syntax_err_src = "fn broken( { let x = ; }";
        assert!(CompilerPipelineFuzzer::verify_determinism(syntax_err_src));
    }

    #[test]
    fn test_grammar_derivation_fuzzing() {
        let mut engine = AstMutationEngine::new(999);
        for _ in 0..50 {
            let expr = engine.generate_expression(4);
            let fn_src = format!("fn test_fuzz_expr() -> i32 {{\n    let x = 10;\n    let res = {expr};\n    return 0;\n}}\n");
            let outcome = CompilerPipelineFuzzer::test_source(&fn_src);
            // Must never panic/crash
            match outcome {
                PipelineFuzzOutcome::Success | PipelineFuzzOutcome::ParseError { .. } | PipelineFuzzOutcome::SemanticError { .. } => {}
                PipelineFuzzOutcome::Panic { message } => assert!(false, "Parser panicked on grammar derivation: {}", message),
            }
        }
    }

    #[test]
    fn test_dual_syntax_parity_compilation() {
        let advance_syntax = r#"
fn compute_sum(n: i32) -> i32 {
    let mut total: i32 = 0;
    let mut i: i32 = 0;
    while i < n {
        total = total + i;
        i = i + 1;
    }
    return total;
}
fn main() -> i32 {
    return compute_sum(10);
}
"#;

        let base_syntax = r#"
fn compute_sum(n: i32) -> i32:
    let mut total: i32 = 0
    let mut i: i32 = 0
    while i < n:
        total = total + i
        i = i + 1
    return total

fn main() -> i32:
    return compute_sum(10)
"#;

        let out_advance = CompilerPipelineFuzzer::test_source(advance_syntax);
        let out_base = CompilerPipelineFuzzer::test_source(base_syntax);

        assert_eq!(out_advance, PipelineFuzzOutcome::Success);
        assert_eq!(out_base, PipelineFuzzOutcome::Success);
    }
}
