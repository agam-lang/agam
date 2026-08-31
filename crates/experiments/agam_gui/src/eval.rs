//! # `agam_gui::eval` — Dynamic GUI AST Tree Evaluator & UI Runtime
//!
//! Under the Agam zero-identity-leak invariant, this module translates raw parsed
//! `.agam` `@ui` AST trees (`ExprKind::StructLiteral`, `ExprKind::Lambda`, `StmtKind::Let`)
//! into live interactive GPU widget trees without hardcoded dispatch.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use agam_ast::Module;
use agam_ast::decl::DeclKind;
use agam_ast::expr::{BinOp, Block, Expr, ExprKind, FieldInit, UnaryOp};
use agam_ast::pattern::{Pattern, PatternKind};
use agam_ast::stmt::{Stmt, StmtKind};

use crate::apps::draw_text;
use crate::diagnostic::{GuiError, GuiResult};
use crate::gpu::{GpuContext, GpuSurface};
use crate::input::{GuiEvent, MouseButton};
use crate::platform::{GuiApp, GuiWindow, WindowConfig};
use crate::scene::{Color, Point, Rect, SceneBuilder, SceneRenderer};
use crate::text::{FontContext, FontWeight, TextAlign};
use crate::widget::{CrossAxisAlignment, FlexDirection, MainAxisAlignment};

fn eval_error(msg: impl Into<String>) -> GuiError {
    GuiError::new(
        msg,
        "Declarative AST evaluation failed",
        Some("Check syntax and property types in .agam @ui script"),
        "Agam Declarative UI Invariant §4",
    )
}

// ── Dynamic UI Value & State Store ──────────────────────────────────────────

/// Represents dynamic primitive values within the declarative UI runtime state store.
#[derive(Debug, Clone, PartialEq)]
pub enum UiValue {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Void,
}

impl UiValue {
    /// Formats the UI value for text display widgets.
    pub fn to_display_string(&self) -> String {
        match self {
            UiValue::Str(s) => s.clone(),
            UiValue::Int(i) => i.to_string(),
            UiValue::Float(f) => {
                if f.is_nan() {
                    "Error".to_string()
                } else if f.fract() == 0.0 && f.abs() < 1e14 {
                    format!("{:.0}", f)
                } else {
                    let s = format!("{:.10}", f);
                    let trimmed = s.trim_end_matches('0').trim_end_matches('.');
                    trimmed.to_string()
                }
            }
            UiValue::Bool(b) => b.to_string(),
            UiValue::Void => String::new(),
        }
    }

    /// Converts the value to `f64` for mathematical calculations.
    pub fn to_f64(&self) -> f64 {
        match self {
            UiValue::Float(f) => *f,
            UiValue::Int(i) => *i as f64,
            UiValue::Str(s) => s.parse::<f64>().unwrap_or(0.0),
            UiValue::Bool(b) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
            UiValue::Void => 0.0,
        }
    }

    /// Converts the value to a boolean condition.
    pub fn to_bool(&self) -> bool {
        match self {
            UiValue::Bool(b) => *b,
            UiValue::Int(i) => *i != 0,
            UiValue::Float(f) => *f != 0.0 && !f.is_nan(),
            UiValue::Str(s) => !s.is_empty() && s != "0",
            UiValue::Void => false,
        }
    }
}

// ── Lightweight UI Expression & Closure Evaluator ──────────────────────────

/// Reactive UI runtime maintaining active widget state variables.
#[derive(Debug, Clone, Default)]
pub struct UiRuntime {
    pub state: HashMap<String, UiValue>,
}

impl UiRuntime {
    /// Create a fresh empty UI runtime state store.
    pub fn new() -> Self {
        Self {
            state: HashMap::new(),
        }
    }

    /// Evaluate an AST statement in the UI scope.
    pub fn eval_stmt(&mut self, stmt: &Stmt) -> GuiResult<UiValue> {
        match &stmt.kind {
            StmtKind::Let { pattern, value, .. } => {
                let val = if let Some(val_expr) = value {
                    self.eval_expr(val_expr)?
                } else {
                    UiValue::Void
                };
                self.bind_pattern(pattern, val);
                Ok(UiValue::Void)
            }
            StmtKind::Expression(expr) => self.eval_expr(expr),
            StmtKind::Return(Some(expr)) => self.eval_expr(expr),
            StmtKind::Return(None) => Ok(UiValue::Void),
            _ => Ok(UiValue::Void),
        }
    }

    fn bind_pattern(&mut self, pattern: &Pattern, val: UiValue) {
        if let PatternKind::Identifier { name, .. } = &pattern.kind {
            self.state.insert(name.name.clone(), val);
        }
    }

