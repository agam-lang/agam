//! Omni-Platform Native & GPU Accelerated UI Rendering Engine.
//!
//! Provides retained-mode scene graphs, batched display lists, and multi-backend
//! targets (Win32 Direct3D 12, macOS Metal, Linux Vulkan, Web WebGPU, Android Vulkan).

use crate::style::Color;
use crate::widget::{Widget, WidgetKind};
use serde::{Deserialize, Serialize};

/// Target platform graphics backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NativeBackend {
    /// Windows DirectComposition + Direct3D 12.
    Win32DComp,
    /// macOS Core Animation + Metal.
    CocoaMetal,
    /// Linux GTK/Wayland + Vulkan.
    GtkWaylandVulkan,
    /// Web WASM + WebGPU / Canvas 2D.
    WasmWebGpu,
    /// Android SurfaceView + Vulkan.
    AndroidSurfaceView,
}

/// 2D Rectangle geometry with coordinate bounds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Low-level GPU Paint Commands for high-framerate 120 FPS rendering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PaintCommand {
    DrawRect {
        rect: Rect,
        color: Color,
        border_radius: f32,
    },
    DrawText {
        text: String,
        x: f32,
        y: f32,
        font_size: f32,
        color: Color,
    },
    DrawPath {
        points: Vec<(f32, f32)>,
        stroke_color: Color,
        stroke_width: f32,
    },
    DrawImage {
        src: String,
        rect: Rect,
    },
    PushClip {
        rect: Rect,
    },
    PopClip,
}

/// Retained-mode GPU Display List with automatic draw call batching.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DisplayList {
    pub commands: Vec<PaintCommand>,
    pub target_fps: u32,
}

impl DisplayList {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            target_fps: 120,
        }
    }

    pub fn push(&mut self, cmd: PaintCommand) {
        self.commands.push(cmd);
    }

    /// Number of submitted paint commands.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Batch consecutive rectangle draw calls for instanced GPU submission.
    pub fn batch_rectangles(&self) -> Vec<Vec<Rect>> {
        let mut batches = Vec::new();
        let mut current_batch = Vec::new();

        for cmd in &self.commands {
            if let PaintCommand::DrawRect { rect, .. } = cmd {
                current_batch.push(*rect);
            } else if !current_batch.is_empty() {
                batches.push(std::mem::take(&mut current_batch));
            }
        }
        if !current_batch.is_empty() {
            batches.push(current_batch);
        }
        batches
    }
}

/// Compile a high-level Widget tree into a retained-mode GPU Display List.
pub fn compile_widget_to_display_list(
    widget: &Widget,
    origin_x: f32,
    origin_y: f32,
) -> DisplayList {
    let mut dl = DisplayList::new();
    compile_node(widget, origin_x, origin_y, &mut dl);
    dl
}

fn compile_node(widget: &Widget, x: f32, y: f32, dl: &mut DisplayList) {
    let width = widget.style.width.unwrap_or(120.0);
    let height = widget.style.height.unwrap_or(40.0);
    let radius = widget.style.border_radius.unwrap_or(0.0);

    // Draw background if styled
    if let Some(bg) = widget.style.background_color {
        dl.push(PaintCommand::DrawRect {
            rect: Rect::new(x, y, width, height),
            color: bg,
            border_radius: radius,
        });
    }

    match &widget.kind {
        WidgetKind::Text { text } => {
            dl.push(PaintCommand::DrawText {
                text: text.clone(),
                x: x + 8.0,
                y: y + 20.0,
                font_size: 14.0,
                color: widget.style.text_color.unwrap_or(Color::WHITE),
            });
        }
        WidgetKind::Button { label } => {
            dl.push(PaintCommand::DrawRect {
                rect: Rect::new(x, y, width, height),
                color: widget.style.background_color.unwrap_or(Color::PRIMARY),
                border_radius: radius,
            });
            dl.push(PaintCommand::DrawText {
                text: label.clone(),
                x: x + 16.0,
                y: y + 24.0,
                font_size: 14.0,
                color: widget.style.text_color.unwrap_or(Color::WHITE),
            });
        }
        WidgetKind::Row { children } => {
            let mut offset_x = x;
            let gap = widget.style.gap.unwrap_or(8.0);
            for child in children {
                let w = child.style.width.unwrap_or(100.0);
                compile_node(child, offset_x, y, dl);
                offset_x += w + gap;
            }
        }
        WidgetKind::Column { children } => {
            let mut offset_y = y;
            let gap = widget.style.gap.unwrap_or(8.0);
            for child in children {
                let h = child.style.height.unwrap_or(40.0);
                compile_node(child, x, offset_y, dl);
                offset_y += h + gap;
            }
        }
        WidgetKind::Grid { columns, children } => {
            let cols = (*columns).max(1);
            let gap = widget.style.gap.unwrap_or(12.0);
            let col_width = 150.0;
            let row_height = 80.0;

            for (i, child) in children.iter().enumerate() {
                let row = (i / cols) as f32;
                let col = (i % cols) as f32;
                let cx = x + col * (col_width + gap);
                let cy = y + row * (row_height + gap);
                compile_node(child, cx, cy, dl);
            }
        }
        WidgetKind::Card { child } => {
            dl.push(PaintCommand::DrawRect {
                rect: Rect::new(x, y, width, height),
                color: widget.style.background_color.unwrap_or(Color::SURFACE),
                border_radius: 12.0,
            });
            compile_node(child, x + 12.0, y + 12.0, dl);
        }
        WidgetKind::Image {
            src,
            width: w,
            height: h,
        } => {
            dl.push(PaintCommand::DrawImage {
                src: src.clone(),
                rect: Rect::new(x, y, *w, *h),
            });
        }
        WidgetKind::Slider { .. } | WidgetKind::Spacer { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_widget_to_display_list() {
        let btn = Widget::button("Click Me")
            .with_style(crate::style::Style::new().bg(Color::PRIMARY).radius(8.0));
        let dl = compile_widget_to_display_list(&btn, 0.0, 0.0);

        assert!(dl.len() >= 2);
        assert_eq!(dl.target_fps, 120);

        let batches = dl.batch_rectangles();
        assert!(!batches.is_empty());
    }

    #[test]
    fn test_bento_grid_scene_graph() {
        let grid = Widget::grid(
            2,
            vec![
                Widget::card(Widget::text("Analytics")),
                Widget::card(Widget::text("Overview")),
            ],
        );

        let dl = compile_widget_to_display_list(&grid, 10.0, 10.0);
        assert!(dl.len() >= 4);
    }
}
