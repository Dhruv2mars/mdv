pub fn help_lines() -> &'static [&'static str] {
    &[
        "Settings",
        "Home: type path and press Enter to open",
        "Tab switch mode (editor/view)",
        "Ctrl+/ open/close settings | Esc close settings",
        "Ctrl+Q quit | Ctrl+S save | Ctrl+R reload",
        "Ctrl+F search | F3/F3+Shift next/prev",
        "Ctrl+H replace | Ctrl+G goto",
        "Ctrl+J/Ctrl+U hunk nav | Ctrl+E apply hunk",
        "Ctrl+K keep local | Ctrl+M merge",
        "Mouse wheel scrolls active mode viewport",
    ]
}

pub fn help_text() -> String {
    help_lines().join("\n")
}

#[cfg(test)]
mod tests {
    use super::help_text;

    #[test]
    fn help_mentions_new_shortcuts() {
        let text = help_text();
        assert!(text.contains("Ctrl+/"));
        assert!(!text.contains("Alt+,/Alt+."));
    }
}
