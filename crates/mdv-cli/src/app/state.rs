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
    pub split_ratio: u16,
    pub help_open: bool,
    pub theme: ThemeChoice,
    pub no_color: bool,
}

pub const MIN_SPLIT_RATIO: u16 = 30;
pub const MAX_SPLIT_RATIO: u16 = 70;

impl Default for UiState {
    fn default() -> Self {
        Self {
            focus: PaneFocus::Editor,
            split_ratio: 50,
            help_open: false,
            theme: ThemeChoice::Auto,
            no_color: false,
        }
    }
}

pub fn clamp_split_ratio(ratio: u16) -> u16 {
    ratio.clamp(MIN_SPLIT_RATIO, MAX_SPLIT_RATIO)
}

pub fn default_split_ratio(width: u16) -> u16 {
    if width >= 140 {
        55
    } else if width >= 100 {
        50
    } else {
        65
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_SPLIT_RATIO, MIN_SPLIT_RATIO, clamp_split_ratio, default_split_ratio};

    #[test]
    fn split_ratio_clamps_bounds() {
        assert_eq!(clamp_split_ratio(0), MIN_SPLIT_RATIO);
        assert_eq!(clamp_split_ratio(50), 50);
        assert_eq!(clamp_split_ratio(100), MAX_SPLIT_RATIO);
    }

    #[test]
    fn split_ratio_defaults_by_width_breakpoints() {
        assert_eq!(default_split_ratio(140), 55);
        assert_eq!(default_split_ratio(120), 50);
        assert_eq!(default_split_ratio(90), 65);
    }
}
