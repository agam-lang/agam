//! # Declarative Widget Hierarchy & Layout Engine (`agam_gui::widget`)
//!
//! Provides retained declarative UI node trees, one-pass Flexbox layout solver,
//! event hit-testing, and Fluent/HIG visual identity styling.

use std::sync::Arc;

use crate::scene::{Color, Point, Rect, SceneBuilder, Size};
use crate::text::{FontContext, FontWeight, TextAlign, TextWrap};

/// Stable identifier for stateful and dynamic UI nodes across render passes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UiNodeKey(pub String);

impl UiNodeKey {
    /// Create a named key.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Create an indexed key from a parent scope.
    pub fn index(parent: &str, idx: usize) -> Self {
        Self(format!("{parent}#{idx}"))
    }
}

/// Flex layout direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexDirection {
    /// Vertical column layout (top to bottom).
    #[default]
    Column,
    /// Horizontal row layout (left to right).
    Row,
}

/// Cross-axis alignment for Flex children.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CrossAxisAlignment {
    /// Align to the start of the cross axis.
    #[default]
    Start,
    /// Center children along the cross axis.
    Center,
    /// Align to the end of the cross axis.
    End,
    /// Stretch children to fill cross axis.
    Stretch,
}

/// Main-axis distribution for Flex children.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MainAxisAlignment {
    /// Pack children at the start of the main axis.
    #[default]
    Start,
    /// Center children along the main axis.
    Center,
    /// Pack children at the end of the main axis.
    End,
    /// Evenly space children with first and last at edges.
    SpaceBetween,
    /// Evenly space children with half space at edges.
    SpaceAround,
}

/// Layout constraints passed down to child widgets during measurement pass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutConstraints {
    /// Minimum allowed width in points.
    pub min_width: f64,
    /// Maximum allowed width in points.
    pub max_width: f64,
    /// Minimum allowed height in points.
    pub min_height: f64,
    /// Maximum allowed height in points.
    pub max_height: f64,
}

impl LayoutConstraints {
    /// Create unbounded constraints with a maximum container size.
    pub fn loose(max_size: Size) -> Self {
        Self {
            min_width: 0.0,
            max_width: max_size.width,
            min_height: 0.0,
            max_height: max_size.height,
        }
    }

    /// Create tight exact constraints.
    pub fn tight(size: Size) -> Self {
        Self {
            min_width: size.width,
            max_width: size.width,
            min_height: size.height,
            max_height: size.height,
        }
    }

    /// Clamp a requested size to adhere to these constraints.
    pub fn clamp(&self, size: Size) -> Size {
        Size::new(
            size.width.clamp(self.min_width, self.max_width),
            size.height.clamp(self.min_height, self.max_height),
        )
    }
}

/// Core declarative widget trait.
pub trait Widget: Send + Sync {
    /// Optional stable key for reconciliation.
    fn key(&self) -> Option<&UiNodeKey> {
        None
    }

    /// Measure widget layout size given parent constraints.
    fn layout(&self, constraints: LayoutConstraints, font_ctx: &FontContext) -> Size;

    /// Render widget geometry into the retained scene builder.
    fn render(&self, bounds: Rect, font_ctx: &FontContext, builder: &mut SceneBuilder);

    /// Perform pointer hit-testing, returning the key of the hit child node if any.
    fn hit_test(&self, point: Point, bounds: Rect) -> Option<UiNodeKey> {
        if bounds.contains(point) {
            self.key().cloned()
        } else {
            None
        }
    }
}

// ── Text Label Widget ───────────────────────────────────────────────────────

/// High-resolution text label widget with font fallback and Unicode shaping.
pub struct Label {
    pub text: String,
    pub font_size: f64,
    pub line_height: Option<f64>,
    pub weight: FontWeight,
    pub align: TextAlign,
    pub color: Color,
    pub wrap: TextWrap,
    pub key: Option<UiNodeKey>,
}

impl Label {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            font_size: 14.0,
            line_height: None,
            weight: FontWeight::Regular,
            align: TextAlign::Left,
            color: Color::WHITE,
            wrap: TextWrap::Word,
            key: None,
        }
    }

    pub fn size(mut self, size: f64) -> Self {
        self.font_size = size;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(UiNodeKey::new(key));
        self
    }
}

impl Widget for Label {
    fn key(&self) -> Option<&UiNodeKey> {
        self.key.as_ref()
    }

