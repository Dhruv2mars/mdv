use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

#[derive(Debug, Clone)]
struct ListState {
    ordered: bool,
    next_index: usize,
}

#[derive(Debug, Clone)]
enum LinkState {
    Link { text: String, dest: String },
    Image { alt: String, dest: String },
}

struct Renderer {
    width: usize,
    lines: Vec<String>,
    current: String,
    blockquote_depth: usize,
    pending_prefix: Option<String>,
    list_stack: Vec<ListState>,
    link_stack: Vec<LinkState>,
    in_code_block: bool,
    table_head_needs_separator: bool,
    in_table_cell: bool,
    table_row: Vec<String>,
    table_cell: String,
}

impl Renderer {
    fn new(width: usize) -> Self {
        Self {
            width,
            lines: Vec::new(),
            current: String::new(),
            blockquote_depth: 0,
            pending_prefix: None,
            list_stack: Vec::new(),
            link_stack: Vec::new(),
            in_code_block: false,
            table_head_needs_separator: false,
            in_table_cell: false,
            table_row: Vec::new(),
            table_cell: String::new(),
        }
    }

    fn quote_prefix(&self) -> String {
        "> ".repeat(self.blockquote_depth)
    }

    fn take_line_prefix(&mut self) -> String {
        let mut prefix = self.quote_prefix();
        if let Some(item_prefix) = self.pending_prefix.take() {
            prefix.push_str(&item_prefix);
        }
        prefix
    }

    fn append_text(&mut self, text: &str) {
        if let Some(link) = self.link_stack.last_mut() {
            match link {
                LinkState::Link { text: inner, .. } => inner.push_str(text),
                LinkState::Image { alt, .. } => alt.push_str(text),
            }
            return;
        }

        if self.in_table_cell {
            self.table_cell.push_str(text);
            return;
        }

        if self.current.is_empty() {
            let prefix = self.take_line_prefix();
            self.current.push_str(&prefix);
        }
        self.current.push_str(text);
    }

    fn flush_current(&mut self) {
        if self.current.is_empty() {
            return;
        }
        let line = std::mem::take(&mut self.current);
        for wrapped in wrap_line(&line, self.width) {
            self.lines.push(wrapped);
        }
    }

    fn push_line(&mut self, line: String) {
        self.flush_current();
        for wrapped in wrap_line(&line, self.width) {
            self.lines.push(wrapped);
        }
    }

    fn push_code_text(&mut self, text: &str) {
        let mut chunks = text.split('\n').peekable();
        while let Some(chunk) = chunks.next() {
            if chunk.is_empty() && chunks.peek().is_none() {
                break;
            }
            let mut line = self.quote_prefix();
            line.push_str(chunk);
            self.push_line(line);
        }
    }
}

