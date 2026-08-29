use serde::{Deserialize, Serialize};
use vello::kurbo::{Affine, BezPath, Rect as KurboRect, RoundedRect as KurboRoundedRect, Stroke};
use vello::peniko::{Color as PenikoColor, Fill};
use vello::{RenderParams, Renderer, RendererOptions, Scene};

use crate::diagnostic::{GuiError, GuiResult};
use crate::gpu::{GpuContext, GpuFrame};

/// An RGBA 32-bit color representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Color {
    /// Red component (0-255).
    pub r: u8,
    /// Green component (0-255).
    pub g: u8,
    /// Blue component (0-255).
    pub b: u8,
    /// Alpha opacity component (0-255, where 255 is fully opaque).
    pub a: u8,
}

impl Color {
    /// Fully transparent black.
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
    /// Solid Black (`#000000`).
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    /// Solid White (`#FFFFFF`).
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    /// Solid Red (`#FF0000`).
    pub const RED: Self = Self::rgb(255, 0, 0);
    /// Solid Green (`#00FF00`).
    pub const GREEN: Self = Self::rgb(0, 255, 0);
    /// Solid Blue (`#0000FF`).
    pub const BLUE: Self = Self::rgb(0, 0, 255);
    /// Golden Yellow (`#FFD700`).
    pub const GOLD: Self = Self::rgb(255, 215, 0);
    /// Bright Amber (`#FFBF00`).
    pub const AMBER: Self = Self::rgb(255, 191, 0);
    /// Bright Yellow (`#FFFF00`).
    pub const YELLOW: Self = Self::rgb(255, 255, 0);
    /// Fluent/Slate Gray (`#1E1E1E`).
    pub const DARK_GRAY: Self = Self::rgb(30, 30, 30);
    /// Light Slate Gray (`#F3F3F3`).
    pub const LIGHT_GRAY: Self = Self::rgb(243, 243, 243);

    /// Construct an opaque RGB color.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Construct an RGBA color with explicit alpha transparency.
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Parse a hexadecimal color string (e.g. `"#RRGGBB"` or `"#RRGGBBAA"`).
    pub fn from_hex(hex: &str) -> Result<Self, GuiError> {
        let clean = hex.trim().trim_start_matches('#');
        match clean.len() {
            6 => {
                let r = u8::from_str_radix(&clean[0..2], 16).map_err(|_| invalid_hex_error(hex))?;
                let g = u8::from_str_radix(&clean[2..4], 16).map_err(|_| invalid_hex_error(hex))?;
                let b = u8::from_str_radix(&clean[4..6], 16).map_err(|_| invalid_hex_error(hex))?;
                Ok(Self::rgb(r, g, b))
            }
            8 => {
                let r = u8::from_str_radix(&clean[0..2], 16).map_err(|_| invalid_hex_error(hex))?;
                let g = u8::from_str_radix(&clean[2..4], 16).map_err(|_| invalid_hex_error(hex))?;
                let b = u8::from_str_radix(&clean[4..6], 16).map_err(|_| invalid_hex_error(hex))?;
                let a = u8::from_str_radix(&clean[6..8], 16).map_err(|_| invalid_hex_error(hex))?;
                Ok(Self::rgba(r, g, b, a))
            }
            _ => Err(invalid_hex_error(hex)),
        }
    }

    /// Convert to Vello/Peniko `Color`.
    pub(crate) fn to_peniko(self) -> PenikoColor {
        PenikoColor::from_rgba8(self.r, self.g, self.b, self.a)
    }
}

fn invalid_hex_error(hex: &str) -> GuiError {
    GuiError::new(
        format!("Invalid hexadecimal color string `{hex}`"),
        "Color strings must be formatted as `#RRGGBB` or `#RRGGBBAA` with valid 0-9a-f digits",
        Some("Use standard format `#1E1E1E` or `Color::rgb(r, g, b)`"),
        "RFC-gui-engine §1: Malformed styling attributes must return structured Nyāya diagnostics",
    )
}

/// A 2D point in logical window coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    /// Horizontal coordinate.
    pub x: f64,
    /// Vertical coordinate.
    pub y: f64,
}

impl Point {
    /// Origin point `(0.0, 0.0)`.
    pub const ZERO: Self = Self::new(0.0, 0.0);

