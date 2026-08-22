//! A2UI (Agent-to-User Interface) Protocol & Dynamic Schema Hydration.
//!
//! Enables AI agents to compose, update, and stream structured UI descriptions
//! which are safely validated and rendered into native Agam virtual component trees.

use crate::catalog::ComponentCatalog;
use crate::style::{Color, Style};
use crate::widget::Widget;
use serde::{Deserialize, Serialize};

/// Errors encountered during A2UI schema parsing or validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum A2UiError {
    JsonError(String),
    UnknownComponent(String),
    MissingProperty(String),
    InvalidStructure(String),
}

impl std::fmt::Display for A2UiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            A2UiError::JsonError(e) => write!(f, "A2UI JSON syntax error: {e}"),
            A2UiError::UnknownComponent(c) => write!(f, "Unknown A2UI component: `{c}`"),
            A2UiError::MissingProperty(p) => write!(f, "Missing required A2UI property: `{p}`"),
            A2UiError::InvalidStructure(s) => write!(f, "Invalid A2UI tree structure: {s}"),
        }
    }
}

impl std::error::Error for A2UiError {}

/// An A2UI Node in the agent-generated AST.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct A2UiNode {
    pub component: String,
    #[serde(default)]
    pub props: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub children: Vec<A2UiNode>,
    #[serde(default)]
    pub key: Option<String>,
}

/// A2UI Protocol Engine.
pub struct A2UiProtocol {
    catalog: ComponentCatalog,
}

impl A2UiProtocol {
    pub fn new() -> Self {
        Self {
            catalog: ComponentCatalog::new(),
        }
    }

    pub fn with_catalog(catalog: ComponentCatalog) -> Self {
        Self { catalog }
    }

    /// Hydrate a JSON string from an AI agent into a validated Agam `Widget` tree.
    pub fn render_from_json(&self, json_str: &str) -> Result<Widget, A2UiError> {
        let root_node: A2UiNode =
            serde_json::from_str(json_str).map_err(|e| A2UiError::JsonError(e.to_string()))?;
        self.hydrate_node(&root_node)
    }

    /// Recursively hydrate and validate an `A2UiNode` into a typed `Widget`.
    pub fn hydrate_node(&self, node: &A2UiNode) -> Result<Widget, A2UiError> {
        // Validate component existence in catalog
        if self.catalog.get(&node.component).is_none() {
            return Err(A2UiError::UnknownComponent(node.component.clone()));
        }

        let mut widget = match node.component.as_str() {
            "Text" => {
                let content = node
                    .props
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Widget::text(content)
            }
            "Button" => {
                let label = node
                    .props
                    .get("label")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Button")
                    .to_string();
                Widget::button(label)
            }
            "Card" => {
                let child = if let Some(first) = node.children.first() {
                    self.hydrate_node(first)?
                } else {
                    Widget::text("")
                };
                Widget::card(child)
            }
            "Row" => {
                let mut children = Vec::with_capacity(node.children.len());
                for c in &node.children {
                    children.push(self.hydrate_node(c)?);
                }
                Widget::row(children)
            }
            "Column" => {
                let mut children = Vec::with_capacity(node.children.len());
                for c in &node.children {
                    children.push(self.hydrate_node(c)?);
                }
                Widget::column(children)
            }
            "Grid" => {
                let columns = node
                    .props
                    .get("columns")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(2) as usize;
                let mut children = Vec::with_capacity(node.children.len());
                for c in &node.children {
                    children.push(self.hydrate_node(c)?);
                }
                Widget::grid(columns, children)
            }
            other => return Err(A2UiError::UnknownComponent(other.to_string())),
        };

        if let Some(key) = &node.key {
            widget.key = Some(key.clone());
        }

        // Apply basic style props if present
        let mut style = Style::default();
        if let Some(gap) = node.props.get("gap").and_then(|v| v.as_f64()) {
            style.gap = Some(gap as f32);
        }
        if let Some(radius) = node.props.get("radius").and_then(|v| v.as_f64()) {
            style.border_radius = Some(radius as f32);
        }
        if let Some(bg_hex) = node.props.get("bg").and_then(|v| v.as_str())
            && bg_hex.starts_with('#')
            && bg_hex.len() == 7
            && let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&bg_hex[1..3], 16),
                u8::from_str_radix(&bg_hex[3..5], 16),
                u8::from_str_radix(&bg_hex[5..7], 16),
            )
        {
            style.background_color = Some(Color::rgb(r, g, b));
        }

        widget.style = style;
        Ok(widget)
    }
}

impl Default for A2UiProtocol {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::WidgetKind;

    #[test]
    fn test_a2ui_hydrate_card_with_button_and_text() {
        let protocol = A2UiProtocol::new();
        let json_payload = r#"{
            "component": "Card",
            "props": { "radius": 16.0 },
            "children": [
                {
                    "component": "Column",
                    "props": { "gap": 12.0 },
                    "children": [
                        {
                            "component": "Text",
                            "props": { "content": "Agent Generated Dashboard" }
                        },
                        {
                            "component": "Button",
                            "props": { "label": "Approve Action" }
                        }
                    ]
                }
            ]
        }"#;

        let widget = protocol.render_from_json(json_payload).unwrap();
        if let WidgetKind::Card { child } = widget.kind {
            if let WidgetKind::Column { children } = child.kind {
                assert_eq!(children.len(), 2);
            } else {
                panic!("Expected Column child in Card");
            }
        } else {
            panic!("Expected Card root widget");
        }
    }

    #[test]
    fn test_a2ui_rejects_unknown_component() {
        let protocol = A2UiProtocol::new();
        let json_payload = r#"{
            "component": "MaliciousCustomIframe",
            "props": {}
        }"#;

        let res = protocol.render_from_json(json_payload);
        assert!(matches!(res, Err(A2UiError::UnknownComponent(_))));
    }
}