    /// Evaluate an AST expression in the UI runtime scope.
    pub fn eval_expr(&mut self, expr: &Expr) -> GuiResult<UiValue> {
        match &expr.kind {
            ExprKind::IntLiteral(i) => Ok(UiValue::Int(*i)),
            ExprKind::FloatLiteral(f) => Ok(UiValue::Float(*f)),
            ExprKind::StringLiteral(s) => Ok(UiValue::Str(s.clone())),
            ExprKind::BoolLiteral(b) => Ok(UiValue::Bool(*b)),
            ExprKind::Identifier(id) => {
                if let Some(val) = self.state.get(&id.name) {
                    Ok(val.clone())
                } else {
                    Ok(UiValue::Str(id.name.clone()))
                }
            }
            ExprKind::Binary { op, left, right } => {
                let l = self.eval_expr(left)?;
                let r = self.eval_expr(right)?;
                self.eval_binary_op(*op, l, r)
            }
            ExprKind::Unary { op, operand } => {
                let val = self.eval_expr(operand)?;
                match op {
                    UnaryOp::Neg => match val {
                        UiValue::Int(i) => Ok(UiValue::Int(-i)),
                        UiValue::Float(f) => Ok(UiValue::Float(-f)),
                        _ => Ok(val),
                    },
                    UnaryOp::Not => Ok(UiValue::Bool(!val.to_bool())),
                    _ => Ok(val),
                }
            }
            ExprKind::Assign { target, value } => {
                let val = self.eval_expr(value)?;
                if let ExprKind::Identifier(id) = &target.kind {
                    self.state.insert(id.name.clone(), val.clone());
                }
                Ok(val)
            }
            ExprKind::CompoundAssign { op, target, value } => {
                if let ExprKind::Identifier(id) = &target.kind {
                    let current = self.state.get(&id.name).cloned().unwrap_or(UiValue::Int(0));
                    let r_val = self.eval_expr(value)?;
                    let new_val = self.eval_binary_op(*op, current, r_val)?;
                    self.state.insert(id.name.clone(), new_val.clone());
                    Ok(new_val)
                } else {
                    Ok(UiValue::Void)
                }
            }
            ExprKind::Call { callee, args } => {
                let mut evaluated_args = Vec::with_capacity(args.len());
                for arg in args {
                    evaluated_args.push(self.eval_expr(arg)?);
                }
                if let ExprKind::Identifier(id) = &callee.kind {
                    self.eval_builtin_function(&id.name, &evaluated_args)
                } else {
                    Ok(UiValue::Void)
                }
            }
            ExprKind::Block(block) | ExprKind::BlockExpr(block) => self.eval_block(block),
            ExprKind::Lambda { body, .. } => self.eval_expr(body),
            _ => Ok(UiValue::Void),
        }
    }

    fn eval_block(&mut self, block: &Block) -> GuiResult<UiValue> {
        let mut last = UiValue::Void;
        for stmt in &block.stmts {
            last = self.eval_stmt(stmt)?;
        }
        if let Some(tail) = &block.expr {
            last = self.eval_expr(tail)?;
        }
        Ok(last)
    }

    fn eval_binary_op(&self, op: BinOp, l: UiValue, r: UiValue) -> GuiResult<UiValue> {
        match op {
            BinOp::Add => match (l, r) {
                (UiValue::Str(s1), UiValue::Str(s2)) => Ok(UiValue::Str(format!("{s1}{s2}"))),
                (UiValue::Str(s), val) => {
                    Ok(UiValue::Str(format!("{}{}", s, val.to_display_string())))
                }
                (val, UiValue::Str(s)) => {
                    Ok(UiValue::Str(format!("{}{}", val.to_display_string(), s)))
                }
                (UiValue::Int(i1), UiValue::Int(i2)) => Ok(UiValue::Int(i1 + i2)),
                (l_val, r_val) => Ok(UiValue::Float(l_val.to_f64() + r_val.to_f64())),
            },
            BinOp::Sub => match (l, r) {
                (UiValue::Int(i1), UiValue::Int(i2)) => Ok(UiValue::Int(i1 - i2)),
                (l_val, r_val) => Ok(UiValue::Float(l_val.to_f64() - r_val.to_f64())),
            },
            BinOp::Mul => match (l, r) {
                (UiValue::Int(i1), UiValue::Int(i2)) => Ok(UiValue::Int(i1 * i2)),
                (l_val, r_val) => Ok(UiValue::Float(l_val.to_f64() * r_val.to_f64())),
            },
            BinOp::Div => {
                let r_num = r.to_f64();
                if r_num == 0.0 {
                    Ok(UiValue::Float(f64::NAN))
                } else {
                    Ok(UiValue::Float(l.to_f64() / r_num))
                }
            }
            BinOp::Eq => Ok(UiValue::Bool(l == r)),
            BinOp::NotEq => Ok(UiValue::Bool(l != r)),
            BinOp::Lt => Ok(UiValue::Bool(l.to_f64() < r.to_f64())),
            BinOp::LtEq => Ok(UiValue::Bool(l.to_f64() <= r.to_f64())),
            BinOp::Gt => Ok(UiValue::Bool(l.to_f64() > r.to_f64())),
            BinOp::GtEq => Ok(UiValue::Bool(l.to_f64() >= r.to_f64())),
            BinOp::And => Ok(UiValue::Bool(l.to_bool() && r.to_bool())),
            BinOp::Or => Ok(UiValue::Bool(l.to_bool() || r.to_bool())),
            _ => Ok(UiValue::Void),
        }
    }

