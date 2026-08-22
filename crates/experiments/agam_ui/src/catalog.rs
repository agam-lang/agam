//! Component Catalog & Metadata Schema for Generative UI (GenUI).
//!
//! Provides compile-time and runtime discoverable component registries allowing
//! AI agents to query available UI widgets, validate props, and compose interfaces via A2UI.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Metadata describing a registered UI component for AI agent inspection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComponentDescriptor {
    pub name: String,
    pub description: String,
    pub category: String,
    pub props_schema: serde_json::Value,
    pub supported_children: bool,
}

/// Dynamic and type-safe catalog of available UI components.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComponentCatalog {
    components: BTreeMap<String, ComponentDescriptor>,
}

impl ComponentCatalog {
    pub fn new() -> Self {
        let mut catalog = Self {
            components: BTreeMap::new(),
        };
        catalog.register_standard_components();
        catalog
    }

    pub fn register(&mut self, descriptor: ComponentDescriptor) {
        self.components.insert(descriptor.name.clone(), descriptor);
    }

    pub fn get(&self, name: &str) -> Option<&ComponentDescriptor> {
        self.components.get(name)
    }

    pub fn list(&self) -> Vec<&ComponentDescriptor> {
        self.components.values().collect()
    }

    /// Register standard Agam UI primitives into the catalog.
    fn register_standard_components(&mut self) {
        self.register(ComponentDescriptor {
            name: "Text".to_string(),
            description: "Renders formatted text content with styling support.".to_string(),
            category: "Typography".to_string(),
            props_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "content": { "type": "string" },
                    "size": { "type": "number" },
                    "color": { "type": "string" }
                },
                "required": ["content"]
            }),
            supported_children: false,
        });

        self.register(ComponentDescriptor {
            name: "Button".to_string(),
            description: "Interactive button trigger for user actions or agent events.".to_string(),
            category: "Input".to_string(),
            props_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "label": { "type": "string" },
                    "variant": { "type": "string", "enum": ["primary", "secondary", "outline"] }
                },
                "required": ["label"]
            }),
            supported_children: false,
        });

        self.register(ComponentDescriptor {
            name: "Card".to_string(),
            description: "Elevated container card supporting Bento or Glassmorphic surfaces."
                .to_string(),
            category: "Layout".to_string(),
            props_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "radius": { "type": "number" },
                    "elevation": { "type": "number" }
                }
            }),
            supported_children: true,
        });

        self.register(ComponentDescriptor {
            name: "Grid".to_string(),
            description: "Multi-column responsive grid layout.".to_string(),
            category: "Layout".to_string(),
            props_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "columns": { "type": "integer", "minimum": 1 },
                    "gap": { "type": "number" }
                },
                "required": ["columns"]
            }),
            supported_children: true,
        });

        self.register(ComponentDescriptor {
            name: "Row".to_string(),
            description: "Horizontal flexbox row container.".to_string(),
            category: "Layout".to_string(),
            props_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "gap": { "type": "number" },
                    "align": { "type": "string" }
                }
            }),
            supported_children: true,
        });

        self.register(ComponentDescriptor {
            name: "Column".to_string(),
            description: "Vertical flexbox column container.".to_string(),
            category: "Layout".to_string(),
            props_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "gap": { "type": "number" },
                    "align": { "type": "string" }
                }
            }),
            supported_children: true,
        });
    }
}