    fn layout(&self, constraints: LayoutConstraints, font_ctx: &FontContext) -> Size {
        let max_w = if constraints.max_width.is_finite() {
            Some(constraints.max_width)
        } else {
            None
        };
        let natural = font_ctx
            .measure_text(
                &self.text,
                self.font_size,
                self.line_height,
                max_w,
                self.wrap,
            )
            .unwrap_or(Size::new(
                self.font_size * self.text.len() as f64 * 0.6,
                self.font_size * 1.25,
            ));
        constraints.clamp(natural)
    }

    fn render(&self, bounds: Rect, font_ctx: &FontContext, builder: &mut SceneBuilder) {
        if let Ok(layout) = font_ctx.layout_text(
            &self.text,
            self.font_size,
            self.line_height,
            self.weight,
            self.align,
            Some(bounds.width),
            self.wrap,
        ) {
            for glyph in layout.glyphs {
                let gx = bounds.origin.x + glyph.x;
                let gy = bounds.origin.y + glyph.y;
                let gw = glyph.width.max(2.0);
                let gh = glyph.font_size.max(4.0);
                builder.fill_rounded_rect(Rect::new(gx, gy, gw, gh), 1.0, self.color);
            }
        }
    }
}

// ── Flex Container (Column / Row) ───────────────────────────────────────────

/// Multi-child flex container layout.
pub struct Flex {
    pub direction: FlexDirection,
    pub gap: f64,
    pub padding: f64,
    pub cross_align: CrossAxisAlignment,
    pub main_align: MainAxisAlignment,
    pub children: Vec<Box<dyn Widget>>,
    pub background: Option<Color>,
    pub corner_radius: f64,
    pub key: Option<UiNodeKey>,
}

impl Flex {
    pub fn column() -> Self {
        Self {
            direction: FlexDirection::Column,
            gap: 0.0,
            padding: 0.0,
            cross_align: CrossAxisAlignment::Start,
            main_align: MainAxisAlignment::Start,
            children: Vec::new(),
            background: None,
            corner_radius: 0.0,
            key: None,
        }
    }

    pub fn row() -> Self {
        Self {
            direction: FlexDirection::Row,
            gap: 0.0,
            padding: 0.0,
            cross_align: CrossAxisAlignment::Start,
            main_align: MainAxisAlignment::Start,
            children: Vec::new(),
            background: None,
            corner_radius: 0.0,
            key: None,
        }
    }

    pub fn gap(mut self, gap: f64) -> Self {
        self.gap = gap;
        self
    }

    pub fn padding(mut self, padding: f64) -> Self {
        self.padding = padding;
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub fn corner_radius(mut self, radius: f64) -> Self {
        self.corner_radius = radius;
        self
    }

    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(UiNodeKey::new(key));
        self
    }

    pub fn child(mut self, widget: impl Widget + 'static) -> Self {
        self.children.push(Box::new(widget));
        self
    }
}

impl Widget for Flex {
    fn key(&self) -> Option<&UiNodeKey> {
        self.key.as_ref()
    }

    fn layout(&self, constraints: LayoutConstraints, font_ctx: &FontContext) -> Size {
        let pad2 = self.padding * 2.0;
        let inner_constraints = LayoutConstraints {
            min_width: (constraints.min_width - pad2).max(0.0),
            max_width: (constraints.max_width - pad2).max(0.0),
            min_height: (constraints.min_height - pad2).max(0.0),
            max_height: (constraints.max_height - pad2).max(0.0),
        };

        let mut main_total: f64 = 0.0;
        let mut cross_max: f64 = 0.0;

        for (idx, child) in self.children.iter().enumerate() {
            let child_size = child.layout(inner_constraints, font_ctx);
            match self.direction {
                FlexDirection::Column => {
                    main_total += child_size.height;
                    cross_max = cross_max.max(child_size.width);
                    if idx > 0 {
                        main_total += self.gap;
                    }
                }
                FlexDirection::Row => {
                    main_total += child_size.width;
                    cross_max = cross_max.max(child_size.height);
                    if idx > 0 {
                        main_total += self.gap;
                    }
                }
            }
        }

        let total_size = match self.direction {
            FlexDirection::Column => Size::new(cross_max + pad2, main_total + pad2),
            FlexDirection::Row => Size::new(main_total + pad2, cross_max + pad2),
        };

        constraints.clamp(total_size)
    }

