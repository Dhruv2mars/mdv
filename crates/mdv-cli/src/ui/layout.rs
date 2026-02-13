use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::app::state::PaneFocus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutKind {
    Horizontal,
    Vertical,
    Compact,
}

#[derive(Debug, Clone, Copy)]
pub struct PaneLayout {
    pub kind: LayoutKind,
    pub editor: Rect,
    pub preview: Rect,
}

pub fn compute_pane_layout(area: Rect, focus: PaneFocus, split_ratio: u16) -> PaneLayout {
    if area.width < 80 || area.height < 24 {
        return compact_layout(area, focus);
    }

    if area.width >= 100 {
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(split_ratio),
                Constraint::Percentage(100 - split_ratio),
            ])
            .split(area);
        return PaneLayout {
            kind: LayoutKind::Horizontal,
            editor: panes[0],
            preview: panes[1],
        };
    }

    let (first, second) = (split_ratio, 100 - split_ratio);
    let panes = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(first),
            Constraint::Percentage(second),
        ])
        .split(area);

    if focus == PaneFocus::Editor {
        PaneLayout {
            kind: LayoutKind::Vertical,
            editor: panes[0],
            preview: panes[1],
        }
    } else {
        PaneLayout {
            kind: LayoutKind::Vertical,
            editor: panes[1],
            preview: panes[0],
        }
    }
}

fn compact_layout(area: Rect, focus: PaneFocus) -> PaneLayout {
    let zero = Rect {
        x: area.x,
        y: area.y,
        width: 0,
        height: 0,
    };
    if focus == PaneFocus::Editor {
        PaneLayout {
            kind: LayoutKind::Compact,
            editor: area,
            preview: zero,
        }
    } else {
        PaneLayout {
            kind: LayoutKind::Compact,
            editor: zero,
            preview: area,
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use crate::app::state::PaneFocus;

    use super::{LayoutKind, compute_pane_layout};

    #[test]
    fn uses_horizontal_layout_from_100_columns() {
        let layout = compute_pane_layout(
            Rect {
                x: 0,
                y: 0,
                width: 120,
                height: 30,
            },
            PaneFocus::Editor,
            55,
        );
        assert_eq!(layout.kind, LayoutKind::Horizontal);
        assert!(layout.editor.width > layout.preview.width);
    }

    #[test]
    fn uses_vertical_layout_below_100_columns() {
        let layout = compute_pane_layout(
            Rect {
                x: 0,
                y: 0,
                width: 90,
                height: 30,
            },
            PaneFocus::Preview,
            65,
        );
        assert_eq!(layout.kind, LayoutKind::Vertical);
        assert!(layout.preview.height > layout.editor.height);
    }

    #[test]
    fn vertical_layout_with_editor_focus_has_editor_on_top() {
        let layout = compute_pane_layout(
            Rect {
                x: 1,
                y: 2,
                width: 90,
                height: 30,
            },
            PaneFocus::Editor,
            65,
        );
        assert_eq!(layout.kind, LayoutKind::Vertical);
        assert!(layout.editor.y < layout.preview.y);
        assert_eq!(layout.editor.x, 1);
    }

    #[test]
    fn uses_compact_layout_for_small_terminal() {
        let layout = compute_pane_layout(
            Rect {
                x: 0,
                y: 0,
                width: 70,
                height: 20,
            },
            PaneFocus::Editor,
            65,
        );
        assert_eq!(layout.kind, LayoutKind::Compact);
        assert_eq!(layout.preview.width, 0);
    }

    #[test]
    fn compact_layout_with_preview_focus_hides_editor() {
        let layout = compute_pane_layout(
            Rect {
                x: 0,
                y: 0,
                width: 70,
                height: 23,
            },
            PaneFocus::Preview,
            65,
        );
        assert_eq!(layout.kind, LayoutKind::Compact);
        assert_eq!(layout.editor.width, 0);
        assert_eq!(layout.preview.width, 70);
    }
}
