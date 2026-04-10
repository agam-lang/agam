//! Scope management for name resolution.
//!
//! Maintains a stack of scopes. Each scope maps names to `SymbolId`s.
//! Pushing a new scope happens when entering a function body, block,
//! loop, or any other construct that introduces a new binding context.

use crate::symbol::{Symbol, SymbolId, SymbolKind};
use agam_errors::Span;
use std::collections::HashMap;

/// A single lexical scope containing name → SymbolId bindings.
#[derive(Debug)]
struct Scope {
    /// Name → SymbolId mapping for this scope.
    bindings: HashMap<String, SymbolId>,
    /// The depth of this scope (0 = global/module).
    depth: u32,
}

/// The scope stack manages nested lexical scopes and the global symbol table.
pub struct ScopeStack {
    /// All symbols ever declared, indexed by SymbolId.
    symbols: Vec<Symbol>,
    /// Stack of active scopes (innermost last).
    scopes: Vec<Scope>,
    /// Counter for generating unique SymbolIds.
    next_id: u32,
}

impl ScopeStack {
    pub fn new() -> Self {
        let global = Scope {
            bindings: HashMap::new(),
            depth: 0,
        };
        Self {
            symbols: Vec::new(),
            scopes: vec![global],
            next_id: 0,
        }
    }

    /// Push a new child scope.
    pub fn push_scope(&mut self) {
        let depth = self.scopes.len() as u32;
        self.scopes.push(Scope {
            bindings: HashMap::new(),
            depth,
        });
    }

    /// Pop the innermost scope. Returns the names that were declared in it.
    pub fn pop_scope(&mut self) -> Vec<SymbolId> {
        let scope = self.scopes.pop().expect("cannot pop global scope");
        scope.bindings.values().copied().collect()
    }

    /// Current scope depth.
    pub fn depth(&self) -> u32 {
        self.scopes.len() as u32 - 1
    }

    /// Declare a new symbol in the current scope.
    ///
    /// Returns `Ok(SymbolId)` on success, `Err(SymbolId)` if a symbol
    /// with the same name already exists **in the same scope** (shadowing
    /// across scopes is allowed).
    pub fn declare(
        &mut self,
        name: String,
        kind: SymbolKind,
        span: Span,
    ) -> Result<SymbolId, SymbolId> {
        let scope = self.scopes.last_mut().expect("no active scope");

        // Check for redeclaration in the *same* scope.
        if let Some(&existing) = scope.bindings.get(&name) {
            return Err(existing);
        }

        let id = SymbolId(self.next_id);
        self.next_id += 1;

        let depth = scope.depth;
        scope.bindings.insert(name.clone(), id);

        self.symbols.push(Symbol {
            id,
            name,
            kind,
            span,
            depth,
            used: false,
        });

        Ok(id)
    }

    /// Look up a name, searching from innermost scope outward.
    ///
    /// Returns `Some(SymbolId)` if found, `None` if the name is not in scope.
    pub fn lookup(&self, name: &str) -> Option<SymbolId> {
        for scope in self.scopes.iter().rev() {
            if let Some(&id) = scope.bindings.get(name) {
                return Some(id);
            }
        }
        None
    }

    /// Look up a name only in the current (innermost) scope.
    pub fn lookup_local(&self, name: &str) -> Option<SymbolId> {
        self.scopes.last()?.bindings.get(name).copied()
    }

    /// Get a reference to a symbol by its ID.
    pub fn get(&self, id: SymbolId) -> &Symbol {
        &self.symbols[id.0 as usize]
    }

    /// Get a mutable reference to a symbol by its ID.
    pub fn get_mut(&mut self, id: SymbolId) -> &mut Symbol {
        &mut self.symbols[id.0 as usize]
    }

    /// Mark a symbol as used (for dead-code analysis later).
    pub fn mark_used(&mut self, id: SymbolId) {
        self.symbols[id.0 as usize].used = true;
    }

    /// Return all symbols (for diagnostics or inspection).
    pub fn all_symbols(&self) -> &[Symbol] {
        &self.symbols
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol::TypeId;

    fn dummy_span() -> Span {
        Span::dummy()
    }

    #[test]
    fn test_declare_and_lookup() {
        let mut scopes = ScopeStack::new();
        let id = scopes
            .declare(
                "x".into(),
                SymbolKind::Variable {
                    mutable: false,
                    ty: TypeId(4),
                },
                dummy_span(),
            )
            .unwrap();

        assert_eq!(scopes.lookup("x"), Some(id));
        assert_eq!(scopes.lookup("y"), None);
    }

    #[test]
    fn test_shadowing_across_scopes() {
        let mut scopes = ScopeStack::new();
        let id_outer = scopes
            .declare(
                "x".into(),
                SymbolKind::Variable {
                    mutable: false,
                    ty: TypeId(4),
                },
                dummy_span(),
            )
            .unwrap();

        scopes.push_scope();
        let id_inner = scopes
            .declare(
                "x".into(),
                SymbolKind::Variable {
                    mutable: true,
                    ty: TypeId(5),
                },
                dummy_span(),
            )
            .unwrap();

        // Inner shadow wins
        assert_eq!(scopes.lookup("x"), Some(id_inner));
        assert_ne!(id_outer, id_inner);

        scopes.pop_scope();
        // Outer is visible again
        assert_eq!(scopes.lookup("x"), Some(id_outer));
    }

    #[test]
    fn test_redeclaration_in_same_scope_errors() {
        let mut scopes = ScopeStack::new();
        scopes
            .declare(
                "x".into(),
                SymbolKind::Variable {
                    mutable: false,
                    ty: TypeId(4),
                },
                dummy_span(),
            )
            .unwrap();

        let result = scopes.declare(
            "x".into(),
            SymbolKind::Variable {
                mutable: false,
                ty: TypeId(4),
            },
            dummy_span(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_nested_scopes() {
        let mut scopes = ScopeStack::new();
        scopes
            .declare(
                "a".into(),
                SymbolKind::Variable {
                    mutable: false,
                    ty: TypeId(4),
                },
                dummy_span(),
            )
            .unwrap();

        scopes.push_scope();
        scopes
            .declare(
                "b".into(),
                SymbolKind::Variable {
                    mutable: false,
                    ty: TypeId(4),
                },
                dummy_span(),
            )
            .unwrap();

        scopes.push_scope();
        scopes
            .declare(
                "c".into(),
                SymbolKind::Variable {
                    mutable: false,
                    ty: TypeId(4),
                },
                dummy_span(),
            )
            .unwrap();

        assert!(scopes.lookup("a").is_some());
        assert!(scopes.lookup("b").is_some());
        assert!(scopes.lookup("c").is_some());

        scopes.pop_scope();
        assert!(scopes.lookup("c").is_none());
        assert!(scopes.lookup("b").is_some());

        scopes.pop_scope();
        assert!(scopes.lookup("b").is_none());
        assert!(scopes.lookup("a").is_some());
    }

    #[test]
    fn test_mark_used() {
        let mut scopes = ScopeStack::new();
        let id = scopes
            .declare(
                "x".into(),
                SymbolKind::Variable {
                    mutable: false,
                    ty: TypeId(4),
                },
                dummy_span(),
            )
            .unwrap();

        assert!(!scopes.get(id).used);
        scopes.mark_used(id);
        assert!(scopes.get(id).used);
    }
}
