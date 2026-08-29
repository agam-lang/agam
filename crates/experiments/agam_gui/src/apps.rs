//! # Built-in Native GUI Applications (`CalculatorApp`, `CounterApp`)
//!
//! Provides production-grade native GPU vector applications adhering to Fluent Dark aesthetics.

use crate::diagnostic::GuiResult;
use crate::gpu::{GpuContext, GpuSurface};
use crate::input::{GuiEvent, Key, MouseButton};
use crate::platform::{GuiApp, GuiWindow};
use crate::scene::{Color, Point, Rect, SceneBuilder, SceneRenderer};
use crate::text::FontContext;

// ── Calculator Engine ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl Operator {
    pub fn symbol(&self) -> char {
        match self {
            Self::Add => '+',
            Self::Subtract => '−',
            Self::Multiply => '×',
            Self::Divide => '÷',
        }
    }

    pub fn apply(&self, lhs: f64, rhs: f64) -> Result<f64, &'static str> {
        match self {
            Self::Add => Ok(lhs + rhs),
            Self::Subtract => Ok(lhs - rhs),
            Self::Multiply => Ok(lhs * rhs),
            Self::Divide => {
                if rhs.abs() < 1e-12 {
                    Err("Error")
                } else {
                    Ok(lhs / rhs)
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct CalculatorEngine {
    pub display: String,
    pub history: String,
    pub stored_value: Option<f64>,
    pub pending_op: Option<Operator>,
    pub start_new_number: bool,
    pub has_error: bool,
}

impl Default for CalculatorEngine {
    fn default() -> Self {
        Self {
            display: "0".to_string(),
            history: String::new(),
            stored_value: None,
            pending_op: None,
            start_new_number: true,
            has_error: false,
        }
    }
}

impl CalculatorEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn input_digit(&mut self, digit: char) {
        if self.has_error || self.start_new_number {
            self.display = digit.to_string();
            self.start_new_number = false;
            self.has_error = false;
        } else if self.display == "0" {
            self.display = digit.to_string();
        } else if self.display == "-0" {
            self.display = format!("-{digit}");
        } else if self.display.len() < 14 {
            self.display.push(digit);
        }
    }

    pub fn input_decimal(&mut self) {
        if self.has_error || self.start_new_number {
            self.display = "0.".to_string();
            self.start_new_number = false;
            self.has_error = false;
        } else if !self.display.contains('.') && self.display.len() < 13 {
            self.display.push('.');
        }
    }

    pub fn input_operator(&mut self, op: Operator) {
        if self.has_error {
            return;
        }
        let current: f64 = self.display.parse().unwrap_or(0.0);
        if let (Some(prev), Some(pending)) = (self.stored_value, self.pending_op) {
            if !self.start_new_number {
                match pending.apply(prev, current) {
                    Ok(res) => {
                        self.stored_value = Some(res);
                        self.display = format_number(res);
                    }
                    Err(_) => {
                        self.display = "Error".to_string();
                        self.has_error = true;
                        self.stored_value = None;
                        self.pending_op = None;
                        return;
                    }
                }
            }
        } else {
            self.stored_value = Some(current);
        }

        self.pending_op = Some(op);
        self.start_new_number = true;
        self.history = format!(
            "{} {}",
            format_number(self.stored_value.unwrap_or(current)),
            op.symbol()
        );
    }

    pub fn calculate_equals(&mut self) {
        if self.has_error {
            return;
        }
        let current: f64 = self.display.parse().unwrap_or(0.0);
        if let (Some(prev), Some(pending)) = (self.stored_value, self.pending_op) {
            self.history = format!(
                "{} {} {} =",
                format_number(prev),
                pending.symbol(),
                format_number(current)
            );
            match pending.apply(prev, current) {
                Ok(res) => {
                    self.display = format_number(res);
                    self.stored_value = None;
                    self.pending_op = None;
                    self.start_new_number = true;
                }
                Err(_) => {
                    self.display = "Error".to_string();
                    self.has_error = true;
                    self.stored_value = None;
                    self.pending_op = None;
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.display = "0".to_string();
        self.history.clear();
        self.stored_value = None;
        self.pending_op = None;
        self.start_new_number = true;
        self.has_error = false;
    }

    pub fn clear_entry(&mut self) {
        self.display = "0".to_string();
        self.start_new_number = true;
        self.has_error = false;
    }

    pub fn backspace(&mut self) {
        if self.has_error || self.start_new_number {
            return;
        }
        self.display.pop();
        if self.display.is_empty() || self.display == "-" {
            self.display = "0".to_string();
            self.start_new_number = true;
        }
    }

    pub fn toggle_sign(&mut self) {
        if self.has_error {
            return;
        }
        if self.start_new_number || self.display == "0" {
            self.display = "-0".to_string();
            self.start_new_number = false;
            return;
        }
        if self.display.starts_with('-') {
            self.display.remove(0);
        } else {
            self.display.insert(0, '-');
        }
    }

    pub fn percentage(&mut self) {
        if self.has_error {
            return;
        }
        let val: f64 = self.display.parse().unwrap_or(0.0);
        let res = val / 100.0;
        self.display = format_number(res);
        self.start_new_number = true;
    }
}

fn format_number(val: f64) -> String {
    if val.is_infinite() || val.is_nan() {
        return "Error".to_string();
    }
    let val = if val.abs() < 1e-12 { 0.0 } else { val };

    if val.abs() >= 1e15 || (val.abs() > 0.0 && val.abs() < 1e-6) {
        let s = format!("{:.6e}", val);
        return s.replace("e+", "e");
    }

    if val.fract().abs() < 1e-10 {
        let s = format!("{:.0}", val);
        if s == "-0" { "0".to_string() } else { s }
    } else {
        let s = format!("{:.8}", val);
        let trimmed = s.trim_end_matches('0').trim_end_matches('.');
        if trimmed == "-0" || trimmed.is_empty() {
            "0".to_string()
        } else {
            trimmed.to_string()
        }
    }
}

// ── Calculator Buttons ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonAction {
    Digit(char),
    Op(Operator),
    Equals,
    Decimal,
    Clear,
    ClearEntry,
    Backspace,
    ToggleSign,
    Percent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonStyle {
    Number,
    Operator,
    Accent,
    Action,
}

#[derive(Debug, Clone)]
pub struct CalcButton {
    pub label: &'static str,
    pub action: ButtonAction,
    pub style: ButtonStyle,
    pub col: usize,
    pub row: usize,
}

const BUTTONS: &[CalcButton] = &[
    CalcButton {
        label: "C",
        action: ButtonAction::Clear,
        style: ButtonStyle::Action,
        col: 0,
        row: 0,
    },
    CalcButton {
        label: "CE",
        action: ButtonAction::ClearEntry,
        style: ButtonStyle::Action,
        col: 1,
        row: 0,
    },
    CalcButton {
        label: "%",
        action: ButtonAction::Percent,
        style: ButtonStyle::Action,
        col: 2,
        row: 0,
    },
    CalcButton {
        label: "÷",
        action: ButtonAction::Op(Operator::Divide),
        style: ButtonStyle::Operator,
        col: 3,
        row: 0,
    },
    CalcButton {
        label: "7",
        action: ButtonAction::Digit('7'),
        style: ButtonStyle::Number,
        col: 0,
        row: 1,
    },
    CalcButton {
        label: "8",
        action: ButtonAction::Digit('8'),
        style: ButtonStyle::Number,
        col: 1,
        row: 1,
    },
    CalcButton {
        label: "9",
        action: ButtonAction::Digit('9'),
        style: ButtonStyle::Number,
        col: 2,
        row: 1,
    },
    CalcButton {
        label: "×",
        action: ButtonAction::Op(Operator::Multiply),
        style: ButtonStyle::Operator,
        col: 3,
        row: 1,
    },
    CalcButton {
        label: "4",
        action: ButtonAction::Digit('4'),
        style: ButtonStyle::Number,
        col: 0,
        row: 2,
    },
    CalcButton {
        label: "5",
        action: ButtonAction::Digit('5'),
        style: ButtonStyle::Number,
        col: 1,
        row: 2,
    },
    CalcButton {
        label: "6",
        action: ButtonAction::Digit('6'),
        style: ButtonStyle::Number,
        col: 2,
        row: 2,
    },
    CalcButton {
        label: "−",
        action: ButtonAction::Op(Operator::Subtract),
        style: ButtonStyle::Operator,
        col: 3,
        row: 2,
    },
    CalcButton {
        label: "1",
        action: ButtonAction::Digit('1'),
        style: ButtonStyle::Number,
        col: 0,
        row: 3,
    },
    CalcButton {
        label: "2",
        action: ButtonAction::Digit('2'),
        style: ButtonStyle::Number,
        col: 1,
        row: 3,
    },
    CalcButton {
        label: "3",
        action: ButtonAction::Digit('3'),
        style: ButtonStyle::Number,
        col: 2,
        row: 3,
    },
    CalcButton {
        label: "+",
        action: ButtonAction::Op(Operator::Add),
        style: ButtonStyle::Operator,
        col: 3,
        row: 3,
    },
    CalcButton {
        label: "±",
        action: ButtonAction::ToggleSign,
        style: ButtonStyle::Action,
        col: 0,
        row: 4,
    },
    CalcButton {
        label: "0",
        action: ButtonAction::Digit('0'),
        style: ButtonStyle::Number,
        col: 1,
        row: 4,
    },
    CalcButton {
        label: ".",
        action: ButtonAction::Decimal,
        style: ButtonStyle::Number,
        col: 2,
        row: 4,
    },
    CalcButton {
        label: "=",
        action: ButtonAction::Equals,
        style: ButtonStyle::Accent,
        col: 3,
        row: 4,
    },
];

// ── Complete High-Fidelity Vector Typography (A-Z, 0-9, Math Symbols) ──────

#[allow(clippy::too_many_arguments)]
pub fn draw_vector_char(
    builder: &mut SceneBuilder,
    c: char,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    stroke_w: f64,
    color: Color,
) {
    let pad_x = w * 0.14;
    let pad_y = h * 0.14;
    let rx = x + pad_x;
    let ry = y + pad_y;
    let rw = (w - pad_x * 2.0).max(stroke_w);
    let rh = (h - pad_y * 2.0).max(stroke_w);
    let mid_y = ry + rh / 2.0;
    let mid_x = rx + rw / 2.0;

    let draw_bar = |b: &mut SceneBuilder, bx: f64, by: f64, bw: f64, bh: f64| {
        let r = (stroke_w / 2.0).min(bw / 2.0).min(bh / 2.0);
        b.fill_rounded_rect(
            Rect::new(bx, by, bw.max(stroke_w), bh.max(stroke_w)),
            r,
            color,
        );
    };

    match c {
        '0' => {
            draw_bar(builder, rx, ry, rw, stroke_w);
            draw_bar(builder, rx, ry, stroke_w, rh);
            draw_bar(builder, rx + rw - stroke_w, ry, stroke_w, rh);
            draw_bar(builder, rx, ry + rh - stroke_w, rw, stroke_w);
        }
        '1' => {
            draw_bar(builder, mid_x - stroke_w / 2.0, ry, stroke_w, rh);
            draw_bar(builder, rx + rw * 0.15, ry + rh * 0.2, rw * 0.35, stroke_w);
            draw_bar(
                builder,
                rx + rw * 0.15,
                ry + rh - stroke_w,
                rw * 0.7,
                stroke_w,
            );
        }
        '2' => {
            draw_bar(builder, rx, ry, rw, stroke_w);
            draw_bar(builder, rx + rw - stroke_w, ry, stroke_w, rh / 2.0);
            draw_bar(builder, rx, mid_y - stroke_w / 2.0, rw, stroke_w);
            draw_bar(builder, rx, mid_y, stroke_w, rh / 2.0);
            draw_bar(builder, rx, ry + rh - stroke_w, rw, stroke_w);
        }
        '3' => {
            draw_bar(builder, rx, ry, rw, stroke_w);
            draw_bar(builder, rx + rw - stroke_w, ry, stroke_w, rh);
            draw_bar(
                builder,
                rx + rw * 0.2,
                mid_y - stroke_w / 2.0,
                rw * 0.8,
                stroke_w,
            );
            draw_bar(builder, rx, ry + rh - stroke_w, rw, stroke_w);
        }
        '4' => {
            draw_bar(builder, rx, ry, stroke_w, rh / 2.0);
            draw_bar(builder, rx, mid_y - stroke_w / 2.0, rw, stroke_w);
            draw_bar(builder, rx + rw * 0.75 - stroke_w, ry, stroke_w, rh);
        }
        '5' => {
            draw_bar(builder, rx, ry, rw, stroke_w);
            draw_bar(builder, rx, ry, stroke_w, rh / 2.0);
            draw_bar(builder, rx, mid_y - stroke_w / 2.0, rw, stroke_w);
            draw_bar(builder, rx + rw - stroke_w, mid_y, stroke_w, rh / 2.0);
            draw_bar(builder, rx, ry + rh - stroke_w, rw, stroke_w);
        }
        '6' => {
            draw_bar(builder, rx, ry, rw, stroke_w);
            draw_bar(builder, rx, ry, stroke_w, rh);
            draw_bar(builder, rx, mid_y - stroke_w / 2.0, rw, stroke_w);
            draw_bar(builder, rx + rw - stroke_w, mid_y, stroke_w, rh / 2.0);
            draw_bar(builder, rx, ry + rh - stroke_w, rw, stroke_w);
        }
        '7' => {
            draw_bar(builder, rx, ry, rw, stroke_w);
            draw_bar(builder, rx + rw - stroke_w, ry, stroke_w, rh);
        }
        '8' => {
            draw_bar(builder, rx, ry, rw, stroke_w);
            draw_bar(builder, rx, ry, stroke_w, rh);
            draw_bar(builder, rx + rw - stroke_w, ry, stroke_w, rh);
            draw_bar(builder, rx, mid_y - stroke_w / 2.0, rw, stroke_w);
            draw_bar(builder, rx, ry + rh - stroke_w, rw, stroke_w);
        }
        '9' => {
            draw_bar(builder, rx, ry, rw, stroke_w);
            draw_bar(builder, rx, ry, stroke_w, rh / 2.0);
            draw_bar(builder, rx + rw - stroke_w, ry, stroke_w, rh);
            draw_bar(builder, rx, mid_y - stroke_w / 2.0, rw, stroke_w);
            draw_bar(builder, rx, ry + rh - stroke_w, rw, stroke_w);
        }
        'A' | 'a' => {
            draw_bar(builder, rx, ry, rw, stroke_w);
            draw_bar(builder, rx, ry, stroke_w, rh);
            draw_bar(builder, rx + rw - stroke_w, ry, stroke_w, rh);
            draw_bar(builder, rx, mid_y - stroke_w / 2.0, rw, stroke_w);
        }
        'B' | 'b' => {
            draw_bar(builder, rx, ry, stroke_w, rh);
            draw_bar(builder, rx, ry, rw * 0.8, stroke_w);
            draw_bar(builder, rx, mid_y - stroke_w / 2.0, rw * 0.8, stroke_w);
            draw_bar(builder, rx, ry + rh - stroke_w, rw * 0.8, stroke_w);
            draw_bar(builder, rx + rw - stroke_w, ry, stroke_w, rh / 2.0);
            draw_bar(builder, rx + rw - stroke_w, mid_y, stroke_w, rh / 2.0);
        }
        'C' | 'c' => {
            draw_bar(builder, rx, ry, rw, stroke_w);
            draw_bar(builder, rx, ry, stroke_w, rh);
            draw_bar(builder, rx, ry + rh - stroke_w, rw, stroke_w);
        }
        'D' | 'd' => {
            draw_bar(builder, rx, ry, stroke_w, rh);
            draw_bar(builder, rx, ry, rw * 0.75, stroke_w);
            draw_bar(builder, rx, ry + rh - stroke_w, rw * 0.75, stroke_w);
            draw_bar(
                builder,
                rx + rw - stroke_w,
                ry + rh * 0.15,
                stroke_w,
                rh * 0.7,
            );
        }
        'E' | 'e' => {
            draw_bar(builder, rx, ry, rw, stroke_w);
            draw_bar(builder, rx, ry, stroke_w, rh);
            draw_bar(builder, rx, mid_y - stroke_w / 2.0, rw * 0.7, stroke_w);
            draw_bar(builder, rx, ry + rh - stroke_w, rw, stroke_w);
        }
        'F' | 'f' => {
            draw_bar(builder, rx, ry, rw, stroke_w);
            draw_bar(builder, rx, ry, stroke_w, rh);
            draw_bar(builder, rx, mid_y - stroke_w / 2.0, rw * 0.7, stroke_w);
        }
        'G' | 'g' => {
            draw_bar(builder, rx, ry, rw, stroke_w);
            draw_bar(builder, rx, ry, stroke_w, rh);
            draw_bar(builder, rx, ry + rh - stroke_w, rw, stroke_w);
            draw_bar(builder, rx + rw - stroke_w, mid_y, stroke_w, rh / 2.0);
            draw_bar(builder, mid_x, mid_y - stroke_w / 2.0, rw / 2.0, stroke_w);
        }
        'H' | 'h' => {
            draw_bar(builder, rx, ry, stroke_w, rh);
            draw_bar(builder, rx + rw - stroke_w, ry, stroke_w, rh);
            draw_bar(builder, rx, mid_y - stroke_w / 2.0, rw, stroke_w);
        }
        'I' | 'i' => {
            draw_bar(builder, mid_x - stroke_w / 2.0, ry, stroke_w, rh);
            draw_bar(builder, rx + rw * 0.2, ry, rw * 0.6, stroke_w);
            draw_bar(
                builder,
                rx + rw * 0.2,
                ry + rh - stroke_w,
                rw * 0.6,
                stroke_w,
            );
        }
        'J' | 'j' => {
            draw_bar(builder, rx + rw - stroke_w, ry, stroke_w, rh);
            draw_bar(builder, rx, ry + rh - stroke_w, rw, stroke_w);
            draw_bar(builder, rx, ry + rh * 0.6, stroke_w, rh * 0.4);
        }
        'K' | 'k' => {
            draw_bar(builder, rx, ry, stroke_w, rh);
            let pts1 = vec![Point::new(rx + stroke_w, mid_y), Point::new(rx + rw, ry)];
            let pts2 = vec![
                Point::new(rx + stroke_w, mid_y),
                Point::new(rx + rw, ry + rh),
            ];
            builder.stroke_polygon(pts1, color, stroke_w);
            builder.stroke_polygon(pts2, color, stroke_w);
        }
        'L' | 'l' => {
            draw_bar(builder, rx, ry, stroke_w, rh);
            draw_bar(builder, rx, ry + rh - stroke_w, rw, stroke_w);
        }
        'M' | 'm' => {
            draw_bar(builder, rx, ry, stroke_w, rh);
            draw_bar(builder, rx + rw - stroke_w, ry, stroke_w, rh);
            draw_bar(builder, rx, ry, rw, stroke_w);
            draw_bar(builder, mid_x - stroke_w / 2.0, ry, stroke_w, rh * 0.65);
        }
        'N' | 'n' => {
            draw_bar(builder, rx, ry, stroke_w, rh);
            draw_bar(builder, rx + rw - stroke_w, ry, stroke_w, rh);
            let pts = vec![
                Point::new(rx + stroke_w, ry),
                Point::new(rx + rw - stroke_w, ry + rh),
            ];
            builder.stroke_polygon(pts, color, stroke_w * 1.1);
        }
        'O' | 'o' => {
            draw_bar(builder, rx, ry, rw, stroke_w);
            draw_bar(builder, rx, ry, stroke_w, rh);
            draw_bar(builder, rx + rw - stroke_w, ry, stroke_w, rh);
            draw_bar(builder, rx, ry + rh - stroke_w, rw, stroke_w);
        }
        'P' | 'p' => {
            draw_bar(builder, rx, ry, stroke_w, rh);
            draw_bar(builder, rx, ry, rw, stroke_w);
            draw_bar(builder, rx + rw - stroke_w, ry, stroke_w, rh / 2.0);
            draw_bar(builder, rx, mid_y - stroke_w / 2.0, rw, stroke_w);
        }
        'Q' | 'q' => {
            draw_bar(builder, rx, ry, rw, stroke_w);
            draw_bar(builder, rx, ry, stroke_w, rh);
            draw_bar(builder, rx + rw - stroke_w, ry, stroke_w, rh);
            draw_bar(builder, rx, ry + rh - stroke_w, rw, stroke_w);
            let pts = vec![Point::new(mid_x, mid_y), Point::new(rx + rw, ry + rh)];
            builder.stroke_polygon(pts, color, stroke_w * 1.1);
        }
        'R' | 'r' => {
            draw_bar(builder, rx, ry, rw, stroke_w);
            draw_bar(builder, rx, ry, stroke_w, rh);
            draw_bar(builder, rx + rw - stroke_w, ry, stroke_w, rh / 2.0);
            draw_bar(builder, rx, mid_y - stroke_w / 2.0, rw, stroke_w);
            draw_bar(builder, rx + rw - stroke_w, mid_y, stroke_w, rh / 2.0);
        }
        'S' | 's' => {
            draw_bar(builder, rx, ry, rw, stroke_w);
            draw_bar(builder, rx, ry, stroke_w, rh / 2.0);
            draw_bar(builder, rx, mid_y - stroke_w / 2.0, rw, stroke_w);
            draw_bar(builder, rx + rw - stroke_w, mid_y, stroke_w, rh / 2.0);
            draw_bar(builder, rx, ry + rh - stroke_w, rw, stroke_w);
        }
        'T' | 't' => {
            draw_bar(builder, rx, ry, rw, stroke_w);
            draw_bar(builder, mid_x - stroke_w / 2.0, ry, stroke_w, rh);
        }
        'U' | 'u' => {
            draw_bar(builder, rx, ry, stroke_w, rh);
            draw_bar(builder, rx + rw - stroke_w, ry, stroke_w, rh);
            draw_bar(builder, rx, ry + rh - stroke_w, rw, stroke_w);
        }
        'V' | 'v' => {
            let pts1 = vec![Point::new(rx, ry), Point::new(mid_x, ry + rh)];
            let pts2 = vec![Point::new(rx + rw, ry), Point::new(mid_x, ry + rh)];
            builder.stroke_polygon(pts1, color, stroke_w);
            builder.stroke_polygon(pts2, color, stroke_w);
        }
        'W' | 'w' => {
            draw_bar(builder, rx, ry, stroke_w, rh);
            draw_bar(builder, rx + rw - stroke_w, ry, stroke_w, rh);
            draw_bar(builder, rx, ry + rh - stroke_w, rw, stroke_w);
            draw_bar(builder, mid_x - stroke_w / 2.0, mid_y, stroke_w, rh / 2.0);
        }
        'X' | 'x' => {
            let pts1 = vec![
                Point::new(rx + rw * 0.15, ry + rh * 0.15),
                Point::new(rx + rw * 0.85, ry + rh * 0.85),
            ];
            let pts2 = vec![
                Point::new(rx + rw * 0.85, ry + rh * 0.15),
                Point::new(rx + rw * 0.15, ry + rh * 0.85),
            ];
            builder.stroke_polygon(pts1, color, stroke_w * 1.1);
            builder.stroke_polygon(pts2, color, stroke_w * 1.1);
        }
        'Y' | 'y' => {
            let pts1 = vec![Point::new(rx, ry), Point::new(mid_x, mid_y)];
            let pts2 = vec![Point::new(rx + rw, ry), Point::new(mid_x, mid_y)];
            builder.stroke_polygon(pts1, color, stroke_w);
            builder.stroke_polygon(pts2, color, stroke_w);
            draw_bar(builder, mid_x - stroke_w / 2.0, mid_y, stroke_w, rh / 2.0);
        }
        'Z' | 'z' => {
            draw_bar(builder, rx, ry, rw, stroke_w);
            draw_bar(builder, rx, ry + rh - stroke_w, rw, stroke_w);
            let pts = vec![Point::new(rx + rw, ry), Point::new(rx, ry + rh)];
            builder.stroke_polygon(pts, color, stroke_w * 1.1);
        }
        '+' => {
            draw_bar(
                builder,
                rx + rw * 0.15,
                mid_y - stroke_w / 2.0,
                rw * 0.7,
                stroke_w,
            );
            draw_bar(
                builder,
                mid_x - stroke_w / 2.0,
                ry + rh * 0.15,
                stroke_w,
                rh * 0.7,
            );
        }
        '-' | '−' => {
            draw_bar(
                builder,
                rx + rw * 0.15,
                mid_y - stroke_w / 2.0,
                rw * 0.7,
                stroke_w,
            );
        }
        '*' | '×' => {
            let pts1 = vec![
                Point::new(rx + rw * 0.2, ry + rh * 0.2),
                Point::new(rx + rw * 0.8, ry + rh * 0.8),
            ];
            let pts2 = vec![
                Point::new(rx + rw * 0.8, ry + rh * 0.2),
                Point::new(rx + rw * 0.2, ry + rh * 0.8),
            ];
            builder.stroke_polygon(pts1, color, stroke_w * 1.1);
            builder.stroke_polygon(pts2, color, stroke_w * 1.1);
        }
        '/' => {
            let pts = vec![
                Point::new(rx + rw * 0.85, ry + rh * 0.15),
                Point::new(rx + rw * 0.15, ry + rh * 0.85),
            ];
            builder.stroke_polygon(pts, color, stroke_w * 1.1);
        }
        '÷' => {
            draw_bar(
                builder,
                rx + rw * 0.15,
                mid_y - stroke_w / 2.0,
                rw * 0.7,
                stroke_w,
            );
            let dot_size = stroke_w * 1.5;
            draw_bar(
                builder,
                mid_x - dot_size / 2.0,
                ry + rh * 0.2,
                dot_size,
                dot_size,
            );
            draw_bar(
                builder,
                mid_x - dot_size / 2.0,
                ry + rh * 0.8 - dot_size,
                dot_size,
                dot_size,
            );
        }
        '=' => {
            draw_bar(
                builder,
                rx + rw * 0.15,
                mid_y - rh * 0.18,
                rw * 0.7,
                stroke_w,
            );
            draw_bar(
                builder,
                rx + rw * 0.15,
                mid_y + rh * 0.18,
                rw * 0.7,
                stroke_w,
            );
        }
        '.' => {
            let dot_size = stroke_w * 1.6;
            draw_bar(
                builder,
                mid_x - dot_size / 2.0,
                ry + rh - dot_size,
                dot_size,
                dot_size,
            );
        }
        '%' => {
            let dot = stroke_w * 1.2;
            draw_bar(builder, rx + rw * 0.15, ry + rh * 0.15, dot, dot);
            draw_bar(
                builder,
                rx + rw * 0.85 - dot,
                ry + rh * 0.85 - dot,
                dot,
                dot,
            );
            let pts = vec![
                Point::new(rx + rw * 0.85, ry + rh * 0.15),
                Point::new(rx + rw * 0.15, ry + rh * 0.85),
            ];
            builder.stroke_polygon(pts, color, stroke_w * 0.9);
        }
        '±' => {
            draw_bar(
                builder,
                rx + rw * 0.15,
                mid_y - rh * 0.2,
                rw * 0.7,
                stroke_w,
            );
            draw_bar(
                builder,
                mid_x - stroke_w / 2.0,
                ry + rh * 0.1,
                stroke_w,
                rh * 0.45,
            );
            draw_bar(
                builder,
                rx + rw * 0.15,
                ry + rh - stroke_w * 1.2,
                rw * 0.7,
                stroke_w,
            );
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_vector_text(
    builder: &mut SceneBuilder,
    text: &str,
    right_x: f64,
    center_y: f64,
    char_w: f64,
    char_h: f64,
    stroke_w: f64,
    color: Color,
) {
    let spacing = char_w * 1.15;
    let total_w = (text.len() as f64) * spacing;
    let mut cur_x = right_x - total_w + (spacing - char_w) / 2.0;
    let top_y = center_y - char_h / 2.0;

    for c in text.chars() {
        if c != ' ' {
            draw_vector_char(builder, c, cur_x, top_y, char_w, char_h, stroke_w, color);
        }
        cur_x += spacing;
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_vector_text_left(
    builder: &mut SceneBuilder,
    text: &str,
    left_x: f64,
    center_y: f64,
    char_w: f64,
    char_h: f64,
    stroke_w: f64,
    color: Color,
) {
    let spacing = char_w * 1.15;
    let mut cur_x = left_x;
    let top_y = center_y - char_h / 2.0;

    for c in text.chars() {
        if c != ' ' {
            draw_vector_char(builder, c, cur_x, top_y, char_w, char_h, stroke_w, color);
        }
        cur_x += spacing;
    }
}

// ── CalculatorApp ───────────────────────────────────────────────────────────

pub struct CalculatorApp {
    pub engine: CalculatorEngine,
    pub font_context: FontContext,
    gpu_context: Option<GpuContext>,
    surface: Option<GpuSurface>,
    renderer: Option<SceneRenderer>,
    cursor: Point,
    hovered_button: Option<usize>,
    pressed_button: Option<usize>,
    dimensions: (u32, u32),
}

impl Default for CalculatorApp {
    fn default() -> Self {
        Self {
            engine: CalculatorEngine::new(),
            font_context: FontContext::default(),
            gpu_context: None,
            surface: None,
            renderer: None,
            cursor: Point::ZERO,
            hovered_button: None,
            pressed_button: None,
            dimensions: (440, 620),
        }
    }
}

impl CalculatorApp {
    fn get_grid_bounds(width: f64, height: f64) -> (f64, f64, f64, f64) {
        let pad_x = 24.0;
        let top_y = 206.0;
        let grid_w = width - pad_x * 2.0;
        let grid_h = height - top_y - 24.0;
        (pad_x, top_y, grid_w, grid_h)
    }

    fn hit_test(&self, p: Point, width: f64, height: f64) -> Option<usize> {
        let (grid_x, grid_y, grid_w, grid_h) = Self::get_grid_bounds(width, height);
        if p.x < grid_x || p.x > grid_x + grid_w || p.y < grid_y || p.y > grid_y + grid_h {
            return None;
        }

        let gap = 10.0;
        let cell_w = (grid_w - gap * 3.0) / 4.0;
        let cell_h = (grid_h - gap * 4.0) / 5.0;

        for (idx, btn) in BUTTONS.iter().enumerate() {
            let bx = grid_x + (btn.col as f64) * (cell_w + gap);
            let by = grid_y + (btn.row as f64) * (cell_h + gap);
            if p.x >= bx && p.x <= bx + cell_w && p.y >= by && p.y <= by + cell_h {
                return Some(idx);
            }
        }
        None
    }

    fn execute_action(&mut self, action: ButtonAction) {
        match action {
            ButtonAction::Digit(d) => self.engine.input_digit(d),
            ButtonAction::Op(op) => self.engine.input_operator(op),
            ButtonAction::Equals => self.engine.calculate_equals(),
            ButtonAction::Decimal => self.engine.input_decimal(),
            ButtonAction::Clear => self.engine.clear(),
            ButtonAction::ClearEntry => self.engine.clear_entry(),
            ButtonAction::Backspace => self.engine.backspace(),
            ButtonAction::ToggleSign => self.engine.toggle_sign(),
            ButtonAction::Percent => self.engine.percentage(),
        }
    }
}

impl GuiApp for CalculatorApp {
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
                let new_hover = self.hit_test(position, w as f64, h as f64);
                if new_hover != self.hovered_button {
                    self.hovered_button = new_hover;
                    window.request_redraw();
                }
            }

            GuiEvent::PointerDown { button, position } => {
                if button == MouseButton::Primary {
                    let (w, h) = self.dimensions;
                    if let Some(idx) = self.hit_test(position, w as f64, h as f64) {
                        self.pressed_button = Some(idx);
                        self.execute_action(BUTTONS[idx].action);
                        window.request_redraw();
                    }
                }
            }

            GuiEvent::PointerUp { button, .. } => {
                if button == MouseButton::Primary && self.pressed_button.is_some() {
                    self.pressed_button = None;
                    window.request_redraw();
                }
            }

            GuiEvent::KeyDown { key, text, .. } => {
                let mut handled = true;
                match key {
                    Key::Character(ref s) => {
                        let c = s.chars().next().unwrap_or('\0');
                        match c {
                            '0'..='9' => self.engine.input_digit(c),
                            '.' | ',' => self.engine.input_decimal(),
                            '+' => self.engine.input_operator(Operator::Add),
                            '-' | '−' => self.engine.input_operator(Operator::Subtract),
                            '*' | '×' => self.engine.input_operator(Operator::Multiply),
                            '/' | '÷' => self.engine.input_operator(Operator::Divide),
                            '=' | '\r' | '\n' => self.engine.calculate_equals(),
                            '%' => self.engine.percentage(),
                            'c' | 'C' => self.engine.clear(),
                            _ => handled = false,
                        }
                    }
                    Key::Delete => self.engine.clear_entry(),
                    Key::Enter => self.engine.calculate_equals(),
                    Key::Backspace => self.engine.backspace(),
                    Key::Escape => self.engine.clear(),
                    _ => {
                        if let Some(txt) = text {
                            let c = txt.chars().next().unwrap_or('\0');
                            match c {
                                '0'..='9' => self.engine.input_digit(c),
                                '.' | ',' => self.engine.input_decimal(),
                                '+' => self.engine.input_operator(Operator::Add),
                                '-' | '−' => self.engine.input_operator(Operator::Subtract),
                                '*' | '×' => self.engine.input_operator(Operator::Multiply),
                                '/' | '÷' => self.engine.input_operator(Operator::Divide),
                                '=' | '\r' | '\n' => self.engine.calculate_equals(),
                                '%' => self.engine.percentage(),
                                'c' | 'C' => self.engine.clear(),
                                _ => handled = false,
                            }
                        } else {
                            handled = false;
                        }
                    }
                }
                if handled {
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

                // Background Fluent Canvas
                builder.fill_rect(Rect::new(0.0, 0.0, w, h), Color::DARK_GRAY);
                builder.fill_rect(Rect::new(0.0, 0.0, w, 3.0), Color::rgb(0, 120, 212));

                // Title Pill
                builder.fill_rounded_rect(
                    Rect::new(24.0, 16.0, 190.0, 28.0),
                    6.0,
                    Color::rgb(32, 32, 32),
                );
                builder.fill_rounded_rect(
                    Rect::new(33.5, 25.5, 9.0, 9.0),
                    4.5,
                    Color::rgb(0, 120, 212),
                );
                draw_vector_text_left(
                    &mut builder,
                    "AGAM CALCULATOR",
                    50.0,
                    30.0,
                    7.0,
                    11.0,
                    1.6,
                    Color::rgb(210, 210, 210),
                );

                // Display Screen Card
                let disp_rect = Rect::new(24.0, 56.0, w - 48.0, 120.0);
                builder.fill_rounded_rect(disp_rect, 12.0, Color::rgb(28, 28, 28));
                builder.stroke_rect(disp_rect, Color::rgb(50, 50, 50), 1.5);

                let right_x = disp_rect.origin.x + disp_rect.width - 20.0;
                if !self.engine.history.is_empty() {
                    let hist_len = self.engine.history.len().max(1);
                    let h_char_w = if hist_len > 28 {
                        6.0
                    } else if hist_len > 18 {
                        7.5
                    } else {
                        9.0
                    };
                    let h_char_h = h_char_w * 1.55;
                    draw_vector_text(
                        &mut builder,
                        &self.engine.history,
                        right_x,
                        disp_rect.origin.y + 30.0,
                        h_char_w,
                        h_char_h,
                        1.6,
                        Color::rgb(140, 140, 140),
                    );
                }

                let disp_len = self.engine.display.len().max(1);
                let (char_w, char_h, stroke_w) = if disp_len > 18 {
                    (9.5, 17.0, 2.0)
                } else if disp_len > 14 {
                    (11.5, 21.0, 2.4)
                } else if disp_len > 10 {
                    (14.5, 27.0, 3.0)
                } else if disp_len > 7 {
                    (17.5, 33.0, 3.4)
                } else {
                    (21.0, 40.0, 3.8)
                };

                let disp_color = if self.engine.has_error {
                    Color::rgb(255, 90, 90)
                } else {
                    Color::WHITE
                };
                draw_vector_text(
                    &mut builder,
                    &self.engine.display,
                    right_x,
                    disp_rect.origin.y + 82.0,
                    char_w,
                    char_h,
                    stroke_w,
                    disp_color,
                );

                // 4x5 Keypad Grid
                let (grid_x, grid_y, grid_w, grid_h) = Self::get_grid_bounds(w, h);
                let gap = 10.0;
                let cell_w = (grid_w - gap * 3.0) / 4.0;
                let cell_h = (grid_h - gap * 4.0) / 5.0;

                for (idx, btn) in BUTTONS.iter().enumerate() {
                    let bx = grid_x + (btn.col as f64) * (cell_w + gap);
                    let by = grid_y + (btn.row as f64) * (cell_h + gap);
                    let rect = Rect::new(bx, by, cell_w, cell_h);
                    let is_hovered = self.hovered_button == Some(idx);
                    let is_pressed = self.pressed_button == Some(idx);

                    let (bg, border, text_color) = match btn.style {
                        ButtonStyle::Accent => {
                            if is_pressed {
                                (Color::rgb(0, 85, 150), Color::WHITE, Color::WHITE)
                            } else if is_hovered {
                                (Color::rgb(24, 140, 235), Color::WHITE, Color::WHITE)
                            } else {
                                (
                                    Color::rgb(0, 120, 212),
                                    Color::rgb(0, 140, 240),
                                    Color::WHITE,
                                )
                            }
                        }
                        ButtonStyle::Operator => {
                            if is_pressed {
                                (
                                    Color::rgb(26, 26, 26),
                                    Color::rgb(0, 120, 212),
                                    Color::WHITE,
                                )
                            } else if is_hovered {
                                (Color::rgb(60, 60, 60), Color::rgb(85, 85, 85), Color::WHITE)
                            } else {
                                (Color::rgb(48, 48, 48), Color::rgb(62, 62, 62), Color::WHITE)
                            }
                        }
                        ButtonStyle::Action => {
                            if is_pressed {
                                (
                                    Color::rgb(30, 30, 30),
                                    Color::rgb(0, 120, 212),
                                    Color::WHITE,
                                )
                            } else if is_hovered {
                                (Color::rgb(56, 56, 56), Color::rgb(80, 80, 80), Color::WHITE)
                            } else {
                                (
                                    Color::rgb(44, 44, 44),
                                    Color::rgb(58, 58, 58),
                                    Color::LIGHT_GRAY,
                                )
                            }
                        }
                        ButtonStyle::Number => {
                            if is_pressed {
                                (
                                    Color::rgb(32, 32, 32),
                                    Color::rgb(0, 120, 212),
                                    Color::WHITE,
                                )
                            } else if is_hovered {
                                (Color::rgb(52, 52, 52), Color::rgb(75, 75, 75), Color::WHITE)
                            } else {
                                (Color::rgb(40, 40, 40), Color::rgb(52, 52, 52), Color::WHITE)
                            }
                        }
                    };

                    builder.fill_rounded_rect(rect, 8.0, bg);
                    builder.stroke_rect(rect, border, 1.0);

                    let cx = bx + cell_w / 2.0;
                    let cy = by + cell_h / 2.0;
                    let btn_str = btn.label;
                    let num_chars = btn_str.chars().count();
                    let b_char_w = if num_chars > 1 { 9.0 } else { 12.0 };
                    let b_char_h = if num_chars > 1 { 15.0 } else { 19.0 };
                    let b_stroke_w = if num_chars > 1 { 2.0 } else { 2.4 };

                    let text_total_w = (num_chars as f64) * (b_char_w * 1.15);
                    let start_x = cx - text_total_w / 2.0 + (b_char_w * 0.15) / 2.0;
                    let mut kx = start_x;
                    let ky = cy - b_char_h / 2.0;

                    for ch in btn_str.chars() {
                        draw_vector_char(
                            &mut builder,
                            ch,
                            kx,
                            ky,
                            b_char_w,
                            b_char_h,
                            b_stroke_w,
                            text_color,
                        );
                        kx += b_char_w * 1.15;
                    }
                }

                let frame = surface.acquire_frame()?;
                renderer.render_to_frame(context, &builder, &frame, Color::DARK_GRAY)?;
                frame.present();
            }

            _ => {}
        }
        Ok(())
    }
}

// ── CounterApp ──────────────────────────────────────────────────────────────

pub struct CounterApp {
    count: i32,
    pub font_context: FontContext,
    gpu_context: Option<GpuContext>,
    surface: Option<GpuSurface>,
    renderer: Option<SceneRenderer>,
    hovered: Option<usize>,
    dimensions: (u32, u32),
}

impl Default for CounterApp {
    fn default() -> Self {
        Self {
            count: 0,
            font_context: FontContext::default(),
            gpu_context: None,
            surface: None,
            renderer: None,
            hovered: None,
            dimensions: (360, 240),
        }
    }
}

impl GuiApp for CounterApp {
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
                let btn_w = 90.0;
                let btn_h = 36.0;
                let by = 160.0;
                let mut new_hover = None;
                for (i, bx) in [30.0, 135.0, 240.0].iter().enumerate() {
                    if position.x >= *bx
                        && position.x <= *bx + btn_w
                        && position.y >= by
                        && position.y <= by + btn_h
                    {
                        new_hover = Some(i);
                        break;
                    }
                }
                if new_hover != self.hovered {
                    self.hovered = new_hover;
                    window.request_redraw();
                }
            }

            GuiEvent::PointerDown { button, position } => {
                if button == MouseButton::Primary {
                    let btn_w = 90.0;
                    let btn_h = 36.0;
                    let by = 160.0;
                    for (i, bx) in [30.0, 135.0, 240.0].iter().enumerate() {
                        if position.x >= *bx
                            && position.x <= *bx + btn_w
                            && position.y >= by
                            && position.y <= by + btn_h
                        {
                            match i {
                                0 => self.count -= 1,
                                1 => self.count += 1,
                                2 => self.count = 0,
                                _ => {}
                            }
                            window.request_redraw();
                            break;
                        }
                    }
                }
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

                builder.fill_rect(Rect::new(0.0, 0.0, w, h), Color::DARK_GRAY);

                // Display Card
                let card_rect = Rect::new(24.0, 24.0, w - 48.0, 110.0);
                builder.fill_rounded_rect(card_rect, 10.0, Color::rgb(35, 35, 35));
                builder.stroke_rect(card_rect, Color::rgb(55, 55, 55), 1.0);

                draw_vector_text(
                    &mut builder,
                    "CURRENT COUNT",
                    card_rect.origin.x + card_rect.width / 2.0 + 50.0,
                    card_rect.origin.y + 30.0,
                    7.0,
                    10.0,
                    1.6,
                    Color::rgb(140, 140, 140),
                );

                let count_str = self.count.to_string();
                draw_vector_text(
                    &mut builder,
                    &count_str,
                    card_rect.origin.x + card_rect.width / 2.0 + (count_str.len() as f64 * 8.0),
                    card_rect.origin.y + 75.0,
                    16.0,
                    28.0,
                    3.2,
                    Color::WHITE,
                );

                // Buttons
                let labels = ["- DEC", "+ INC", "RESET"];
                let btn_w = 90.0;
                let btn_h = 36.0;
                let by = 160.0;
                for (i, bx) in [30.0, 135.0, 240.0].iter().enumerate() {
                    let rect = Rect::new(*bx, by, btn_w, btn_h);
                    let is_hovered = self.hovered == Some(i);
                    let bg = if i == 1 {
                        if is_hovered {
                            Color::rgb(24, 140, 235)
                        } else {
                            Color::rgb(0, 120, 212)
                        }
                    } else if is_hovered {
                        Color::rgb(60, 60, 60)
                    } else {
                        Color::rgb(42, 42, 42)
                    };
                    builder.fill_rounded_rect(rect, 6.0, bg);
                    builder.stroke_rect(rect, Color::rgb(70, 70, 70), 1.0);
                    draw_vector_text_left(
                        &mut builder,
                        labels[i],
                        bx + 18.0,
                        by + 18.0,
                        7.0,
                        12.0,
                        1.8,
                        Color::WHITE,
                    );
                }

                let frame = surface.acquire_frame()?;
                renderer.render_to_frame(context, &builder, &frame, Color::DARK_GRAY)?;
                frame.present();
            }

            _ => {}
        }
        Ok(())
    }
}
