//! Modern declarative Virtual Component and Widget Tree.

use crate::style::Style;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum WidgetKind {
    Text {
        text: String,
    },
    Button {
        label: String,
    },
    Row {
        children: Vec<Widget>,
    },
    Column {
        children: Vec<Widget>,
    },
    Grid {
        columns: usize,
        children: Vec<Widget>,
    },
    Card {
        child: Box<Widget>,
    },
    Image {
        src: String,
        width: f32,
        height: f32,
    },
    Slider {
        value: f32,
        min: f32,
        max: f32,
    },
    Spacer {
        flex: f32,
    },
}

/// A node in the virtual UI tree.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Widget {
    pub kind: WidgetKind,
    pub style: Style,
    pub key: Option<String>,
}

impl Widget {
    pub fn new(kind: WidgetKind) -> Self {
        Self {
            kind,
            style: Style::default(),
            key: None,
        }
    }

    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    // ── Constructors ──

    pub fn text(text: impl Into<String>) -> Self {
        Self::new(WidgetKind::Text { text: text.into() })
    }

    pub fn button(label: impl Into<String>) -> Self {
        Self::new(WidgetKind::Button {
            label: label.into(),
        })
    }

    pub fn row(children: Vec<Widget>) -> Self {
        Self::new(WidgetKind::Row { children })
    }

    pub fn column(children: Vec<Widget>) -> Self {
        Self::new(WidgetKind::Column { children })
    }

    pub fn grid(columns: usize, children: Vec<Widget>) -> Self {
        Self::new(WidgetKind::Grid { columns, children })
    }

    pub fn card(child: Widget) -> Self {
        Self::new(WidgetKind::Card {
            child: Box::new(child),
        })
    }

    pub fn image(src: impl Into<String>, width: f32, height: f32) -> Self {
        Self::new(WidgetKind::Image {
            src: src.into(),
            width,
            height,
        })
    }

    pub fn slider(value: f32, min: f32, max: f32) -> Self {
        Self::new(WidgetKind::Slider { value, min, max })
    }

    pub fn spacer(flex: f32) -> Self {
        Self::new(WidgetKind::Spacer { flex })
    }
}