    /// Construct a new 2D point.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// 2D dimensions `(width, height)`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Size {
    /// Width dimension.
    pub width: f64,
    /// Height dimension.
    pub height: f64,
}

impl Size {
    /// Construct new 2D dimensions.
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }
}

/// A 2D rectangular area in logical window coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    /// Origin point (top-left corner).
    pub origin: Point,
    /// Width in logical pixels.
    pub width: f64,
    /// Height in logical pixels.
    pub height: f64,
}

impl Rect {
    /// Construct a new rectangle from origin coordinates and dimensions.
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            origin: Point::new(x, y),
            width,
            height,
        }
    }

    /// Construct a new rectangle from origin `Point` and `Size`.
    pub const fn from_origin_size(origin: Point, size: Size) -> Self {
        Self {
            origin,
            width: size.width,
            height: size.height,
        }
    }

    /// Return minimum X coordinate.
    pub const fn min_x(&self) -> f64 {
        self.origin.x
    }

    /// Return minimum Y coordinate.
    pub const fn min_y(&self) -> f64 {
        self.origin.y
    }

    /// Return maximum X coordinate.
    pub fn max_x(&self) -> f64 {
        self.origin.x + self.width
    }

    /// Return maximum Y coordinate.
    pub fn max_y(&self) -> f64 {
        self.origin.y + self.height
    }

    /// Return `true` if the point lies within the rectangle's bounds.
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.min_x() && p.x <= self.max_x() && p.y >= self.min_y() && p.y <= self.max_y()
    }

    /// Convert to Vello/Kurbo `Rect`.
    pub(crate) fn to_kurbo(self) -> KurboRect {
        KurboRect::new(self.min_x(), self.min_y(), self.max_x(), self.max_y())
    }
}

/// A rounded rectangle with uniform corner radius.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RoundedRect {
    /// Base rectangular bounding box.
    pub rect: Rect,
    /// Uniform corner radius.
    pub radius: f64,
}

impl RoundedRect {
    /// Construct a new rounded rectangle.
    pub const fn new(rect: Rect, radius: f64) -> Self {
        Self { rect, radius }
    }

    /// Convert to Vello/Kurbo `RoundedRect`.
    pub(crate) fn to_kurbo(self) -> KurboRoundedRect {
        KurboRoundedRect::new(
            self.rect.min_x(),
            self.rect.min_y(),
            self.rect.max_x(),
            self.rect.max_y(),
            self.radius,
        )
    }
}

/// Clip shapes for bounding drawing operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClipShape {
    /// Rectangular clipping region.
    Rect(Rect),
    /// Rounded rectangular clipping region.
    RoundedRect(RoundedRect),
}

/// Retained 2D scene graph node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SceneNode {
    /// Filled rectangular area.
    FillRect {
        /// Bounds of the rectangle.
        rect: Rect,
        /// Fill color.
        color: Color,
    },
    /// Filled rounded rectangular area.
    FillRoundedRect {
        /// Bounds and corner radius.
        rounded_rect: RoundedRect,
        /// Fill color.
        color: Color,
    },
    /// Outlined stroke rectangle.
    StrokeRect {
        /// Bounds of the rectangle.
        rect: Rect,
        /// Stroke outline color.
        color: Color,
        /// Line width in points.
        line_width: f64,
    },
    /// Filled multi-pointed star shape.
    FillStar {
        /// Center point of the star.
        center: Point,
        /// Outer point radius.
        outer_radius: f64,
        /// Inner valley radius.
        inner_radius: f64,
        /// Number of points (e.g. 5 for classic pentagram).
        points: usize,
        /// Fill color.
        color: Color,
    },
    /// Stroked multi-pointed star outline.
    StrokeStar {
        /// Center point of the star.
        center: Point,
        /// Outer point radius.
        outer_radius: f64,
        /// Inner valley radius.
        inner_radius: f64,
        /// Number of points.
        points: usize,
        /// Stroke outline color.
        color: Color,
        /// Line width in points.
        line_width: f64,
    },
    /// Filled arbitrary closed polygon.
    FillPolygon {
        /// Vertex sequence.
        points: Vec<Point>,
        /// Fill color.
        color: Color,
    },
    /// Stroked arbitrary closed polygon.
    StrokePolygon {
        /// Vertex sequence.
        points: Vec<Point>,
        /// Stroke outline color.
        color: Color,
        /// Line width in points.
        line_width: f64,
    },
    /// Rendered image texture.
    DrawImage {
        /// Target bounding rectangle.
        rect: Rect,
        /// Aspect ratio scaling mode.
        fit: crate::image::ImageFit,
        /// GPU texture handle.
        texture: crate::image::ImageTexture,
    },
    /// Push a clipping boundary onto the render stack.
    PushClip(ClipShape),
    /// Pop the most recently pushed clipping boundary.
    PopClip,
}

