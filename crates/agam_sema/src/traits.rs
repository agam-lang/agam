//! Trait resolution and method dispatch.
//!
//! Handles:
//! 1. **Trait declaration registration** — collects trait definitions and their methods.
//! 2. **Impl registration** — records which types implement which traits.
//! 3. **Method dispatch** — resolves `obj.method()` to the correct impl.
//! 4. **Coherence checking** — ensures no duplicate impl for the same trait+type pair.

use std::collections::HashMap;
use agam_errors::Span;
use crate::symbol::{SymbolId, TypeId};

/// A trait definition in the trait registry.
#[derive(Debug, Clone)]
pub struct TraitDef {
    /// The symbol ID of this trait.
    pub symbol: SymbolId,
    /// Trait name (for diagnostics).
    pub name: String,
    /// Required method signatures: name → (param types, return type).
    pub methods: HashMap<String, MethodSig>,
    /// Super-traits that this trait requires.
    pub super_traits: Vec<SymbolId>,
}

/// A method signature within a trait or impl.
#[derive(Debug, Clone)]
pub struct MethodSig {
    pub name: String,
    pub params: Vec<TypeId>,
    pub return_ty: TypeId,
    pub has_self: bool,
    pub span: Span,
}

/// An impl entry: "Type X implements Trait Y with these methods."
#[derive(Debug, Clone)]
pub struct ImplEntry {
    /// The type that implements the trait.
    pub target_type: TypeId,
    /// The trait being implemented (None = inherent impl).
    pub trait_id: Option<SymbolId>,
    /// Provided method implementations: name → method signature.
    pub methods: HashMap<String, MethodSig>,
    pub span: Span,
}

/// Error during trait resolution.
#[derive(Debug, Clone)]
pub struct TraitError {
    pub message: String,
    pub span: Span,
}

/// The trait registry: stores all trait definitions and impl blocks.
pub struct TraitRegistry {
    /// All registered trait definitions.
    pub traits: HashMap<SymbolId, TraitDef>,
    /// All registered impl blocks.
    pub impls: Vec<ImplEntry>,
    /// Errors accumulated during resolution.
    pub errors: Vec<TraitError>,
}