    fn eval_builtin_function(&mut self, name: &str, args: &[UiValue]) -> GuiResult<UiValue> {
        match name {
            "parse_f64" => {
                let s = args
                    .first()
                    .map(|v| v.to_display_string())
                    .unwrap_or_default();
                let f = s.parse::<f64>().unwrap_or(0.0);
                Ok(UiValue::Float(f))
            }
            "append_digit" => {
                let display = args
                    .first()
                    .map(|v| v.to_display_string())
                    .unwrap_or_else(|| "0".to_string());
                let digit = args
                    .get(1)
                    .map(|v| v.to_display_string())
                    .unwrap_or_default();
                let start_new = args.get(2).map(|v| v.to_bool()).unwrap_or(false);
                let result = if start_new || display == "0" {
                    digit
                } else {
                    format!("{display}{digit}")
                };
                Ok(UiValue::Str(result))
            }
            "append_decimal" => {
                let display = args
                    .first()
                    .map(|v| v.to_display_string())
                    .unwrap_or_else(|| "0".to_string());
                let start_new = args.get(1).map(|v| v.to_bool()).unwrap_or(false);
                let result = if start_new {
                    "0.".to_string()
                } else if !display.contains('.') {
                    format!("{display}.")
                } else {
                    display
                };
                Ok(UiValue::Str(result))
            }
            "toggle_sign" => {
                let display = args
                    .first()
                    .map(|v| v.to_display_string())
                    .unwrap_or_else(|| "0".to_string());
                let result = if let Some(stripped) = display.strip_prefix('-') {
                    stripped.to_string()
                } else if display != "0" {
                    format!("-{display}")
                } else {
                    display
                };
                Ok(UiValue::Str(result))
            }
            "percent_op" => {
                let display = args
                    .first()
                    .map(|v| v.to_display_string())
                    .unwrap_or_else(|| "0".to_string());
                let val = display.parse::<f64>().unwrap_or(0.0) / 100.0;
                Ok(UiValue::Str(UiValue::Float(val).to_display_string()))
            }
            "evaluate" => {
                let a = args.first().map(|v| v.to_f64()).unwrap_or(0.0);
                let op = args
                    .get(1)
                    .map(|v| v.to_display_string())
                    .unwrap_or_default();
                let b = args.get(2).map(|v| v.to_f64()).unwrap_or(0.0);
                let result = match op.as_str() {
                    "+" => a + b,
                    "-" | "−" => a - b,
                    "*" | "×" => a * b,
                    "/" | "÷" => {
                        if b.abs() < 1e-12 {
                            f64::NAN
                        } else {
                            a / b
                        }
                    }
                    _ => b,
                };
                Ok(UiValue::Str(UiValue::Float(result).to_display_string()))
            }
            _ => Ok(UiValue::Void),
        }
    }
}

// ── Dynamic AST Widget Tree Nodes ──────────────────────────────────────────

/// A declarative dynamic UI widget node constructed from parsed AST expressions.
#[derive(Debug, Clone)]
pub enum DynamicNode {
    Flex {
        direction: FlexDirection,
        gap: f64,
        padding: f64,
        cross_align: CrossAxisAlignment,
        main_align: MainAxisAlignment,
        children: Vec<DynamicNode>,
    },
    Card {
        padding: f64,
        background: Color,
        border_color: Option<Color>,
        corner_radius: f64,
        child: Option<Box<DynamicNode>>,
    },
    Label {
        text_expr: Expr,
        font_size: f64,
        weight: FontWeight,
        color: Color,
        align: TextAlign,
    },
    Button {
        key: String,
        label_expr: Expr,
        background: Color,
        text_color: Color,
        corner_radius: f64,
        on_click_expr: Option<Expr>,
    },
}

