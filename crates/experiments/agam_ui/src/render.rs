//! HTML/CSS Web and Native Render Target Serializer.

use crate::style::{Alignment, FlexDirection, Style};
use crate::widget::{Widget, WidgetKind};

fn style_to_css(style: &Style) -> String {
    let mut css = Vec::new();

    if let Some(w) = style.width {
        css.push(format!("width: {w}px;"));
    }
    if let Some(h) = style.height {
        css.push(format!("height: {h}px;"));
    }
    if let Some(bg) = style.background_color {
        css.push(format!(
            "background-color: rgba({}, {}, {}, {});",
            bg.r,
            bg.g,
            bg.b,
            bg.a as f32 / 255.0
        ));
    }
    if let Some(c) = style.text_color {
        css.push(format!(
            "color: rgba({}, {}, {}, {});",
            c.r,
            c.g,
            c.b,
            c.a as f32 / 255.0
        ));
    }
    if let Some(r) = style.border_radius {
        css.push(format!("border-radius: {r}px;"));
    }
    if let Some(pad) = style.padding {
        css.push(format!(
            "padding: {}px {}px {}px {}px;",
            pad.top, pad.right, pad.bottom, pad.left
        ));
    }
    if let Some(gap) = style.gap {
        css.push(format!("gap: {gap}px;"));
    }
    if let Some(dir) = style.flex_direction {
        let dir_str = match dir {
            FlexDirection::Row => "row",
            FlexDirection::Column => "column",
            FlexDirection::RowReverse => "row-reverse",
            FlexDirection::ColumnReverse => "column-reverse",
        };
        css.push(format!("flex-direction: {dir_str};"));
    }
    if let Some(align) = style.align_items {
        let align_str = match align {
            Alignment::Start => "flex-start",
            Alignment::Center => "center",
            Alignment::End => "flex-end",
            Alignment::Stretch => "stretch",
            Alignment::SpaceBetween => "space-between",
            Alignment::SpaceAround => "space-around",
            Alignment::SpaceEvenly => "space-evenly",
        };
        css.push(format!("align-items: {align_str};"));
    }

    css.join(" ")
}

/// Serialize a Virtual Widget Tree into HTML with inline styling.
pub fn render_to_html(widget: &Widget) -> String {
    let css = style_to_css(&widget.style);
    let style_attr = if css.is_empty() {
        String::new()
    } else {
        format!(" style=\"{css}\"")
    };

    match &widget.kind {
        WidgetKind::Text { text } => {
            format!("<span{style_attr}>{text}</span>")
        }
        WidgetKind::Button { label } => {
            format!("<button{style_attr}>{label}</button>")
        }
        WidgetKind::Row { children } => {
            let inner = children.iter().map(render_to_html).collect::<String>();
            format!("<div{style_attr} class=\"agam-row\">{inner}</div>")
        }
        WidgetKind::Column { children } => {
            let inner = children.iter().map(render_to_html).collect::<String>();
            format!("<div{style_attr} class=\"agam-column\">{inner}</div>")
        }
        WidgetKind::Grid { columns, children } => {
            let inner = children.iter().map(render_to_html).collect::<String>();
            format!(
                "<div{style_attr} class=\"agam-grid\" style=\"display: grid; grid-template-columns: repeat({columns}, minmax(0, 1fr));\">{inner}</div>"
            )
        }
        WidgetKind::Card { child } => {
            let inner = render_to_html(child);
            format!("<div{style_attr} class=\"agam-card\">{inner}</div>")
        }
        WidgetKind::Image { src, width, height } => {
            format!("<img{style_attr} src=\"{src}\" width=\"{width}\" height=\"{height}\" />")
        }
        WidgetKind::Slider { value, min, max } => {
            format!(
                "<input{style_attr} type=\"range\" value=\"{value}\" min=\"{min}\" max=\"{max}\" />"
            )
        }
        WidgetKind::Spacer { flex } => {
            format!("<div style=\"flex-grow: {flex};\"></div>")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::Color;

    #[test]
    fn test_render_button_to_html() {
        let btn = Widget::button("Submit").with_style(Style::new().bg(Color::PRIMARY).radius(8.0));
        let html = render_to_html(&btn);
        assert!(html.contains("<button"));
        assert!(html.contains("Submit</button>"));
        assert!(html.contains("border-radius: 8px;"));
    }

    #[test]
    fn test_render_bento_grid_to_html() {
        let card1 = Widget::card(Widget::text("AI Summary"));
        let grid = Widget::grid(2, vec![card1]);
        let html = render_to_html(&grid);
        assert!(html.contains("grid-template-columns: repeat(2, minmax(0, 1fr));"));
        assert!(html.contains("<span"));
        assert!(html.contains("AI Summary</span>"));
    }
}
