//! Semantic analysis, AST indexing, and code intelligence engine for LSP.

use agam_ast::decl::DeclKind;
use agam_ast::pattern::PatternKind;
use agam_errors::span::SourceId;
use agam_lexer::tokenize;
use agam_parser::parse;
use serde_json::{Value, json};

// LSP Symbol Kinds
pub const SYMBOL_KIND_FILE: u8 = 1;
pub const SYMBOL_KIND_MODULE: u8 = 2;
pub const SYMBOL_KIND_CLASS: u8 = 5;
pub const SYMBOL_KIND_METHOD: u8 = 6;
pub const SYMBOL_KIND_PROPERTY: u8 = 7;
pub const SYMBOL_KIND_FIELD: u8 = 8;
pub const SYMBOL_KIND_CONSTRUCTOR: u8 = 9;
pub const SYMBOL_KIND_ENUM: u8 = 10;
pub const SYMBOL_KIND_INTERFACE: u8 = 11;
pub const SYMBOL_KIND_FUNCTION: u8 = 12;
pub const SYMBOL_KIND_VARIABLE: u8 = 13;
pub const SYMBOL_KIND_CONSTANT: u8 = 14;
pub const SYMBOL_KIND_STRING: u8 = 15;
pub const SYMBOL_KIND_NUMBER: u8 = 16;
pub const SYMBOL_KIND_BOOLEAN: u8 = 17;
pub const SYMBOL_KIND_ARRAY: u8 = 18;
pub const SYMBOL_KIND_STRUCT: u8 = 23;

// LSP Completion Item Kinds
pub const COMPLETION_KIND_TEXT: u8 = 1;
pub const COMPLETION_KIND_METHOD: u8 = 2;
pub const COMPLETION_KIND_FUNCTION: u8 = 3;
pub const COMPLETION_KIND_CONSTRUCTOR: u8 = 4;
pub const COMPLETION_KIND_FIELD: u8 = 5;
pub const COMPLETION_KIND_VARIABLE: u8 = 6;
pub const COMPLETION_KIND_CLASS: u8 = 7;
pub const COMPLETION_KIND_INTERFACE: u8 = 8;
pub const COMPLETION_KIND_MODULE: u8 = 9;
pub const COMPLETION_KIND_PROPERTY: u8 = 10;
pub const COMPLETION_KIND_UNIT: u8 = 11;
pub const COMPLETION_KIND_VALUE: u8 = 12;
pub const COMPLETION_KIND_ENUM: u8 = 13;
pub const COMPLETION_KIND_KEYWORD: u8 = 14;
pub const COMPLETION_KIND_SNIPPET: u8 = 15;
pub const COMPLETION_KIND_STRUCT: u8 = 22;

/// Map byte offset in source string to 0-indexed (line, character).
pub fn offset_to_position(source: &str, offset: usize) -> (u32, u32) {
    let mut line = 0u32;
    let mut character = 0u32;
    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            character = 0;
        } else if ch != '\r' {
            character += 1;
        }
    }
    (line, character)
}

/// Map 0-indexed (line, character) to byte offset in source string.
pub fn position_to_offset(source: &str, line: u32, character: u32) -> usize {
    let mut cur_line = 0u32;
    let mut cur_col = 0u32;
    for (i, ch) in source.char_indices() {
        if cur_line == line && cur_col == character {
            return i;
        }
        if ch == '\n' {
            if cur_line == line {
                return i;
            }
            cur_line += 1;
            cur_col = 0;
        } else if ch != '\r' {
            cur_col += 1;
        }
    }
    source.len()
}

/// Extract word/identifier at given line and character.
pub fn word_at_position(source: &str, line: u32, character: u32) -> Option<String> {
    let offset = position_to_offset(source, line, character);
    let bytes = source.as_bytes();
    if bytes.is_empty() {
        return None;
    }

    let mut start = offset.min(bytes.len().saturating_sub(1));
    while start > 0 && is_ident_byte(bytes[start - 1]) {
        start -= 1;
    }

    let mut end = offset.min(bytes.len());
    while end < bytes.len() && is_ident_byte(bytes[end]) {
        end += 1;
    }

    if start < end {
        Some(source[start..end].to_string())
    } else {
        None
    }
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'.'
}

