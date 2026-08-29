//! # Text Shaping, Layout & Font Fallback (`agam_gui::text`)
//!
//! Encapsulates `cosmic-text` to provide high-performance Unicode text shaping,
//! multiline layout wrapping, font fallback chains, and Vello glyph rasterization.
//! No third-party font types leak across public module boundaries.

use std::sync::{Arc, Mutex};

use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache, Weight, Wrap};

use crate::diagnostic::{GuiError, GuiResult};
use crate::scene::Size;

/// Font weight specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FontWeight {
    /// Thin weight (100).
    Thin,
    /// Extra Light weight (200).
    ExtraLight,
    /// Light weight (300).
    Light,
    /// Normal regular weight (400).
    #[default]
    Regular,
    /// Medium weight (500).
    Medium,
    /// Semi-Bold weight (600).
    SemiBold,
    /// Bold weight (700).
    Bold,
    /// Extra Bold weight (800).
    ExtraBold,
    /// Black heavy weight (900).
    Black,
}

impl FontWeight {
    fn to_cosmic(self) -> Weight {
        match self {
            Self::Thin => Weight::THIN,
            Self::ExtraLight => Weight::EXTRA_LIGHT,
            Self::Light => Weight::LIGHT,
            Self::Regular => Weight::NORMAL,
            Self::Medium => Weight::MEDIUM,
            Self::SemiBold => Weight::SEMIBOLD,
            Self::Bold => Weight::BOLD,
            Self::ExtraBold => Weight::EXTRA_BOLD,
            Self::Black => Weight::BLACK,
        }
    }
}

/// Horizontal text alignment within a bounding container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextAlign {
    /// Align text to the left.
    #[default]
    Left,
    /// Center text horizontally.
    Center,
    /// Align text to the right.
    Right,
}

/// Text wrapping behavior for constrained layout boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextWrap {
    /// Wrap text at word boundaries.
    #[default]
    Word,
    /// Wrap text at individual character/glyph boundaries.
    Glyph,
    /// Do not wrap text; overflow bounds.
    None,
}

impl TextWrap {
    fn to_cosmic(self) -> Wrap {
        match self {
            Self::Word => Wrap::Word,
            Self::Glyph => Wrap::Glyph,
            Self::None => Wrap::None,
        }
    }
}

/// Central font system and glyph rasterization cache.
#[derive(Clone)]
pub struct FontContext {
    inner: Arc<Mutex<FontSystemInner>>,
}

struct FontSystemInner {
    font_system: FontSystem,
    #[allow(dead_code)]
    swash_cache: SwashCache,
}

impl Default for FontContext {
    fn default() -> Self {
        Self::new()
    }
}

impl FontContext {
    /// Initialize a new font context scanning platform system fonts.
    pub fn new() -> Self {
        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        Self {
            inner: Arc::new(Mutex::new(FontSystemInner {
                font_system,
                swash_cache,
            })),
        }
    }

    /// Load custom TrueType/OpenType font bytes into the font database.
    pub fn load_font_data(&self, _name: &str, data: Vec<u8>) -> GuiResult<()> {
        let mut inner = self.inner.lock().map_err(|_| font_lock_error())?;
        inner.font_system.db_mut().load_font_data(data);
        Ok(())
    }

    /// Measure text layout bounds without rendering.
    pub fn measure_text(
        &self,
        text: &str,
        font_size: f64,
        line_height: Option<f64>,
        max_width: Option<f64>,
        wrap: TextWrap,
    ) -> GuiResult<Size> {
        let mut inner = self.inner.lock().map_err(|_| font_lock_error())?;
        let lh = line_height.unwrap_or(font_size * 1.25);
        let metrics = Metrics::new(font_size as f32, lh as f32);

        let mut buffer = Buffer::new(&mut inner.font_system, metrics);
        buffer.set_wrap(wrap.to_cosmic());
        buffer.set_size(max_width.map(|w| w as f32), None);

        let attrs = Attrs::new().family(Family::SansSerif);
        buffer.set_text(text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut inner.font_system, false);

        let mut width: f32 = 0.0;
        let mut height: f32 = 0.0;

        for run in buffer.layout_runs() {
            width = width.max(run.line_w);
            height = height.max(run.line_top + run.line_height);
        }

        Ok(Size::new(width as f64, height as f64))
    }

