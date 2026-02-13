use ratatui::layout::Rect;

use crate::app::state::PaneFocus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutKind {
    Single,
    Compact,
}

#[derive(Debug, Clone, Copy)]
pub struct PaneLayout {
    pub kind: LayoutKind,
    pub editor: Rect,
    pub preview: Rect,
}

pub fn compute_pane_layout(area: Rect, focus: PaneFocus) -> PaneLayout {
    let zero = Rect {
        x: area.x,
        y: area.y,
        width: 0,
        height: 0,
    };
    let kind = if area.width < 80 || area.height < 24 {
        LayoutKind::Compact
    } else {
        LayoutKind::Single
    };

    if focus == PaneFocus::Editor {
        PaneLayout {
            kind,
            editor: area,
            preview: zero,
        }
    } else {
        PaneLayout {
            kind,
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
    fn editor_mode_uses_single_editor_pane() {
        let layout = compute_pane_layout(
            Rect {
                x: 0,
                y: 0,
                width: 120,
                height: 30,
            },
            PaneFocus::Editor,
        );
        assert_eq!(layout.kind, LayoutKind::Single);
        assert_eq!(layout.editor.width, 120);
        assert_eq!(layout.preview.width, 0);
    }

    #[test]
    fn view_mode_uses_single_preview_pane() {
        let layout = compute_pane_layout(
            Rect {
                x: 1,
                y: 2,
                width: 90,
                height: 26,
            },
            PaneFocus::Preview,
        );
        assert_eq!(layout.kind, LayoutKind::Single);
        assert_eq!(layout.preview.width, 90);
        assert_eq!(layout.editor.width, 0);
        assert_eq!(layout.preview.x, 1);
    }

    #[test]
    fn compact_layout_detected_but_stays_single_mode() {
        let layout = compute_pane_layout(
            Rect {
                x: 0,
                y: 0,
                width: 70,
                height: 20,
            },
            PaneFocus::Preview,
        );
        assert_eq!(layout.kind, LayoutKind::Compact);
        assert_eq!(layout.editor.width, 0);
        assert_eq!(layout.preview.width, 70);
    }
}
