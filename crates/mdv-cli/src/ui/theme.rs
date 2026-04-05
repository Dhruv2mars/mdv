use mdv_core::SegmentKind;
use ratatui::style::{Color, Modifier, Style};

use crate::app::state::ThemeChoice;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ThemeTokens {
    pub top_bar: Style,
    pub status_ok: Style,
    pub status_warn: Style,
    pub status_error: Style,
    pub status_bg: Style,
    pub status_mode: Style,
    pub status_position: Style,
    pub status_file: Style,
    pub status_dirty: Style,
    pub status_clean: Style,
    pub status_separator: Style,
    pub pane_border: Style,
    pub pane_focus: Style,
    pub help: Style,
    pub heading: Style,
    pub list_bullet: Style,
    pub link: Style,
    pub code: Style,
    pub quote: Style,
    pub table_header: Style,
    pub conflict_local: Style,
    pub conflict_external: Style,
    pub plain: Style,
    pub line_number: Style,
    pub line_number_current: Style,
    pub scroll_indicator: Style,
    pub emphasis: Style,
    pub strong: Style,
    pub strikethrough: Style,
    pub hr: Style,
    pub task_done: Style,
    pub task_pending: Style,
}

pub fn build_theme(choice: ThemeChoice, no_color: bool) -> ThemeTokens {
    if no_color {
        return monochrome_theme();
    }

    match choice {
        ThemeChoice::Auto | ThemeChoice::Default => default_theme(),
        ThemeChoice::HighContrast => high_contrast_theme(),
    }
}

pub fn style_for_segment(tokens: &ThemeTokens, kind: SegmentKind) -> Style {
    match kind {
        SegmentKind::Plain => tokens.plain,
        SegmentKind::Heading => tokens.heading,
        SegmentKind::ListBullet => tokens.list_bullet,
        SegmentKind::Link => tokens.link,
        SegmentKind::Code => tokens.code,
        SegmentKind::Quote => tokens.quote,
        SegmentKind::TableHeader => tokens.table_header,
        SegmentKind::ConflictLocal => tokens.conflict_local,
        SegmentKind::ConflictExternal => tokens.conflict_external,
    }
}

