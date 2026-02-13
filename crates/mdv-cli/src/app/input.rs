use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::action::Action;

pub fn map_global_key(key: KeyEvent) -> Option<Action> {
    match (key.code, key.modifiers) {
        (KeyCode::Tab, KeyModifiers::NONE) => Some(Action::ToggleFocus),
        (KeyCode::Char('/'), KeyModifiers::CONTROL) => Some(Action::ToggleHelp),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use crate::app::action::Action;

    use super::map_global_key;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn maps_new_ui_shortcuts() {
        assert_eq!(
            map_global_key(key(KeyCode::Tab, KeyModifiers::NONE)),
            Some(Action::ToggleFocus)
        );
        assert_eq!(
            map_global_key(key(KeyCode::Char('/'), KeyModifiers::CONTROL)),
            Some(Action::ToggleHelp)
        );
        assert_eq!(
            map_global_key(key(KeyCode::Char(','), KeyModifiers::ALT)),
            None
        );
        assert_eq!(
            map_global_key(key(KeyCode::Char('.'), KeyModifiers::ALT)),
            None
        );
        assert_eq!(
            map_global_key(key(KeyCode::Char('w'), KeyModifiers::CONTROL)),
            None
        );
    }
}
