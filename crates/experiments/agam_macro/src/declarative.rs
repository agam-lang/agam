//! Declarative Pattern-Matching Macro Expander (`macro_rules!`).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::token_stream::{Delimiter, Group, TokenStream, TokenTree};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MacroError {
    NoMatchingRule(String),
    RecursionLimitExceeded(usize),
    SyntaxError(String),
}

impl std::fmt::Display for MacroError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoMatchingRule(name) => write!(f, "no matching macro rule found for '{name}'"),
            Self::RecursionLimitExceeded(limit) => {
                write!(f, "macro expansion recursion limit of {limit} exceeded")
            }
            Self::SyntaxError(msg) => write!(f, "macro syntax error: {msg}"),
        }
    }
}

impl std::error::Error for MacroError {}

/// Pattern element inside a macro rule matcher.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatcherElement {
    LiteralToken(String),
    PunctToken(char),
    Variable { name: String, kind: String }, // e.g. $x:expr, $name:ident
    Group(Delimiter, Vec<MatcherElement>),
}

/// A single rule in a declarative macro definition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroRule {
    pub matcher: Vec<MatcherElement>,
    pub template: TokenStream,
}

/// Declarative macro definition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclarativeMacro {
    pub name: String,
    pub rules: Vec<MacroRule>,
    pub recursion_limit: usize,
}

impl DeclarativeMacro {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            rules: Vec::new(),
            recursion_limit: 64,
        }
    }

    pub fn with_rule(mut self, matcher: Vec<MatcherElement>, template: TokenStream) -> Self {
        self.rules.push(MacroRule { matcher, template });
        self
    }

    /// Expand the macro with the provided invocation TokenStream.
    pub fn expand(&self, input: &TokenStream) -> Result<TokenStream, MacroError> {
        for rule in &self.rules {
            if let Some(bindings) = match_pattern(&rule.matcher, &input.trees) {
                return Ok(substitute_template(&rule.template, &bindings));
            }
        }
        Err(MacroError::NoMatchingRule(self.name.clone()))
    }
}

fn match_pattern(
    matcher: &[MatcherElement],
    tokens: &[TokenTree],
) -> Option<HashMap<String, TokenStream>> {
    let mut bindings = HashMap::new();
    let mut token_idx = 0;

    for (m_idx, element) in matcher.iter().enumerate() {
        if token_idx >= tokens.len() {
            return None;
        }

        match element {
            MatcherElement::LiteralToken(expected) => match &tokens[token_idx] {
                TokenTree::Ident(id) if &id.name == expected => {
                    token_idx += 1;
                }
                TokenTree::Literal(lit) if &lit.text == expected => {
                    token_idx += 1;
                }
                _ => return None,
            },
            MatcherElement::PunctToken(expected) => match &tokens[token_idx] {
                TokenTree::Punct(p) if p.ch == *expected => {
                    token_idx += 1;
                }
                _ => return None,
            },
            MatcherElement::Variable { name, .. } => {
                // If it's the last element in the matcher, capture all remaining tokens in this level
                let is_last = m_idx + 1 == matcher.len();
                let captured = if is_last {
                    let rest = tokens[token_idx..].to_vec();
                    token_idx = tokens.len();
                    TokenStream::from_trees(rest)
                } else {
                    let single = TokenStream::from_trees(vec![tokens[token_idx].clone()]);
                    token_idx += 1;
                    single
                };
                bindings.insert(name.clone(), captured);
            }
            MatcherElement::Group(delim, inner_matcher) => match &tokens[token_idx] {
                TokenTree::Group(g) if g.delimiter == *delim => {
                    let sub_bindings = match_pattern(inner_matcher, &g.stream.trees)?;
                    bindings.extend(sub_bindings);
                    token_idx += 1;
                }
                _ => return None,
            },
        }
    }

    if token_idx == tokens.len() {
        Some(bindings)
    } else {
        None
    }
}

fn substitute_template(
    template: &TokenStream,
    bindings: &HashMap<String, TokenStream>,
) -> TokenStream {
    let mut out_trees = Vec::new();
    let mut i = 0;

    while i < template.trees.len() {
        match &template.trees[i] {
            TokenTree::Punct(p) if p.ch == '$' && i + 1 < template.trees.len() => {
                if let TokenTree::Ident(id) = &template.trees[i + 1]
                    && let Some(bound_stream) = bindings.get(&id.name)
                {
                    out_trees.extend(bound_stream.trees.clone());
                    i += 2;
                    continue;
                }
                out_trees.push(template.trees[i].clone());
                i += 1;
            }
            TokenTree::Group(g) => {
                let substituted_inner = substitute_template(&g.stream, bindings);
                out_trees.push(TokenTree::Group(Group {
                    delimiter: g.delimiter,
                    stream: substituted_inner,
                }));
                i += 1;
            }
            other => {
                out_trees.push(other.clone());
                i += 1;
            }
        }
    }

    TokenStream::from_trees(out_trees)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_declarative_macro_expansion() {
        // macro_rules! my_assert { ($val:expr) => { if !($val) { panic!() } } }
        let matcher = vec![MatcherElement::Group(
            Delimiter::Parenthesis,
            vec![MatcherElement::Variable {
                name: "val".into(),
                kind: "expr".into(),
            }],
        )];

        let template = TokenStream::parse("if ! ( $val ) { panic ( ) }");
        let macro_def = DeclarativeMacro::new("my_assert").with_rule(matcher, template);

        let invocation = TokenStream::parse("( x > 0 )");
        let expanded = macro_def.expand(&invocation).expect("expansion succeeds");

        let rendered = expanded.to_source();
        assert!(rendered.contains("if ! (x > 0)"));
        assert!(rendered.contains("panic ()"));
    }
}
