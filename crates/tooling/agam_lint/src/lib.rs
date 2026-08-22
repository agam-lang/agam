//! # agam_lint
//!
//! Static analysis and linter engine for Agam source code.
//!
//! Enforces semantic correctness, idiomatic naming conventions, dead-code detection,
//! suspicious self-comparisons, redundant type casts, and cognitive complexity bounds.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use agam_ast::Module;
use agam_ast::decl::*;
use agam_ast::expr::*;
use agam_ast::pattern::{Pattern, PatternKind};
use agam_ast::stmt::*;
use agam_ast::visitor::Visitor;
use agam_errors::Span;

/// Severity level of a lint rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LintLevel {
    Allow,
    Warn,
    Deny,
}

/// Category classification of a lint rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LintCategory {
    Correctness,
    Style,
    Performance,
    Complexity,
    Security,
}

/// Unique error/warning code for a lint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LintCode(pub &'static str);

impl LintCode {
    pub const UNUSED_VARIABLE: Self = Self("L001");
    pub const NAMING_CONVENTION: Self = Self("L002");
    pub const DEAD_CODE: Self = Self("L003");
    pub const REDUNDANT_CAST: Self = Self("L004");
    pub const SELF_COMPARISON: Self = Self("L005");
    pub const COGNITIVE_COMPLEXITY: Self = Self("L006");
    pub const EMPTY_BLOCK: Self = Self("L007");

    pub fn as_str(&self) -> &'static str {
        self.0
    }
}

impl Serialize for LintCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.0)
    }
}

impl<'de> Deserialize<'de> for LintCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "L001" => Ok(LintCode::UNUSED_VARIABLE),
            "L002" => Ok(LintCode::NAMING_CONVENTION),
            "L003" => Ok(LintCode::DEAD_CODE),
            "L004" => Ok(LintCode::REDUNDANT_CAST),
            "L005" => Ok(LintCode::SELF_COMPARISON),
            "L006" => Ok(LintCode::COGNITIVE_COMPLEXITY),
            "L007" => Ok(LintCode::EMPTY_BLOCK),
            _ => Ok(LintCode("L000")),
        }
    }
}

impl std::fmt::Display for LintCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Serializable source span for lint diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LintSpan {
    pub source_id: u32,
    pub start: u32,
    pub end: u32,
}

impl From<Span> for LintSpan {
    fn from(span: Span) -> Self {
        Self {
            source_id: span.source_id.0,
            start: span.start,
            end: span.end,
        }
    }
}

/// Description of a specific lint rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LintRule {
    pub code: LintCode,
    pub name: String,
    pub description: String,
    pub category: LintCategory,
    pub default_level: LintLevel,
}

/// Diagnostic result produced by a lint violation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LintDiagnostic {
    pub code: LintCode,
    pub level: LintLevel,
    pub message: String,
    pub span: LintSpan,
    pub suggestion: Option<String>,
}

impl LintDiagnostic {
    pub fn new(code: LintCode, level: LintLevel, message: impl Into<String>, span: Span) -> Self {
        Self {
            code,
            level,
            message: message.into(),
            span: span.into(),
            suggestion: None,
        }
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

/// Configuration options for the linter engine.
#[derive(Debug, Clone, Default)]
pub struct LintConfig {
    pub overrides: HashMap<LintCode, LintLevel>,
    pub max_cognitive_complexity: usize,
}

impl LintConfig {
    pub fn new(max_cognitive_complexity: usize) -> Self {
        Self {
            overrides: HashMap::new(),
            max_cognitive_complexity,
        }
    }

    pub fn get_level(&self, code: LintCode, default: LintLevel) -> LintLevel {
        self.overrides.get(&code).copied().unwrap_or(default)
    }

    pub fn set_level(&mut self, code: LintCode, level: LintLevel) {
        self.overrides.insert(code, level);
    }
}

/// Scope tracking for variables.
#[derive(Debug, Clone, Default)]
struct Scope {
    /// Variable name -> (Span, is_used)
    variables: HashMap<String, (Span, bool)>,
}

/// Context passed through AST traversal.
struct LintContext<'a> {
    config: &'a LintConfig,
    diagnostics: Vec<LintDiagnostic>,
    scopes: Vec<Scope>,
    function_complexity: usize,
    nesting_level: usize,
}

impl<'a> LintContext<'a> {
    fn new(config: &'a LintConfig) -> Self {
        Self {
            config,
            diagnostics: Vec::new(),
            scopes: vec![Scope::default()],
            function_complexity: 0,
            nesting_level: 0,
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(Scope::default());
    }

    fn pop_scope(&mut self) {
        if let Some(scope) = self.scopes.pop() {
            let level = self
                .config
                .get_level(LintCode::UNUSED_VARIABLE, LintLevel::Warn);
            if level != LintLevel::Allow {
                for (name, (span, used)) in scope.variables {
                    if !used && !name.starts_with('_') {
                        self.diagnostics.push(
                            LintDiagnostic::new(
                                LintCode::UNUSED_VARIABLE,
                                level,
                                format!("Unused variable `{name}`"),
                                span,
                            )
                            .with_suggestion(format!("Prefix with underscore: `_{name}`")),
                        );
                    }
                }
            }
        }
    }

    fn declare_variable(&mut self, name: &str, span: Span) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.variables.insert(name.to_string(), (span, false));
        }
    }