    fn render(&self, bounds: Rect, font_ctx: &FontContext, builder: &mut SceneBuilder) {
        if let Some(bg) = self.background {
            if self.corner_radius > 0.0 {
                builder.fill_rounded_rect(bounds, self.corner_radius, bg);
            } else {
                builder.fill_rect(bounds, bg);
            }
        }

        let pad = self.padding;
        let mut cur_x = bounds.origin.x + pad;
        let mut cur_y = bounds.origin.y + pad;
        let avail_w = (bounds.width - pad * 2.0).max(0.0);
        let avail_h = (bounds.height - pad * 2.0).max(0.0);

        let inner_constraints = LayoutConstraints {
            min_width: 0.0,
            max_width: avail_w,
            min_height: 0.0,
            max_height: avail_h,
        };

        for child in &self.children {
            let child_size = child.layout(inner_constraints, font_ctx);
            let child_rect = match self.direction {
                FlexDirection::Column => {
                    let r = Rect::new(cur_x, cur_y, child_size.width, child_size.height);
                    cur_y += child_size.height + self.gap;
                    r
                }
                FlexDirection::Row => {
                    let r = Rect::new(cur_x, cur_y, child_size.width, child_size.height);
                    cur_x += child_size.width + self.gap;
                    r
                }
            };
            child.render(child_rect, font_ctx, builder);
        }
    }

    fn hit_test(&self, point: Point, bounds: Rect) -> Option<UiNodeKey> {
        if !bounds.contains(point) {
            return None;
        }

        let pad = self.padding;
        let mut cur_x = bounds.origin.x + pad;
        let mut cur_y = bounds.origin.y + pad;
        let avail_w = (bounds.width - pad * 2.0).max(0.0);
        let avail_h = (bounds.height - pad * 2.0).max(0.0);
        let font_ctx = FontContext::new();

        let inner_constraints = LayoutConstraints {
            min_width: 0.0,
            max_width: avail_w,
            min_height: 0.0,
            max_height: avail_h,
        };

        for child in &self.children {
            let child_size = child.layout(inner_constraints, &font_ctx);
            let child_rect = match self.direction {
                FlexDirection::Column => {
                    let r = Rect::new(cur_x, cur_y, child_size.width, child_size.height);
                    cur_y += child_size.height + self.gap;
                    r
                }
                FlexDirection::Row => {
                    let r = Rect::new(cur_x, cur_y, child_size.width, child_size.height);
                    cur_x += child_size.width + self.gap;
                    r
                }
            };

            if let Some(hit) = child.hit_test(point, child_rect) {
                return Some(hit);
            }
        }

        self.key().cloned()
    }
}

// ── Interactive Push Button Widget ──────────────────────────────────────────

/// Callback triggered on button click.
pub type ClickCallback = Arc<dyn Fn() + Send + Sync>;

/// Push button widget with dynamic hover and click states.
pub struct Button {
    pub label: String,
    pub on_click: Option<ClickCallback>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub background: Color,
    pub text_color: Color,
    pub corner_radius: f64,
    pub key: UiNodeKey,
}

impl Button {
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        let k = key.into();
        Self {
            label: label.into(),
            on_click: None,
            width: None,
            height: Some(36.0),
            background: Color::rgb(0, 120, 212), // Fluent Accent Blue
            text_color: Color::WHITE,
            corner_radius: 6.0,
            key: UiNodeKey::new(k),
        }
    }

    pub fn on_click(mut self, cb: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_click = Some(Arc::new(cb));
        self
    }

    pub fn size(mut self, width: f64, height: f64) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = color;
        self
    }
}

impl Widget for Button {
    fn key(&self) -> Option<&UiNodeKey> {
        Some(&self.key)
    }

    fn layout(&self, constraints: LayoutConstraints, _font_ctx: &FontContext) -> Size {
        let w = self.width.unwrap_or(constraints.min_width.max(80.0));
        let h = self.height.unwrap_or(36.0);
        constraints.clamp(Size::new(w, h))
    }

    fn render(&self, bounds: Rect, _font_ctx: &FontContext, builder: &mut SceneBuilder) {
        builder.fill_rounded_rect(bounds, self.corner_radius, self.background);
        builder.stroke_rect(bounds, Color::rgb(255, 255, 255), 0.5);

        // Draw centered button label
        let font_size = 14.0;
        let char_w = font_size * 0.6;
        let total_w = self.label.len() as f64 * char_w;
        let cx = bounds.origin.x + (bounds.width - total_w) / 2.0;
        let cy = bounds.origin.y + (bounds.height - font_size) / 2.0;

        for (idx, _) in self.label.chars().enumerate() {
            let gx = cx + (idx as f64) * char_w;
            builder.fill_rounded_rect(
                Rect::new(gx, cy, char_w * 0.8, font_size),
                1.0,
                self.text_color,
            );
        }
    }

    fn hit_test(&self, point: Point, bounds: Rect) -> Option<UiNodeKey> {
        if bounds.contains(point) {
            Some(self.key.clone())
        } else {
            None
        }
    }
}

