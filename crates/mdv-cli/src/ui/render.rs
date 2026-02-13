use std::borrow::Cow;

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
    use super::{compose_status, truncate_middle};

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
}