    /// Format and layout text into structured glyph runs.
    #[allow(clippy::too_many_arguments)]
    pub fn layout_text(
        &self,
        text: &str,
        font_size: f64,
        line_height: Option<f64>,
        weight: FontWeight,
        align: TextAlign,
        max_width: Option<f64>,
        wrap: TextWrap,
    ) -> GuiResult<ShapedTextLayout> {
        let mut inner = self.inner.lock().map_err(|_| font_lock_error())?;
        let lh = line_height.unwrap_or(font_size * 1.25);
        let metrics = Metrics::new(font_size as f32, lh as f32);

        let mut buffer = Buffer::new(&mut inner.font_system, metrics);
        buffer.set_wrap(wrap.to_cosmic());
        buffer.set_size(max_width.map(|w| w as f32), None);

        let attrs = Attrs::new()
            .family(Family::SansSerif)
            .weight(weight.to_cosmic());

        buffer.set_text(text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut inner.font_system, false);

        let mut glyph_runs = Vec::new();
        let mut total_width: f32 = 0.0;
        let mut total_height: f32 = 0.0;

        for run in buffer.layout_runs() {
            total_width = total_width.max(run.line_w);
            total_height = total_height.max(run.line_top + run.line_height);

            // Compute alignment offset for this line
            let align_offset = match align {
                TextAlign::Left => 0.0,
                TextAlign::Center => {
                    if let Some(mw) = max_width {
                        ((mw as f32 - run.line_w) / 2.0).max(0.0)
                    } else {
                        0.0
                    }
                }
                TextAlign::Right => {
                    if let Some(mw) = max_width {
                        (mw as f32 - run.line_w).max(0.0)
                    } else {
                        0.0
                    }
                }
            };

            for glyph in run.glyphs {
                glyph_runs.push(ShapedGlyph {
                    x: (glyph.x + align_offset) as f64,
                    y: (run.line_top + glyph.y) as f64,
                    width: glyph.w as f64,
                    font_size,
                    line_height: run.line_height as f64,
                });
            }
        }

        Ok(ShapedTextLayout {
            text: text.to_string(),
            size: Size::new(total_width as f64, total_height as f64),
            font_size,
            glyphs: glyph_runs,
        })
    }
}

/// A shaped glyph instance with computed layout position.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedGlyph {
    /// X position in layout coordinates.
    pub x: f64,
    /// Y position in layout coordinates.
    pub y: f64,
    /// Advance width of the glyph.
    pub width: f64,
    /// Active font size in points.
    pub font_size: f64,
    /// Line height of the parent line.
    pub line_height: f64,
}

/// Retained shaped text layout ready for Vello scene generation.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedTextLayout {
    /// Original text string.
    pub text: String,
    /// Total computed bounding box dimensions.
    pub size: Size,
    /// Active font size.
    pub font_size: f64,
    /// List of shaped glyphs.
    pub glyphs: Vec<ShapedGlyph>,
}

impl ShapedTextLayout {
    /// Returns the total width in points.
    pub fn width(&self) -> f64 {
        self.size.width
    }

    /// Returns the total height in points.
    pub fn height(&self) -> f64 {
        self.size.height
    }

    /// Returns `true` if no glyphs were produced.
    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
    }
}

fn font_lock_error() -> GuiError {
    GuiError::new(
        "Failed to acquire font system mutex lock",
        "Font system inner lock was poisoned or contended",
        Some("Ensure font context is not accessed concurrently across panicking worker threads"),
        "RFC-gui-engine §2: Font system lock failures must return structured Nyāya diagnostics",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_context_creation_and_measure() {
        let font_ctx = FontContext::new();
        let size = font_ctx
            .measure_text("Hello Agam!", 16.0, None, None, TextWrap::Word)
            .expect("Text measurement");
        assert!(size.width > 0.0);
        assert!(size.height > 0.0);
    }

    #[test]
    fn test_multiline_text_wrapping() {
        let font_ctx = FontContext::new();
        let single_line = font_ctx
            .measure_text("Short text", 16.0, None, None, TextWrap::Word)
            .expect("Single line measure");

        let multiline = font_ctx
            .measure_text(
                "This is a longer paragraph intended to test multiline wrapping behavior in the Agam GUI text layout engine.",
                16.0,
                None,
                Some(150.0),
                TextWrap::Word,
            )
            .expect("Multiline measure");

        assert!(multiline.height > single_line.height);
    }

    #[test]
    fn test_text_layout_glyph_generation() {
        let font_ctx = FontContext::new();
        let layout = font_ctx
            .layout_text(
                "Agam 2026",
                18.0,
                None,
                FontWeight::Bold,
                TextAlign::Center,
                Some(200.0),
                TextWrap::None,
            )
            .expect("Text layout");

        assert!(!layout.is_empty());
        assert_eq!(layout.text, "Agam 2026");
        assert!(layout.glyphs.len() >= 8);
    }
}