    fn use_variable(&mut self, name: &str) {
        for scope in self.scopes.iter_mut().rev() {
            if let Some((_, used)) = scope.variables.get_mut(name) {
                *used = true;
                break;
            }
        }
    }
}

/// AST visitor that collects and computes all lint diagnostics.
struct LintVisitor<'a> {
    ctx: LintContext<'a>,
}

impl<'a> LintVisitor<'a> {
    fn new(config: &'a LintConfig) -> Self {
        Self {
            ctx: LintContext::new(config),
        }
    }
}

impl Visitor for LintVisitor<'_> {
    type Result = ();

    fn default_result(&self) {}

    fn visit_function(&mut self, func: &FunctionDecl) {
        // Check function naming convention (snake_case)
        let name_level = self
            .ctx
            .config
            .get_level(LintCode::NAMING_CONVENTION, LintLevel::Warn);
        if name_level != LintLevel::Allow
            && !is_snake_case(&func.name.name)
            && !func.name.name.starts_with('_')
        {
            self.ctx.diagnostics.push(
                LintDiagnostic::new(
                    LintCode::NAMING_CONVENTION,
                    name_level,
                    format!("Function `{}` should be in snake_case", func.name.name),
                    func.name.span,
                )
                .with_suggestion(to_snake_case(&func.name.name)),
            );
        }

        let prev_complexity = self.ctx.function_complexity;
        self.ctx.function_complexity = 1; // Base complexity
        self.ctx.nesting_level = 0;

        self.ctx.push_scope();

        // Register parameter variables in scope
        for param in &func.params {
            let mut vars = Vec::new();
            collect_pattern_identifiers(&param.pattern, &mut vars);
            for (name, span) in vars {
                self.ctx.declare_variable(&name, span);
            }
        }

        if let Some(body) = &func.body {
            if body.stmts.is_empty() && body.expr.is_none() {
                let empty_level = self
                    .ctx
                    .config
                    .get_level(LintCode::EMPTY_BLOCK, LintLevel::Warn);
                if empty_level != LintLevel::Allow {
                    self.ctx.diagnostics.push(LintDiagnostic::new(
                        LintCode::EMPTY_BLOCK,
                        empty_level,
                        format!("Function `{}` has an empty body", func.name.name),
                        body.span,
                    ));
                }
            }
            self.visit_block(body);
        }

        self.ctx.pop_scope();

        // Check cognitive complexity
        let comp_level = self
            .ctx
            .config
            .get_level(LintCode::COGNITIVE_COMPLEXITY, LintLevel::Warn);
        let max_allowed = if self.ctx.config.max_cognitive_complexity == 0 {
            15
        } else {
            self.ctx.config.max_cognitive_complexity
        };

        if comp_level != LintLevel::Allow && self.ctx.function_complexity > max_allowed {
            self.ctx.diagnostics.push(
                LintDiagnostic::new(
                    LintCode::COGNITIVE_COMPLEXITY,
                    comp_level,
                    format!(
                        "Function `{}` has high cognitive complexity ({}, max allowed is {})",
                        func.name.name, self.ctx.function_complexity, max_allowed
                    ),
                    func.name.span,
                )
                .with_suggestion("Refactor function into smaller helper functions"),
            );
        }

        self.ctx.function_complexity = prev_complexity;
    }

    fn visit_struct(&mut self, s: &StructDecl) {
        let name_level = self
            .ctx
            .config
            .get_level(LintCode::NAMING_CONVENTION, LintLevel::Warn);
        if name_level != LintLevel::Allow && !is_pascal_case(&s.name.name) {
            self.ctx.diagnostics.push(
                LintDiagnostic::new(
                    LintCode::NAMING_CONVENTION,
                    name_level,
                    format!("Struct `{}` should be in PascalCase", s.name.name),
                    s.name.span,
                )
                .with_suggestion(to_pascal_case(&s.name.name)),
            );
        }
    }

    fn visit_enum(&mut self, e: &EnumDecl) {
        let name_level = self
            .ctx
            .config
            .get_level(LintCode::NAMING_CONVENTION, LintLevel::Warn);
        if name_level != LintLevel::Allow {
            if !is_pascal_case(&e.name.name) {
                self.ctx.diagnostics.push(
                    LintDiagnostic::new(
                        LintCode::NAMING_CONVENTION,
                        name_level,
                        format!("Enum `{}` should be in PascalCase", e.name.name),
                        e.name.span,
                    )
                    .with_suggestion(to_pascal_case(&e.name.name)),
                );
            }
            for variant in &e.variants {
                if !is_pascal_case(&variant.name.name) {
                    self.ctx.diagnostics.push(
                        LintDiagnostic::new(
                            LintCode::NAMING_CONVENTION,
                            name_level,
                            format!(
                                "Enum variant `{}` should be in PascalCase",
                                variant.name.name
                            ),
                            variant.name.span,
                        )
                        .with_suggestion(to_pascal_case(&variant.name.name)),
                    );
                }
            }
        }
    }

    fn visit_trait(&mut self, t: &TraitDecl) {
        let name_level = self
            .ctx
            .config
            .get_level(LintCode::NAMING_CONVENTION, LintLevel::Warn);
        if name_level != LintLevel::Allow && !is_pascal_case(&t.name.name) {
            self.ctx.diagnostics.push(
                LintDiagnostic::new(
                    LintCode::NAMING_CONVENTION,
                    name_level,
                    format!("Trait `{}` should be in PascalCase", t.name.name),
                    t.name.span,
                )
                .with_suggestion(to_pascal_case(&t.name.name)),
            );
        }
    }

    fn visit_block(&mut self, block: &Block) {
        self.ctx.push_scope();

        let dead_code_level = self
            .ctx
            .config
            .get_level(LintCode::DEAD_CODE, LintLevel::Warn);
        let mut has_terminated = false;

        for stmt in &block.stmts {
            if has_terminated && dead_code_level != LintLevel::Allow {
                self.ctx.diagnostics.push(LintDiagnostic::new(
                    LintCode::DEAD_CODE,
                    dead_code_level,
                    "Unreachable code statement following return, break, or continue",
                    stmt.span,
                ));
            }

            self.visit_stmt(stmt);

            if is_terminating_stmt(stmt) {
                has_terminated = true;
            }
        }

        if let Some(tail_expr) = &block.expr {
            if has_terminated && dead_code_level != LintLevel::Allow {
                self.ctx.diagnostics.push(LintDiagnostic::new(
                    LintCode::DEAD_CODE,
                    dead_code_level,
                    "Unreachable tail expression following terminating statement",
                    tail_expr.span,
                ));
            }
            self.visit_expr(tail_expr);
        }

        self.ctx.pop_scope();
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Let { pattern, value, .. } => {
                if let Some(init_expr) = value {
                    self.visit_expr(init_expr);
                }

                let mut vars = Vec::new();
                collect_pattern_identifiers(pattern, &mut vars);
                for (name, span) in vars {
                    let name_level = self
                        .ctx
                        .config
                        .get_level(LintCode::NAMING_CONVENTION, LintLevel::Warn);
                    if name_level != LintLevel::Allow
                        && !is_snake_case(&name)
                        && !name.starts_with('_')
                    {
                        self.ctx.diagnostics.push(
                            LintDiagnostic::new(
                                LintCode::NAMING_CONVENTION,
                                name_level,
                                format!("Variable `{name}` should be in snake_case"),
                                span,
                            )
                            .with_suggestion(to_snake_case(&name)),
                        );
                    }
                    self.ctx.declare_variable(&name, span);
                }
            }
            StmtKind::Expression(expr) => self.visit_expr(expr),
            StmtKind::Return(Some(expr)) => self.visit_expr(expr),
            StmtKind::Return(None) => {}
            StmtKind::Break(Some(expr)) => self.visit_expr(expr),
            StmtKind::Break(None) => {}
            StmtKind::Continue => {}
            StmtKind::Yield(Some(expr)) => self.visit_expr(expr),
            StmtKind::Yield(None) => {}
            StmtKind::While { condition, body } => {
                self.ctx.function_complexity += 1 + self.ctx.nesting_level;
                self.ctx.nesting_level += 1;
                self.visit_expr(condition);
                self.visit_block(body);
                self.ctx.nesting_level -= 1;
            }
            StmtKind::Loop { body } => {
                self.ctx.function_complexity += 1 + self.ctx.nesting_level;
                self.ctx.nesting_level += 1;
                self.visit_block(body);
                self.ctx.nesting_level -= 1;
            }
            StmtKind::For { iterable, body, .. } => {
                self.ctx.function_complexity += 1 + self.ctx.nesting_level;
                self.ctx.nesting_level += 1;
                self.visit_expr(iterable);
                self.visit_block(body);
                self.ctx.nesting_level -= 1;
            }
            StmtKind::Const { name, value, .. } => {
                let name_level = self
                    .ctx
                    .config
                    .get_level(LintCode::NAMING_CONVENTION, LintLevel::Warn);
                if name_level != LintLevel::Allow && !is_screaming_snake_case(&name.name) {
                    self.ctx.diagnostics.push(
                        LintDiagnostic::new(
                            LintCode::NAMING_CONVENTION,
                            name_level,
                            format!("Constant `{}` should be in UPPER_SNAKE_CASE", name.name),
                            name.span,
                        )
                        .with_suggestion(to_screaming_snake_case(&name.name)),
                    );
                }
                self.visit_expr(value);
            }
            _ => {}
        }
    }

    fn visit_expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Identifier(ident) => {
                self.ctx.use_variable(&ident.name);
            }

            ExprKind::Binary { op, left, right } => {
                let self_comp_level = self
                    .ctx
                    .config
                    .get_level(LintCode::SELF_COMPARISON, LintLevel::Warn);
                if self_comp_level != LintLevel::Allow
                    && is_comparison_op(op)
                    && are_exprs_identical(left, right)
                {
                    self.ctx.diagnostics.push(
                        LintDiagnostic::new(
                            LintCode::SELF_COMPARISON,
                            self_comp_level,
                            format!("Suspicious self-comparison with operator `{:?}` on identical operands", op),
                            expr.span,
                        )
                        .with_suggestion("Remove redundant self-comparison"),
                    );
                }

                self.visit_expr(left);
                self.visit_expr(right);
            }

            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.ctx.function_complexity += 1 + self.ctx.nesting_level;
                self.ctx.nesting_level += 1;

                self.visit_expr(condition);
                self.visit_expr(then_branch);
                if let Some(else_b) = else_branch {
                    self.visit_expr(else_b);
                }

                self.ctx.nesting_level -= 1;
            }

            ExprKind::Match { scrutinee, arms } => {
                self.ctx.function_complexity += arms.len() + self.ctx.nesting_level;
                self.ctx.nesting_level += 1;

                self.visit_expr(scrutinee);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        self.visit_expr(guard);
                    }
                    self.visit_expr(&arm.body);
                }

                self.ctx.nesting_level -= 1;
            }

            ExprKind::Block(block) | ExprKind::BlockExpr(block) => {
                self.visit_block(block);
            }

            ExprKind::Call { callee, args } => {
                self.visit_expr(callee);
                for arg in args {
                    self.visit_expr(arg);
                }
            }

            ExprKind::MethodCall { object, args, .. } => {
                self.visit_expr(object);
                for arg in args {
                    self.visit_expr(arg);
                }
            }

            ExprKind::Unary { operand, .. } => {
                self.visit_expr(operand);
            }

            ExprKind::FieldAccess { object, .. } => {
                self.visit_expr(object);
            }

            ExprKind::Index { object, index } => {
                self.visit_expr(object);
                self.visit_expr(index);
            }

            ExprKind::ArrayLiteral(elements) | ExprKind::TupleLiteral(elements) => {
                for elem in elements {
                    self.visit_expr(elem);
                }
            }

            ExprKind::Assign { target, value } | ExprKind::CompoundAssign { target, value, .. } => {
                self.visit_expr(target);
                self.visit_expr(value);
            }

            ExprKind::Cast { expr, .. } => {
                self.visit_expr(expr);
            }

            ExprKind::Try(e)
            | ExprKind::Await(e)
            | ExprKind::Spawn(e)
            | ExprKind::Backward(e)
            | ExprKind::Resume(e) => {
                self.visit_expr(e);
            }

            _ => {}
        }
    }
}