fn default_theme() -> ThemeTokens {
    ThemeTokens {
        top_bar: Style::default()
            .fg(Color::Rgb(220, 220, 220))
            .bg(Color::Rgb(40, 44, 52))
            .add_modifier(Modifier::BOLD),
        status_ok: Style::default().fg(Color::Rgb(152, 195, 121)),
        status_warn: Style::default().fg(Color::Rgb(229, 192, 123)),
        status_error: Style::default().fg(Color::Rgb(224, 108, 117)),
        status_bg: Style::default()
            .fg(Color::Rgb(171, 178, 191))
            .bg(Color::Rgb(40, 44, 52)),
        status_mode: Style::default()
            .fg(Color::Rgb(40, 44, 52))
            .bg(Color::Rgb(97, 175, 239))
            .add_modifier(Modifier::BOLD),
        status_position: Style::default()
            .fg(Color::Rgb(40, 44, 52))
            .bg(Color::Rgb(198, 120, 221))
            .add_modifier(Modifier::BOLD),
        status_file: Style::default()
            .fg(Color::Rgb(220, 220, 220))
            .bg(Color::Rgb(55, 59, 67)),
        status_dirty: Style::default()
            .fg(Color::Rgb(229, 192, 123))
            .bg(Color::Rgb(55, 59, 67))
            .add_modifier(Modifier::BOLD),
        status_clean: Style::default()
            .fg(Color::Rgb(152, 195, 121))
            .bg(Color::Rgb(55, 59, 67)),
        status_separator: Style::default()
            .fg(Color::Rgb(92, 99, 112))
            .bg(Color::Rgb(40, 44, 52)),
        pane_border: Style::default().fg(Color::Rgb(92, 99, 112)),
        pane_focus: Style::default()
            .fg(Color::Rgb(97, 175, 239))
            .add_modifier(Modifier::BOLD),
        help: Style::default().fg(Color::Rgb(171, 178, 191)),
        heading: Style::default()
            .fg(Color::Rgb(224, 108, 117))
            .add_modifier(Modifier::BOLD),
        list_bullet: Style::default().fg(Color::Rgb(198, 120, 221)),
        link: Style::default()
            .fg(Color::Rgb(97, 175, 239))
            .add_modifier(Modifier::UNDERLINED),
        code: Style::default()
            .fg(Color::Rgb(229, 192, 123))
            .bg(Color::Rgb(40, 44, 52)),
        quote: Style::default()
            .fg(Color::Rgb(92, 99, 112))
            .add_modifier(Modifier::ITALIC),
        table_header: Style::default()
            .fg(Color::Rgb(86, 182, 194))
            .add_modifier(Modifier::BOLD),
        conflict_local: Style::default()
            .fg(Color::Rgb(198, 120, 221))
            .add_modifier(Modifier::BOLD),
        conflict_external: Style::default()
            .fg(Color::Rgb(152, 195, 121))
            .add_modifier(Modifier::BOLD),
        plain: Style::default().fg(Color::Rgb(171, 178, 191)),
        line_number: Style::default().fg(Color::Rgb(76, 82, 99)),
        line_number_current: Style::default()
            .fg(Color::Rgb(229, 192, 123))
            .add_modifier(Modifier::BOLD),
        scroll_indicator: Style::default()
            .fg(Color::Rgb(97, 175, 239))
            .add_modifier(Modifier::BOLD),
        emphasis: Style::default()
            .fg(Color::Rgb(171, 178, 191))
            .add_modifier(Modifier::ITALIC),
        strong: Style::default()
            .fg(Color::Rgb(220, 220, 220))
            .add_modifier(Modifier::BOLD),
        strikethrough: Style::default()
            .fg(Color::Rgb(92, 99, 112))
            .add_modifier(Modifier::CROSSED_OUT),
        hr: Style::default().fg(Color::Rgb(92, 99, 112)),
        task_done: Style::default().fg(Color::Rgb(152, 195, 121)),
        task_pending: Style::default().fg(Color::Rgb(229, 192, 123)),
    }
}

fn high_contrast_theme() -> ThemeTokens {
    ThemeTokens {
        top_bar: Style::default()
            .fg(Color::White)
            .bg(Color::Black)
            .add_modifier(Modifier::BOLD),
        status_ok: Style::default().fg(Color::Green).bg(Color::Black),
        status_warn: Style::default().fg(Color::Yellow).bg(Color::Black),
        status_error: Style::default().fg(Color::Red).bg(Color::Black),
        status_bg: Style::default().fg(Color::White).bg(Color::Black),
        status_mode: Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        status_position: Style::default()
            .fg(Color::Black)
            .bg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
        status_file: Style::default().fg(Color::White).bg(Color::DarkGray),
        status_dirty: Style::default()
            .fg(Color::Yellow)
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
        status_clean: Style::default().fg(Color::Green).bg(Color::DarkGray),
        status_separator: Style::default().fg(Color::DarkGray).bg(Color::Black),
        pane_border: Style::default().fg(Color::White),
        pane_focus: Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        help: Style::default().fg(Color::White).bg(Color::Black),
        heading: Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
        list_bullet: Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
        link: Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::UNDERLINED),
        code: Style::default().fg(Color::Yellow),
        quote: Style::default().fg(Color::Gray),
        table_header: Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
        conflict_local: Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
        conflict_external: Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
        plain: Style::default().fg(Color::White),
        line_number: Style::default().fg(Color::DarkGray),
        line_number_current: Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        scroll_indicator: Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        emphasis: Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::ITALIC),
        strong: Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
        strikethrough: Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::CROSSED_OUT),
        hr: Style::default().fg(Color::White),
        task_done: Style::default().fg(Color::Green),
        task_pending: Style::default().fg(Color::Yellow),
    }
}

