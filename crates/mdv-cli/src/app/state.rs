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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpState {
    pub open: bool,
    pub index_focus: bool,
    pub section_idx: usize,
    pub scroll: usize,
    pub onboarding_step: Option<usize>,
}

impl Default for HelpState {
    fn default() -> Self {
        Self {
            open: false,
            index_focus: true,
            section_idx: 0,
            scroll: 0,
            onboarding_step: None,
        }
    }
}

impl HelpState {
    pub fn open_docs(&mut self) {
        self.open = true;
        self.index_focus = true;
        self.onboarding_step = None;
    }

    pub fn open_onboarding(&mut self) {
        self.open = true;
        self.index_focus = false;
        self.section_idx = 0;
        self.scroll = 0;
        self.onboarding_step = Some(0);
    }

    pub fn close(&mut self) {
        self.open = false;
        self.index_focus = true;
        self.scroll = 0;
        self.onboarding_step = None;
    }

    pub fn is_onboarding(&self) -> bool {
        self.onboarding_step.is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpNavAction {
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
    FocusLeft,
    FocusRight,
    ToggleFocus,
    OpenSection,
}

pub fn apply_help_nav(
    help: &mut HelpState,
    action: HelpNavAction,
    section_count: usize,
    content_lines: usize,
    page_lines: usize,
) {
    if section_count == 0 {
        return;
    }
    let max_section = section_count.saturating_sub(1);
    let viewport = page_lines.max(1);
    let max_scroll = content_lines.saturating_sub(viewport);

    match action {
        HelpNavAction::Up => {
            if help.index_focus {
                help.section_idx = help.section_idx.saturating_sub(1);
                help.scroll = 0;
                if help.onboarding_step.is_some() {
                    help.onboarding_step = Some(help.section_idx);
                }
            } else {
                help.scroll = help.scroll.saturating_sub(1);
            }
        }
        HelpNavAction::Down => {
            if help.index_focus {
                help.section_idx = (help.section_idx + 1).min(max_section);
                help.scroll = 0;
                if help.onboarding_step.is_some() {
                    help.onboarding_step = Some(help.section_idx);
                }
            } else {
                help.scroll = (help.scroll + 1).min(max_scroll);
            }
        }
        HelpNavAction::PageUp => {
            help.scroll = help.scroll.saturating_sub(viewport);
        }
        HelpNavAction::PageDown => {
            help.scroll = (help.scroll + viewport).min(max_scroll);
        }
        HelpNavAction::Home => help.scroll = 0,
        HelpNavAction::End => help.scroll = max_scroll,
        HelpNavAction::FocusLeft => help.index_focus = true,
        HelpNavAction::FocusRight => help.index_focus = false,
        HelpNavAction::ToggleFocus => help.index_focus = !help.index_focus,
        HelpNavAction::OpenSection => {
            help.index_focus = false;
            help.scroll = 0;
            if help.onboarding_step.is_some() {
                help.onboarding_step = Some(help.section_idx.min(max_section));
            }
        }
    }
    help.section_idx = help.section_idx.min(max_section);
    help.scroll = help.scroll.min(max_scroll);
}

#[derive(Debug, Clone)]
pub struct UiState {
    pub focus: PaneFocus,
    pub help: HelpState,
    pub theme: ThemeChoice,
    pub no_color: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            focus: PaneFocus::Editor,
            help: HelpState::default(),
            theme: ThemeChoice::Auto,
            no_color: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{HelpNavAction, HelpState, PaneFocus, ThemeChoice, UiState, apply_help_nav};

    #[test]
    fn default_ui_state_prefers_editor_mode() {
        let ui = UiState::default();
        assert_eq!(ui.focus, PaneFocus::Editor);
        assert_eq!(ui.theme, ThemeChoice::Auto);
        assert!(!ui.no_color);
        assert!(!ui.help.open);
    }

    #[test]
    fn help_nav_changes_index_and_scroll_with_bounds() {
        let mut help = HelpState::default();
        help.open = true;
        help.section_idx = 1;
        help.scroll = 8;
        apply_help_nav(&mut help, HelpNavAction::Up, 4, 100, 10);
        assert_eq!(help.section_idx, 0);
        assert_eq!(help.scroll, 0);

        apply_help_nav(&mut help, HelpNavAction::Up, 4, 100, 10);
        assert_eq!(help.section_idx, 0);

        apply_help_nav(&mut help, HelpNavAction::Down, 4, 100, 10);
        apply_help_nav(&mut help, HelpNavAction::Down, 4, 100, 10);
        apply_help_nav(&mut help, HelpNavAction::Down, 4, 100, 10);
        apply_help_nav(&mut help, HelpNavAction::Down, 4, 100, 10);
        assert_eq!(help.section_idx, 3);
        assert_eq!(help.scroll, 0);
    }

    #[test]
    fn help_nav_scrolls_content_and_switches_focus() {
        let mut help = HelpState::default();
        help.open = true;
        help.index_focus = false;

        apply_help_nav(&mut help, HelpNavAction::Down, 3, 24, 10);
        apply_help_nav(&mut help, HelpNavAction::Down, 3, 24, 10);
        assert_eq!(help.scroll, 2);

        apply_help_nav(&mut help, HelpNavAction::PageDown, 3, 24, 10);
        assert_eq!(help.scroll, 12);
        apply_help_nav(&mut help, HelpNavAction::PageDown, 3, 24, 10);
        assert_eq!(help.scroll, 14);
        apply_help_nav(&mut help, HelpNavAction::End, 3, 24, 10);
        assert_eq!(help.scroll, 14);
        apply_help_nav(&mut help, HelpNavAction::Home, 3, 24, 10);
        assert_eq!(help.scroll, 0);

        apply_help_nav(&mut help, HelpNavAction::FocusLeft, 3, 24, 10);
        assert!(help.index_focus);
        apply_help_nav(&mut help, HelpNavAction::FocusRight, 3, 24, 10);
        assert!(!help.index_focus);
        apply_help_nav(&mut help, HelpNavAction::ToggleFocus, 3, 24, 10);
        assert!(help.index_focus);
    }

    #[test]
    fn help_nav_open_section_resets_scroll_and_syncs_onboarding() {
        let mut help = HelpState::default();
        help.open_onboarding();
        help.index_focus = true;
        help.section_idx = 2;
        help.scroll = 7;

        apply_help_nav(&mut help, HelpNavAction::OpenSection, 4, 50, 10);
        assert!(!help.index_focus);
        assert_eq!(help.scroll, 0);
        assert_eq!(help.onboarding_step, Some(2));
    }
}
