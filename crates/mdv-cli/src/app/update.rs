use super::action::Action;
use super::state::{PaneFocus, UiState, clamp_split_ratio, default_split_ratio};

pub fn apply_action(ui: &mut UiState, action: Action, term_width: u16) {
    match action {
        Action::ToggleFocus => {
            ui.focus = match ui.focus {
                PaneFocus::Editor => PaneFocus::Preview,
                PaneFocus::Preview => PaneFocus::Editor,
            };
        }
        Action::ToggleHelp => ui.help_open = !ui.help_open,
        Action::AdjustSplit(delta) => {
            let next = if delta < 0 {
                ui.split_ratio.saturating_sub(delta.unsigned_abs())
            } else {
                ui.split_ratio.saturating_add(delta as u16)
            };
            ui.split_ratio = clamp_split_ratio(next);
        }
        Action::ResetSplit => ui.split_ratio = default_split_ratio(term_width),
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
    fn split_adjust_and_reset() {
        let mut ui = UiState {
            split_ratio: 50,
            ..UiState::default()
        };
        apply_action(&mut ui, Action::AdjustSplit(-50), 120);
        assert_eq!(ui.split_ratio, 30);
        apply_action(&mut ui, Action::AdjustSplit(60), 120);
        assert_eq!(ui.split_ratio, 70);

        apply_action(&mut ui, Action::ResetSplit, 140);
        assert_eq!(ui.split_ratio, 55);
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