fn monochrome_theme() -> ThemeTokens {
    let base = Style::default();
    ThemeTokens {
        top_bar: base.add_modifier(Modifier::BOLD),
        status_ok: base,
        status_warn: base,
        status_error: base,
        status_bg: base,
        status_mode: base
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::REVERSED),
        status_position: base.add_modifier(Modifier::BOLD),
        status_file: base,
        status_dirty: base.add_modifier(Modifier::BOLD),
        status_clean: base,
        status_separator: base,
        pane_border: base,
        pane_focus: base.add_modifier(Modifier::BOLD),
        help: base,
        heading: base.add_modifier(Modifier::BOLD),
        list_bullet: base,
        link: base.add_modifier(Modifier::UNDERLINED),
        code: base,
        quote: base,
        table_header: base.add_modifier(Modifier::BOLD),
        conflict_local: base.add_modifier(Modifier::BOLD),
        conflict_external: base.add_modifier(Modifier::BOLD),
        plain: base,
        line_number: base,
        line_number_current: base.add_modifier(Modifier::BOLD),
        scroll_indicator: base.add_modifier(Modifier::BOLD),
        emphasis: base.add_modifier(Modifier::ITALIC),
        strong: base.add_modifier(Modifier::BOLD),
        strikethrough: base.add_modifier(Modifier::CROSSED_OUT),
        hr: base,
        task_done: base,
        task_pending: base,
    }
}

#[cfg(test)]
mod tests {
    use mdv_core::SegmentKind;
    use ratatui::style::{Color, Modifier};

    use crate::app::state::ThemeChoice;

    use super::{build_theme, style_for_segment};

    #[test]
    fn no_color_theme_is_bold_for_heading() {
        let theme = build_theme(ThemeChoice::Default, true);
        let style = style_for_segment(&theme, SegmentKind::Heading);
        assert!(style.add_modifier.contains(ratatui::style::Modifier::BOLD));
    }

    #[test]
    fn auto_and_default_themes_match() {
        let auto = build_theme(ThemeChoice::Auto, false);
        let default = build_theme(ThemeChoice::Default, false);
        assert_eq!(auto.heading.fg, default.heading.fg);
        assert_eq!(auto.link.fg, default.link.fg);
    }

    #[test]
    fn high_contrast_theme_sets_bg_and_focus() {
        let theme = build_theme(ThemeChoice::HighContrast, false);
        assert_eq!(theme.top_bar.bg, Some(Color::Black));
        assert_eq!(theme.status_warn.bg, Some(Color::Black));
        assert!(theme.pane_focus.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn no_color_overrides_selected_theme() {
        let theme = build_theme(ThemeChoice::HighContrast, true);
        assert_eq!(theme.plain.fg, None);
        assert!(theme.top_bar.add_modifier.contains(Modifier::BOLD));
        assert!(theme.link.add_modifier.contains(Modifier::UNDERLINED));
    }

    #[test]
    fn style_for_segment_maps_all_kinds() {
        let theme = build_theme(ThemeChoice::Default, false);
        let kinds = [
            SegmentKind::Plain,
            SegmentKind::Heading,
            SegmentKind::ListBullet,
            SegmentKind::Link,
            SegmentKind::Code,
            SegmentKind::Quote,
            SegmentKind::TableHeader,
            SegmentKind::ConflictLocal,
            SegmentKind::ConflictExternal,
        ];

        for kind in kinds {
            let style = style_for_segment(&theme, kind);
            // All segment styles should have a color set
            assert!(style.fg.is_some() || !style.add_modifier.is_empty());
        }
    }

    #[test]
    fn default_theme_has_line_numbers_styled() {
        let theme = build_theme(ThemeChoice::Default, false);
        assert!(theme.line_number.fg.is_some());
        assert!(theme.line_number_current.fg.is_some());
        assert!(
            theme
                .line_number_current
                .add_modifier
                .contains(Modifier::BOLD)
        );
    }

    #[test]
    fn monochrome_theme_uses_modifiers_only() {
        let theme = build_theme(ThemeChoice::Default, true);
        assert!(theme.emphasis.add_modifier.contains(Modifier::ITALIC));
        assert!(theme.strong.add_modifier.contains(Modifier::BOLD));
        assert!(
            theme
                .strikethrough
                .add_modifier
                .contains(Modifier::CROSSED_OUT)
        );
    }
}