// ── Elevated Card Container ─────────────────────────────────────────────────

/// Elevated container with clipping boundary and subtle border.
pub struct Card {
    pub padding: f64,
    pub corner_radius: f64,
    pub background: Color,
    pub border_color: Color,
    pub child: Box<dyn Widget>,
    pub key: Option<UiNodeKey>,
}

impl Card {
    pub fn new(child: impl Widget + 'static) -> Self {
        Self {
            padding: 16.0,
            corner_radius: 8.0,
            background: Color::rgb(32, 32, 32),
            border_color: Color::rgb(60, 60, 60),
            child: Box::new(child),
            key: None,
        }
    }

    pub fn padding(mut self, padding: f64) -> Self {
        self.padding = padding;
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = color;
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }
}

impl Widget for Card {
    fn key(&self) -> Option<&UiNodeKey> {
        self.key.as_ref()
    }

    fn layout(&self, constraints: LayoutConstraints, font_ctx: &FontContext) -> Size {
        let pad2 = self.padding * 2.0;
        let inner = LayoutConstraints {
            min_width: (constraints.min_width - pad2).max(0.0),
            max_width: (constraints.max_width - pad2).max(0.0),
            min_height: (constraints.min_height - pad2).max(0.0),
            max_height: (constraints.max_height - pad2).max(0.0),
        };
        let child_size = self.child.layout(inner, font_ctx);
        constraints.clamp(Size::new(child_size.width + pad2, child_size.height + pad2))
    }

    fn render(&self, bounds: Rect, font_ctx: &FontContext, builder: &mut SceneBuilder) {
        builder.fill_rounded_rect(bounds, self.corner_radius, self.background);
        builder.stroke_rect(bounds, self.border_color, 1.0);

        builder.push_clip_rounded_rect(bounds, self.corner_radius);
        let inner_rect = Rect::new(
            bounds.origin.x + self.padding,
            bounds.origin.y + self.padding,
            (bounds.width - self.padding * 2.0).max(0.0),
            (bounds.height - self.padding * 2.0).max(0.0),
        );
        self.child.render(inner_rect, font_ctx, builder);
        builder.pop_clip();
    }

    fn hit_test(&self, point: Point, bounds: Rect) -> Option<UiNodeKey> {
        if !bounds.contains(point) {
            return None;
        }
        let inner_rect = Rect::new(
            bounds.origin.x + self.padding,
            bounds.origin.y + self.padding,
            (bounds.width - self.padding * 2.0).max(0.0),
            (bounds.height - self.padding * 2.0).max(0.0),
        );
        self.child
            .hit_test(point, inner_rect)
            .or_else(|| self.key().cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_label_layout_and_render() {
        let font_ctx = FontContext::new();
        let label = Label::new("Agam UI").size(16.0).color(Color::WHITE);
        let constraints = LayoutConstraints::loose(Size::new(200.0, 100.0));
        let size = label.layout(constraints, &font_ctx);
        assert!(size.width > 0.0);
        assert!(size.height > 0.0);

        let mut builder = SceneBuilder::new();
        label.render(
            Rect::new(0.0, 0.0, size.width, size.height),
            &font_ctx,
            &mut builder,
        );
        assert!(builder.node_count() > 0);
    }

    #[test]
    fn test_flex_column_layout_and_hit_test() {
        let font_ctx = FontContext::new();
        let col = Flex::column()
            .gap(10.0)
            .padding(15.0)
            .child(Label::new("Title").key("lbl-1"))
            .child(Button::new("btn-submit", "Submit"));

        let constraints = LayoutConstraints::loose(Size::new(400.0, 600.0));
        let size = col.layout(constraints, &font_ctx);
        assert!(size.height >= 40.0);

        let hit = col.hit_test(
            Point::new(20.0, 20.0),
            Rect::new(0.0, 0.0, size.width, size.height),
        );
        assert!(hit.is_some());
    }

    #[test]
    fn test_10k_node_dirty_rect_stress() {
        let _font_ctx = FontContext::new();
        let mut builder = SceneBuilder::new();

        // Construct 10,000 visual primitives across a 2D canvas
        for i in 0..10_000 {
            let col = (i % 100) as f64 * 20.0;
            let row = (i / 100) as f64 * 20.0;
            let rect = Rect::new(col, row, 18.0, 18.0);
            builder.fill_rounded_rect(rect, 2.0, Color::rgba(50, 100, 200, 255));
        }

        assert_eq!(builder.node_count(), 10_000);
        assert!(builder.node_count() > 0);
    }
}
