//! Virtual UI Tree Reconciliation and Diffing Engine.

use crate::style::Style;
use crate::widget::{Widget, WidgetKind};
use serde::{Deserialize, Serialize};

/// A patch operation representing a minimal mutation to apply to the render target.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PatchOp {
    CreateNode { node_id: u64, widget: Widget },
    RemoveNode { node_id: u64 },
    ReplaceNode { node_id: u64, new_widget: Widget },
    UpdateStyle { node_id: u64, new_style: Style },
    UpdateText { node_id: u64, new_text: String },
    AppendChild { parent_id: u64, child_id: u64 },
    RemoveChild { parent_id: u64, child_id: u64 },
}

/// Compute the minimal diff between two virtual UI trees.
pub fn diff_trees(old_tree: &Widget, new_tree: &Widget, node_id: u64) -> Vec<PatchOp> {
    let mut patches = Vec::new();

    // 1. Key mismatch or completely different widget kind -> Replace
    let old_discriminant = std::mem::discriminant(&old_tree.kind);
    let new_discriminant = std::mem::discriminant(&new_tree.kind);

    if old_tree.key != new_tree.key || old_discriminant != new_discriminant {
        patches.push(PatchOp::ReplaceNode {
            node_id,
            new_widget: new_tree.clone(),
        });
        return patches;
    }

    // 2. Style updates
    if old_tree.style != new_tree.style {
        patches.push(PatchOp::UpdateStyle {
            node_id,
            new_style: new_tree.style.clone(),
        });
    }

    // 3. Node content and children reconciliation
    match (&old_tree.kind, &new_tree.kind) {
        (WidgetKind::Text { text: old_t }, WidgetKind::Text { text: new_t }) => {
            if old_t != new_t {
                patches.push(PatchOp::UpdateText {
                    node_id,
                    new_text: new_t.clone(),
                });
            }
        }
        (WidgetKind::Button { label: old_l }, WidgetKind::Button { label: new_l }) => {
            if old_l != new_l {
                patches.push(PatchOp::UpdateText {
                    node_id,
                    new_text: new_l.clone(),
                });
            }
        }
        (WidgetKind::Row { children: old_c }, WidgetKind::Row { children: new_c })
        | (WidgetKind::Column { children: old_c }, WidgetKind::Column { children: new_c })
        | (
            WidgetKind::Grid {
                children: old_c, ..
            },
            WidgetKind::Grid {
                children: new_c, ..
            },
        ) => {
            let common_len = old_c.len().min(new_c.len());
            for i in 0..common_len {
                let child_id = node_id * 100 + (i as u64) + 1;
                patches.extend(diff_trees(&old_c[i], &new_c[i], child_id));
            }
            if new_c.len() > old_c.len() {
                for (i, child) in new_c.iter().enumerate().skip(old_c.len()) {
                    let child_id = node_id * 100 + (i as u64) + 1;
                    patches.push(PatchOp::CreateNode {
                        node_id: child_id,
                        widget: child.clone(),
                    });
                    patches.push(PatchOp::AppendChild {
                        parent_id: node_id,
                        child_id,
                    });
                }
            } else if old_c.len() > new_c.len() {
                for i in old_c.len()..old_c.len() {
                    let child_id = node_id * 100 + (i as u64) + 1;
                    patches.push(PatchOp::RemoveChild {
                        parent_id: node_id,
                        child_id,
                    });
                    patches.push(PatchOp::RemoveNode { node_id: child_id });
                }
            }
        }
        (WidgetKind::Card { child: old_c }, WidgetKind::Card { child: new_c }) => {
            let child_id = node_id * 100 + 1;
            patches.extend(diff_trees(old_c, new_c, child_id));
        }
        _ => {}
    }

    patches
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::Color;

    #[test]
    fn test_diff_text_change() {
        let old = Widget::text("Hello");
        let new = Widget::text("Hello World");
        let patches = diff_trees(&old, &new, 1);

        assert_eq!(patches.len(), 1);
        assert_eq!(
            patches[0],
            PatchOp::UpdateText {
                node_id: 1,
                new_text: "Hello World".into()
            }
        );
    }

    #[test]
    fn test_diff_style_change() {
        let old = Widget::button("Click").with_style(Style::new().bg(Color::BLACK));
        let new = Widget::button("Click").with_style(Style::new().bg(Color::PRIMARY));
        let patches = diff_trees(&old, &new, 1);

        assert_eq!(patches.len(), 1);
        match &patches[0] {
            PatchOp::UpdateStyle { node_id, new_style } => {
                assert_eq!(*node_id, 1);
                assert_eq!(new_style.background_color, Some(Color::PRIMARY));
            }
            _ => panic!("Expected UpdateStyle patch"),
        }
    }

    #[test]
    fn test_diff_child_addition() {
        let old = Widget::column(vec![Widget::text("Line 1")]);
        let new = Widget::column(vec![Widget::text("Line 1"), Widget::text("Line 2")]);
        let patches = diff_trees(&old, &new, 1);

        assert_eq!(patches.len(), 2);
        assert_eq!(
            patches[0],
            PatchOp::CreateNode {
                node_id: 102,
                widget: Widget::text("Line 2")
            }
        );
        assert_eq!(
            patches[1],
            PatchOp::AppendChild {
                parent_id: 1,
                child_id: 102
            }
        );
    }
}
