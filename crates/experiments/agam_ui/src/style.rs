//! CSS-inspired styling engine for modern declarative components.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const WHITE: Self = Self::rgb(255, 255, 255);
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
    pub const PRIMARY: Self = Self::rgb(99, 102, 241); // Indigo
    pub const SECONDARY: Self = Self::rgb(168, 85, 247); // Purple
    pub const SUCCESS: Self = Self::rgb(34, 197, 94); // Green
    pub const DANGER: Self = Self::rgb(239, 68, 68); // Red
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Insets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Insets {
    pub const fn all(val: f32) -> Self {
        Self {
            top: val,
            right: val,
            bottom: val,
            left: val,
        }
    }

    pub const fn symmetric(v: f32, h: f32) -> Self {
        Self {
            top: v,
            right: h,
            bottom: v,
            left: h,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Shadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur_radius: f32,
    pub spread_radius: f32,
    pub color: Color,
}

impl Shadow {
    pub const fn soft() -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 4.0,
            blur_radius: 12.0,
            spread_radius: 0.0,
            color: Color::rgba(0, 0, 0, 25),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlexDirection {
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Alignment {
    Start,
    Center,
    End,
    Stretch,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

/// Comprehensive UI element styling attributes.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Style {
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub background_color: Option<Color>,
    pub text_color: Option<Color>,
    pub font_size: Option<f32>,
    pub font_weight: Option<u16>,
    pub padding: Option<Insets>,
    pub margin: Option<Insets>,
    pub border_radius: Option<f32>,
    pub border_width: Option<f32>,
    pub border_color: Option<Color>,
    pub shadow: Option<Shadow>,
    pub flex_direction: Option<FlexDirection>,
    pub align_items: Option<Alignment>,
    pub justify_content: Option<Alignment>,
    pub gap: Option<f32>,
}

impl Style {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bg(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    pub fn pad(mut self, insets: Insets) -> Self {
        self.padding = Some(insets);
        self
    }

    pub fn radius(mut self, r: f32) -> Self {
        self.border_radius = Some(r);
        self
    }

    pub fn gap(mut self, g: f32) -> Self {
        self.gap = Some(g);
        self
    }

    /// Compose / merge two styles together with right-hand precedence.
    pub fn merge(self, other: Self) -> Self {
        Self {
            width: other.width.or(self.width),
            height: other.height.or(self.height),
            background_color: other.background_color.or(self.background_color),
            text_color: other.text_color.or(self.text_color),
            font_size: other.font_size.or(self.font_size),
            font_weight: other.font_weight.or(self.font_weight),
            padding: other.padding.or(self.padding),
            margin: other.margin.or(self.margin),
            border_radius: other.border_radius.or(self.border_radius),
            border_width: other.border_width.or(self.border_width),
            border_color: other.border_color.or(self.border_color),
            shadow: other.shadow.or(self.shadow),
            flex_direction: other.flex_direction.or(self.flex_direction),
            align_items: other.align_items.or(self.align_items),
            justify_content: other.justify_content.or(self.justify_content),
            gap: other.gap.or(self.gap),
        }
    }
}
