use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::ui::theme::ThemeTokens;

#[derive(Debug, Clone, Copy)]
pub struct DocSection {
    pub id: &'static str,
    pub title: &'static str,
    pub body: &'static str,
}

#[derive(Debug)]
pub struct DocCatalog {
    pub title: &'static str,
    pub sections: &'static [DocSection],
}

static USER_DOC_SECTIONS: [DocSection; 11] = [
    DocSection {
        id: "welcome",
        title: "Welcome + Mental Model",
        body: include_str!("docs/01-welcome.md"),
    },
    DocSection {
        id: "quickstart",
        title: "Quickstart",
        body: include_str!("docs/02-quickstart.md"),
    },
    DocSection {
        id: "modes",
        title: "Editor vs View",
        body: include_str!("docs/03-modes.md"),
    },
    DocSection {
        id: "editing",
        title: "Editing Fundamentals",
        body: include_str!("docs/04-editing.md"),
    },
    DocSection {
        id: "selection",
        title: "Selection + Deletion",
        body: include_str!("docs/05-selection.md"),
    },
    DocSection {
        id: "search",
        title: "Search + Replace + Goto",
        body: include_str!("docs/06-search.md"),
    },
    DocSection {
        id: "mouse",
        title: "Mouse Behavior",
        body: include_str!("docs/07-mouse.md"),
    },
    DocSection {
        id: "conflicts",
        title: "Conflict Workflow",
        body: include_str!("docs/08-conflicts.md"),
    },
    DocSection {
        id: "cli",
        title: "CLI Usage",
        body: include_str!("docs/09-cli.md"),
    },
    DocSection {
        id: "troubleshooting",
        title: "Troubleshooting + FAQ",
        body: include_str!("docs/10-troubleshooting.md"),
    },
    DocSection {
        id: "settings",
        title: "Settings + Theme",
        body: include_str!("docs/11-settings.md"),
    },
];

static USER_DOCS: DocCatalog = DocCatalog {
    title: "Docs + Settings",
    sections: &USER_DOC_SECTIONS,
};

static ONBOARDING_SECTIONS: [DocSection; 4] = [
    DocSection {
        id: "onboarding-welcome",
        title: "1. Welcome",
        body: "# Welcome\n\nThis quick guide shows core mdv flow.\n\n- Continue with Enter\n- Exit onboarding with Esc",
    },
    DocSection {
        id: "onboarding-open",
        title: "2. Open File",
        body: "# Open a File\n\nFrom Home, type a path then press `Enter`.\n\n> Missing path creates a new file on first save.",
    },
    DocSection {
        id: "onboarding-modes",
        title: "3. Modes + Keys",
        body: "# Modes + Keys\n\n- Toggle editor/view: `Shift+Tab`\n- Save: `Ctrl+S`\n- Search: `Ctrl+F`\n- Quit: `Ctrl+Q`",
    },
    DocSection {
        id: "onboarding-reopen",
        title: "4. Reopen Docs",
        body: "# Reopen Docs Anytime\n\nUse `Cmd+,` on macOS or `Ctrl+,` on Windows/Linux.\n\n> This guide auto-shows once on Home.",
    },
];

static ONBOARDING_DOCS: DocCatalog = DocCatalog {
    title: "First-Run Guide",
    sections: &ONBOARDING_SECTIONS,
};

pub fn user_docs_catalog() -> &'static DocCatalog {
    &USER_DOCS
}

pub fn onboarding_catalog() -> &'static DocCatalog {
    &ONBOARDING_DOCS
}

pub fn section_count(catalog: &DocCatalog) -> usize {
    catalog.sections.len()
}

pub fn section(catalog: &DocCatalog, idx: usize) -> DocSection {
    catalog.sections[idx.min(catalog.sections.len().saturating_sub(1))]
}

pub fn section_line_count(section: &DocSection) -> usize {
    section.body.lines().count().max(1)
}

