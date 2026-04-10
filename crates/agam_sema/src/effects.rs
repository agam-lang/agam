//! Algebraic effects — type tracking, polymorphism, and CPS transformation.
//!
//! Algebraic effects provide a structured way to handle side effects
//! (IO, exceptions, state, async) without monads or colored functions.
//!
//! ## Architecture
//! 1. **Effect declarations** define a set of operations (like an interface).
//! 2. **Handlers** provide implementations for those operations.
//! 3. **CPS transformation** converts effect invocations into continuation-passing
//!    style during HIR lowering, enabling efficient compilation.
//!
//! ## Example (Agam syntax)
//! ```text
//! effect Console {
//!     fn print(msg: String)
//!     fn read() -> String
//! }
//!
//! handle stdio for Console {
//!     fn print(msg): resume(io_print(msg))
//!     fn read(): resume(io_readline())
//! }
//! ```

use std::collections::HashMap;

/// A registered effect with its operations.
#[derive(Debug, Clone)]
pub struct EffectDef {
    pub name: String,
    pub operations: Vec<EffectOpDef>,
}

/// Definition of a single effect operation.
#[derive(Debug, Clone)]
pub struct EffectOpDef {
    pub name: String,
    pub param_count: usize,
    pub has_return: bool,
}

/// A registered handler for an effect.
#[derive(Debug, Clone)]
pub struct HandlerDef {
    pub name: String,
    pub effect_name: String,
    pub clauses: Vec<HandlerClauseDef>,
}

#[derive(Debug, Clone)]
pub struct HandlerClauseDef {
    pub op_name: String,
    pub param_names: Vec<String>,
}

/// Effect registry: tracks all declared effects and their handlers.
pub struct EffectRegistry {
    effects: HashMap<String, EffectDef>,
    handlers: HashMap<String, Vec<HandlerDef>>,
}

impl EffectRegistry {
    pub fn new() -> Self {
        Self {
            effects: HashMap::new(),
            handlers: HashMap::new(),
        }
    }

    /// Register a new effect.
    pub fn register_effect(&mut self, effect: EffectDef) {
        self.effects.insert(effect.name.clone(), effect);
    }

    /// Register a handler for an effect.
    pub fn register_handler(&mut self, handler: HandlerDef) {
        self.handlers
            .entry(handler.effect_name.clone())
            .or_default()
            .push(handler);
    }

    /// Look up an effect by name.
    pub fn get_effect(&self, name: &str) -> Option<&EffectDef> {
        self.effects.get(name)
    }