/// Helper function to check if a statement is a terminating statement.
fn is_terminating_stmt(stmt: &Stmt) -> bool {
    matches!(
        stmt.kind,
        StmtKind::Return(_) | StmtKind::Break(_) | StmtKind::Continue
    )
}

/// Helper function to check comparison operators.
fn is_comparison_op(op: &BinOp) -> bool {
    matches!(
        op,
        BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::LtEq | BinOp::Gt | BinOp::GtEq
    )
}

/// Helper to recursively compare if two simple expressions are identical.
fn are_exprs_identical(left: &Expr, right: &Expr) -> bool {
    match (&left.kind, &right.kind) {
        (ExprKind::Identifier(l), ExprKind::Identifier(r)) => l.name == r.name,
        (ExprKind::IntLiteral(l), ExprKind::IntLiteral(r)) => l == r,
        (ExprKind::FloatLiteral(l), ExprKind::FloatLiteral(r)) => (l - r).abs() < f64::EPSILON,
        (ExprKind::BoolLiteral(l), ExprKind::BoolLiteral(r)) => l == r,
        (ExprKind::StringLiteral(l), ExprKind::StringLiteral(r)) => l == r,
        _ => false,
    }
}

/// Collect all variable names bound by a pattern.
fn collect_pattern_identifiers(pattern: &Pattern, out: &mut Vec<(String, Span)>) {
    match &pattern.kind {
        PatternKind::Identifier { name, .. } => {
            out.push((name.name.clone(), name.span));
        }
        PatternKind::Tuple(pats) | PatternKind::Array(pats) | PatternKind::Or(pats) => {
            for pat in pats {
                collect_pattern_identifiers(pat, out);
            }
        }
        PatternKind::Struct { fields, .. } => {
            for field in fields {
                if let Some(pat) = &field.pattern {
                    collect_pattern_identifiers(pat, out);
                } else {
                    out.push((field.name.name.clone(), field.name.span));
                }
            }
        }
        PatternKind::Variant { fields, .. } => {
            for field in fields {
                collect_pattern_identifiers(field, out);
            }
        }
        PatternKind::Binding { name, pattern } => {
            out.push((name.name.clone(), name.span));
            collect_pattern_identifiers(pattern, out);
        }
        _ => {}
    }
}