/// Builder for constructing retained 2D scene graphs.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneBuilder {
    nodes: Vec<SceneNode>,
}

impl SceneBuilder {
    /// Create a new empty scene builder.
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Serialize the scene graph to a compact JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Serialize the scene graph to a formatted pretty-printed JSON string.
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize a scene graph from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Draw a filled rectangle.
    pub fn fill_rect(&mut self, rect: Rect, color: Color) -> &mut Self {
        self.nodes.push(SceneNode::FillRect { rect, color });
        self
    }

    /// Draw a filled rounded rectangle.
    pub fn fill_rounded_rect(&mut self, rect: Rect, radius: f64, color: Color) -> &mut Self {
        self.nodes.push(SceneNode::FillRoundedRect {
            rounded_rect: RoundedRect::new(rect, radius),
            color,
        });
        self
    }

    /// Draw an outlined stroked rectangle.
    pub fn stroke_rect(&mut self, rect: Rect, color: Color, line_width: f64) -> &mut Self {
        self.nodes.push(SceneNode::StrokeRect {
            rect,
            color,
            line_width,
        });
        self
    }

    /// Draw a filled multi-pointed star.
    pub fn fill_star(
        &mut self,
        center: Point,
        outer_radius: f64,
        inner_radius: f64,
        points: usize,
        color: Color,
    ) -> &mut Self {
        self.nodes.push(SceneNode::FillStar {
            center,
            outer_radius,
            inner_radius,
            points,
            color,
        });
        self
    }

    /// Draw an outlined stroked multi-pointed star.
    pub fn stroke_star(
        &mut self,
        center: Point,
        outer_radius: f64,
        inner_radius: f64,
        points: usize,
        color: Color,
        line_width: f64,
    ) -> &mut Self {
        self.nodes.push(SceneNode::StrokeStar {
            center,
            outer_radius,
            inner_radius,
            points,
            color,
            line_width,
        });
        self
    }

    /// Draw a filled arbitrary polygon.
    pub fn fill_polygon(&mut self, points: Vec<Point>, color: Color) -> &mut Self {
        self.nodes.push(SceneNode::FillPolygon { points, color });
        self
    }

    /// Draw an outlined stroked polygon.
    pub fn stroke_polygon(
        &mut self,
        points: Vec<Point>,
        color: Color,
        line_width: f64,
    ) -> &mut Self {
        self.nodes.push(SceneNode::StrokePolygon {
            points,
            color,
            line_width,
        });
        self
    }

    /// Draw an image texture inside a bounding rectangle.
    pub fn draw_image(
        &mut self,
        rect: Rect,
        fit: crate::image::ImageFit,
        texture: crate::image::ImageTexture,
    ) -> &mut Self {
        self.nodes.push(SceneNode::DrawImage { rect, fit, texture });
        self
    }

    /// Push a rectangular clipping boundary.
    pub fn push_clip_rect(&mut self, rect: Rect) -> &mut Self {
        self.nodes.push(SceneNode::PushClip(ClipShape::Rect(rect)));
        self
    }

    /// Push a rounded rectangular clipping boundary.
    pub fn push_clip_rounded_rect(&mut self, rect: Rect, radius: f64) -> &mut Self {
        self.nodes.push(SceneNode::PushClip(ClipShape::RoundedRect(
            RoundedRect::new(rect, radius),
        )));
        self
    }

    /// Pop the topmost clipping boundary from the stack.
    pub fn pop_clip(&mut self) -> &mut Self {
        self.nodes.push(SceneNode::PopClip);
        self
    }

    /// Access the retained list of scene nodes.
    pub fn nodes(&self) -> &[SceneNode] {
        &self.nodes
    }

    /// Return the total count of scene nodes recorded.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Clear all nodes from the scene builder.
    pub fn clear(&mut self) {
        self.nodes.clear();
    }

