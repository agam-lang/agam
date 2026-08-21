//! # agam_macro
//!
//! Macro engine and embedded DSL system.

pub mod declarative;
pub mod derive;
pub mod dsl;
pub mod token_stream;

pub use declarative::{DeclarativeMacro, MacroError, MacroRule, MatcherElement};
pub use derive::{DeriveTrait, EnumDescriptor, StructDescriptor, generate_derive_struct};
pub use dsl::{NnDslLayer, emit_nn_model_definition, parse_nn_dsl};
pub use token_stream::{Delimiter, Group, Ident, Literal, Punct, TokenStream, TokenTree};
