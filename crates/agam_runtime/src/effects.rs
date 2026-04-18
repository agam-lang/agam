//! Runtime effect handler dispatch table.
//!
//! Provides the generic dispatch infrastructure (`EffectHandlerTable`,
//! `EffectValue`, `EffectError`) that codegen uses when encountering
//! `Op::EffectPerform` MIR instructions.
//!
//! Concrete handler implementations (e.g. FileSystem operations backed
//! by `agam_std::io`) are registered by consumer crates like `agam_std`
//! to avoid circular dependencies.

use std::collections::HashMap;
use std::fmt;

/// A runtime value passed to and returned from effect handlers.
#[derive(Debug, Clone, PartialEq)]
pub enum EffectValue {
    Unit,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<EffectValue>),
}

impl fmt::Display for EffectValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unit => write!(f, "()"),
            Self::Bool(v) => write!(f, "{v}"),
            Self::Int(v) => write!(f, "{v}"),
            Self::Float(v) => write!(f, "{v}"),
            Self::String(v) => write!(f, "{v}"),
            Self::List(v) => {
                write!(f, "[")?;
                for (i, item) in v.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, "]")
            }
        }
    }
}

/// Error returned when effect dispatch fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectError {
    pub effect: String,
    pub operation: String,
    pub message: String,
}

impl fmt::Display for EffectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "effect {}.{} failed: {}",
            self.effect, self.operation, self.message
        )
    }
}

impl std::error::Error for EffectError {}

/// Handler function signature: takes a list of argument values and returns
/// a result value or an effect error.
pub type EffectHandlerFn = fn(&[EffectValue]) -> Result<EffectValue, EffectError>;

/// Key for looking up a handler in the dispatch table.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct HandlerKey {
    effect: String,
    operation: String,
}

/// Runtime dispatch table mapping effect operations to handler functions.
#[derive(Debug)]
pub struct EffectHandlerTable {
    handlers: HashMap<HandlerKey, EffectHandlerFn>,
}

impl EffectHandlerTable {
    /// Create an empty handler table.
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a handler function for a specific effect operation.
    pub fn register(
        &mut self,
        effect: &str,
        operation: &str,
        handler: EffectHandlerFn,
    ) {
        self.handlers.insert(
            HandlerKey {
                effect: effect.to_string(),
                operation: operation.to_string(),
            },
            handler,
        );
    }

    /// Look up a handler for the given effect and operation.
    pub fn get(&self, effect: &str, operation: &str) -> Option<EffectHandlerFn> {
        self.handlers
            .get(&HandlerKey {
                effect: effect.to_string(),
                operation: operation.to_string(),
            })
            .copied()
    }

    /// Dispatch an effect operation with the provided arguments.
    pub fn dispatch(
        &self,
        effect: &str,
        operation: &str,
        args: &[EffectValue],
    ) -> Result<EffectValue, EffectError> {
        match self.get(effect, operation) {
            Some(handler) => handler(args),
            None => Err(EffectError {
                effect: effect.to_string(),
                operation: operation.to_string(),
                message: "no handler registered".to_string(),
            }),
        }
    }

    /// Return the number of registered handler entries.
    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    /// Check if the table is empty.
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }
}

impl Default for EffectHandlerTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn echo_handler(args: &[EffectValue]) -> Result<EffectValue, EffectError> {
        Ok(args.first().cloned().unwrap_or(EffectValue::Unit))
    }

    #[test]
    fn empty_table_returns_no_handler_error() {
        let table = EffectHandlerTable::new();
        let err = table
            .dispatch("TestEffect", "op", &[])
            .expect_err("unregistered handler should fail");
        assert!(err.message.contains("no handler registered"));
        assert_eq!(err.effect, "TestEffect");
        assert_eq!(err.operation, "op");
    }

    #[test]
    fn register_and_dispatch() {
        let mut table = EffectHandlerTable::new();
        table.register("TestEffect", "echo", echo_handler);

        assert_eq!(table.len(), 1);
        assert!(!table.is_empty());

        let result = table
            .dispatch(
                "TestEffect",
                "echo",
                &[EffectValue::String("hello".into())],
            )
            .expect("echo handler should succeed");
        assert_eq!(result, EffectValue::String("hello".into()));
    }

    #[test]
    fn get_returns_none_for_missing() {
        let table = EffectHandlerTable::new();
        assert!(table.get("X", "Y").is_none());
    }

    #[test]
    fn effect_value_display() {
        assert_eq!(EffectValue::Unit.to_string(), "()");
        assert_eq!(EffectValue::Bool(true).to_string(), "true");
        assert_eq!(EffectValue::Int(42).to_string(), "42");
        assert_eq!(EffectValue::Float(3.14).to_string(), "3.14");
        assert_eq!(EffectValue::String("hello".into()).to_string(), "hello");
        assert_eq!(
            EffectValue::List(vec![EffectValue::Int(1), EffectValue::Int(2)]).to_string(),
            "[1, 2]"
        );
    }

    #[test]
    fn effect_error_display() {
        let err = EffectError {
            effect: "FS".into(),
            operation: "read".into(),
            message: "not found".into(),
        };
        assert_eq!(err.to_string(), "effect FS.read failed: not found");
    }
}
