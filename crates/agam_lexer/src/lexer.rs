//! The main lexer/scanner for Agam source code.
//!
//! Converts a source string into a stream of [`Token`]s. Handles both
//! syntax modes, all operators, keywords, literals, and comments.

use crate::cursor::Cursor;
use crate::token::{Token, TokenKind, lookup_keyword};
use agam_errors::span::{SourceId, Span};
use std::collections::VecDeque;

/// Syntax mode for the file being lexed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxMode {
    /// Python-like significant indentation, but requires `let` or `var` for declarations.
    BaseStatic,
    /// Pure Python style: implicit variable declarations (`x = 1`) at runtime.
    BaseDynamic,
    /// C-like braces and strict semicolons.
    Advance,
}

/// The Agam lexer.
pub struct Lexer<'src> {
    cursor: Cursor<'src>,
    source_id: SourceId,
    /// The detected syntax mode of the file.
    pub mode: SyntaxMode,
    /// Stack of indentation depths (in spaces) for Base mode.
    indent_stack: Vec<usize>,
    /// Buffer of tokens waiting to be emitted (used for multiple Dedents).
    pending: VecDeque<Token>,
    /// Whether we've reached EOF.
    eof_emitted: bool,
}

impl<'src> Lexer<'src> {
    /// Create a new lexer for the given source.
    pub fn new(source: &'src str, source_id: SourceId) -> Self {
        let mut lexer = Self {
            cursor: Cursor::new(source),
            source_id,
            mode: SyntaxMode::Advance, // Default, will be updated by detect_mode
            indent_stack: vec![0],
            pending: VecDeque::new(),
            eof_emitted: false,
        };
        lexer.detect_mode();
        lexer
    }