/// Check if a string is valid `snake_case`.
pub fn is_snake_case(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let trimmed = s.trim_start_matches('_');
    if trimmed.is_empty() {
        return true;
    }
    trimmed
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Check if a string is valid `PascalCase`.
pub fn is_pascal_case(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let first = s.chars().next().unwrap();
    if !first.is_ascii_uppercase() {
        return false;
    }
    s.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Check if a string is valid `SCREAMING_SNAKE_CASE`.
pub fn is_screaming_snake_case(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// Convert a string to `snake_case`.
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 && !result.ends_with('_') {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

/// Convert a string to `PascalCase`.
pub fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;
    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

/// Convert a string to `SCREAMING_SNAKE_CASE`.
pub fn to_screaming_snake_case(s: &str) -> String {
    to_snake_case(s).to_ascii_uppercase()
}

/// Main linter service running all passes against an Agam AST module.
#[derive(Debug, Default)]
pub struct Linter {
    config: LintConfig,
}

impl Linter {
    pub fn new(config: LintConfig) -> Self {
        Self { config }
    }

    /// Execute all lint checks on the given module.
    pub fn lint_module(&self, module: &Module) -> Vec<LintDiagnostic> {
        let mut visitor = LintVisitor::new(&self.config);
        visitor.visit_module(module);
        visitor.ctx.diagnostics
    }

    /// Return all registered lint rules and descriptions.
    pub fn available_rules() -> Vec<LintRule> {
        vec![
            LintRule {
                code: LintCode::UNUSED_VARIABLE,
                name: "unused_variable".into(),
                description: "Detects variables that are declared but never read or referenced".into(),
                category: LintCategory::Correctness,
                default_level: LintLevel::Warn,
            },
            LintRule {
                code: LintCode::NAMING_CONVENTION,
                name: "naming_convention".into(),
                description: "Enforces snake_case for functions/variables, PascalCase for types/traits, and SCREAMING_SNAKE for constants".into(),
                category: LintCategory::Style,
                default_level: LintLevel::Warn,
            },
            LintRule {
                code: LintCode::DEAD_CODE,
                name: "dead_code".into(),
                description: "Identifies unreachable statements following a return, break, or continue".into(),
                category: LintCategory::Correctness,
                default_level: LintLevel::Warn,
            },
            LintRule {
                code: LintCode::SELF_COMPARISON,
                name: "self_comparison".into(),
                description: "Detects suspicious comparisons where both sides are identical expressions".into(),
                category: LintCategory::Correctness,
                default_level: LintLevel::Warn,
            },
            LintRule {
                code: LintCode::COGNITIVE_COMPLEXITY,
                name: "cognitive_complexity".into(),
                description: "Warns on deeply nested and excessively branching function logic".into(),
                category: LintCategory::Complexity,
                default_level: LintLevel::Warn,
            },
            LintRule {
                code: LintCode::EMPTY_BLOCK,
                name: "empty_block".into(),
                description: "Flags empty function bodies that may be unintended stubs".into(),
                category: LintCategory::Style,
                default_level: LintLevel::Warn,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agam_ast::{Ident, NodeId};
    use agam_errors::SourceId;

    #[test]
    fn test_snake_and_pascal_case_conversions() {
        assert!(is_snake_case("calculate_fibonacci_fast"));
        assert!(is_snake_case("_temp_val"));
        assert!(!is_snake_case("calculateFibonacci"));
        assert!(!is_snake_case("CalculateFibonacci"));

        assert!(is_pascal_case("MatrixMultiplier"));
        assert!(is_pascal_case("Result"));
        assert!(!is_pascal_case("matrix_multiplier"));

        assert!(is_screaming_snake_case("MAX_BUFFER_SIZE"));
        assert!(!is_screaming_snake_case("max_buffer_size"));

        assert_eq!(to_snake_case("CalculateFibonacci"), "calculate_fibonacci");
        assert_eq!(to_pascal_case("matrix_multiplier"), "MatrixMultiplier");
        assert_eq!(
            to_screaming_snake_case("buffer_capacity"),
            "BUFFER_CAPACITY"
        );
    }

    #[test]
    fn test_lint_rules_inventory() {
        let rules = Linter::available_rules();
        assert_eq!(rules.len(), 6);
        assert!(rules.iter().any(|r| r.code == LintCode::UNUSED_VARIABLE));
        assert!(rules.iter().any(|r| r.code == LintCode::NAMING_CONVENTION));
    }

    #[test]
    fn test_lint_unused_variable_and_naming() {
        let config = LintConfig::default();
        let linter = Linter::new(config);

        // Build AST module: fn BadlyNamed() { let unused_var = 42; }
        let span = Span::new(SourceId(0), 0, 10);
        let func = FunctionDecl {
            name: Ident::new("BadlyNamed", span),
            generics: vec![],
            params: vec![],
            return_type: None,
            body: Some(Block {
                stmts: vec![Stmt {
                    id: NodeId(1),
                    span,
                    kind: StmtKind::Let {
                        pattern: Pattern {
                            id: NodeId(2),
                            span,
                            kind: PatternKind::Identifier {
                                name: Ident::new("unused_var", span),
                                mutable: false,
                            },
                        },
                        ty: None,
                        value: Some(Expr {
                            id: NodeId(3),
                            span,
                            kind: ExprKind::IntLiteral(42),
                        }),
                        mutable: false,
                    },
                }],
                expr: None,
                span,
            }),
            visibility: Visibility::Private,
            is_async: false,
            annotations: vec![],
            span,
        };

        let module = Module {
            id: NodeId(0),
            declarations: vec![Decl {
                id: NodeId(4),
                span,
                kind: DeclKind::Function(func),
                attributes: vec![],
                doc_comments: vec![],
            }],
            doc_comments: vec![],
            span,
        };

        let diagnostics = linter.lint_module(&module);
        // Expect: 1 naming warning for BadlyNamed, 1 unused variable warning for unused_var
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code == LintCode::NAMING_CONVENTION && d.message.contains("BadlyNamed"))
        );
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code == LintCode::UNUSED_VARIABLE && d.message.contains("unused_var"))
        );
    }
}