/// Compute syntax and semantic diagnostics for a document.
pub fn compute_diagnostics(source: &str) -> Vec<Value> {
    let mut diagnostics = Vec::new();
    let source_id = SourceId(0);
    let tokens = tokenize(source, source_id);

    match parse(tokens, source_id) {
        Ok(_) => {}
        Err(errs) => {
            for err in errs {
                let (start_line, start_col) = offset_to_position(source, err.span.start as usize);
                let (end_line, end_col) = offset_to_position(source, err.span.end as usize);
                diagnostics.push(json!({
                    "range": {
                        "start": { "line": start_line, "character": start_col },
                        "end": { "line": end_line, "character": end_col.max(start_col + 1) }
                    },
                    "severity": 1, // Error
                    "source": "agamc",
                    "message": err.message
                }));
            }
        }
    }

    diagnostics
}

/// Compute hover information at position.
pub fn compute_hover(source: &str, line: u32, character: u32) -> Option<Value> {
    let word = word_at_position(source, line, character)?;
    let clean_word = word.trim_matches('.');

    // 1. Built-in keywords
    let keyword_doc = match clean_word {
        "fn" => {
            Some("**`fn` keyword** — Declares a function with parameters, return type, and body.")
        }
        "let" => Some(
            "**`let` keyword** — Binds a variable locally. Use `let mut` for mutable bindings.",
        ),
        "mut" => Some("**`mut` keyword** — Declares a variable or parameter as mutable."),
        "struct" => Some("**`struct` keyword** — Defines a nominal aggregate data structure."),
        "enum" => Some("**`enum` keyword** — Defines an algebraic tagged union type."),
        "trait" => Some("**`trait` keyword** — Defines a behavioral interface contract."),
        "impl" => Some("**`impl` keyword** — Implements inherent methods or traits for a type."),
        "async" => Some("**`async` keyword** — Declares a coroutine function returning a future."),
        "await" => {
            Some("**`await` operator** — Suspends the current coroutine until the future resolves.")
        }
        "effect" => Some("**`effect` keyword** — Defines an algebraic effect signature."),
        "handle" => {
            Some("**`handle` keyword** — Handles an algebraic effect with a resumption closure.")
        }
        "resume" => Some("**`resume` keyword** — Resumes an algebraic effect continuation."),
        "match" => Some("**`match` keyword** — Performs pattern matching over expressions."),
        _ => None,
    };

    if let Some(doc) = keyword_doc {
        return Some(json!({
            "contents": {
                "kind": "markdown",
                "value": doc
            }
        }));
    }

    // 2. Built-in primitive types
    let type_doc = match clean_word {
        "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => {
            Some(format!("**`{clean_word}`** — Signed integer primitive."))
        }
        "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => {
            Some(format!("**`{clean_word}`** — Unsigned integer primitive."))
        }
        "f32" | "f64" => Some(format!(
            "**`{clean_word}`** — IEEE 754 floating-point primitive."
        )),
        "bool" => Some("**`bool`** — Boolean truth value (`true` or `false`).".to_string()),
        "str" => Some("**`str`** — UTF-8 string slice primitive.".to_string()),
        "char" => Some("**`char`** — 32-bit Unicode scalar character.".to_string()),
        "unit" => Some("**`unit` / `()`** — The zero-sized unit type.".to_string()),
        "Tensor" => {
            Some("**`Tensor[T, Shape]`** — First-class multi-dimensional tensor type.".to_string())
        }
        _ => None,
    };

    if let Some(doc) = type_doc {
        return Some(json!({
            "contents": {
                "kind": "markdown",
                "value": doc
            }
        }));
    }

    // 3. User-defined items in AST
    let source_id = SourceId(0);
    let tokens = tokenize(source, source_id);
    if let Ok(module) = parse(tokens, source_id) {
        for decl in &module.declarations {
            match &decl.kind {
                DeclKind::Function(f) if f.name.name == clean_word => {
                    let mut doc_text = String::new();
                    if !decl.doc_comments.is_empty() {
                        doc_text.push_str(&decl.doc_comments.join("\n"));
                        doc_text.push_str("\n\n");
                    }
                    doc_text.push_str(&format!("```agam\nfn {}(...) -> ...\n```", f.name.name));
                    return Some(json!({
                        "contents": {
                            "kind": "markdown",
                            "value": doc_text
                        }
                    }));
                }
                DeclKind::Struct(s) if s.name.name == clean_word => {
                    let mut doc_text = String::new();
                    if !decl.doc_comments.is_empty() {
                        doc_text.push_str(&decl.doc_comments.join("\n"));
                        doc_text.push_str("\n\n");
                    }
                    doc_text.push_str(&format!("```agam\nstruct {} {{ ... }}\n```", s.name.name));
                    return Some(json!({
                        "contents": {
                            "kind": "markdown",
                            "value": doc_text
                        }
                    }));
                }
                DeclKind::Enum(e) if e.name.name == clean_word => {
                    return Some(json!({
                        "contents": {
                            "kind": "markdown",
                            "value": format!("```agam\nenum {} {{ ... }}\n```", e.name.name)
                        }
                    }));
                }
                DeclKind::Trait(t) if t.name.name == clean_word => {
                    return Some(json!({
                        "contents": {
                            "kind": "markdown",
                            "value": format!("```agam\ntrait {} {{ ... }}\n```", t.name.name)
                        }
                    }));
                }
                _ => {}
            }
        }
    }

    None
}

