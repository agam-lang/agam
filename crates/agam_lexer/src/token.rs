//! Token definitions for the Agam language.
//!
//! The [`TokenKind`] enum defines every kind of token the lexer can produce.

use agam_errors::span::Span;
use std::fmt;

/// A token produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// What kind of token this is.
    pub kind: TokenKind,
    /// The source text of the token.
    pub lexeme: String,
    /// Location in the source file.
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, lexeme: String, span: Span) -> Self {
        Self { kind, lexeme, span }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}(\"{}\")", self.kind, self.lexeme)
    }
}

/// Every kind of token in the Agam language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    // ── Literals ──────────────────────────────────────────
    /// Integer literal (decimal, hex, binary, octal)
    IntLiteral,
    /// Floating-point literal
    FloatLiteral,
    /// String literal (single or double quoted)
    StringLiteral,
    /// Format string literal (f"...")
    FStringLiteral,
    /// Raw string literal (r"...")
    RawStringLiteral,
    /// Boolean `true`
    True,
    /// Boolean `false`
    False,

    // ── Identifiers & Keywords ───────────────────────────
    /// An identifier (variable, function, type name)
    Identifier,

    // Keywords
    Fn,
    Let,
    Const,
    Mut,
    If,
    Else,
    While,
    For,
    In,
    Loop,
    Break,
    Continue,
    Return,
    Match,
    Struct,
    Enum,
    Trait,
    Impl,
    Mod,
    Use,
    Pub,
    As,
    Type,
    Self_,       // `self`
    SelfType,    // `Self`
    Async,
    Await,
    Spawn,
    Try,
    Catch,
    Throw,
    Unsafe,
    Where,
    Yield,
    Import,
    Export,
    From,
    Class,
    /// `dyn` — dynamic typing mode
    Dyn,
    /// `static` — explicit static typing
    Static,
    /// `var` — variable with inferred/dynamic type
    Var,
    /// `grad` — automatic differentiation
    Grad,
    /// `backward` — reverse-mode autodiff
    Backward,
    /// `tensor` — tensor type
    Tensor,
    /// `strict` — strict ownership block
    Strict,

    // ── Operators ────────────────────────────────────────
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `%`
    Percent,
    /// `**`
    DoubleStar,
    /// `=`
    Eq,
    /// `==`
    EqEq,
    /// `!=`
    BangEq,
    /// `<`
    Lt,
    /// `<=`
    LtEq,
    /// `>`
    Gt,
    /// `>=`
    GtEq,
    /// `&&`
    AmpAmp,
    /// `||`
    PipePipe,
    /// `!`
    Bang,
    /// `&`
    Amp,
    /// `|`
    Pipe,
    /// `^`
    Caret,
    /// `~`
    Tilde,
    /// `<<`
    Shl,
    /// `>>`
    Shr,
    /// `+=`
    PlusEq,
    /// `-=`
    MinusEq,
    /// `*=`
    StarEq,
    /// `/=`
    SlashEq,
    /// `%=`
    PercentEq,
    /// `->`
    Arrow,
    /// `=>`
    FatArrow,
    /// `::`
    ColonColon,
    /// `..`
    DotDot,
    /// `...`
    DotDotDot,
    /// `?`
    Question,

    // ── Delimiters ───────────────────────────────────────
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `{`
    LBrace,
    /// `}`
    RBrace,

    // ── Punctuation ──────────────────────────────────────
    /// `,`
    Comma,
    /// `:`
    Colon,
    /// `;`
    Semicolon,
    /// `.`
    Dot,
    /// `@`
    At,
    /// `#`
    Hash,

    // ── Special ──────────────────────────────────────────
    /// A newline (significant in base mode for indentation)
    Newline,
    /// Indentation increase (synthesized in base mode)
    Indent,
    /// Indentation decrease (synthesized in base mode)
    Dedent,
    /// A line comment (`//` or `#` in base mode)
    LineComment,
    /// A block comment (`/* ... */`)
    BlockComment,
    /// A doc comment (`///`)
    DocComment,
    /// End of file
    Eof,
    /// An unrecognized character (error token)
    Error,
}

