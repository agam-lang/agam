//! Headless compiler session pipeline orchestrator.

use std::path::PathBuf;
use agam_ast::Module;
use agam_errors::{Diagnostic, Label, SourceId};
use agam_mir::ir::MirModule;
use crate::config::SessionConfig;

#[derive(Debug)]
pub struct CompiledArtifact {
    pub ast: Option<Module>,
    pub mir: Option<MirModule>,
    pub target_path: Option<PathBuf>,
}

pub struct CompilerSession {
    pub config: SessionConfig,
    pub diagnostics: Vec<Diagnostic>,
}

impl CompilerSession {
    pub fn new(config: SessionConfig) -> Self {
        Self {
            config,
            diagnostics: Vec::new(),
        }
    }

    /// Parse source string into AST module.
    pub fn parse_source(&mut self, source: &str, source_id: SourceId) -> Result<Module, Vec<Diagnostic>> {
        let tokens = agam_lexer::tokenize(source, source_id);

        match agam_parser::parse(tokens, source_id) {
            Ok(module) => Ok(module),
            Err(parse_errors) => {
                let diags: Vec<Diagnostic> = parse_errors
                    .into_iter()
                    .map(|e| {
                        Diagnostic::error("E0002", e.message)
                            .with_label(Label::primary(e.span, "parse error"))
                    })
                    .collect();
                self.diagnostics.extend(diags.clone());
                Err(diags)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compiler_session_parse_simple() {
        let mut session = CompilerSession::new(SessionConfig::default());
        let source = "@lang.base\nfn add(x: i64, y: i64) -> i64:\n    return x + y\n";
        let res = session.parse_source(source, SourceId(0));
        assert!(res.is_ok());
    }
}
