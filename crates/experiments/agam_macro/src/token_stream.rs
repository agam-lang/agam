//! Token Stream and Token Tree representations for hygienic metaprogramming.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Delimiter {
    Parenthesis,
    Brace,
    Bracket,
    None,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Group {
    pub delimiter: Delimiter,
    pub stream: TokenStream,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ident {
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Punct {
    pub ch: char,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Literal {
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TokenTree {
    Group(Group),
    Ident(Ident),
    Punct(Punct),
    Literal(Literal),
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenStream {
    pub trees: Vec<TokenTree>,
}

impl TokenStream {
    pub fn new() -> Self {
        Self { trees: Vec::new() }
    }

    pub fn from_trees(trees: Vec<TokenTree>) -> Self {
        Self { trees }
    }

    pub fn is_empty(&self) -> bool {
        self.trees.is_empty()
    }

    pub fn len(&self) -> usize {
        self.trees.len()
    }

    /// Parse simple textual code into a TokenStream.
    pub fn parse(input: &str) -> Self {
        let mut trees = Vec::new();
        let mut chars = input.chars().peekable();

        while let Some(&c) = chars.peek() {
            if c.is_whitespace() {
                chars.next();
                continue;
            }

            if c.is_alphabetic() || c == '_' {
                let mut ident_str = String::new();
                while let Some(&ic) = chars.peek() {
                    if ic.is_alphanumeric() || ic == '_' {
                        ident_str.push(ic);
                        chars.next();
                    } else {
                        break;
                    }
                }
                trees.push(TokenTree::Ident(Ident { name: ident_str }));
            } else if c.is_numeric() {
                let mut lit_str = String::new();
                while let Some(&nc) = chars.peek() {
                    if nc.is_numeric() || nc == '.' {
                        lit_str.push(nc);
                        chars.next();
                    } else {
                        break;
                    }
                }
                trees.push(TokenTree::Literal(Literal { text: lit_str }));
            } else if c == '"' {
                let mut str_lit = String::new();
                str_lit.push(c);
                chars.next();
                for sc in chars.by_ref() {
                    str_lit.push(sc);
                    if sc == '"' {
                        break;
                    }
                }
                trees.push(TokenTree::Literal(Literal { text: str_lit }));
            } else if c == '(' || c == '{' || c == '[' {
                let delim = match c {
                    '(' => Delimiter::Parenthesis,
                    '{' => Delimiter::Brace,
                    '[' => Delimiter::Bracket,
                    _ => Delimiter::None,
                };
                chars.next();
                let close_char = match delim {
                    Delimiter::Parenthesis => ')',
                    Delimiter::Brace => '}',
                    Delimiter::Bracket => ']',
                    Delimiter::None => ' ',
                };

                let mut group_content = String::new();
                let mut depth = 1;
                for gc in chars.by_ref() {
                    if gc == c {
                        depth += 1;
                    } else if gc == close_char {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    group_content.push(gc);
                }
                let inner_stream = TokenStream::parse(&group_content);
                trees.push(TokenTree::Group(Group {
                    delimiter: delim,
                    stream: inner_stream,
                }));
            } else {
                chars.next();
                trees.push(TokenTree::Punct(Punct { ch: c }));
            }
        }

        Self { trees }
    }

    /// Render TokenStream back to standard source code string.
    pub fn to_source(&self) -> String {
        let mut out = String::new();
        for (i, tree) in self.trees.iter().enumerate() {
            if i > 0 {
                out.push(' ');
            }
            match tree {
                TokenTree::Ident(id) => out.push_str(&id.name),
                TokenTree::Literal(lit) => out.push_str(&lit.text),
                TokenTree::Punct(p) => out.push(p.ch),
                TokenTree::Group(g) => {
                    let (open, close) = match g.delimiter {
                        Delimiter::Parenthesis => ('(', ')'),
                        Delimiter::Brace => ('{', '}'),
                        Delimiter::Bracket => ('[', ']'),
                        Delimiter::None => (' ', ' '),
                    };
                    out.push(open);
                    out.push_str(&g.stream.to_source());
                    out.push(close);
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_stream_parsing_and_rendering() {
        let code = "fn add ( a , b ) { a + b }";
        let stream = TokenStream::parse(code);
        assert_eq!(stream.len(), 4); // fn, add, (a, b), { a + b }

        let rendered = stream.to_source();
        assert!(rendered.contains("fn add"));
        assert!(rendered.contains("(a , b)"));
        assert!(rendered.contains("{a + b}"));
    }
}
