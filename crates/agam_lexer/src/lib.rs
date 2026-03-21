//! # agam_lexer
//!
//! Dual-mode tokenizer for the Agam programming language.
//!
//! Supports two syntax modes:
//! - `@lang.base` — Python-like, indentation-significant
//! - `@lang.advance` — C++/Java-like, brace-delimited
//!
//! Both modes produce the same `Token` stream, enabling a unified parser.

mod token;
mod cursor;
mod lexer;

pub use token::{Token, TokenKind};
pub use lexer::Lexer;

use agam_errors::span::SourceId;

/// Convenience function: tokenize a source string into a Vec of tokens.
pub fn tokenize(source: &str, source_id: SourceId) -> Vec<Token> {
    let mut lexer = Lexer::new(source, source_id);
    let mut tokens = Vec::new();
    loop {
        let tok = lexer.next_token();
        let is_eof = tok.kind == TokenKind::Eof;
        tokens.push(tok);
        if is_eof {
            break;
        }
    }
    tokens
}
