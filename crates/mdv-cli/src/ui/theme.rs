use mdv_core::SegmentKind;
use ratatui::style::{Color, Modifier, Style};

use crate::app::state::ThemeChoice;

#[derive(Debug, Clone)]
pub struct ThemeTokens {
    pub top_bar: Style,
    pub status_ok: Style,
    pub status_warn: Style,
    pub status_error: Style,
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
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        status_ok: Style::default().fg(Color::Green),
        status_warn: Style::default().fg(Color::Yellow),
        status_error: Style::default().fg(Color::Red),
        pane_border: Style::default().fg(Color::DarkGray),
        pane_focus: Style::default()
            .fg(Color::LightBlue)
            .add_modifier(Modifier::BOLD),
        help: Style::default().fg(Color::White),
        heading: Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        list_bullet: Style::default().fg(Color::Magenta),
        link: Style::default()
            .fg(Color::LightBlue)
            .add_modifier(Modifier::UNDERLINED),
        code: Style::default().fg(Color::LightYellow),
        quote: Style::default().fg(Color::Gray),
        table_header: Style::default()
            .fg(Color::LightCyan)
            .add_modifier(Modifier::BOLD),
        conflict_local: Style::default().fg(Color::LightMagenta),
        conflict_external: Style::default().fg(Color::LightGreen),
        plain: Style::default().fg(Color::White),
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
    }
}

fn monochrome_theme() -> ThemeTokens {
    let base = Style::default();
    ThemeTokens {
        top_bar: base.add_modifier(Modifier::BOLD),
        status_ok: base,
        status_warn: base,
        status_error: base,
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
            match kind {
                SegmentKind::Heading => assert_eq!(style.fg, Some(Color::Cyan)),
                SegmentKind::Link => assert_eq!(style.fg, Some(Color::LightBlue)),
                SegmentKind::Code => assert_eq!(style.fg, Some(Color::LightYellow)),
                _ => assert!(style.fg.is_some() || !style.add_modifier.is_empty()),
            }
        }
    }
}
