//! # agam_parser
//!
//! Recursive-descent / Pratt parser for the Agam language.

mod parser;

pub use parser::Parser;

use agam_ast::Module;
use agam_errors::span::SourceId;
use agam_lexer::Token;

/// Parse a token stream into an AST module.
pub fn parse(tokens: Vec<Token>, source_id: SourceId) -> Result<Module, Vec<ParseError>> {
    let mut parser = Parser::new(tokens);
    parser.parse_module(source_id)
}

/// A parse error.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: agam_errors::Span,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "parse error: {}", self.message)
    }
}