/// Compute auto-completion suggestions.
pub fn compute_completion(source: &str, _line: u32, _character: u32) -> Vec<Value> {
    let mut items = Vec::new();

    // 1. Keywords
    let keywords = [
        ("fn", COMPLETION_KIND_KEYWORD, "Function declaration"),
        ("let", COMPLETION_KIND_KEYWORD, "Variable binding"),
        ("mut", COMPLETION_KIND_KEYWORD, "Mutable qualifier"),
        ("struct", COMPLETION_KIND_KEYWORD, "Struct definition"),
        ("enum", COMPLETION_KIND_KEYWORD, "Enum definition"),
        ("trait", COMPLETION_KIND_KEYWORD, "Trait definition"),
        ("impl", COMPLETION_KIND_KEYWORD, "Implementation block"),
        ("if", COMPLETION_KIND_KEYWORD, "Conditional expression"),
        ("else", COMPLETION_KIND_KEYWORD, "Else branch"),
        ("while", COMPLETION_KIND_KEYWORD, "While loop"),
        ("for", COMPLETION_KIND_KEYWORD, "For loop"),
        ("in", COMPLETION_KIND_KEYWORD, "Iterator binding"),
        ("match", COMPLETION_KIND_KEYWORD, "Pattern matching"),
        ("async", COMPLETION_KIND_KEYWORD, "Async function"),
        ("await", COMPLETION_KIND_KEYWORD, "Await future"),
        ("return", COMPLETION_KIND_KEYWORD, "Return from function"),
        ("break", COMPLETION_KIND_KEYWORD, "Break loop"),
        ("continue", COMPLETION_KIND_KEYWORD, "Continue loop"),
        ("pub", COMPLETION_KIND_KEYWORD, "Public visibility"),
        ("effect", COMPLETION_KIND_KEYWORD, "Algebraic effect"),
        ("handle", COMPLETION_KIND_KEYWORD, "Effect handler"),
        ("resume", COMPLETION_KIND_KEYWORD, "Resume continuation"),
    ];

    for (kw, kind, detail) in keywords {
        items.push(json!({
            "label": kw,
            "kind": kind,
            "detail": detail
        }));
    }

    // 2. Types
    let types = [
        "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
        "f32", "f64", "bool", "str", "char", "unit", "Tensor",
    ];
    for ty in types {
        items.push(json!({
            "label": ty,
            "kind": COMPLETION_KIND_CLASS,
            "detail": "Built-in primitive type"
        }));
    }

    // 3. Built-in functions
    let builtins = [
        ("print", "print(value)"),
        ("println", "println(value)"),
        ("len", "len(collection) -> usize"),
        ("assert", "assert(condition)"),
        ("panic", "panic(message)"),
        ("agam.gpu.thread_id_x", "agam.gpu.thread_id_x() -> i32"),
        ("agam.gpu.block_id_x", "agam.gpu.block_id_x() -> i32"),
        ("agam.gpu.block_dim_x", "agam.gpu.block_dim_x() -> i32"),
        ("agam.gpu.barrier", "agam.gpu.barrier()"),
    ];
    for (name, detail) in builtins {
        items.push(json!({
            "label": name,
            "kind": COMPLETION_KIND_FUNCTION,
            "detail": detail
        }));
    }

    // 4. File-local AST items
    let source_id = SourceId(0);
    let tokens = tokenize(source, source_id);
    if let Ok(module) = parse(tokens, source_id) {
        for decl in &module.declarations {
            match &decl.kind {
                DeclKind::Function(f) => items.push(json!({
                    "label": f.name.name,
                    "kind": COMPLETION_KIND_FUNCTION,
                    "detail": format!("fn {}(...)", f.name.name)
                })),
                DeclKind::Struct(s) => items.push(json!({
                    "label": s.name.name,
                    "kind": COMPLETION_KIND_STRUCT,
                    "detail": format!("struct {}", s.name.name)
                })),
                DeclKind::Enum(e) => items.push(json!({
                    "label": e.name.name,
                    "kind": COMPLETION_KIND_ENUM,
                    "detail": format!("enum {}", e.name.name)
                })),
                DeclKind::Trait(t) => items.push(json!({
                    "label": t.name.name,
                    "kind": COMPLETION_KIND_INTERFACE,
                    "detail": format!("trait {}", t.name.name)
                })),
                _ => {}
            }
        }
    }

    items
}