impl DynamicNode {
    /// Render the dynamic widget into the Vello vector scene.
    pub fn render(
        &self,
        bounds: Rect,
        font_ctx: &FontContext,
        builder: &mut SceneBuilder,
        runtime: &UiRuntime,
        hovered_key: Option<&str>,
        pressed_key: Option<&str>,
    ) {
        match self {
            DynamicNode::Flex {
                direction,
                gap,
                padding,
                cross_align: _,
                main_align: _,
                children,
            } => {
                let inner_x = bounds.origin.x + padding;
                let inner_y = bounds.origin.y + padding;
                let inner_w = (bounds.width - padding * 2.0).max(0.0);
                let inner_h = (bounds.height - padding * 2.0).max(0.0);

                let n = children.len();
                if n == 0 {
                    return;
                }

                match direction {
                    FlexDirection::Column => {
                        let total_gap = gap * (n as f64 - 1.0).max(0.0);
                        let child_h = (inner_h - total_gap) / (n as f64);
                        for (i, child) in children.iter().enumerate() {
                            let cy = inner_y + (i as f64) * (child_h + gap);
                            let child_bounds = Rect::new(inner_x, cy, inner_w, child_h);
                            child.render(
                                child_bounds,
                                font_ctx,
                                builder,
                                runtime,
                                hovered_key,
                                pressed_key,
                            );
                        }
                    }
                    FlexDirection::Row => {
                        let total_gap = gap * (n as f64 - 1.0).max(0.0);
                        let child_w = (inner_w - total_gap) / (n as f64);
                        for (i, child) in children.iter().enumerate() {
                            let cx = inner_x + (i as f64) * (child_w + gap);
                            let child_bounds = Rect::new(cx, inner_y, child_w, inner_h);
                            child.render(
                                child_bounds,
                                font_ctx,
                                builder,
                                runtime,
                                hovered_key,
                                pressed_key,
                            );
                        }
                    }
                }
            }

            DynamicNode::Card {
                padding,
                background,
                border_color,
                corner_radius,
                child,
            } => {
                builder.fill_rounded_rect(bounds, *corner_radius, *background);
                if let Some(bc) = border_color {
                    builder.stroke_rect(bounds, *bc, 1.0);
                }
                if let Some(ch) = child {
                    let inner = Rect::new(
                        bounds.origin.x + padding,
                        bounds.origin.y + padding,
                        (bounds.width - padding * 2.0).max(0.0),
                        (bounds.height - padding * 2.0).max(0.0),
                    );
                    ch.render(inner, font_ctx, builder, runtime, hovered_key, pressed_key);
                }
            }

            DynamicNode::Label {
                text_expr,
                font_size,
                weight,
                color,
                align,
            } => {
                let text = match &text_expr.kind {
                    ExprKind::StringLiteral(s) => s.clone(),
                    ExprKind::Identifier(id) => runtime
                        .state
                        .get(&id.name)
                        .map(|v| v.to_display_string())
                        .unwrap_or_else(|| id.name.clone()),
                    _ => {
                        let mut temp_rt = runtime.clone();
                        temp_rt
                            .eval_expr(text_expr)
                            .map(|v| v.to_display_string())
                            .unwrap_or_default()
                    }
                };
                draw_text(
                    builder, font_ctx, &text, *font_size, *weight, *align, bounds, *color,
                );
            }

            DynamicNode::Button {
                key,
                label_expr,
                background,
                text_color,
                corner_radius,
                on_click_expr: _,
            } => {
                let label = match &label_expr.kind {
                    ExprKind::StringLiteral(s) => s.clone(),
                    _ => {
                        let mut temp_rt = runtime.clone();
                        temp_rt
                            .eval_expr(label_expr)
                            .map(|v| v.to_display_string())
                            .unwrap_or_default()
                    }
                };

                let is_hovered = hovered_key == Some(key.as_str());
                let is_pressed = pressed_key == Some(key.as_str());

                let bg = if is_pressed {
                    Color::rgba(
                        background.r.saturating_sub(20),
                        background.g.saturating_sub(20),
                        background.b.saturating_sub(20),
                        background.a,
                    )
                } else if is_hovered {
                    Color::rgba(
                        background.r.saturating_add(30),
                        background.g.saturating_add(30),
                        background.b.saturating_add(30),
                        background.a,
                    )
                } else {
                    *background
                };

                builder.fill_rounded_rect(bounds, *corner_radius, bg);
                builder.stroke_rect(bounds, Color::rgba(255, 255, 255, 20), 0.5);

                draw_text(
                    builder,
                    font_ctx,
                    &label,
                    15.0,
                    FontWeight::Medium,
                    TextAlign::Center,
                    bounds,
                    *text_color,
                );
            }
        }
    }

    /// Perform recursive hit-testing to find any button under the mouse pointer.
    pub fn hit_test(
        &self,
        p: Point,
        bounds: Rect,
        runtime: &UiRuntime,
    ) -> Option<(String, Option<Expr>)> {
        if !bounds.contains(p) {
            return None;
        }

        match self {
            DynamicNode::Flex {
                direction,
                gap,
                padding,
                cross_align: _,
                main_align: _,
                children,
            } => {
                let inner_x = bounds.origin.x + padding;
                let inner_y = bounds.origin.y + padding;
                let inner_w = (bounds.width - padding * 2.0).max(0.0);
                let inner_h = (bounds.height - padding * 2.0).max(0.0);
                let n = children.len();
                if n == 0 {
                    return None;
                }

                match direction {
                    FlexDirection::Column => {
                        let total_gap = gap * (n as f64 - 1.0).max(0.0);
                        let child_h = (inner_h - total_gap) / (n as f64);
                        for (i, child) in children.iter().enumerate() {
                            let cy = inner_y + (i as f64) * (child_h + gap);
                            let child_bounds = Rect::new(inner_x, cy, inner_w, child_h);
                            if let Some(hit) = child.hit_test(p, child_bounds, runtime) {
                                return Some(hit);
                            }
                        }
                    }
                    FlexDirection::Row => {
                        let total_gap = gap * (n as f64 - 1.0).max(0.0);
                        let child_w = (inner_w - total_gap) / (n as f64);
                        for (i, child) in children.iter().enumerate() {
                            let cx = inner_x + (i as f64) * (child_w + gap);
                            let child_bounds = Rect::new(cx, inner_y, child_w, inner_h);
                            if let Some(hit) = child.hit_test(p, child_bounds, runtime) {
                                return Some(hit);
                            }
                        }
                    }
                }
                None
            }

            DynamicNode::Card { padding, child, .. } => {
                if let Some(ch) = child {
                    let inner = Rect::new(
                        bounds.origin.x + padding,
                        bounds.origin.y + padding,
                        (bounds.width - padding * 2.0).max(0.0),
                        (bounds.height - padding * 2.0).max(0.0),
                    );
                    ch.hit_test(p, inner, runtime)
                } else {
                    None
                }
            }

            DynamicNode::Button {
                key, on_click_expr, ..
            } => Some((key.clone(), on_click_expr.clone())),

            DynamicNode::Label { .. } => None,
        }
    }
}

