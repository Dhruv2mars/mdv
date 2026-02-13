use super::action::Action;
use super::state::{PaneFocus, UiState};

pub fn apply_action(ui: &mut UiState, action: Action, _term_width: u16) {
    match action {
        Action::ToggleFocus => {
            ui.focus = match ui.focus {
                PaneFocus::Editor => PaneFocus::Preview,
                PaneFocus::Preview => PaneFocus::Editor,
            };
        }
        Action::ToggleHelp => ui.help_open = !ui.help_open,
        Action::ApplyPrefs {
            focus,
            theme,
            no_color,
        } => {
            ui.focus = focus;
            ui.theme = theme;
            ui.no_color = no_color;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::action::Action;
    use crate::app::state::{PaneFocus, ThemeChoice, UiState};

    use super::apply_action;

    #[test]
    fn toggles_focus_and_help() {
        let mut ui = UiState::default();
        apply_action(&mut ui, Action::ToggleFocus, 120);
        assert_eq!(ui.focus, PaneFocus::Preview);

        apply_action(&mut ui, Action::ToggleHelp, 120);
        assert!(ui.help_open);
    }

    #[test]
    fn set_theme_focus_and_no_color() {
        let mut ui = UiState::default();
        apply_action(
            &mut ui,
            Action::ApplyPrefs {
                focus: PaneFocus::Preview,
                theme: ThemeChoice::HighContrast,
                no_color: true,
            },
            120,
        );

        assert_eq!(ui.theme, ThemeChoice::HighContrast);
        assert!(ui.no_color);
        assert_eq!(ui.focus, PaneFocus::Preview);
    }
}