/// Compute go-to-definition location.
pub fn compute_definition(uri: &str, source: &str, line: u32, character: u32) -> Option<Value> {
    let word = word_at_position(source, line, character)?;
    let clean_word = word.trim_matches('.');

    let source_id = SourceId(0);
    let tokens = tokenize(source, source_id);
    if let Ok(module) = parse(tokens, source_id) {
        for decl in &module.declarations {
            match &decl.kind {
                DeclKind::Function(f) if f.name.name == clean_word => {
                    let (start_l, start_c) = offset_to_position(source, decl.span.start as usize);
                    let (end_l, end_c) = offset_to_position(source, decl.span.end as usize);
                    return Some(json!({
                        "uri": uri,
                        "range": {
                            "start": { "line": start_l, "character": start_c },
                            "end": { "line": end_l, "character": end_c }
                        }
                    }));
                }
                DeclKind::Struct(s) if s.name.name == clean_word => {
                    let (start_l, start_c) = offset_to_position(source, decl.span.start as usize);
                    let (end_l, end_c) = offset_to_position(source, decl.span.end as usize);
                    return Some(json!({
                        "uri": uri,
                        "range": {
                            "start": { "line": start_l, "character": start_c },
                            "end": { "line": end_l, "character": end_c }
                        }
                    }));
                }
                DeclKind::Enum(e) if e.name.name == clean_word => {
                    let (start_l, start_c) = offset_to_position(source, decl.span.start as usize);
                    let (end_l, end_c) = offset_to_position(source, decl.span.end as usize);
                    return Some(json!({
                        "uri": uri,
                        "range": {
                            "start": { "line": start_l, "character": start_c },
                            "end": { "line": end_l, "character": end_c }
                        }
                    }));
                }
                DeclKind::Trait(t) if t.name.name == clean_word => {
                    let (start_l, start_c) = offset_to_position(source, decl.span.start as usize);
                    let (end_l, end_c) = offset_to_position(source, decl.span.end as usize);
                    return Some(json!({
                        "uri": uri,
                        "range": {
                            "start": { "line": start_l, "character": start_c },
                            "end": { "line": end_l, "character": end_c }
                        }
                    }));
                }
                _ => {}
            }
        }
    }

    None
}

/// Compute document symbol outline.
pub fn compute_document_symbols(source: &str) -> Vec<Value> {
    let mut symbols = Vec::new();
    let source_id = SourceId(0);
    let tokens = tokenize(source, source_id);

    if let Ok(module) = parse(tokens, source_id) {
        for decl in &module.declarations {
            let (start_l, start_c) = offset_to_position(source, decl.span.start as usize);
            let (end_l, end_c) = offset_to_position(source, decl.span.end as usize);
            let range = json!({
                "start": { "line": start_l, "character": start_c },
                "end": { "line": end_l, "character": end_c }
            });

            match &decl.kind {
                DeclKind::Function(f) => symbols.push(json!({
                    "name": f.name.name,
                    "kind": SYMBOL_KIND_FUNCTION,
                    "range": range,
                    "selectionRange": range,
                    "detail": "function"
                })),
                DeclKind::Struct(s) => symbols.push(json!({
                    "name": s.name.name,
                    "kind": SYMBOL_KIND_STRUCT,
                    "range": range,
                    "selectionRange": range,
                    "detail": "struct"
                })),
                DeclKind::Enum(e) => symbols.push(json!({
                    "name": e.name.name,
                    "kind": SYMBOL_KIND_ENUM,
                    "range": range,
                    "selectionRange": range,
                    "detail": "enum"
                })),
                DeclKind::Trait(t) => symbols.push(json!({
                    "name": t.name.name,
                    "kind": SYMBOL_KIND_INTERFACE,
                    "range": range,
                    "selectionRange": range,
                    "detail": "trait"
                })),
                DeclKind::Impl(i) => symbols.push(json!({
                    "name": format!("impl {:?}", i.target_type),
                    "kind": SYMBOL_KIND_CLASS,
                    "range": range,
                    "selectionRange": range,
                    "detail": "impl"
                })),
                _ => {}
            }
        }
    }

    symbols
}