pub fn render_section(section: &DocSection, theme: &ThemeTokens) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    let mut in_code_block = false;

    for raw in section.body.lines() {
        let line = raw.trim_end();

        if line.starts_with("```") {
            in_code_block = !in_code_block;
            out.push(Line::from(Span::styled(line.to_string(), theme.code)));
            continue;
        }

        if line.is_empty() {
            out.push(Line::from(Span::raw(String::new())));
            continue;
        }

        if in_code_block {
            out.push(Line::from(Span::styled(line.to_string(), theme.code)));
            continue;
        }

        if let Some(rest) = line.strip_prefix("# ") {
            out.push(Line::from(Span::styled(rest.to_string(), theme.heading)));
            continue;
        }

        if let Some(rest) = line.strip_prefix("## ") {
            out.push(Line::from(Span::styled(rest.to_string(), theme.heading)));
            continue;
        }

        if let Some(rest) = line.strip_prefix("> ") {
            out.push(Line::from(inline_code_spans(rest, theme.quote, theme.code)));
            continue;
        }

        if let Some((bullet, rest)) = parse_bullet(line) {
            let mut spans = Vec::new();
            spans.push(Span::styled(bullet, theme.list_bullet));
            spans.extend(inline_code_spans(rest, theme.plain, theme.code));
            out.push(Line::from(spans));
            continue;
        }

        out.push(Line::from(inline_code_spans(line, theme.plain, theme.code)));
    }

    if out.is_empty() {
        out.push(Line::from(Span::raw(String::new())));
    }

    out
}

fn parse_bullet(line: &str) -> Option<(String, &str)> {
    if let Some(rest) = line.strip_prefix("- ") {
        return Some(("- ".to_string(), rest));
    }
    let (lhs, rhs) = line.split_once(". ")?;
    if lhs.chars().all(|c| c.is_ascii_digit()) {
        Some((format!("{lhs}. "), rhs))
    } else {
        None
    }
}

fn inline_code_spans(text: &str, plain_style: Style, code_style: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut current = String::new();
    let mut in_code = false;

    for ch in text.chars() {
        if ch == '`' {
            if !current.is_empty() {
                let style = if in_code { code_style } else { plain_style };
                spans.push(Span::styled(std::mem::take(&mut current), style));
            }
            in_code = !in_code;
            continue;
        }
        current.push(ch);
    }

    if !current.is_empty() {
        let style = if in_code { code_style } else { plain_style };
        spans.push(Span::styled(current, style));
    }

    if spans.is_empty() {
        spans.push(Span::styled(String::new(), plain_style));
    }

    spans
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::app::ThemeChoice;
    use crate::ui::theme::build_theme;

    use super::{
        onboarding_catalog, render_section, section, section_count, user_docs_catalog,
    };

    #[test]
    fn user_catalog_has_expected_sections() {
        let docs = user_docs_catalog();
        assert_eq!(docs.title, "Docs + Settings");
        assert_eq!(section_count(docs), 11);
        assert_eq!(docs.sections[0].id, "welcome");
        assert_eq!(docs.sections[10].id, "settings");
    }

    #[test]
    fn section_ids_unique_and_non_empty() {
        let docs = user_docs_catalog();
        let mut ids = HashSet::new();
        for sec in docs.sections {
            assert!(!sec.id.is_empty());
            assert!(!sec.title.is_empty());
            assert!(!sec.body.trim().is_empty());
            assert!(ids.insert(sec.id));
        }
    }

    #[test]
    fn onboarding_catalog_has_four_steps() {
        let docs = onboarding_catalog();
        assert_eq!(docs.title, "First-Run Guide");
        assert_eq!(section_count(docs), 4);
        assert_eq!(docs.sections[0].title, "1. Welcome");
        assert_eq!(docs.sections[3].title, "4. Reopen Docs");
    }

    #[test]
    fn section_accessor_clamps_bounds() {
        let docs = user_docs_catalog();
        let sec = section(docs, 999);
        assert_eq!(sec.id, "settings");
    }

    #[test]
    fn renderer_supports_heading_bullet_callout_and_inline_code() {
        let theme = build_theme(ThemeChoice::Default, false);
        let section = super::DocSection {
            id: "t",
            title: "t",
            body: "# Head\n- item `code`\n> note\nplain `x`",
        };

        let lines = render_section(&section, &theme);
        assert_eq!(lines[0].spans[0].content.as_ref(), "Head");
        assert_eq!(lines[0].spans[0].style.fg, theme.heading.fg);
        assert_eq!(lines[1].spans[0].content.as_ref(), "- ");
        assert_eq!(lines[1].spans[0].style.fg, theme.list_bullet.fg);
        assert_eq!(lines[1].spans[2].content.as_ref(), "code");
        assert_eq!(lines[1].spans[2].style.fg, theme.code.fg);
        assert_eq!(lines[2].spans[0].style.fg, theme.quote.fg);
        assert_eq!(lines[3].spans[1].style.fg, theme.code.fg);
    }
}
