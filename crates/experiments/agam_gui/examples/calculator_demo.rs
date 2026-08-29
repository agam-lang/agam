//! # Agam Native Calculator GUI Demo
//!
//! A GPU-accelerated desktop calculator featuring:
//! - Full mouse click, press, and hover interaction on an interactive button grid.
//! - Comprehensive keyboard input (0-9, +, -, *, /, Enter, Backspace, Escape, Decimal).
//! - Vector stroke glyph typography scaled smoothly via Vello compute shaders.
//! - Retained scene graph rendering with clipping cards and fluent dark aesthetics.

use agam_gui::CalculatorApp;
use agam_gui::platform::{GuiEventLoop, WindowConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = GuiEventLoop::new()?;
    let config = WindowConfig::new("Agam Native Calculator", 440, 620);
    let app = CalculatorApp::default();
    event_loop.run(config, app)?;
    Ok(())
}