    /// Look up handlers for an effect.
    pub fn get_handlers(&self, effect_name: &str) -> &[HandlerDef] {
        self.handlers
            .get(effect_name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Check if an operation belongs to a registered effect.
    pub fn find_operation(&self, op_name: &str) -> Option<(&EffectDef, &EffectOpDef)> {
        for effect in self.effects.values() {
            for op in &effect.operations {
                if op.name == op_name {
                    return Some((effect, op));
                }
            }
        }
        None
    }

    /// Validate that a handler covers all operations of its effect.
    pub fn validate_handler(&self, handler: &HandlerDef) -> Vec<String> {
        let mut errors = Vec::new();
        if let Some(effect) = self.get_effect(&handler.effect_name) {
            let handled: Vec<&str> = handler.clauses.iter().map(|c| c.op_name.as_str()).collect();
            for op in &effect.operations {
                if !handled.contains(&op.name.as_str()) {
                    errors.push(format!(
                        "handler '{}' does not implement operation '{}' of effect '{}'",
                        handler.name, op.name, handler.effect_name
                    ));
                }
            }
        } else {
            errors.push(format!("unknown effect '{}'", handler.effect_name));
        }
        errors
    }
}

/// CPS-transformed effect invocation.
/// `perform op(args)` becomes `op(args, continuation)` in CPS form.
#[derive(Debug, Clone)]
pub struct CpsNode {
    pub kind: CpsNodeKind,
}

#[derive(Debug, Clone)]
pub enum CpsNodeKind {
    /// A pure value (no effects).
    Pure(String),

    /// An effect operation invocation: op(args, k) where k is the continuation.
    Perform {
        effect_name: String,
        op_name: String,
        args: Vec<String>,
        continuation: Box<CpsNode>,
    },

    /// A handler wrapping a computation.
    Handle {
        handler_name: String,
        body: Box<CpsNode>,
        return_clause: Box<CpsNode>,
    },

    /// Resume from a handler clause with a value.
    Resume {
        value: String,
        continuation: Box<CpsNode>,
    },

    /// Sequence two CPS nodes.
    Bind {
        name: String,
        value: Box<CpsNode>,
        body: Box<CpsNode>,
    },
}

impl CpsNode {
    pub fn pure_val(name: &str) -> Self {
        CpsNode {
            kind: CpsNodeKind::Pure(name.to_string()),
        }
    }

    pub fn perform(effect: &str, op: &str, args: Vec<String>, cont: CpsNode) -> Self {
        CpsNode {
            kind: CpsNodeKind::Perform {
                effect_name: effect.to_string(),
                op_name: op.to_string(),
                args,
                continuation: Box::new(cont),
            },
        }
    }

    pub fn resume(value: &str, cont: CpsNode) -> Self {
        CpsNode {
            kind: CpsNodeKind::Resume {
                value: value.to_string(),
                continuation: Box::new(cont),
            },
        }
    }

    pub fn bind(name: &str, value: CpsNode, body: CpsNode) -> Self {
        CpsNode {
            kind: CpsNodeKind::Bind {
                name: name.to_string(),
                value: Box::new(value),
                body: Box::new(body),
            },
        }
    }

    /// Check if this CPS node is effectful (contains Perform).
    pub fn is_effectful(&self) -> bool {
        match &self.kind {
            CpsNodeKind::Pure(_) => false,
            CpsNodeKind::Perform { .. } => true,
            CpsNodeKind::Handle { body, .. } => body.is_effectful(),
            CpsNodeKind::Resume { continuation, .. } => continuation.is_effectful(),
            CpsNodeKind::Bind { value, body, .. } => value.is_effectful() || body.is_effectful(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_io_effect() -> EffectDef {
        EffectDef {
            name: "IO".to_string(),
            operations: vec![
                EffectOpDef {
                    name: "print".to_string(),
                    param_count: 1,
                    has_return: false,
                },
                EffectOpDef {
                    name: "read".to_string(),
                    param_count: 0,
                    has_return: true,
                },
            ],
        }
    }

    fn make_state_effect() -> EffectDef {
        EffectDef {
            name: "State".to_string(),
            operations: vec![
                EffectOpDef {
                    name: "get".to_string(),
                    param_count: 0,
                    has_return: true,
                },
                EffectOpDef {
                    name: "set".to_string(),
                    param_count: 1,
                    has_return: false,
                },
            ],
        }
    }

    #[test]
    fn test_register_effect() {
        let mut reg = EffectRegistry::new();
        reg.register_effect(make_io_effect());
        assert!(reg.get_effect("IO").is_some());
        assert!(reg.get_effect("State").is_none());
    }

    #[test]
    fn test_register_handler() {
        let mut reg = EffectRegistry::new();
        reg.register_effect(make_io_effect());
        reg.register_handler(HandlerDef {
            name: "stdio".to_string(),
            effect_name: "IO".to_string(),
            clauses: vec![
                HandlerClauseDef {
                    op_name: "print".to_string(),
                    param_names: vec!["msg".to_string()],
                },
                HandlerClauseDef {
                    op_name: "read".to_string(),
                    param_names: vec![],
                },
            ],
        });
        let handlers = reg.get_handlers("IO");
        assert_eq!(handlers.len(), 1);
        assert_eq!(handlers[0].name, "stdio");
    }

    #[test]
    fn test_find_operation() {
        let mut reg = EffectRegistry::new();
        reg.register_effect(make_io_effect());
        let (eff, op) = reg.find_operation("print").unwrap();
        assert_eq!(eff.name, "IO");
        assert_eq!(op.param_count, 1);
    }

    #[test]
    fn test_validate_handler_complete() {
        let mut reg = EffectRegistry::new();
        reg.register_effect(make_io_effect());
        let handler = HandlerDef {
            name: "stdio".to_string(),
            effect_name: "IO".to_string(),
            clauses: vec![
                HandlerClauseDef {
                    op_name: "print".to_string(),
                    param_names: vec![],
                },
                HandlerClauseDef {
                    op_name: "read".to_string(),
                    param_names: vec![],
                },
            ],
        };
        let errors = reg.validate_handler(&handler);
        assert!(errors.is_empty(), "errors: {:?}", errors);
    }

    #[test]
    fn test_validate_handler_incomplete() {
        let mut reg = EffectRegistry::new();
        reg.register_effect(make_io_effect());
        let handler = HandlerDef {
            name: "partial".to_string(),
            effect_name: "IO".to_string(),
            clauses: vec![
                HandlerClauseDef {
                    op_name: "print".to_string(),
                    param_names: vec![],
                },
                // missing "read"
            ],
        };
        let errors = reg.validate_handler(&handler);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("read"));
    }

    #[test]
    fn test_validate_unknown_effect() {
        let reg = EffectRegistry::new();
        let handler = HandlerDef {
            name: "bad".to_string(),
            effect_name: "NonExistent".to_string(),
            clauses: vec![],
        };
        let errors = reg.validate_handler(&handler);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("unknown effect"));
    }

    #[test]
    fn test_multiple_effects() {
        let mut reg = EffectRegistry::new();
        reg.register_effect(make_io_effect());
        reg.register_effect(make_state_effect());
        assert!(reg.get_effect("IO").is_some());
        assert!(reg.get_effect("State").is_some());
        let (eff, _) = reg.find_operation("get").unwrap();
        assert_eq!(eff.name, "State");
    }

    #[test]
    fn test_cps_pure() {
        let node = CpsNode::pure_val("42");
        assert!(!node.is_effectful());
    }

    #[test]
    fn test_cps_perform() {
        let node = CpsNode::perform(
            "IO",
            "print",
            vec!["hello".to_string()],
            CpsNode::pure_val("unit"),
        );
        assert!(node.is_effectful());
    }

    #[test]
    fn test_cps_bind_chain() {
        // let x = perform IO.read(); let y = perform IO.print(x); pure(y)
        let chain = CpsNode::bind(
            "x",
            CpsNode::perform("IO", "read", vec![], CpsNode::pure_val("x")),
            CpsNode::bind(
                "y",
                CpsNode::perform(
                    "IO",
                    "print",
                    vec!["x".to_string()],
                    CpsNode::pure_val("unit"),
                ),
                CpsNode::pure_val("y"),
            ),
        );
        assert!(chain.is_effectful());
    }

    #[test]
    fn test_cps_resume() {
        let node = CpsNode::resume("hello", CpsNode::pure_val("k"));
        assert!(!node.is_effectful()); // resume itself is not an effect invocation
    }
}
