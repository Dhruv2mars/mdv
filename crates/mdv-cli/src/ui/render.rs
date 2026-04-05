use std::borrow::Cow;

use ratatui::text::{Line, Span};

use crate::ui::theme::ThemeTokens;

/// Configuration for the styled status bar
#[allow(dead_code)]
pub struct StatusBarConfig<'a> {
    pub mode: &'a str,
    pub filename: &'a str,
    pub dirty: bool,
    pub readonly: bool,
    pub line: usize,
    pub col: usize,
    pub total_lines: usize,
    pub scroll_percent: u8,
    pub message: &'a str,
    pub hint: &'a str,
    pub is_error: bool,
    pub is_warning: bool,
    pub width: usize,
}

/// Build a styled status bar line with multiple visual segments
pub fn build_status_bar<'a>(config: &StatusBarConfig<'_>, theme: &ThemeTokens) -> Line<'a> {
    let mut spans: Vec<Span<'a>> = Vec::new();

    // Mode badge (e.g., NORMAL, SEARCH, INSERT)
    let mode_text = format!(" {} ", config.mode.to_uppercase());
    spans.push(Span::styled(mode_text, theme.status_mode));

    // Separator
    spans.push(Span::styled(" ", theme.status_bg));

    // File status (dirty/clean indicator + RO badge)
    if config.readonly {
        spans.push(Span::styled(" RO ", theme.status_warn));
    } else if config.dirty {
        spans.push(Span::styled(" [+] ", theme.status_dirty));
    } else {
        spans.push(Span::styled(" [-] ", theme.status_clean));
    }

    // Filename (truncated if needed)
    let available_for_filename = config.width.saturating_sub(50).max(10);
    let display_name = truncate_middle(config.filename, available_for_filename);
    spans.push(Span::styled(
        format!(" {} ", display_name),
        theme.status_file,
    ));

    spans.push(Span::styled(" ", theme.status_bg));

    // Message area (with appropriate color for status)
    let message_style = if config.is_error {
        theme.status_error
    } else if config.is_warning {
        theme.status_warn
    } else {
        theme.status_ok
    };

    if !config.message.is_empty() {
        spans.push(Span::styled(config.message.to_string(), message_style));
    }

    // Calculate used width so far
    let used_width: usize = spans.iter().map(|s| s.content.chars().count()).sum();

    // Right side: position info
    let position_text = format!(" Ln {}, Col {} ", config.line + 1, config.col + 1);
    let scroll_text = format!(" {}% ", config.scroll_percent);
    let right_width = position_text.chars().count() + scroll_text.chars().count() + 1;

    // Fill middle with spaces
    let fill_width = config.width.saturating_sub(used_width + right_width);
    if fill_width > 0 {
        spans.push(Span::styled(" ".repeat(fill_width), theme.status_bg));
    }

    // Position indicator
    spans.push(Span::styled(position_text, theme.status_file));
    spans.push(Span::styled(scroll_text, theme.status_position));

    Line::from(spans)
}

pub fn truncate_middle<'a>(value: &'a str, max_chars: usize) -> Cow<'a, str> {
    if value.chars().count() <= max_chars {
        return Cow::Borrowed(value);
    }

    if max_chars <= 3 {
        return Cow::Owned("...".chars().take(max_chars).collect());
    }

    let keep = max_chars - 3;
    let left = keep / 2;
    let right = keep - left;
    let start: String = value.chars().take(left).collect();
    let end: String = value
        .chars()
        .rev()
        .take(right)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    Cow::Owned(format!("{start}...{end}"))
}

pub fn compose_status(left: &str, right: &str, width: usize) -> String {
    let left_chars = left.chars().count();
    let right_chars = right.chars().count();

    if right.is_empty() || left_chars + 1 + right_chars >= width {
        return left.to_string();
    }

    let spaces = width.saturating_sub(left_chars + right_chars);
    format!("{left}{}{}", " ".repeat(spaces), right)
}

#[cfg(test)]
mod tests {
    use crate::app::ThemeChoice;
    use crate::ui::theme::build_theme;

    use super::{StatusBarConfig, build_status_bar, compose_status, truncate_middle};

    #[test]
    fn truncates_middle() {
        let got = truncate_middle("/very/long/path/to/file.md", 12);
        assert_eq!(got.chars().count(), 12);
        assert!(got.contains("..."));
    }

    #[test]
    fn composes_right_hint_when_space_exists() {
        let out = compose_status("left", "right", 20);
        assert!(out.starts_with("left"));
        assert!(out.ends_with("right"));
    }

    #[test]
    fn truncate_middle_handles_short_caps() {
        assert_eq!(truncate_middle("abcdef", 3), "...");
        assert_eq!(truncate_middle("abcdef", 2), "..");
    }

    #[test]
    fn truncate_middle_returns_borrowed_when_no_truncation() {
        let out = truncate_middle("abc", 10);
        assert_eq!(out, "abc");
        assert!(matches!(out, std::borrow::Cow::Borrowed(_)));
    }

    #[test]
    fn compose_status_returns_left_when_no_space_or_empty_right() {
        assert_eq!(compose_status("left", "", 20), "left");
        assert_eq!(compose_status("left", "right", 8), "left");
    }

    #[test]
    fn build_status_bar_creates_line_with_mode_badge() {
        let theme = build_theme(ThemeChoice::Default, false);
        let config = StatusBarConfig {
            mode: "normal",
            filename: "test.md",
            dirty: false,
            readonly: false,
            line: 0,
            col: 0,
            total_lines: 10,
            scroll_percent: 0,
            message: "Ready",
            hint: "",
            is_error: false,
            is_warning: false,
            width: 80,
        };
        let line = build_status_bar(&config, &theme);
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains("NORMAL"));
        assert!(text.contains("test.md"));
        assert!(text.contains("Ln 1"));
    }

    #[test]
    fn build_status_bar_shows_dirty_indicator() {
        let theme = build_theme(ThemeChoice::Default, false);
        let config = StatusBarConfig {
            mode: "normal",
            filename: "test.md",
            dirty: true,
            readonly: false,
            line: 5,
            col: 10,
            total_lines: 100,
            scroll_percent: 50,
            message: "",
            hint: "",
            is_error: false,
            is_warning: false,
            width: 100,
        };
        let line = build_status_bar(&config, &theme);
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains("[+]"));
        assert!(text.contains("Ln 6"));
        assert!(text.contains("Col 11"));
        assert!(text.contains("50%"));
    }

    #[test]
    fn build_status_bar_shows_readonly_badge() {
        let theme = build_theme(ThemeChoice::Default, false);
        let config = StatusBarConfig {
            mode: "normal",
            filename: "test.md",
            dirty: false,
            readonly: true,
            line: 0,
            col: 0,
            total_lines: 10,
            scroll_percent: 0,
            message: "",
            hint: "",
            is_error: false,
            is_warning: false,
            width: 80,
        };
        let line = build_status_bar(&config, &theme);
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains("RO"));
    }
}