/// Find all references to identifier under cursor.
pub fn compute_references(uri: &str, source: &str, line: u32, character: u32) -> Vec<Value> {
    let mut locations = Vec::new();
    let Some(word) = word_at_position(source, line, character) else {
        return locations;
    };
    let clean_word = word.trim_matches('.');

    let source_id = SourceId(0);
    let tokens = tokenize(source, source_id);
    for tok in &tokens {
        if tok.lexeme == clean_word {
            let (start_l, start_c) = offset_to_position(source, tok.span.start as usize);
            let (end_l, end_c) = offset_to_position(source, tok.span.end as usize);
            locations.push(json!({
                "uri": uri,
                "range": {
                    "start": { "line": start_l, "character": start_c },
                    "end": { "line": end_l, "character": end_c }
                }
            }));
        }
    }

    locations
}

/// Compute signature help on active call site.
pub fn compute_signature_help(source: &str, line: u32, character: u32) -> Option<Value> {
    let offset = position_to_offset(source, line, character);
    let prefix = &source[..offset.min(source.len())];

    // Find the last unclosed '('
    let open_paren_idx = prefix.rfind('(')?;
    let before_paren = prefix[..open_paren_idx].trim_end();
    let func_name = before_paren
        .rsplit(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .next()?;

    let comma_count = prefix[open_paren_idx..]
        .chars()
        .filter(|&c| c == ',')
        .count() as u32;

    let source_id = SourceId(0);
    let tokens = tokenize(source, source_id);
    if let Ok(module) = parse(tokens.clone(), source_id) {
        for decl in &module.declarations {
            if let DeclKind::Function(f) = &decl.kind
                && f.name.name == func_name
            {
                let params: Vec<Value> = f
                    .params
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        let label = match &p.pattern.kind {
                            PatternKind::Identifier { name, .. } => name.name.clone(),
                            _ => format!("param_{i}"),
                        };
                        json!({
                            "label": label
                        })
                    })
                    .collect();

                let sig_label = format!("fn {}(...)", f.name.name);
                return Some(json!({
                    "signatures": [{
                        "label": sig_label,
                        "parameters": params
                    }],
                    "activeSignature": 0,
                    "activeParameter": comma_count
                }));
            }
        }
    }

    // Fallback: Scan token stream for `fn <func_name>(...)` in incomplete/editing documents
    let mut i = 0;
    while i < tokens.len() {
        if tokens[i].lexeme == "fn" && i + 1 < tokens.len() && tokens[i + 1].lexeme == func_name {
            let mut j = i + 2;
            while j < tokens.len() && tokens[j].lexeme != "(" {
                j += 1;
            }
            if j < tokens.len() {
                j += 1;
                let mut params = Vec::new();
                let mut current_param = String::new();
                while j < tokens.len() && tokens[j].lexeme != ")" {
                    if tokens[j].lexeme == "," {
                        if !current_param.is_empty() {
                            params.push(json!({ "label": current_param.trim() }));
                            current_param.clear();
                        }
                    } else {
                        if !current_param.is_empty() {
                            current_param.push(' ');
                        }
                        current_param.push_str(&tokens[j].lexeme);
                    }
                    j += 1;
                }
                if !current_param.is_empty() {
                    params.push(json!({ "label": current_param.trim() }));
                }

                let sig_label = format!("fn {}(...)", func_name);
                return Some(json!({
                    "signatures": [{
                        "label": sig_label,
                        "parameters": params
                    }],
                    "activeSignature": 0,
                    "activeParameter": comma_count
                }));
            }
        }
        i += 1;
    }

    None
}