pub fn render_preview_lines(markdown: &str, width: u16) -> Vec<String> {
    let max_width = width.max(8) as usize;
    let mut renderer = Renderer::new(max_width);
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_MATH);

    for event in Parser::new_ext(markdown, options) {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    renderer.flush_current();
                    renderer.append_text(heading_prefix(level));
                }
                Tag::BlockQuote(_) => {
                    renderer.flush_current();
                    renderer.blockquote_depth += 1;
                }
                Tag::List(start) => {
                    renderer.flush_current();
                    renderer.list_stack.push(ListState {
                        ordered: start.is_some(),
                        next_index: start.unwrap_or(1) as usize,
                    });
                }
                Tag::Item => {
                    renderer.flush_current();
                    let list = renderer
                        .list_stack
                        .last_mut()
                        .expect("pulldown-cmark item outside list");
                    let bullet = if list.ordered {
                        let p = format!("{}. ", list.next_index);
                        list.next_index += 1;
                        p
                    } else {
                        "- ".to_string()
                    };
                    let depth = renderer.list_stack.len().saturating_sub(1);
                    let prefix = format!("{}{bullet}", "  ".repeat(depth));
                    renderer.pending_prefix = Some(prefix);
                }
                Tag::CodeBlock(kind) => {
                    renderer.flush_current();
                    renderer.in_code_block = true;
                    let mut fence = renderer.quote_prefix();
                    fence.push_str("```");
                    if let CodeBlockKind::Fenced(lang) = kind {
                        let lang = lang.trim();
                        if !lang.is_empty() {
                            fence.push_str(lang);
                        }
                    }
                    renderer.push_line(fence);
                }
                Tag::Table(_) => {
                    renderer.flush_current();
                }
                Tag::TableHead => {
                    renderer.flush_current();
                    renderer.table_head_needs_separator = true;
                }
                Tag::TableRow => {
                    renderer.flush_current();
                    renderer.table_row.clear();
                }
                Tag::TableCell => {
                    renderer.in_table_cell = true;
                    renderer.table_cell.clear();
                }
                Tag::Link { dest_url, .. } => {
                    renderer.link_stack.push(LinkState::Link {
                        text: String::new(),
                        dest: dest_url.to_string(),
                    });
                }
                Tag::Image { dest_url, .. } => {
                    renderer.link_stack.push(LinkState::Image {
                        alt: String::new(),
                        dest: dest_url.to_string(),
                    });
                }
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Heading(_) | TagEnd::Paragraph => renderer.flush_current(),
                TagEnd::BlockQuote(_) => {
                    renderer.flush_current();
                    renderer.blockquote_depth = renderer.blockquote_depth.saturating_sub(1);
                }
                TagEnd::List(_) => {
                    renderer.flush_current();
                    renderer.list_stack.pop();
                }
                TagEnd::Item => renderer.flush_current(),
                TagEnd::CodeBlock => {
                    renderer.flush_current();
                    renderer.in_code_block = false;
                    let mut fence = renderer.quote_prefix();
                    fence.push_str("```");
                    renderer.push_line(fence);
                }
                TagEnd::TableHead => {
                    if renderer.table_head_needs_separator && !renderer.table_row.is_empty() {
                        let row = format!("| {} |", renderer.table_row.join(" | "));
                        let mut line = renderer.quote_prefix();
                        line.push_str(&row);
                        renderer.push_line(line);

                        let sep =
                            format!("| {} |", vec!["-"; renderer.table_row.len()].join(" | "));
                        let mut sep_line = renderer.quote_prefix();
                        sep_line.push_str(&sep);
                        renderer.push_line(sep_line);

                        renderer.table_row.clear();
                        renderer.table_head_needs_separator = false;
                    }
                }
                TagEnd::TableRow => {
                    let row = format!("| {} |", renderer.table_row.join(" | "));
                    let mut line = renderer.quote_prefix();
                    line.push_str(&row);
                    renderer.push_line(line);
                }
                TagEnd::TableCell => {
                    renderer.in_table_cell = false;
                    renderer.table_row.push(renderer.table_cell.clone());
                    renderer.table_cell.clear();
                }
                TagEnd::Link => {
                    if let Some(LinkState::Link { text, dest }) = renderer.link_stack.pop() {
                        renderer.append_text(&format!("[{text}]({dest})"));
                    }
                }
                TagEnd::Image => {
                    if let Some(LinkState::Image { alt, dest }) = renderer.link_stack.pop() {
                        renderer.append_text(&format!("![{alt}]({dest})"));
                    }
                }
                _ => {}
            },
            Event::Text(text) => {
                if renderer.in_code_block {
                    renderer.push_code_text(&text);
                } else {
                    renderer.append_text(&text);
                }
            }
            Event::Code(code) => renderer.append_text(&format!("`{code}`")),
            Event::SoftBreak => {
                if renderer.link_stack.last().is_some() {
                    renderer.append_text(" ");
                } else {
                    renderer.flush_current();
                }
            }
            Event::HardBreak => renderer.flush_current(),
            Event::TaskListMarker(done) => renderer.append_text(if done { "[x] " } else { "[ ] " }),
            Event::Rule => {
                renderer.flush_current();
                let mut line = renderer.quote_prefix();
                line.push_str("---");
                renderer.push_line(line);
            }
            Event::Html(html) | Event::InlineHtml(html) => renderer.append_text(&html),
            Event::FootnoteReference(name) => renderer.append_text(&name),
            Event::InlineMath(math) | Event::DisplayMath(math) => renderer.append_text(&math),
        }
    }

    renderer.flush_current();
    if renderer.lines.is_empty() {
        renderer.lines.push(String::new());
    }
    renderer.lines
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

    #[test]
    fn renders_blockquote_and_ordered_list() {
        let src = "> quoted\n\n1. first\n2. second";
        let lines = render_preview_lines(src, 80);
        assert_eq!(lines[0], "> quoted");
        assert_eq!(lines[1], "1. first");
        assert_eq!(lines[2], "2. second");
    }

    #[test]
    fn renders_nested_lists_with_indent() {
        let src = "- parent\n  - child\n\n> - q1\n>   - q2";
        let lines = render_preview_lines(src, 80);
        assert_eq!(lines[0], "- parent");
        assert_eq!(lines[1], "  - child");
        assert_eq!(lines[2], "> - q1");
        assert_eq!(lines[3], ">   - q2");
    }

    #[test]
    fn preserves_ordered_list_start_index() {
        let src = "3. three\n4. four";
        let lines = render_preview_lines(src, 80);
        assert_eq!(lines[0], "3. three");
        assert_eq!(lines[1], "4. four");
    }

    #[test]
    fn renders_link_image_and_thematic_break() {
        let src = "[site](https://example.com)\n\n![alt](img.png)\n\n---";
        let lines = render_preview_lines(src, 80);
        assert_eq!(lines[0], "[site](https://example.com)");
        assert_eq!(lines[1], "![alt](img.png)");
        assert_eq!(lines[2], "---");
    }

    #[test]
    fn renders_fenced_code_and_table_rows() {
        let src = "```rs\nlet x = 1;\n```\n\n| a | b |\n| - | - |\n| 1 | 2 |";
        let lines = render_preview_lines(src, 80);
        assert_eq!(lines[0], "```rs");
        assert_eq!(lines[1], "let x = 1;");
        assert_eq!(lines[2], "```");
        assert_eq!(lines[3], "| a | b |");
        assert_eq!(lines[4], "| - | - |");
        assert_eq!(lines[5], "| 1 | 2 |");
    }

    #[test]
    fn renders_inline_code_and_strike_text_fallback() {
        let src = "a `code` and ~~gone~~";
        let lines = render_preview_lines(src, 80);
        assert_eq!(lines, vec!["a `code` and gone"]);
    }

    #[test]
    fn renders_empty_input_as_single_blank_line() {
        let lines = render_preview_lines("", 80);
        assert_eq!(lines, vec![String::new()]);
    }

    #[test]
    fn renders_hard_break_softbreak_in_link_and_html() {
        let src = "[line1\nline2](https://x)\nA\\\nB\n<div>x</div>";
        let lines = render_preview_lines(src, 80);
        assert_eq!(lines[0], "[line1 line2](https://x)");
        assert_eq!(lines[1], "A");
        assert_eq!(lines[2], "B");
        assert_eq!(lines[3], "<div>x</div>");
    }

    #[test]
    fn renders_indented_code_and_heading_levels() {
        let src = "## h2\n### h3\n#### h4\n##### h5\n###### h6\n\n    code";
        let lines = render_preview_lines(src, 80);
        assert_eq!(lines[0], "## h2");
        assert_eq!(lines[1], "### h3");
        assert_eq!(lines[2], "#### h4");
        assert_eq!(lines[3], "##### h5");
        assert_eq!(lines[4], "###### h6");
        assert_eq!(lines[5], "```");
        assert_eq!(lines[6], "code");
        assert_eq!(lines[7], "```");
    }

    #[test]
    fn renders_footnote_and_math_events() {
        let src = "x[^n]\n\n[^n]: note\n\n$y$";
        let lines = render_preview_lines(src, 80);
        assert!(lines.join("\n").contains("note"));
        assert!(lines.join("\n").contains("y"));
    }
}