// ── Property Helpers ────────────────────────────────────────────────────────

fn extract_string(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::StringLiteral(s) => Some(s.clone()),
        ExprKind::Identifier(id) => Some(id.name.clone()),
        _ => None,
    }
}

fn extract_f64(expr: &Expr) -> Option<f64> {
    match &expr.kind {
        ExprKind::FloatLiteral(f) => Some(*f),
        ExprKind::IntLiteral(i) => Some(*i as f64),
        _ => None,
    }
}

fn extract_color(expr: &Expr) -> Option<Color> {
    if let Some(s) = extract_string(expr) {
        Color::from_hex(&s).ok()
    } else {
        None
    }
}

fn extract_font_weight(expr: &Expr) -> Option<FontWeight> {
    if let Some(s) = extract_string(expr) {
        match s.to_lowercase().as_str() {
            "bold" => Some(FontWeight::Bold),
            "semibold" => Some(FontWeight::SemiBold),
            "medium" => Some(FontWeight::Medium),
            "light" => Some(FontWeight::Light),
            _ => Some(FontWeight::Regular),
        }
    } else {
        None
    }
}

fn extract_size_tuple(expr: &Expr) -> Option<(u32, u32)> {
    if let ExprKind::TupleLiteral(elems) = &expr.kind {
        if elems.len() >= 2 {
            let w = extract_f64(&elems[0]).unwrap_or(440.0) as u32;
            let h = extract_f64(&elems[1]).unwrap_or(620.0) as u32;
            return Some((w, h));
        }
    }
    None
}

// ── UiEvaluator — AST to Dynamic Widget Tree Builder ────────────────────────

/// Walks parsed Agam AST modules and compiles declarative widget trees.
#[derive(Debug, Default, Clone)]
pub struct UiEvaluator;

impl UiEvaluator {
    /// Create a new UI AST evaluator.
    pub fn new() -> Self {
        Self
    }

    /// Build an executable dynamic GUI application from the parsed `Module` AST.
    pub fn build_app(&self, module: &Module) -> GuiResult<(WindowConfig, DynamicGuiApp)> {
        let mut runtime = UiRuntime::new();
        let mut root_expr: Option<&Expr> = None;

        // Locate the main UI function returning a Window struct
        for decl in &module.declarations {
            if let DeclKind::Function(ref f) = decl.kind {
                if let Some(ref body) = f.body {
                    for stmt in &body.stmts {
                        if let StmtKind::Let { .. } = &stmt.kind {
                            runtime.eval_stmt(stmt)?;
                        }
                        if let StmtKind::Return(Some(expr)) = &stmt.kind {
                            root_expr = Some(expr);
                        }
                    }
                    if root_expr.is_none() {
                        if let Some(ref tail) = body.expr {
                            root_expr = Some(tail.as_ref());
                        }
                    }
                }
            }
        }

        let root_expr = root_expr
            .ok_or_else(|| eval_error("No UI Window return expression found in module"))?;

        let (config, root_node) = self.eval_window(root_expr)?;
        let shared_runtime = Arc::new(Mutex::new(runtime));
        let app = DynamicGuiApp::new(config.clone(), shared_runtime, root_node);

        Ok((config, app))
    }

    fn eval_window(&self, expr: &Expr) -> GuiResult<(WindowConfig, DynamicNode)> {
        let ExprKind::StructLiteral { path, fields } = &expr.kind else {
            return Err(eval_error(
                "Root UI expression must be a Window struct literal",
            ));
        };

        let struct_name = path
            .segments
            .last()
            .map(|id| id.name.as_str())
            .unwrap_or_default();
        if struct_name != "Window" {
            return Err(eval_error(format!(
                "Expected Window struct, found {struct_name}"
            )));
        }

        let mut title = "Agam Native Window".to_string();
        let mut dimensions = (440, 620);
        let mut child_node = None;

        for FieldInit { name, value, .. } in fields {
            match name.name.as_str() {
                "title" => {
                    if let Some(t) = extract_string(value) {
                        title = t;
                    }
                }
                "size" => {
                    if let Some(sz) = extract_size_tuple(value) {
                        dimensions = sz;
                    }
                }
                "child" => {
                    child_node = Some(self.eval_node(value)?);
                }
                _ => {}
            }
        }

        let child = child_node.unwrap_or_else(|| DynamicNode::Flex {
            direction: FlexDirection::Column,
            gap: 0.0,
            padding: 0.0,
            cross_align: CrossAxisAlignment::Center,
            main_align: MainAxisAlignment::Start,
            children: Vec::new(),
        });

        let config = WindowConfig::new(title, dimensions.0, dimensions.1);
        Ok((config, child))
    }