    /// Detect the mode from the first line (e.g., `@lang.base`)
    fn detect_mode(&mut self) {
        let saved_pos = self.cursor.pos();
        // Skip BOM or initial whitespaces
        self.cursor
            .eat_while(|c| c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\u{FEFF}');

        if self.cursor.starts_with("@lang.base.dynamic") {
            self.mode = SyntaxMode::BaseDynamic;
        } else if self.cursor.starts_with("@lang.base") {
            self.mode = SyntaxMode::BaseStatic; // Base explicitly requires `let` / `var`
        } else if self.cursor.starts_with("@lang.advance") {
            self.mode = SyntaxMode::Advance;
        } else {
            // Default to BaseStatic if no annotation
            self.mode = SyntaxMode::BaseStatic;
        }

        // Restore cursor; we'll parse the annotation as normal tokens (e.g. `@`, `lang`, `.`, `base`)
        self.cursor.reset_pos(saved_pos);
    }

    /// Produce the next token.
    pub fn next_token(&mut self) -> Token {
        if let Some(tok) = self.pending.pop_front() {
            return tok;
        }

        if self.eof_emitted {
            return self.make_token(TokenKind::Eof, self.cursor.pos(), "");
        }

        // Handle line starts for indentation tracking before skipping whitespace
        let is_line_start = self.cursor.pos() == 0 || self.cursor.peek_prev() == Some('\n');

        let start_pos_before_ws = self.cursor.pos();
        self.skip_whitespace();
        let spaces = self.cursor.pos() - start_pos_before_ws;

        if self.cursor.is_eof() {
            // Emit remaining dedents before EOF
            if self.mode == SyntaxMode::BaseStatic || self.mode == SyntaxMode::BaseDynamic {
                while self.indent_stack.len() > 1 {
                    self.indent_stack.pop();
                    self.pending.push_back(self.make_token(
                        TokenKind::Dedent,
                        self.cursor.pos(),
                        "",
                    ));
                }
            }
            self.eof_emitted = true;
            if let Some(tok) = self.pending.pop_front() {
                self.pending
                    .push_back(self.make_token(TokenKind::Eof, self.cursor.pos(), ""));
                return tok;
            }
            return self.make_token(TokenKind::Eof, self.cursor.pos(), "");
        }

        // If in base mode, handle indentation if we're at the start of a line and it's not empty
        if (self.mode == SyntaxMode::BaseStatic || self.mode == SyntaxMode::BaseDynamic)
            && is_line_start
        {
            if let Some(c) = self.cursor.peek() {
                // Ignore indentation on empty lines or comments
                if c != '\n'
                    && c != '\r'
                    && c != '#'
                    && !(c == '/' && self.cursor.peek_second() == Some('/'))
                {
                    let current_indent = spaces;
                    let last_indent = *self.indent_stack.last().unwrap();

                    if current_indent > last_indent {
                        self.indent_stack.push(current_indent);
                        self.pending.push_back(self.make_token(
                            TokenKind::Indent,
                            self.cursor.pos(),
                            "",
                        ));
                    } else if current_indent < last_indent {
                        while let Some(&top) = self.indent_stack.last() {
                            if top > current_indent {
                                self.indent_stack.pop();
                                self.pending.push_back(self.make_token(
                                    TokenKind::Dedent,
                                    self.cursor.pos(),
                                    "",
                                ));
                            } else {
                                break;
                            }
                        }
                    }

                    if let Some(tok) = self.pending.pop_front() {
                        return tok;
                    }
                }
            }
        }

        let start = self.cursor.pos();
        let c = self.cursor.advance().unwrap();

        match c {
            // ── Newlines ──
            '\n' => self.make_token(TokenKind::Newline, start, "\n"),
            '\r' => {
                // Handle \r\n as a single newline
                if self.cursor.peek() == Some('\n') {
                    self.cursor.advance();
                }
                self.make_token(TokenKind::Newline, start, "\n")
            }

            // ── Single-char delimiters ──
            '(' => self.make_token(TokenKind::LParen, start, "("),
            ')' => self.make_token(TokenKind::RParen, start, ")"),
            '[' => self.make_token(TokenKind::LBracket, start, "["),
            ']' => self.make_token(TokenKind::RBracket, start, "]"),
            '{' => self.make_token(TokenKind::LBrace, start, "{"),
            '}' => self.make_token(TokenKind::RBrace, start, "}"),
            ',' => self.make_token(TokenKind::Comma, start, ","),
            ';' => self.make_token(TokenKind::Semicolon, start, ";"),
            '~' => self.make_token(TokenKind::Tilde, start, "~"),
            '@' => self.make_token(TokenKind::At, start, "@"),

            // ── Colon / ColonColon ──
            ':' => {
                if self.cursor.peek() == Some(':') {
                    self.cursor.advance();
                    self.make_token(TokenKind::ColonColon, start, "::")
                } else {
                    self.make_token(TokenKind::Colon, start, ":")
                }
            }

            // ── Dot / DotDot / DotDotDot ──
            '.' => {
                if self.cursor.peek() == Some('.') {
                    self.cursor.advance();
                    if self.cursor.peek() == Some('.') {
                        self.cursor.advance();
                        self.make_token(TokenKind::DotDotDot, start, "...")
                    } else {
                        self.make_token(TokenKind::DotDot, start, "..")
                    }
                } else {
                    self.make_token(TokenKind::Dot, start, ".")
                }
            }

            // ── Plus / PlusEq ──
            '+' => {
                if self.cursor.peek() == Some('=') {
                    self.cursor.advance();
                    self.make_token(TokenKind::PlusEq, start, "+=")
                } else {
                    self.make_token(TokenKind::Plus, start, "+")
                }
            }

            // ── Minus / MinusEq / Arrow ──
            '-' => {
                if self.cursor.peek() == Some('=') {
                    self.cursor.advance();
                    self.make_token(TokenKind::MinusEq, start, "-=")
                } else if self.cursor.peek() == Some('>') {
                    self.cursor.advance();
                    self.make_token(TokenKind::Arrow, start, "->")
                } else {
                    self.make_token(TokenKind::Minus, start, "-")
                }
            }

            // ── Star / StarEq / DoubleStar ──
            '*' => {
                if self.cursor.peek() == Some('=') {
                    self.cursor.advance();
                    self.make_token(TokenKind::StarEq, start, "*=")
                } else if self.cursor.peek() == Some('*') {
                    self.cursor.advance();
                    self.make_token(TokenKind::DoubleStar, start, "**")
                } else {
                    self.make_token(TokenKind::Star, start, "*")
                }
            }

            // ── Slash / SlashEq / Line Comment / Block Comment ──
            '/' => {
                if self.cursor.peek() == Some('/') {
                    self.cursor.advance();
                    // Check for doc comment ///
                    if self.cursor.peek() == Some('/') {
                        self.cursor.advance();
                        self.scan_line_comment(start, TokenKind::DocComment)
                    } else {
                        self.scan_line_comment(start, TokenKind::LineComment)
                    }
                } else if self.cursor.peek() == Some('*') {
                    self.cursor.advance();
                    self.scan_block_comment(start)
                } else if self.cursor.peek() == Some('=') {
                    self.cursor.advance();
                    self.make_token(TokenKind::SlashEq, start, "/=")
                } else {
                    self.make_token(TokenKind::Slash, start, "/")
                }
            }

            // ── Hash comment (base mode) ──
            '#' => self.scan_line_comment(start, TokenKind::LineComment),

            // ── Percent / PercentEq ──
            '%' => {
                if self.cursor.peek() == Some('=') {
                    self.cursor.advance();
                    self.make_token(TokenKind::PercentEq, start, "%=")
                } else {
                    self.make_token(TokenKind::Percent, start, "%")
                }
            }

            // ── Eq / EqEq / FatArrow ──
            '=' => {
                if self.cursor.peek() == Some('=') {
                    self.cursor.advance();
                    self.make_token(TokenKind::EqEq, start, "==")
                } else if self.cursor.peek() == Some('>') {
                    self.cursor.advance();
                    self.make_token(TokenKind::FatArrow, start, "=>")
                } else {
                    self.make_token(TokenKind::Eq, start, "=")
                }
            }

            // ── Bang / BangEq ──
            '!' => {
                if self.cursor.peek() == Some('=') {
                    self.cursor.advance();
                    self.make_token(TokenKind::BangEq, start, "!=")
                } else {
                    self.make_token(TokenKind::Bang, start, "!")
                }
            }

            // ── Lt / LtEq / Shl ──
            '<' => {
                if self.cursor.peek() == Some('=') {
                    self.cursor.advance();
                    self.make_token(TokenKind::LtEq, start, "<=")
                } else if self.cursor.peek() == Some('<') {
                    self.cursor.advance();
                    self.make_token(TokenKind::Shl, start, "<<")
                } else {
                    self.make_token(TokenKind::Lt, start, "<")
                }
            }

            // ── Gt / GtEq / Shr ──
            '>' => {
                if self.cursor.peek() == Some('=') {
                    self.cursor.advance();
                    self.make_token(TokenKind::GtEq, start, ">=")
                } else if self.cursor.peek() == Some('>') {
                    self.cursor.advance();
                    self.make_token(TokenKind::Shr, start, ">>")
                } else {
                    self.make_token(TokenKind::Gt, start, ">")
                }
            }

            // ── Amp / AmpAmp ──
            '&' => {
                if self.cursor.peek() == Some('&') {
                    self.cursor.advance();
                    self.make_token(TokenKind::AmpAmp, start, "&&")
                } else {
                    self.make_token(TokenKind::Amp, start, "&")
                }
            }

            // ── Pipe / PipePipe ──
            '|' => {
                if self.cursor.peek() == Some('|') {
                    self.cursor.advance();
                    self.make_token(TokenKind::PipePipe, start, "||")
                } else {
                    self.make_token(TokenKind::Pipe, start, "|")
                }
            }

            // ── Caret ──
            '^' => self.make_token(TokenKind::Caret, start, "^"),

            // ── Question ──
            '?' => self.make_token(TokenKind::Question, start, "?"),

            // ── String literals ──
            '"' => self.scan_string(start, '"'),
            '\'' => self.scan_string(start, '\''),

            // ── Numeric literals ──
            '0'..='9' => self.scan_number(start, c),

            // ── Identifiers, keywords, f-strings, r-strings ──
            c if is_ident_start(c) => self.scan_identifier(start, c),

            // ── Unrecognized character ──
            _ => {
                let lexeme = self.cursor.slice_from(start);
                self.make_token(TokenKind::Error, start, lexeme)
            }
        }
    }

    // ── Helper: construct a token ──

    fn make_token(&self, kind: TokenKind, start: usize, lexeme: &str) -> Token {
        Token::new(
            kind,
            lexeme.to_string(),
            Span::new(self.source_id, start as u32, self.cursor.pos() as u32),
        )
    }

    // ── Whitespace (spaces and tabs only, not newlines) ──

    fn skip_whitespace(&mut self) {
        self.cursor.eat_while(|c| c == ' ' || c == '\t');
    }

    // ── Comments ──

    fn scan_line_comment(&mut self, start: usize, kind: TokenKind) -> Token {
        self.cursor.eat_while(|c| c != '\n');
        let lexeme = self.cursor.slice_from(start);
        self.make_token(kind, start, lexeme)
    }

    fn scan_block_comment(&mut self, start: usize) -> Token {
        let mut depth = 1;
        while depth > 0 && !self.cursor.is_eof() {
            if self.cursor.starts_with("/*") {
                self.cursor.advance();
                self.cursor.advance();
                depth += 1;
            } else if self.cursor.starts_with("*/") {
                self.cursor.advance();
                self.cursor.advance();
                depth -= 1;
            } else {
                self.cursor.advance();
            }
        }
        let lexeme = self.cursor.slice_from(start);
        self.make_token(TokenKind::BlockComment, start, lexeme)
    }

    // ── String scanning ──

    fn scan_string(&mut self, start: usize, quote: char) -> Token {
        // Check for triple-quoted string
        if self.cursor.peek() == Some(quote) && self.cursor.peek_second() == Some(quote) {
            // This would be the start of a triple-quoted string but we already consumed one quote char.
            // Actually: we've consumed the first quote. If next two are also quotes, it's triple-quoted.
            // But peek shows 2nd and 3rd quotes. Let's consume them.
            self.cursor.advance(); // 2nd quote
            self.cursor.advance(); // 3rd quote
            // Now scan until closing triple quotes
            loop {
                if self.cursor.is_eof() {
                    break;
                }
                if self.cursor.peek() == Some(quote) {
                    self.cursor.advance();
                    if self.cursor.peek() == Some(quote) {
                        self.cursor.advance();
                        if self.cursor.peek() == Some(quote) {
                            self.cursor.advance();
                            break;
                        }
                    }
                } else {
                    self.cursor.advance();
                }
            }
        } else {
            // Single-line string
            while let Some(c) = self.cursor.peek() {
                if c == quote {
                    self.cursor.advance();
                    break;
                }
                if c == '\\' {
                    self.cursor.advance(); // skip backslash
                    self.cursor.advance(); // skip escaped char
                    continue;
                }
                if c == '\n' {
                    break; // Unterminated string
                }
                self.cursor.advance();
            }
        }

        let lexeme = self.cursor.slice_from(start);
        self.make_token(TokenKind::StringLiteral, start, lexeme)
    }

    // ── Number scanning ──

    fn scan_number(&mut self, start: usize, first: char) -> Token {
        let mut is_float = false;

        if first == '0' {
            match self.cursor.peek() {
                Some('x') | Some('X') => {
                    self.cursor.advance();
                    self.cursor.eat_while(|c| c.is_ascii_hexdigit() || c == '_');
                    let lexeme = self.cursor.slice_from(start);
                    return self.make_token(TokenKind::IntLiteral, start, lexeme);
                }
                Some('b') | Some('B') => {
                    self.cursor.advance();
                    self.cursor.eat_while(|c| c == '0' || c == '1' || c == '_');
                    let lexeme = self.cursor.slice_from(start);
                    return self.make_token(TokenKind::IntLiteral, start, lexeme);
                }
                Some('o') | Some('O') => {
                    self.cursor.advance();
                    self.cursor
                        .eat_while(|c| ('0'..='7').contains(&c) || c == '_');
                    let lexeme = self.cursor.slice_from(start);
                    return self.make_token(TokenKind::IntLiteral, start, lexeme);
                }
                _ => {}
            }
        }

        // Decimal digits
        self.cursor.eat_while(|c| c.is_ascii_digit() || c == '_');

        // Fractional part
        if self.cursor.peek() == Some('.')
            && self
                .cursor
                .peek_second()
                .is_some_and(|c| c.is_ascii_digit())
        {
            is_float = true;
            self.cursor.advance(); // consume '.'
            self.cursor.eat_while(|c| c.is_ascii_digit() || c == '_');
        }

        // Exponent part
        if let Some('e' | 'E') = self.cursor.peek() {
            is_float = true;
            self.cursor.advance();
            if let Some('+' | '-') = self.cursor.peek() {
                self.cursor.advance();
            }
            self.cursor.eat_while(|c| c.is_ascii_digit() || c == '_');
        }

        let lexeme = self.cursor.slice_from(start);
        let kind = if is_float {
            TokenKind::FloatLiteral
        } else {
            TokenKind::IntLiteral
        };
        self.make_token(kind, start, lexeme)
    }

    // ── Identifier / keyword scanning ──

    fn scan_identifier(&mut self, start: usize, _first: char) -> Token {
        self.cursor.eat_while(is_ident_continue);
        let lexeme = self.cursor.slice_from(start);

        // Check for f-string: f"..."
        if lexeme == "f" && self.cursor.peek() == Some('"') {
            self.cursor.advance(); // consume opening "
            while let Some(c) = self.cursor.peek() {
                if c == '"' {
                    self.cursor.advance();
                    break;
                }
                if c == '\\' {
                    self.cursor.advance();
                    self.cursor.advance();
                    continue;
                }
                if c == '\n' {
                    break;
                }
                self.cursor.advance();
            }
            let full_lexeme = self.cursor.slice_from(start);
            return self.make_token(TokenKind::FStringLiteral, start, full_lexeme);
        }

        // Check for raw string: r"..."
        if lexeme == "r" && self.cursor.peek() == Some('"') {
            self.cursor.advance();
            while let Some(c) = self.cursor.peek() {
                if c == '"' {
                    self.cursor.advance();
                    break;
                }
                if c == '\n' {
                    break;
                }
                self.cursor.advance();
            }
            let full_lexeme = self.cursor.slice_from(start);
            return self.make_token(TokenKind::RawStringLiteral, start, full_lexeme);
        }

        // Check for keyword
        let kind = lookup_keyword(lexeme).unwrap_or(TokenKind::Identifier);
        self.make_token(kind, start, lexeme)
    }
}

/// Characters valid at the start of an identifier.
fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

/// Characters valid in the continuation of an identifier.
fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(source: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(source, SourceId(0));
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

    fn kinds(source: &str) -> Vec<TokenKind> {
        lex(source).iter().map(|t| t.kind).collect()
    }

    // ── Basic tokens ──

    #[test]
    fn test_empty_source() {
        assert_eq!(kinds(""), vec![TokenKind::Eof]);
    }

    #[test]
    fn test_single_char_tokens() {
        assert_eq!(
            kinds("( ) [ ] { } , ; . @"),
            vec![
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::LBracket,
                TokenKind::RBracket,
                TokenKind::LBrace,
                TokenKind::RBrace,
                TokenKind::Comma,
                TokenKind::Semicolon,
                TokenKind::Dot,
                TokenKind::At,
                TokenKind::Eof,
            ]
        );
    }

    // ── Operators ──

    #[test]
    fn test_compound_operators() {
        assert_eq!(
            kinds("== != <= >= -> => :: .. ... ** && || << >>"),
            vec![
                TokenKind::EqEq,
                TokenKind::BangEq,
                TokenKind::LtEq,
                TokenKind::GtEq,
                TokenKind::Arrow,
                TokenKind::FatArrow,
                TokenKind::ColonColon,
                TokenKind::DotDot,
                TokenKind::DotDotDot,
                TokenKind::DoubleStar,
                TokenKind::AmpAmp,
                TokenKind::PipePipe,
                TokenKind::Shl,
                TokenKind::Shr,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_assignment_operators() {
        assert_eq!(
            kinds("+= -= *= /= %="),
            vec![
                TokenKind::PlusEq,
                TokenKind::MinusEq,
                TokenKind::StarEq,
                TokenKind::SlashEq,
                TokenKind::PercentEq,
                TokenKind::Eof,
            ]
        );
    }

    // ── Keywords ──

    #[test]
    fn test_keywords() {
        assert_eq!(
            kinds("fn let const if else while for in return"),
            vec![
                TokenKind::Fn,
                TokenKind::Let,
                TokenKind::Const,
                TokenKind::If,
                TokenKind::Else,
                TokenKind::While,
                TokenKind::For,
                TokenKind::In,
                TokenKind::Return,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_more_keywords() {
        assert_eq!(
            kinds("struct enum trait impl pub match async await"),
            vec![
                TokenKind::Struct,
                TokenKind::Enum,
                TokenKind::Trait,
                TokenKind::Impl,
                TokenKind::Pub,
                TokenKind::Match,
                TokenKind::Async,
                TokenKind::Await,
                TokenKind::Eof,
            ]
        );
    }

    // ── Identifiers ──

    #[test]
    fn test_identifiers() {
        let tokens = lex("hello world _private __dunder my_var x1");
        let idents: Vec<_> = tokens
            .iter()
            .filter(|t| t.kind == TokenKind::Identifier)
            .map(|t| t.lexeme.as_str())
            .collect();
        assert_eq!(
            idents,
            vec!["hello", "world", "_private", "__dunder", "my_var", "x1"]
        );
    }

    // ── Numbers ──

    #[test]
    fn test_integers() {
        let tokens = lex("42 1_000 0xFF 0b1010 0o777");
        let nums: Vec<_> = tokens
            .iter()
            .filter(|t| t.kind == TokenKind::IntLiteral)
            .map(|t| t.lexeme.as_str())
            .collect();
        assert_eq!(nums, vec!["42", "1_000", "0xFF", "0b1010", "0o777"]);
    }

    #[test]
    fn test_floats() {
        let tokens = lex("3.14 1.0e10 2.5E-3 0.001");
        let nums: Vec<_> = tokens
            .iter()
            .filter(|t| t.kind == TokenKind::FloatLiteral)
            .map(|t| t.lexeme.as_str())
            .collect();
        assert_eq!(nums, vec!["3.14", "1.0e10", "2.5E-3", "0.001"]);
    }

    // ── Strings ──

    #[test]
    fn test_string_literal() {
        let tokens = lex(r#""hello world""#);
        assert_eq!(tokens[0].kind, TokenKind::StringLiteral);
        assert_eq!(tokens[0].lexeme, "\"hello world\"");
    }

    #[test]
    fn test_escaped_string() {
        let tokens = lex(r#""hello \"world\"""#);
        assert_eq!(tokens[0].kind, TokenKind::StringLiteral);
    }

    #[test]
    fn test_fstring() {
        let tokens = lex(r#"f"hello {name}""#);
        assert_eq!(tokens[0].kind, TokenKind::FStringLiteral);
    }

    #[test]
    fn test_raw_string() {
        let tokens = lex(r#"r"no \escapes""#);
        assert_eq!(tokens[0].kind, TokenKind::RawStringLiteral);
    }

    // ── Comments ──

    #[test]
    fn test_line_comment() {
        let tokens = lex("// this is a comment\n42");
        assert_eq!(tokens[0].kind, TokenKind::LineComment);
        assert_eq!(tokens[2].kind, TokenKind::IntLiteral);
    }

    #[test]
    fn test_hash_comment() {
        let tokens = lex("# python-style comment\n42");
        assert_eq!(tokens[0].kind, TokenKind::LineComment);
    }

    #[test]
    fn test_doc_comment() {
        let tokens = lex("/// Doc comment\n42");
        assert_eq!(tokens[0].kind, TokenKind::DocComment);
    }

    #[test]
    fn test_block_comment() {
        let tokens = lex("/* block */ 42");
        assert_eq!(tokens[0].kind, TokenKind::BlockComment);
        assert_eq!(tokens[1].kind, TokenKind::IntLiteral);
    }

    #[test]
    fn test_nested_block_comment() {
        let tokens = lex("/* outer /* inner */ end */ 42");
        assert_eq!(tokens[0].kind, TokenKind::BlockComment);
        assert_eq!(tokens[1].kind, TokenKind::IntLiteral);
    }

    // ── Full expressions ──

    #[test]
    fn test_function_decl() {
        assert_eq!(
            kinds("fn main() -> i32:"),
            vec![
                TokenKind::Fn,
                TokenKind::Identifier,
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::Arrow,
                TokenKind::Identifier,
                TokenKind::Colon,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_let_binding() {
        assert_eq!(
            kinds("let x: i32 = 42;"),
            vec![
                TokenKind::Let,
                TokenKind::Identifier,
                TokenKind::Colon,
                TokenKind::Identifier,
                TokenKind::Eq,
                TokenKind::IntLiteral,
                TokenKind::Semicolon,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_method_call() {
        assert_eq!(
            kinds("scores.iter().mean()"),
            vec![
                TokenKind::Identifier,
                TokenKind::Dot,
                TokenKind::Identifier,
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::Dot,
                TokenKind::Identifier,
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_booleans() {
        assert_eq!(
            kinds("true false"),
            vec![TokenKind::True, TokenKind::False, TokenKind::Eof]
        );
    }

    #[test]
    fn test_question_mark() {
        assert_eq!(
            kinds("result?"),
            vec![TokenKind::Identifier, TokenKind::Question, TokenKind::Eof]
        );
    }
}