    /// Encode the retained scene graph into a Vello `Scene` for GPU rasterization.
    pub fn build_vello_scene(&self) -> Scene {
        let mut scene = Scene::new();
        for node in &self.nodes {
            match node {
                SceneNode::FillRect { rect, color } => {
                    scene.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        (*color).to_peniko(),
                        None,
                        &rect.to_kurbo(),
                    );
                }
                SceneNode::FillRoundedRect {
                    rounded_rect,
                    color,
                } => {
                    scene.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        (*color).to_peniko(),
                        None,
                        &rounded_rect.to_kurbo(),
                    );
                }
                SceneNode::StrokeRect {
                    rect,
                    color,
                    line_width,
                } => {
                    scene.stroke(
                        &Stroke::new(*line_width),
                        Affine::IDENTITY,
                        (*color).to_peniko(),
                        None,
                        &rect.to_kurbo(),
                    );
                }
                SceneNode::FillStar {
                    center,
                    outer_radius,
                    inner_radius,
                    points,
                    color,
                } => {
                    let path = build_star_path(*center, *outer_radius, *inner_radius, *points);
                    scene.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        (*color).to_peniko(),
                        None,
                        &path,
                    );
                }
                SceneNode::StrokeStar {
                    center,
                    outer_radius,
                    inner_radius,
                    points,
                    color,
                    line_width,
                } => {
                    let path = build_star_path(*center, *outer_radius, *inner_radius, *points);
                    scene.stroke(
                        &Stroke::new(*line_width),
                        Affine::IDENTITY,
                        (*color).to_peniko(),
                        None,
                        &path,
                    );
                }
                SceneNode::FillPolygon { points, color } => {
                    if let Some(path) = build_polygon_path(points) {
                        scene.fill(
                            Fill::NonZero,
                            Affine::IDENTITY,
                            (*color).to_peniko(),
                            None,
                            &path,
                        );
                    }
                }
                SceneNode::StrokePolygon {
                    points,
                    color,
                    line_width,
                } => {
                    if let Some(path) = build_polygon_path(points) {
                        scene.stroke(
                            &Stroke::new(*line_width),
                            Affine::IDENTITY,
                            (*color).to_peniko(),
                            None,
                            &path,
                        );
                    }
                }
                SceneNode::DrawImage { rect, fit, texture } => {
                    let fit_rect = texture.compute_fit_rect(*rect, *fit);
                    let scale_x = fit_rect.width / (texture.width() as f64).max(1.0);
                    let scale_y = fit_rect.height / (texture.height() as f64).max(1.0);
                    let transform = Affine::translate((fit_rect.origin.x, fit_rect.origin.y))
                        * Affine::scale_non_uniform(scale_x, scale_y);
                    let peniko_img = texture.to_peniko();
                    scene.draw_image(&peniko_img, transform);
                }
                SceneNode::PushClip(ClipShape::Rect(rect)) => {
                    scene.push_layer(
                        Fill::NonZero,
                        vello::peniko::BlendMode::default(),
                        1.0,
                        Affine::IDENTITY,
                        &rect.to_kurbo(),
                    );
                }
                SceneNode::PushClip(ClipShape::RoundedRect(rrect)) => {
                    scene.push_layer(
                        Fill::NonZero,
                        vello::peniko::BlendMode::default(),
                        1.0,
                        Affine::IDENTITY,
                        &rrect.to_kurbo(),
                    );
                }
                SceneNode::PopClip => {
                    scene.pop_layer();
                }
            }
        }
        scene
    }
}

/// Construct a multi-pointed star BezPath.
fn build_star_path(center: Point, outer_radius: f64, inner_radius: f64, points: usize) -> BezPath {
    let mut path = BezPath::new();
    let num_points = points.max(3);
    let total_steps = num_points * 2;
    for i in 0..total_steps {
        let r = if i % 2 == 0 {
            outer_radius
        } else {
            inner_radius
        };
        let angle =
            (i as f64) * std::f64::consts::PI / (num_points as f64) - std::f64::consts::FRAC_PI_2;
        let x = center.x + r * angle.cos();
        let y = center.y + r * angle.sin();
        if i == 0 {
            path.move_to((x, y));
        } else {
            path.line_to((x, y));
        }
    }
    path.close_path();
    path
}

