#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneFocus {
    Editor,
    Preview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeChoice {
    Auto,
    Default,
    HighContrast,
}

#[derive(Debug, Clone)]
pub struct UiState {
    pub focus: PaneFocus,
    pub help_open: bool,
    pub theme: ThemeChoice,
    pub no_color: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            focus: PaneFocus::Editor,
            help_open: false,
            theme: ThemeChoice::Auto,
            no_color: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PaneFocus, ThemeChoice, UiState};

    #[test]
    fn default_ui_state_prefers_editor_mode() {
        let ui = UiState::default();
        assert_eq!(ui.focus, PaneFocus::Editor);
        assert_eq!(ui.theme, ThemeChoice::Auto);
        assert!(!ui.no_color);
        assert!(!ui.help_open);
    }
}