impl TraitRegistry {
    pub fn new() -> Self {
        Self {
            traits: HashMap::new(),
            impls: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Register a trait definition.
    pub fn register_trait(&mut self, def: TraitDef) {
        if self.traits.contains_key(&def.symbol) {
            self.errors.push(TraitError {
                message: format!("trait '{}' is already defined", def.name),
                span: Span::dummy(),
            });
            return;
        }
        self.traits.insert(def.symbol, def);
    }

    /// Register an impl block.
    pub fn register_impl(&mut self, entry: ImplEntry) {
        // Coherence check: no duplicate impl for same trait+type pair.
        if let Some(trait_id) = entry.trait_id {
            for existing in &self.impls {
                if existing.trait_id == Some(trait_id) && existing.target_type == entry.target_type {
                    self.errors.push(TraitError {
                        message: format!(
                            "conflicting implementations: trait already implemented for this type"
                        ),
                        span: entry.span,
                    });
                    return;
                }
            }
        }
        self.impls.push(entry);
    }

    /// Look up a method on a type.
    ///
    /// Searches inherent impls first, then trait impls.
    /// Returns the method signature if found.
    pub fn resolve_method(&self, target: TypeId, method_name: &str) -> Option<&MethodSig> {
        // 1. Search inherent impls (trait_id == None)
        for imp in &self.impls {
            if imp.target_type == target && imp.trait_id.is_none() {
                if let Some(sig) = imp.methods.get(method_name) {
                    return Some(sig);
                }
            }
        }
        // 2. Search trait impls
        for imp in &self.impls {
            if imp.target_type == target && imp.trait_id.is_some() {
                if let Some(sig) = imp.methods.get(method_name) {
                    return Some(sig);
                }
            }
        }
        None
    }

    /// Check that all required trait methods are implemented.
    pub fn check_completeness(&mut self) {
        for imp in &self.impls {
            if let Some(trait_id) = imp.trait_id {
                if let Some(trait_def) = self.traits.get(&trait_id) {
                    for (method_name, _) in &trait_def.methods {
                        if !imp.methods.contains_key(method_name) {
                            self.errors.push(TraitError {
                                message: format!(
                                    "missing method '{}' required by trait '{}'",
                                    method_name, trait_def.name
                                ),
                                span: imp.span,
                            });
                        }
                    }
                }
            }
        }
    }

    /// Check if a type implements a specific trait.
    pub fn type_implements_trait(&self, target: TypeId, trait_id: SymbolId) -> bool {
        self.impls.iter().any(|imp| {
            imp.target_type == target && imp.trait_id == Some(trait_id)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_span() -> Span { Span::dummy() }

    #[test]
    fn test_register_and_resolve_inherent_method() {
        let mut reg = TraitRegistry::new();
        let target = TypeId(10);

        let mut methods = HashMap::new();
        methods.insert("len".into(), MethodSig {
            name: "len".into(),
            params: vec![],
            return_ty: TypeId(4), // i32
            has_self: true,
            span: dummy_span(),
        });

        reg.register_impl(ImplEntry {
            target_type: target,
            trait_id: None,
            methods,
            span: dummy_span(),
        });

        let sig = reg.resolve_method(target, "len");
        assert!(sig.is_some());
        assert_eq!(sig.unwrap().name, "len");
    }

    #[test]
    fn test_trait_method_resolution() {
        let mut reg = TraitRegistry::new();
        let trait_sym = SymbolId(100);
        let target = TypeId(10);

        let mut trait_methods = HashMap::new();
        trait_methods.insert("display".into(), MethodSig {
            name: "display".into(),
            params: vec![],
            return_ty: TypeId(3), // str
            has_self: true,
            span: dummy_span(),
        });

        reg.register_trait(TraitDef {
            symbol: trait_sym,
            name: "Display".into(),
            methods: trait_methods,
            super_traits: vec![],
        });

        let mut impl_methods = HashMap::new();
        impl_methods.insert("display".into(), MethodSig {
            name: "display".into(),
            params: vec![],
            return_ty: TypeId(3),
            has_self: true,
            span: dummy_span(),
        });

        reg.register_impl(ImplEntry {
            target_type: target,
            trait_id: Some(trait_sym),
            methods: impl_methods,
            span: dummy_span(),
        });

        assert!(reg.type_implements_trait(target, trait_sym));
        assert!(reg.resolve_method(target, "display").is_some());
    }

    #[test]
    fn test_coherence_rejects_duplicate_impl() {
        let mut reg = TraitRegistry::new();
        let trait_sym = SymbolId(100);
        let target = TypeId(10);

        reg.register_impl(ImplEntry {
            target_type: target,
            trait_id: Some(trait_sym),
            methods: HashMap::new(),
            span: dummy_span(),
        });

        reg.register_impl(ImplEntry {
            target_type: target,
            trait_id: Some(trait_sym),
            methods: HashMap::new(),
            span: dummy_span(),
        });

        assert_eq!(reg.errors.len(), 1);
        assert!(reg.errors[0].message.contains("conflicting"));
    }

    #[test]
    fn test_missing_method_check() {
        let mut reg = TraitRegistry::new();
        let trait_sym = SymbolId(100);
        let target = TypeId(10);

        let mut trait_methods = HashMap::new();
        trait_methods.insert("required_method".into(), MethodSig {
            name: "required_method".into(),
            params: vec![],
            return_ty: TypeId(0),
            has_self: true,
            span: dummy_span(),
        });

        reg.register_trait(TraitDef {
            symbol: trait_sym,
            name: "MyTrait".into(),
            methods: trait_methods,
            super_traits: vec![],
        });

        // Register impl without the required method
        reg.register_impl(ImplEntry {
            target_type: target,
            trait_id: Some(trait_sym),
            methods: HashMap::new(),
            span: dummy_span(),
        });

        reg.check_completeness();
        assert_eq!(reg.errors.len(), 1);
        assert!(reg.errors[0].message.contains("missing method"));
    }

    #[test]
    fn test_inherent_before_trait() {
        let mut reg = TraitRegistry::new();
        let trait_sym = SymbolId(100);
        let target = TypeId(10);

        // Inherent impl
        let mut inherent = HashMap::new();
        inherent.insert("foo".into(), MethodSig {
            name: "foo".into(),
            params: vec![],
            return_ty: TypeId(4),
            has_self: true,
            span: dummy_span(),
        });
        reg.register_impl(ImplEntry {
            target_type: target,
            trait_id: None,
            methods: inherent,
            span: dummy_span(),
        });

        // Trait impl with same method name
        let mut trait_impl = HashMap::new();
        trait_impl.insert("foo".into(), MethodSig {
            name: "foo".into(),
            params: vec![],
            return_ty: TypeId(5), // different return type
            has_self: true,
            span: dummy_span(),
        });
        reg.register_impl(ImplEntry {
            target_type: target,
            trait_id: Some(trait_sym),
            methods: trait_impl,
            span: dummy_span(),
        });

        // Inherent impl should win
        let sig = reg.resolve_method(target, "foo").unwrap();
        assert_eq!(sig.return_ty, TypeId(4));
    }
}