    fn eval_node(&self, expr: &Expr) -> GuiResult<DynamicNode> {
        let ExprKind::StructLiteral { path, fields } = &expr.kind else {
            return Err(eval_error(format!(
                "Expected Widget struct literal, found {:?}",
                expr.kind
            )));
        };

        let struct_name = path
            .segments
            .last()
            .map(|id| id.name.as_str())
            .unwrap_or_default();

        match struct_name {
            "Column" | "Row" | "Flex" => {
                let direction = if struct_name == "Row" {
                    FlexDirection::Row
                } else {
                    FlexDirection::Column
                };
                let mut gap = 12.0;
                let mut padding = 0.0;
                let mut cross_align = CrossAxisAlignment::Center;
                let mut main_align = MainAxisAlignment::Start;
                let mut children = Vec::new();

                for FieldInit { name, value, .. } in fields {
                    match name.name.as_str() {
                        "gap" => gap = extract_f64(value).unwrap_or(gap),
                        "padding" => padding = extract_f64(value).unwrap_or(padding),
                        "cross_align" => {
                            if let Some(s) = extract_string(value) {
                                cross_align = match s.as_str() {
                                    "Start" => CrossAxisAlignment::Start,
                                    "End" => CrossAxisAlignment::End,
                                    "Stretch" => CrossAxisAlignment::Stretch,
                                    _ => CrossAxisAlignment::Center,
                                };
                            }
                        }
                        "main_align" => {
                            if let Some(s) = extract_string(value) {
                                main_align = match s.as_str() {
                                    "Center" => MainAxisAlignment::Center,
                                    "End" => MainAxisAlignment::End,
                                    "SpaceBetween" => MainAxisAlignment::SpaceBetween,
                                    _ => MainAxisAlignment::Start,
                                };
                            }
                        }
                        "children" => {
                            if let ExprKind::ArrayLiteral(elems) = &value.kind {
                                for elem in elems {
                                    children.push(self.eval_node(elem)?);
                                }
                            }
                        }
                        _ => {}
                    }
                }

                Ok(DynamicNode::Flex {
                    direction,
                    gap,
                    padding,
                    cross_align,
                    main_align,
                    children,
                })
            }

            "Card" => {
                let mut padding = 16.0;
                let mut background = Color::rgb(37, 37, 37);
                let mut border_color = None;
                let corner_radius = 8.0;
                let mut child = None;

                for FieldInit { name, value, .. } in fields {
                    match name.name.as_str() {
                        "padding" => padding = extract_f64(value).unwrap_or(padding),
                        "background" => background = extract_color(value).unwrap_or(background),
                        "border_color" => border_color = extract_color(value),
                        "child" => child = Some(Box::new(self.eval_node(value)?)),
                        _ => {}
                    }
                }

                Ok(DynamicNode::Card {
                    padding,
                    background,
                    border_color,
                    corner_radius,
                    child,
                })
            }

            "Label" => {
                let mut text_expr = None;
                let mut font_size = 14.0;
                let mut weight = FontWeight::Regular;
                let mut color = Color::WHITE;
                let mut align = TextAlign::Left;

                for FieldInit { name, value, .. } in fields {
                    match name.name.as_str() {
                        "text" => text_expr = Some(value.clone()),
                        "size" => font_size = extract_f64(value).unwrap_or(font_size),
                        "weight" => weight = extract_font_weight(value).unwrap_or(weight),
                        "color" => color = extract_color(value).unwrap_or(color),
                        "align" => {
                            if let Some(s) = extract_string(value) {
                                align = match s.as_str() {
                                    "Center" => TextAlign::Center,
                                    "Right" => TextAlign::Right,
                                    _ => TextAlign::Left,
                                };
                            }
                        }
                        _ => {}
                    }
                }

                let text_expr = text_expr.unwrap_or_else(|| Expr {
                    id: agam_ast::NodeId(0),
                    span: expr.span,
                    kind: ExprKind::StringLiteral(String::new()),
                });

                Ok(DynamicNode::Label {
                    text_expr,
                    font_size,
                    weight,
                    color,
                    align,
                })
            }

            "Button" => {
                let mut label_expr = None;
                let mut background = Color::rgb(0, 120, 212);
                let text_color = Color::WHITE;
                let corner_radius = 6.0;
                let mut on_click_expr = None;

                for FieldInit { name, value, .. } in fields {
                    match name.name.as_str() {
                        "label" => label_expr = Some(value.clone()),
                        "background" => background = extract_color(value).unwrap_or(background),
                        "on_click" => {
                            if let ExprKind::Lambda { body, .. } = &value.kind {
                                on_click_expr = Some(*body.clone());
                            } else {
                                on_click_expr = Some(value.clone());
                            }
                        }
                        _ => {}
                    }
                }

                let label_expr = label_expr.unwrap_or_else(|| Expr {
                    id: agam_ast::NodeId(0),
                    span: expr.span,
                    kind: ExprKind::StringLiteral("Button".to_string()),
                });

                let key =
                    extract_string(&label_expr).unwrap_or_else(|| format!("btn_{:?}", expr.span));

                Ok(DynamicNode::Button {
                    key,
                    label_expr,
                    background,
                    text_color,
                    corner_radius,
                    on_click_expr,
                })
            }

            _ => Err(eval_error(format!(
                "Unknown GUI widget type: {struct_name}"
            ))),
        }
    }
}