/// Construct an arbitrary closed polygon BezPath.
fn build_polygon_path(points: &[Point]) -> Option<BezPath> {
    if points.is_empty() {
        return None;
    }
    let mut path = BezPath::new();
    path.move_to((points[0].x, points[0].y));
    for p in &points[1..] {
        path.line_to((p.x, p.y));
    }
    path.close_path();
    Some(path)
}

/// High-performance Vello 2D scene renderer.
pub struct SceneRenderer {
    renderer: Renderer,
}

impl SceneRenderer {
    /// Initialize a new Vello scene renderer for the given GPU context.
    pub fn new(context: &GpuContext) -> GuiResult<Self> {
        let options = RendererOptions {
            pipeline_cache: None,
            use_cpu: false,
            antialiasing_support: vello::AaSupport::all(),
            num_init_threads: None,
        };

        let renderer = Renderer::new(context.device(), options).map_err(|e| {
            GuiError::new(
                format!("Failed to initialize Vello 2D scene renderer: {e}"),
                "Vello compute pipeline or shader compilation failed on active GPU adapter",
                Some("Verify GPU driver support for compute shaders or fallback to safe tier"),
                "RFC-gui-engine §1: Renderer initialization errors must yield structured Nyāya diagnostics",
            )
        })?;

        Ok(Self { renderer })
    }

    /// Render a scene to a presentation GPU frame texture.
    pub fn render_to_frame(
        &mut self,
        context: &GpuContext,
        scene: &SceneBuilder,
        frame: &GpuFrame,
        background_color: Color,
    ) -> GuiResult<()> {
        let (width, height) = frame.dimensions();
        let vello_scene = scene.build_vello_scene();
        let view = frame.create_view();

        let params = RenderParams {
            base_color: background_color.to_peniko(),
            width,
            height,
            antialiasing_method: vello::AaConfig::Area,
        };

        self.renderer
            .render_to_texture(context.device(), context.queue(), &vello_scene, &view, &params)
            .map_err(|e| {
                GuiError::new(
                    format!("Failed to rasterize 2D scene to surface texture: {e}"),
                    "Vello GPU compute execution failed during scene rasterization pass",
                    Some("Verify viewport dimensions and texture memory allocations"),
                    "RFC-gui-engine §1: GPU render execution failures must return structured diagnostics",
                )
            })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_hex_parsing() {
        assert_eq!(Color::from_hex("#FF0000"), Ok(Color::RED));
        assert_eq!(Color::from_hex("00FF00"), Ok(Color::GREEN));
        assert_eq!(
            Color::from_hex("#0000FF80"),
            Ok(Color::rgba(0, 0, 255, 128))
        );
        assert!(Color::from_hex("XYZ").is_err());
    }

    #[test]
    fn test_scene_builder_3_rects_1_rounded_clip() {
        // Acceptance criterion for Task 4:
        // Build a scene with 3 rects + 1 rounded clip, serialize/inspect the scene graph, assert node count and structure.
        let mut builder = SceneBuilder::new();

        // 1. Background rect
        builder.fill_rect(Rect::new(0.0, 0.0, 800.0, 600.0), Color::DARK_GRAY);

        // 2. Push rounded clip
        builder.push_clip_rounded_rect(Rect::new(50.0, 50.0, 300.0, 200.0), 16.0);

        // 3. Card surface rect inside clip
        builder.fill_rect(Rect::new(50.0, 50.0, 300.0, 200.0), Color::rgb(45, 45, 45));

        // 4. Accent button rect inside clip
        builder.fill_rounded_rect(Rect::new(70.0, 70.0, 120.0, 40.0), 8.0, Color::BLUE);

        // 5. Pop clip
        builder.pop_clip();

        assert_eq!(builder.node_count(), 5);
        let nodes = builder.nodes();

        assert!(matches!(nodes[0], SceneNode::FillRect { .. }));
        assert!(matches!(
            nodes[1],
            SceneNode::PushClip(ClipShape::RoundedRect(_))
        ));
        assert!(matches!(nodes[2], SceneNode::FillRect { .. }));
        assert!(matches!(nodes[3], SceneNode::FillRoundedRect { .. }));
        assert!(matches!(nodes[4], SceneNode::PopClip));

        // Compile to Vello scene (headless validation)
        let vello_scene = builder.build_vello_scene();
        // Vello scene encodes primitives without error
        let _ = vello_scene;
    }
}
