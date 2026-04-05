use ratatui::layout::Rect;

use crate::app::state::PaneFocus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutKind {
    /// Wide enough for split view (editor + preview side by side)
    Split,
    /// Single pane mode - only show focused pane
    Single,
    /// Compact terminal (<80x24) - single pane with reduced chrome
    Compact,
}

#[derive(Debug, Clone, Copy)]
pub struct PaneLayout {
    pub kind: LayoutKind,
    pub editor: Rect,
    pub preview: Rect,
}

/// Minimum width for split view (each pane needs ~50 cols for comfortable editing)
const MIN_SPLIT_WIDTH: u16 = 120;
/// Minimum width/height for compact mode
const MIN_NORMAL_WIDTH: u16 = 80;
const MIN_NORMAL_HEIGHT: u16 = 24;

pub fn compute_pane_layout(area: Rect, focus: PaneFocus) -> PaneLayout {
    let zero = Rect {
        x: area.x,
        y: area.y,
        width: 0,
        height: 0,
    };

    // Compact mode for very small terminals
    if area.width < MIN_NORMAL_WIDTH || area.height < MIN_NORMAL_HEIGHT {
        return if focus == PaneFocus::Editor {
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
        };
    }

    // Split view for wide terminals
    if area.width >= MIN_SPLIT_WIDTH {
        // 50/50 split with 1 char divider
        let half = area.width / 2;
        let editor_rect = Rect {
            x: area.x,
            y: area.y,
            width: half,
            height: area.height,
        };
        let preview_rect = Rect {
            x: area.x + half,
            y: area.y,
            width: area.width - half,
            height: area.height,
        };
        return PaneLayout {
            kind: LayoutKind::Split,
            editor: editor_rect,
            preview: preview_rect,
        };
    }

    // Single pane mode for medium width terminals
    if focus == PaneFocus::Editor {
        PaneLayout {
            kind: LayoutKind::Single,
            editor: area,
            preview: zero,
        }
    } else {
        PaneLayout {
            kind: LayoutKind::Single,
            editor: zero,
            preview: area,
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use crate::app::state::PaneFocus;

    use super::{LayoutKind, MIN_SPLIT_WIDTH, compute_pane_layout};

    #[test]
    fn wide_terminal_uses_split_layout() {
        let layout = compute_pane_layout(
            Rect {
                x: 0,
                y: 0,
                width: 160,
                height: 40,
            },
            PaneFocus::Editor,
        );
        assert_eq!(layout.kind, LayoutKind::Split);
        assert_eq!(layout.editor.width, 80);
        assert_eq!(layout.preview.width, 80);
        assert_eq!(layout.editor.x, 0);
        assert_eq!(layout.preview.x, 80);
    }

    #[test]
    fn split_layout_at_min_width() {
        let layout = compute_pane_layout(
            Rect {
                x: 0,
                y: 0,
                width: MIN_SPLIT_WIDTH,
                height: 30,
            },
            PaneFocus::Preview,
        );
        assert_eq!(layout.kind, LayoutKind::Split);
        assert!(layout.editor.width > 0);
        assert!(layout.preview.width > 0);
    }

    #[test]
    fn medium_terminal_uses_single_pane_editor() {
        let layout = compute_pane_layout(
            Rect {
                x: 0,
                y: 0,
                width: 100,
                height: 30,
            },
            PaneFocus::Editor,
        );
        assert_eq!(layout.kind, LayoutKind::Single);
        assert_eq!(layout.editor.width, 100);
        assert_eq!(layout.preview.width, 0);
    }

    #[test]
    fn medium_terminal_uses_single_pane_preview() {
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
    fn compact_layout_for_small_terminal() {
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

    #[test]
    fn compact_layout_editor_focus() {
        let layout = compute_pane_layout(
            Rect {
                x: 0,
                y: 0,
                width: 60,
                height: 18,
            },
            PaneFocus::Editor,
        );
        assert_eq!(layout.kind, LayoutKind::Compact);
        assert_eq!(layout.editor.width, 60);
        assert_eq!(layout.preview.width, 0);
    }
}