// ── DynamicGuiApp — Native Platform GuiApp Runner ──────────────────────────

/// Native GPU application instance driven entirely by dynamically parsed AST declarative trees.
pub struct DynamicGuiApp {
    pub config: WindowConfig,
    pub runtime: Arc<Mutex<UiRuntime>>,
    pub root_node: DynamicNode,
    pub font_context: FontContext,
    gpu_context: Option<GpuContext>,
    surface: Option<GpuSurface>,
    renderer: Option<SceneRenderer>,
    cursor: Point,
    hovered_key: Option<String>,
    pressed_key: Option<String>,
    dimensions: (u32, u32),
}

impl DynamicGuiApp {
    /// Create a new dynamic GUI application.
    pub fn new(
        config: WindowConfig,
        runtime: Arc<Mutex<UiRuntime>>,
        root_node: DynamicNode,
    ) -> Self {
        let (w, h) = (config.width, config.height);
        Self {
            config,
            runtime,
            root_node,
            font_context: FontContext::default(),
            gpu_context: None,
            surface: None,
            renderer: None,
            cursor: Point::ZERO,
            hovered_key: None,
            pressed_key: None,
            dimensions: (w, h),
        }
    }
}

impl GuiApp for DynamicGuiApp {
    fn on_event(&mut self, window: &mut GuiWindow, event: GuiEvent) -> GuiResult<()> {
        if self.gpu_context.is_none() {
            let context = GpuContext::new()?;
            let surface = context.create_surface(window)?;
            let renderer = SceneRenderer::new(&context)?;
            self.gpu_context = Some(context);
            self.surface = Some(surface);
            self.renderer = Some(renderer);
        }

        match event {
            GuiEvent::PointerMoved { position } => {
                self.cursor = position;
                let (w, h) = self.dimensions;
                let bounds = Rect::new(0.0, 0.0, w as f64, h as f64);
                let rt = self
                    .runtime
                    .lock()
                    .map_err(|_| eval_error("State lock poisoned"))?;
                let new_hover = self
                    .root_node
                    .hit_test(self.cursor, bounds, &rt)
                    .map(|(k, _)| k);

                if new_hover != self.hovered_key {
                    self.hovered_key = new_hover;
                    window.request_redraw();
                }
            }

            GuiEvent::PointerDown { button, position } => {
                if button == MouseButton::Primary {
                    self.cursor = position;
                    let (w, h) = self.dimensions;
                    let bounds = Rect::new(0.0, 0.0, w as f64, h as f64);
                    let (hit_key, on_click) = {
                        let rt = self
                            .runtime
                            .lock()
                            .map_err(|_| eval_error("State lock poisoned"))?;
                        self.root_node.hit_test(self.cursor, bounds, &rt).unzip()
                    };

                    self.pressed_key = hit_key;

                    if let Some(Some(click_expr)) = on_click {
                        let mut rt = self
                            .runtime
                            .lock()
                            .map_err(|_| eval_error("State lock poisoned"))?;
                        let _ = rt.eval_expr(&click_expr);
                        drop(rt);
                        window.request_redraw();
                    }
                }
            }

            GuiEvent::PointerUp { button, .. } => {
                if button == MouseButton::Primary && self.pressed_key.is_some() {
                    self.pressed_key = None;
                    window.request_redraw();
                }
            }

            GuiEvent::Resized { width, height, .. } => {
                self.dimensions = (width, height);
                if let Some(ref mut surface) = self.surface {
                    let _ = surface.resize(width, height);
                }
                window.request_redraw();
            }

            GuiEvent::RedrawRequested => {
                let (Some(context), Some(surface), Some(renderer)) =
                    (&self.gpu_context, &mut self.surface, &mut self.renderer)
                else {
                    return Ok(());
                };

                let (win_w, win_h) = self.dimensions;
                let w = win_w as f64;
                let h = win_h as f64;
                let mut builder = SceneBuilder::new();

                // Background canvas fill
                builder.fill_rect(Rect::new(0.0, 0.0, w, h), Color::rgb(24, 24, 24));
                builder.fill_rect(Rect::new(0.0, 0.0, w, 3.0), Color::rgb(0, 120, 212));

                // Render dynamic widget tree
                let bounds = Rect::new(0.0, 0.0, w, h);
                {
                    let rt = self
                        .runtime
                        .lock()
                        .map_err(|_| eval_error("State lock poisoned"))?;
                    self.root_node.render(
                        bounds,
                        &self.font_context,
                        &mut builder,
                        &rt,
                        self.hovered_key.as_deref(),
                        self.pressed_key.as_deref(),
                    );
                }

                let frame = surface.acquire_frame()?;
                renderer.render_to_frame(context, &builder, &frame, Color::rgb(24, 24, 24))?;
                frame.present();
            }

            _ => {}
        }

        Ok(())
    }
}

