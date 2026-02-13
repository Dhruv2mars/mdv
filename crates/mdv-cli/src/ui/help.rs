pub fn help_lines() -> &'static [&'static str] {
    &[
        "Ctrl+Q quit | Ctrl+S save | Ctrl+R reload",
        "Ctrl+F search | F3/F3+Shift next/prev",
        "Ctrl+H replace | Ctrl+G goto",
        "Ctrl+J/Ctrl+U hunk nav | Ctrl+E apply hunk",
        "Ctrl+K keep local | Ctrl+M merge",
        "Tab switch focus | Ctrl+/ help",
        "Alt+,/Alt+. adjust split | Ctrl+W reset split",
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
        assert!(text.contains("Ctrl+/ help"));
        assert!(text.contains("Alt+,/Alt+."));
    }
}
