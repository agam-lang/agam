//! Shared compiler orchestration between the CLI and other tooling.

use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;

use agam_ast::decl::DeclKind;
use agam_errors::{Diagnostic, DiagnosticEmitter, Label, SourceFile, SourceId, Span};
use agam_lexer::{Token, TokenKind};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default)]
pub struct FeatureFlags {
    pub call_cache: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceFeatureFlags {
    pub call_cache: CallCacheSelection,
    pub experimental_usages: Vec<ExperimentalFeatureUsage>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallCacheSelection {
    pub disable_all: bool,
    pub enable_all: bool,
    pub optimize_all: bool,
    pub include_functions: BTreeSet<String>,
    pub optimize_functions: BTreeSet<String>,
    pub exclude_functions: BTreeSet<String>,
}

impl CallCacheSelection {
    pub fn is_enabled(&self) -> bool {
        self.resolved_enable_all()
            || self.optimize_all
            || !self.include_functions.is_empty()
            || !self.optimize_functions.is_empty()
    }

    pub fn resolved_enable_all(&self) -> bool {
        self.enable_all || !self.disable_all
    }

    pub fn merge_cli(&self, cli_enabled: bool) -> Self {
        let mut merged = self.clone();
        if cli_enabled {
            merged.disable_all = false;
            merged.enable_all = true;
        }
        merged
    }

    pub fn included_functions(&self) -> Vec<String> {
        self.include_functions
            .union(&self.optimize_functions)
            .cloned()
            .collect()
    }

    pub fn excluded_functions(&self) -> Vec<String> {
        self.exclude_functions.iter().cloned().collect()
    }

    pub fn optimized_functions(&self) -> Vec<String> {
        self.optimize_functions.iter().cloned().collect()
    }

    pub fn caches_function(&self, function: &str) -> bool {
        if self.exclude_functions.contains(function) {
            return false;
        }

        self.resolved_enable_all()
            || self.optimize_all
            || self.include_functions.contains(function)
            || self.optimize_functions.contains(function)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExperimentalFeature {
    CallCacheOptimize,
}

#[derive(Clone, Copy, Debug)]
pub struct ExperimentalFeatureSpec {
    pub code: &'static str,
    pub annotation: &'static str,
    pub warning: &'static str,
    pub help: &'static str,
}

impl ExperimentalFeature {
    pub fn spec(self) -> ExperimentalFeatureSpec {
        match self {
            ExperimentalFeature::CallCacheOptimize => ExperimentalFeatureSpec {
                code: "W2001",
                annotation: "@experimental.call_cache.optimize",
                warning: "call-cache optimize mode is experimental",
                help: "keep this opt-in local to hot paths; admission and eviction heuristics may change",
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExperimentalFeatureUsage {
    pub feature: ExperimentalFeature,
    pub span: Span,
}

#[derive(Clone)]
pub struct ParsedSource {
    pub module: agam_ast::Module,
    pub source_features: SourceFeatureFlags,
    pub source: String,
}

/// Shared lowered compiler state for a specific source version.
#[derive(Debug)]
pub struct WarmState {
    pub source_features: Option<SourceFeatureFlags>,
    pub module: Option<agam_ast::Module>,
    pub hir: Option<agam_hir::nodes::HirModule>,
    pub mir: Option<agam_mir::ir::MirModule>,
}

pub fn parse_source_file(path: &PathBuf, verbose: bool) -> Result<ParsedSource, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("could not read `{}`: {}", path.display(), e))?;
    let source_file = SourceFile::new(
        SourceId(0),
        path.to_string_lossy().to_string(),
        source.clone(),
    );
    let mut emitter = DiagnosticEmitter::new();
    emitter.add_source(source_file);

    if verbose {
        eprintln!("[agamc] Read {} ({} bytes)", path.display(), source.len());
    }

    let tokens = agam_lexer::tokenize(&source, SourceId(0));
    if verbose {
        eprintln!("[agamc] Lexed {} tokens", tokens.len());
    }

    let mut source_features = source_feature_flags_from_tokens(&tokens);
    let module = agam_parser::parse(tokens, SourceId(0)).map_err(|errors| {
        for err in &errors {
            eprintln!("\x1b[1;31merror\x1b[0m: {}", err.message);
        }
        format!("{} parse error(s)", errors.len())
    })?;

    if verbose {
        eprintln!(
            "[agamc] Parsed {} top-level declarations",
            module.declarations.len()
        );
    }

    merge_function_call_cache_annotations(&module, &mut source_features.call_cache);
    collect_experimental_function_features(&module, &mut source_features.experimental_usages);
    emit_experimental_feature_warnings(&mut emitter, &source_features.experimental_usages);

    Ok(ParsedSource {
        module,
        source_features,
        source,
    })
}

pub fn semantic_check_parsed_source(
    path: &PathBuf,
    parsed: &ParsedSource,
    verbose: bool,
) -> Result<(), String> {
    let source_file = SourceFile::new(
        SourceId(0),
        path.to_string_lossy().to_string(),
        parsed.source.clone(),
    );
    let mut emitter = DiagnosticEmitter::new();
    emitter.add_source(source_file);

    let mut resolver = agam_sema::resolver::Resolver::new();
    resolver.resolve_module(&parsed.module);
    let resolve_error_count = resolver.errors.len();
    if verbose {
        eprintln!("[agamc] Name resolution: {} error(s)", resolve_error_count);
    }
    for error in &resolver.errors {
        emit_resolve_error(&mut emitter, error);
    }
    if resolve_error_count > 0 {
        return Err(format!("{resolve_error_count} semantic error(s)"));
    }

    let mut checker = agam_sema::checker::TypeChecker::from_resolver(resolver);
    checker.check_module(&parsed.module);
    let type_error_count = checker.errors.len();
    if verbose {
        eprintln!("[agamc] Type checking: {} error(s)", type_error_count);
    }
    for error in &checker.errors {
        emit_type_error(&mut emitter, error);
    }
    if type_error_count > 0 {
        return Err(format!("{type_error_count} type error(s)"));
    }

    Ok(())
}

pub fn source_feature_flags_from_tokens(tokens: &[Token]) -> SourceFeatureFlags {
    let mut features = SourceFeatureFlags::default();
    let mut index = skip_trivia_tokens(tokens, 0);

    while index < tokens.len() {
        let Some(annotation) = parse_annotation_name(tokens, index) else {
            break;
        };
        match annotation.name.as_str() {
            "experimental.call_cache" | "lang.feat.call_cache" => {
                features.call_cache.disable_all = false;
                features.call_cache.enable_all = true;
            }
            "experimental.no_call_cache" | "lang.feat.no_call_cache" => {
                features.call_cache.disable_all = true;
                features.call_cache.enable_all = false;
                features.call_cache.optimize_all = false;
            }
            "experimental.call_cache.optimize" => {
                features.call_cache.disable_all = false;
                features.call_cache.enable_all = true;
                features.call_cache.optimize_all = true;
                features.experimental_usages.push(ExperimentalFeatureUsage {
                    feature: ExperimentalFeature::CallCacheOptimize,
                    span: annotation.span,
                });
            }
            "experimental.no_call_cache.optimize" => {
                features.call_cache.optimize_all = false;
            }
            _ => {}
        }
        index = skip_trivia_tokens(tokens, annotation.next_index);
    }

    features
}

pub fn merge_function_call_cache_annotations(
    module: &agam_ast::Module,
    selection: &mut CallCacheSelection,
) {
    for decl in &module.declarations {
        let DeclKind::Function(function) = &decl.kind else {
            continue;
        };
        for annotation in &function.annotations {
            match annotation.name.name.as_str() {
                "experimental.call_cache" | "lang.feat.call_cache" => {
                    selection
                        .exclude_functions
                        .remove(function.name.name.as_str());
                    selection
                        .include_functions
                        .insert(function.name.name.clone());
                }
                "experimental.call_cache.optimize" => {
                    selection
                        .exclude_functions
                        .remove(function.name.name.as_str());
                    selection
                        .include_functions
                        .insert(function.name.name.clone());
                    selection
                        .optimize_functions
                        .insert(function.name.name.clone());
                }
                "experimental.no_call_cache" | "lang.feat.no_call_cache" => {
                    selection
                        .include_functions
                        .remove(function.name.name.as_str());
                    selection
                        .optimize_functions
                        .remove(function.name.name.as_str());
                    selection
                        .exclude_functions
                        .insert(function.name.name.clone());
                }
                "experimental.no_call_cache.optimize" => {
                    selection
                        .optimize_functions
                        .remove(function.name.name.as_str());
                }
                _ => {}
            }
        }
    }
}

pub fn lower_module_to_hir_and_optimized_mir(
    module: &agam_ast::Module,
    verbose: bool,
) -> (agam_hir::nodes::HirModule, agam_mir::ir::MirModule) {
    let mut hir_lowering = agam_hir::lower::HirLowering::new();
    let hir = hir_lowering.lower_module(module);

    if verbose {
        eprintln!("[agamc] Lowered to HIR: {} functions", hir.functions.len());
    }

    let mut mir_lowering = agam_mir::lower::MirLowering::new();
    let mut mir = mir_lowering.lower_module(&hir);

    let optimized = agam_mir::opt::optimize_module(&mut mir);

    if verbose {
        eprintln!("[agamc] Lowered to MIR: {} functions", mir.functions.len());
        if optimized {
            eprintln!("[agamc] Applied MIR optimization passes");
        }
    }

    let purity = agam_mir::opt::escape::CalleePurityInfo::default();
    let (escape_results, promo_results) = agam_mir::opt::run_escape_and_promote(&mut mir, &purity);

    if verbose {
        eprintln!(
            "[agamc] Escape analysis: {} function(s) analyzed",
            escape_results.functions.len()
        );
        if promo_results.total_promoted > 0 {
            eprintln!(
                "[agamc] Stack promotion: {} local(s) promoted, {} ARC elision(s)",
                promo_results.total_promoted, promo_results.total_arc_elided
            );
        }
        for (func_name, fr) in &promo_results.functions {
            if !fr.promoted_locals.is_empty() {
                eprintln!(
                    "[agamc]   {}: promoted [{}]",
                    func_name,
                    fr.promoted_locals.join(", ")
                );
            }
            for (local, reason) in &fr.skipped {
                eprintln!("[agamc]   {}: skipped `{}` ({})", func_name, local, reason);
            }
        }
    }

    (hir, mir)
}

pub fn build_warm_state(
    path: &PathBuf,
    parsed: ParsedSource,
    verbose: bool,
) -> Result<WarmState, String> {
    semantic_check_parsed_source(path, &parsed, verbose)?;
    let ParsedSource {
        module,
        source_features,
        ..
    } = parsed;
    let (hir, mir) = lower_module_to_hir_and_optimized_mir(&module, verbose);
    Ok(WarmState {
        source_features: Some(source_features),
        module: Some(module),
        hir: Some(hir),
        mir: Some(mir),
    })
}

pub fn compile_file_with_warm_state(path: &PathBuf, verbose: bool) -> Result<WarmState, String> {
    let parsed = parse_source_file(path, verbose)?;
    build_warm_state(path, parsed, verbose)
}

pub fn lower_parsed_to_optimized_mir(
    parsed: &ParsedSource,
    verbose: bool,
) -> agam_mir::ir::MirModule {
    let (_, mir) = lower_module_to_hir_and_optimized_mir(&parsed.module, verbose);
    mir
}

pub fn lower_to_optimized_mir(
    path: &PathBuf,
    verbose: bool,
) -> Result<(agam_mir::ir::MirModule, SourceFeatureFlags), String> {
    let parsed = parse_source_file(path, verbose)?;
    semantic_check_parsed_source(path, &parsed, verbose)?;
    let mir = lower_parsed_to_optimized_mir(&parsed, verbose);

    Ok((mir, parsed.source_features))
}

pub fn emit_resolve_error(
    emitter: &mut DiagnosticEmitter,
    error: &agam_sema::resolver::ResolveError,
) {
    let diagnostic = if error.span.is_dummy() {
        Diagnostic::error("E3001", error.message.clone())
    } else {
        Diagnostic::error("E3001", error.message.clone())
            .with_label(Label::primary(error.span, error.message.clone()))
    };
    emitter.emit(diagnostic);
}

pub fn emit_type_error(emitter: &mut DiagnosticEmitter, error: &agam_sema::checker::TypeError) {
    let diagnostic = if error.span.is_dummy() {
        Diagnostic::error("E3002", error.message.clone())
    } else {
        Diagnostic::error("E3002", error.message.clone())
            .with_label(Label::primary(error.span, error.message.clone()))
    };
    emitter.emit(diagnostic);
}

pub fn collect_experimental_function_features(
    module: &agam_ast::Module,
    usages: &mut Vec<ExperimentalFeatureUsage>,
) {
    for decl in &module.declarations {
        let DeclKind::Function(function) = &decl.kind else {
            continue;
        };
        for annotation in &function.annotations {
            if annotation.name.name.as_str() == "experimental.call_cache.optimize" {
                usages.push(ExperimentalFeatureUsage {
                    feature: ExperimentalFeature::CallCacheOptimize,
                    span: annotation.span,
                });
            }
        }
    }
}

pub fn emit_experimental_feature_warnings(
    emitter: &mut DiagnosticEmitter,
    usages: &[ExperimentalFeatureUsage],
) {
    let mut emitted = HashSet::new();
    for usage in usages {
        if !emitted.insert((usage.feature, usage.span)) {
            continue;
        }
        let spec = usage.feature.spec();
        emitter.emit(
            Diagnostic::warning(spec.code, spec.warning)
                .with_label(Label::primary(
                    usage.span,
                    format!("`{}` is enabled here", spec.annotation),
                ))
                .with_help(spec.help),
        );
    }
}

fn skip_trivia_tokens(tokens: &[Token], mut index: usize) -> usize {
    while let Some(token) = tokens.get(index) {
        match token.kind {
            TokenKind::Newline
            | TokenKind::LineComment
            | TokenKind::BlockComment
            | TokenKind::DocComment => index += 1,
            _ => break,
        }
    }
    index
}

struct ParsedAnnotationName {
    name: String,
    span: Span,
    next_index: usize,
}

fn parse_annotation_name(tokens: &[Token], start: usize) -> Option<ParsedAnnotationName> {
    if tokens.get(start)?.kind != TokenKind::At {
        return None;
    }
    let mut index = start + 1;
    let mut parts = Vec::new();
    let start_span = tokens.get(start)?.span.start;
    let source_id = tokens.get(start)?.span.source_id;
    let mut end_span;

    loop {
        let token = tokens.get(index)?;
        if token.kind != TokenKind::Identifier {
            return None;
        }
        parts.push(token.lexeme.clone());
        end_span = token.span.end;
        index += 1;

        match tokens.get(index).map(|token| token.kind) {
            Some(TokenKind::Dot) => {
                index += 1;
            }
            _ => break,
        }
    }

    Some(ParsedAnnotationName {
        name: parts.join("."),
        span: Span::new(source_id, start_span, end_span),
        next_index: index,
    })
}