// ── Unit Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use agam_errors::SourceId;
    use agam_lexer::tokenize;

    #[test]
    fn test_ui_value_operations() {
        let mut rt = UiRuntime::new();
        rt.state.insert("count".to_string(), UiValue::Int(42));
        if let Some(val) = rt.state.get("count") {
            assert_eq!(val.to_display_string(), "42");
        }

        if let Ok(res) = rt.eval_builtin_function(
            "evaluate",
            &[
                UiValue::Float(10.0),
                UiValue::Str("+".to_string()),
                UiValue::Float(5.5),
            ],
        ) {
            assert_eq!(res.to_display_string(), "15.5");
        }
    }

    #[test]
    fn test_dynamic_counter_ast_evaluation() {
        let src = r##"
@lang.advance
@ui
fn counter_app() -> Window {
    let mut count: i32 = 0;
    return Window {
        title: "Dynamic Counter",
        size: (360, 240),
        child: Column {
            gap: 16,
            padding: 24,
            children: [
                Card {
                    padding: 16,
                    background: "#252525",
                    child: Label { text: count, size: 36, weight: "Bold", color: "#FFFFFF" }
                },
                Row {
                    gap: 12,
                    children: [
                        Button { label: "Increment", on_click: || { count += 1; } }
                    ]
                }
            ]
        }
    };
}
"##;
        let tokens = tokenize(src, SourceId(0));
        let module_res = agam_parser::parse(tokens, SourceId(0));
        assert!(module_res.is_ok());
        let module = module_res.unwrap_or_else(|_| unreachable!());
        let app_res = UiEvaluator::new().build_app(&module);
        assert!(app_res.is_ok());
        let (config, app) = app_res.unwrap_or_else(|_| unreachable!());

        assert_eq!(config.title, "Dynamic Counter");
        assert_eq!(config.width, 360);
        assert_eq!(config.height, 240);

        // Initial state
        if let Ok(rt) = app.runtime.lock() {
            if let Some(val) = rt.state.get("count") {
                assert_eq!(val.to_display_string(), "0");
            }
        }

        // Simulate click on increment button
        if let Ok(rt) = app.runtime.lock() {
            let hit = app.root_node.hit_test(
                Point::new(100.0, 150.0),
                Rect::new(0.0, 0.0, 360.0, 240.0),
                &rt,
            );
            assert!(hit.is_some());
            if let Some((key, Some(on_click))) = hit {
                assert_eq!(key, "Increment");
                drop(rt);
                if let Ok(mut rt) = app.runtime.lock() {
                    let _ = rt.eval_expr(&on_click);
                    if let Some(val) = rt.state.get("count") {
                        assert_eq!(val.to_display_string(), "1");
                    }
                }
            }
        }
    }

    #[test]
    fn test_dynamic_calculator_ast_evaluation() {
        let src = r##"
@lang.advance
@ui
fn calculator_app() -> Window {
    let mut display: String = "0";
    let mut start_new: bool = true;
    return Window {
        title: "Dynamic Calculator",
        size: (440, 620),
        child: Column {
            gap: 12,
            padding: 20,
            children: [
                Label { text: display, size: 40, weight: "Bold", color: "#FFFFFF" },
                Button { label: "7", on_click: || { display = append_digit(display, "7", start_new); start_new = false; } }
            ]
        }
    };
}
"##;
        let tokens = tokenize(src, SourceId(0));
        let module_res = agam_parser::parse(tokens, SourceId(0));
        assert!(module_res.is_ok());
        let module = module_res.unwrap_or_else(|_| unreachable!());
        let app_res = UiEvaluator::new().build_app(&module);
        assert!(app_res.is_ok());
        let (config, app) = app_res.unwrap_or_else(|_| unreachable!());

        assert_eq!(config.title, "Dynamic Calculator");
        assert_eq!(config.width, 440);
        assert_eq!(config.height, 620);

        if let Ok(rt) = app.runtime.lock() {
            let hit = app.root_node.hit_test(
                Point::new(100.0, 350.0),
                Rect::new(0.0, 0.0, 440.0, 620.0),
                &rt,
            );
            assert!(hit.is_some());
            if let Some((key, Some(on_click))) = hit {
                assert_eq!(key, "7");
                drop(rt);
                if let Ok(mut rt) = app.runtime.lock() {
                    let _ = rt.eval_expr(&on_click);
                    if let Some(val) = rt.state.get("display") {
                        assert_eq!(val.to_display_string(), "7");
                    }
                }
            }
        }
    }
}
