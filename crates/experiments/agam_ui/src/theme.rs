//! Modern UI Theme Engine (Bento, Neumorphic, Neobrutalist, Glassmorphic, Material).

use crate::style::{Color, Insets, Style};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeKind {
    Material,
    Fluent,
    Cupertino,
    Bento,
    Neumorphic,
    Neobrutalist,
    Glassmorphic,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    pub kind: ThemeKind,
    pub primary: Color,
    pub secondary: Color,
    pub background: Color,
    pub surface: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub border: Color,
    pub default_radius: f32,
    pub default_padding: Insets,
}

impl Theme {
    pub fn bento() -> Self {
        Self {
            kind: ThemeKind::Bento,
            primary: Color::rgb(99, 102, 241),
            secondary: Color::rgb(244, 63, 94),
            background: Color::rgb(15, 23, 42),
            surface: Color::rgb(30, 41, 59),
            text_primary: Color::rgb(248, 250, 252),
            text_secondary: Color::rgb(148, 163, 184),
            border: Color::rgb(51, 65, 85),
            default_radius: 16.0,
            default_padding: Insets::all(20.0),
        }
    }

    pub fn glassmorphic() -> Self {
        Self {
            kind: ThemeKind::Glassmorphic,
            primary: Color::rgba(99, 102, 241, 200),
            secondary: Color::rgba(236, 72, 153, 200),
            background: Color::rgb(10, 15, 30),
            surface: Color::rgba(255, 255, 255, 25),
            text_primary: Color::WHITE,
            text_secondary: Color::rgba(255, 255, 255, 180),
            border: Color::rgba(255, 255, 255, 40),
            default_radius: 20.0,
            default_padding: Insets::all(16.0),
        }
    }

    pub fn neobrutalist() -> Self {
        Self {
            kind: ThemeKind::Neobrutalist,
            primary: Color::rgb(255, 222, 89), // Electric yellow
            secondary: Color::rgb(255, 87, 87),
            background: Color::rgb(245, 245, 240),
            surface: Color::WHITE,
            text_primary: Color::BLACK,
            text_secondary: Color::rgb(50, 50, 50),
            border: Color::BLACK,
            default_radius: 0.0,
            default_padding: Insets::all(16.0),
        }
    }

    pub fn card_style(&self) -> Style {
        match self.kind {
            ThemeKind::Bento => Style::new()
                .bg(self.surface)
                .radius(self.default_radius)
                .pad(self.default_padding),
            ThemeKind::Glassmorphic => Style::new()
                .bg(self.surface)
                .radius(self.default_radius)
                .pad(self.default_padding),
            ThemeKind::Neobrutalist => Style::new()
                .bg(self.surface)
                .radius(0.0)
                .pad(self.default_padding),
            _ => Style::new()
                .bg(self.surface)
                .radius(8.0)
                .pad(Insets::all(16.0)),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::bento()
    }
}