impl TokenKind {
    /// Check if this token is a keyword.
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            TokenKind::Fn
                | TokenKind::Let
                | TokenKind::Const
                | TokenKind::Mut
                | TokenKind::If
                | TokenKind::Else
                | TokenKind::While
                | TokenKind::For
                | TokenKind::In
                | TokenKind::Loop
                | TokenKind::Break
                | TokenKind::Continue
                | TokenKind::Return
                | TokenKind::Match
                | TokenKind::Struct
                | TokenKind::Enum
                | TokenKind::Trait
                | TokenKind::Impl
                | TokenKind::Mod
                | TokenKind::Use
                | TokenKind::Pub
                | TokenKind::As
                | TokenKind::Type
                | TokenKind::Self_
                | TokenKind::SelfType
                | TokenKind::Async
                | TokenKind::Await
                | TokenKind::Spawn
                | TokenKind::Try
                | TokenKind::Catch
                | TokenKind::Throw
                | TokenKind::Unsafe
                | TokenKind::Where
                | TokenKind::Yield
                | TokenKind::Import
                | TokenKind::Export
                | TokenKind::From
                | TokenKind::Class
                | TokenKind::Dyn
                | TokenKind::Static
                | TokenKind::Var
                | TokenKind::True
                | TokenKind::False
        )
    }

    /// Check if this token is a literal.
    pub fn is_literal(&self) -> bool {
        matches!(
            self,
            TokenKind::IntLiteral
                | TokenKind::FloatLiteral
                | TokenKind::StringLiteral
                | TokenKind::FStringLiteral
                | TokenKind::RawStringLiteral
                | TokenKind::True
                | TokenKind::False
        )
    }

    /// Check if this token is a binary operator.
    pub fn is_binary_op(&self) -> bool {
        matches!(
            self,
            TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::Percent
                | TokenKind::DoubleStar
                | TokenKind::EqEq
                | TokenKind::BangEq
                | TokenKind::Lt
                | TokenKind::LtEq
                | TokenKind::Gt
                | TokenKind::GtEq
                | TokenKind::AmpAmp
                | TokenKind::PipePipe
                | TokenKind::Amp
                | TokenKind::Pipe
                | TokenKind::Caret
                | TokenKind::Shl
                | TokenKind::Shr
        )
    }
}

/// Look up a keyword from an identifier string.
pub fn lookup_keyword(ident: &str) -> Option<TokenKind> {
    match ident {
        "fn" => Some(TokenKind::Fn),
        "let" => Some(TokenKind::Let),
        "const" => Some(TokenKind::Const),
        "mut" => Some(TokenKind::Mut),
        "if" => Some(TokenKind::If),
        "else" => Some(TokenKind::Else),
        "while" => Some(TokenKind::While),
        "for" => Some(TokenKind::For),
        "in" => Some(TokenKind::In),
        "loop" => Some(TokenKind::Loop),
        "break" => Some(TokenKind::Break),
        "continue" => Some(TokenKind::Continue),
        "return" => Some(TokenKind::Return),
        "match" => Some(TokenKind::Match),
        "struct" => Some(TokenKind::Struct),
        "enum" => Some(TokenKind::Enum),
        "trait" => Some(TokenKind::Trait),
        "impl" => Some(TokenKind::Impl),
        "mod" => Some(TokenKind::Mod),
        "use" => Some(TokenKind::Use),
        "pub" => Some(TokenKind::Pub),
        "as" => Some(TokenKind::As),
        "type" => Some(TokenKind::Type),
        "self" => Some(TokenKind::Self_),
        "Self" => Some(TokenKind::SelfType),
        "async" => Some(TokenKind::Async),
        "await" => Some(TokenKind::Await),
        "spawn" => Some(TokenKind::Spawn),
        "try" => Some(TokenKind::Try),
        "catch" => Some(TokenKind::Catch),
        "throw" => Some(TokenKind::Throw),
        "unsafe" => Some(TokenKind::Unsafe),
        "where" => Some(TokenKind::Where),
        "yield" => Some(TokenKind::Yield),
        "import" => Some(TokenKind::Import),
        "export" => Some(TokenKind::Export),
        "from" => Some(TokenKind::From),
        "class" => Some(TokenKind::Class),
        "dyn" => Some(TokenKind::Dyn),
        "static" => Some(TokenKind::Static),
        "var" => Some(TokenKind::Var),
        "grad" => Some(TokenKind::Grad),
        "backward" => Some(TokenKind::Backward),
        "tensor" => Some(TokenKind::Tensor),
        "strict" => Some(TokenKind::Strict),
        "true" => Some(TokenKind::True),
        "false" => Some(TokenKind::False),
        _ => None,
    }
}
