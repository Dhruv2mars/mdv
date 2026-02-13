use super::state::{PaneFocus, ThemeChoice};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    ToggleFocus,
    ToggleHelp,
    AdjustSplit(i16),
    ResetSplit,
    ApplyPrefs {
        focus: PaneFocus,
        theme: ThemeChoice,
        no_color: bool,
    },
}
