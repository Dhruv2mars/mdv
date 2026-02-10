use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

pub fn render_preview_lines(markdown: &str, width: u16) -> Vec<String> {
    let mut lines = Vec::<String>::new();
    let mut current = String::new();
    let max_width = width.max(8) as usize;

    for event in Parser::new(markdown) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                flush_current(&mut current, &mut lines, max_width);
                current.push_str(heading_prefix(level));
            }
            Event::End(TagEnd::Heading(_)) => {
                flush_current(&mut current, &mut lines, max_width);
            }
            Event::Start(Tag::Item) => {
                flush_current(&mut current, &mut lines, max_width);
                current.push_str("- ");
            }
            Event::End(TagEnd::Item) => {
                flush_current(&mut current, &mut lines, max_width);
            }
            Event::TaskListMarker(done) => {
                current.push_str(if done { "[x] " } else { "[ ] " });
            }
            Event::Code(code) => {
                current.push('`');
                current.push_str(&code);
                current.push('`');
            }
            Event::Text(text) => {
                current.push_str(&text);
            }
            Event::SoftBreak | Event::HardBreak => {
                flush_current(&mut current, &mut lines, max_width);
            }
            _ => {}
        }
    }

    flush_current(&mut current, &mut lines, max_width);

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

fn heading_prefix(level: HeadingLevel) -> &'static str {
    match level {
        HeadingLevel::H1 => "# ",
        HeadingLevel::H2 => "## ",
        HeadingLevel::H3 => "### ",
        HeadingLevel::H4 => "#### ",
        HeadingLevel::H5 => "##### ",
        HeadingLevel::H6 => "###### ",
    }
}

fn flush_current(current: &mut String, lines: &mut Vec<String>, width: usize) {
    if current.is_empty() {
        return;
    }

    for wrapped in wrap_line(current, width) {
        lines.push(wrapped);
    }

    current.clear();
}

fn wrap_line(input: &str, width: usize) -> Vec<String> {
    if input.chars().count() <= width {
        return vec![input.to_string()];
    }

    let mut chunks = Vec::new();
    let mut buf = String::new();

    for ch in input.chars() {
        buf.push(ch);
        if buf.chars().count() == width {
            chunks.push(buf.clone());
            buf.clear();
        }
    }

    if !buf.is_empty() {
        chunks.push(buf);
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::render_preview_lines;

    #[test]
    fn renders_heading_and_list() {
        let src = "# Title\n- item";
        let lines = render_preview_lines(src, 80);
        assert_eq!(lines[0], "# Title");
        assert_eq!(lines[1], "- item");
    }

    #[test]
    fn renders_task_list_marker() {
        let src = "- [x] done\n- [ ] todo";
        let lines = render_preview_lines(src, 80);
        assert_eq!(lines[0], "- [x] done");
        assert_eq!(lines[1], "- [ ] todo");
    }

    #[test]
    fn wraps_long_lines() {
        let src = "abcdefghij";
        let lines = render_preview_lines(src, 4);
        assert_eq!(lines, vec!["abcdefgh", "ij"]);
    }
}
