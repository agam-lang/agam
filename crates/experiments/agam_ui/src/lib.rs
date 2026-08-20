//! # agam_ui
//!
//! Modern, fine-grained reactive declarative UI framework.
//!
//! Features:
//! - Fine-grained reactive state primitives (`Signal`, `Computed`, `Effect`, `batch`).
//! - Bento Box, Glassmorphic, and Neobrutalist design systems.
//! - Declarative Virtual UI tree (`Row`, `Column`, `Grid`, `Card`, `Button`, `Text`, `Slider`).
//! - CSS-inspired composable styling engine.

pub mod reactive;
pub mod style;
pub mod theme;
pub mod widget;

pub use reactive::{Computed, Signal, batch, create_effect};
pub use style::{Alignment, Color, FlexDirection, Insets, Shadow, Style};
pub use theme::{Theme, ThemeKind};
pub use widget::{Widget, WidgetKind};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bento_grid_component_construction() {
        let theme = Theme::bento();

        let count = Signal::new(0);
        let count_clone = count.clone();
        let display_text = Computed::new(move || format!("Count: {}", count_clone.get()));

        let card1 = Widget::card(Widget::text(display_text.get())).with_style(theme.card_style());
        let card2 = Widget::card(Widget::button("Increment")).with_style(theme.card_style());

        let bento_grid = Widget::grid(2, vec![card1, card2])
            .with_style(Style::new().bg(theme.background).gap(16.0));

        if let WidgetKind::Grid { columns, children } = &bento_grid.kind {
            assert_eq!(*columns, 2);
            assert_eq!(children.len(), 2);
        } else {
            panic!("Expected Grid widget");
        }

        // Trigger reactive state update
        count.set(5);
        assert_eq!(display_text.get(), "Count: 5");
    }

    #[test]
    fn test_style_composition() {
        let base = Style::new().bg(Color::BLACK).radius(8.0);
        let override_style = Style::new().bg(Color::PRIMARY).pad(Insets::all(12.0));
        let merged = base.merge(override_style);

        assert_eq!(merged.background_color, Some(Color::PRIMARY));
        assert_eq!(merged.border_radius, Some(8.0));
        assert_eq!(merged.padding, Some(Insets::all(12.0)));
    }
}
