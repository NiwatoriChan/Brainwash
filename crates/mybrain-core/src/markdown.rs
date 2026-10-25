//! A small, line-oriented Markdown parser used by the preview renderer.
//!
//! It deliberately supports only the subset people use daily in notes
//! (headings, lists, tasks, quotes, code blocks, rules, emphasis, inline
//! code, URLs, `#tags` and `[[wiki links]]`). Keeping it here — free of any
//! UI types — makes the renderer trivial and the parsing unit-testable.

use crate::frontmatter::Frontmatter;
use crate::links::{parse_links, WikiLink};

/// List item marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListMarker {
    Bullet,
    Ordered(String),
    Task { checked: bool },
}

/// A block-level element. Each block knows the (0-based) source line it comes
/// from so the UI can map interactions (e.g. ticking a task) back to the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block<'a> {
    Heading { level: u8, text: &'a str },
    Paragraph(&'a str),
    ListItem { indent: usize, marker: ListMarker, text: &'a str },
    Quote(&'a str),
    Code { lang: &'a str, code: String },
    Rule,
    Blank,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineBlock<'a> {
    pub line: usize,
    pub block: Block<'a>,
}

/// Inline span inside a block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inline<'a> {
    Text(&'a str),
    Bold(&'a str),
    Italic(&'a str),
    Code(&'a str),
    Url(&'a str),
    Tag(&'a str),
    Link(WikiLink),
    Image { alt: &'a str, url: &'a str },
}

/// Parses the body of a note (front-matter is skipped, but line numbers are
/// still relative to the full `content`).
pub fn parse_blocks(content: &str) -> Vec<LineBlock<'_>> {
    let (_, body) = Frontmatter::parse(content);
    let skipped_lines = content[..content.len() - body.len()].lines().count();

    let mut blocks = Vec::new();
    let mut lines = body.lines().enumerate().peekable();

    while let Some((i, line)) = lines.next() {
        let line_no = i + skipped_lines;
        let trimmed = line.trim_start();

        if let Some(lang) = trimmed.strip_prefix("```") {
            let mut code = String::new();
            for (_, l) in lines.by_ref() {
                if l.trim_start().starts_with("```") {
                    break;
                }
                code.push_str(l);
                code.push('\n');
            }
            blocks.push(LineBlock {
                line: line_no,
                block: Block::Code { lang: lang.trim(), code },
            });
            continue;
        }

        blocks.push(LineBlock {
            line: line_no,
            block: parse_line(line),
        });
    }
    blocks
}

fn parse_line(line: &str) -> Block<'_> {
    let trimmed = line.trim_start();
    let indent = (line.len() - trimmed.len()) / 2;

    if trimmed.is_empty() {
        return Block::Blank;
    }
    if is_rule(trimmed) {
        return Block::Rule;
    }
    if let Some(block) = parse_heading(trimmed) {
        return block;
    }
    if let Some(rest) = trimmed.strip_prefix('>') {
        return Block::Quote(rest.trim_start());
    }
    if let Some(rest) = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
        .or_else(|| trimmed.strip_prefix("+ "))
    {
        let (marker, text) = if let Some(t) = rest.strip_prefix("[ ] ") {
            (ListMarker::Task { checked: false }, t)
        } else if let Some(t) = rest.strip_prefix("[x] ").or_else(|| rest.strip_prefix("[X] ")) {
            (ListMarker::Task { checked: true }, t)
        } else {
            (ListMarker::Bullet, rest)
        };
        return Block::ListItem { indent, marker, text };
    }
    if let Some((num, rest)) = trimmed.split_once(". ") {
        if !num.is_empty() && num.chars().all(|c| c.is_ascii_digit()) {
            return Block::ListItem {
                indent,
                marker: ListMarker::Ordered(num.to_owned()),
                text: rest,
            };
        }
    }
    Block::Paragraph(trimmed)
}

fn parse_heading(line: &str) -> Option<Block<'_>> {
    let level = line.chars().take_while(|&c| c == '#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let rest = &line[level..];
    if !rest.is_empty() && !rest.starts_with(' ') {
        return None; // `#tag`, not a heading
    }
    Some(Block::Heading {
        level: level as u8,
        text: rest.trim(),
    })
}

fn is_rule(line: &str) -> bool {
    let line = line.trim();
    line.len() >= 3
        && ['-', '*', '_']
            .iter()
            .any(|&c| line.chars().all(|x| x == c || x == ' ') && line.contains(c))
}

/// Splits a line of text into inline spans.
pub fn parse_inline(text: &str) -> Vec<Inline<'_>> {
    let mut out = Vec::new();
    let mut last = 0;
    for link in parse_links(text) {
        parse_inline_plain(&text[last..link.range.start], &mut out);
        last = link.range.end;
        out.push(Inline::Link(link));
    }
    parse_inline_plain(&text[last..], &mut out);
    out
}

/// Inline parsing for text known not to contain wiki links.
fn parse_inline_plain<'a>(text: &'a str, out: &mut Vec<Inline<'a>>) {
    let mut plain_start = 0;
    let mut i = 0;
    let bytes = text.as_bytes();

    let flush = |out: &mut Vec<Inline<'a>>, from: usize, to: usize| {
        if from < to {
            out.push(Inline::Text(&text[from..to]));
        }
    };

    while i < bytes.len() {
        let rest = &text[i..];
        let at_word_start = i == 0 || bytes[i - 1].is_ascii_whitespace();

        let span: Option<(Inline<'a>, usize)> = if let Some(r) = rest.strip_prefix("![") {
            if let Some(close_bracket) = r.find("](") {
                let alt = &r[..close_bracket];
                let after = &r[close_bracket + 2..];
                if let Some(close_paren) = after.find(')') {
                    let url = &after[..close_paren];
                    let len = 2 + close_bracket + 2 + close_paren + 1;
                    Some((Inline::Image { alt, url }, len))
                } else {
                    None
                }
            } else {
                None
            }
        } else if let Some(r) = rest.strip_prefix('`') {
            r.find('`').map(|end| (Inline::Code(&r[..end]), end + 2))
        } else if let Some(r) = rest.strip_prefix("**").or_else(|| rest.strip_prefix("__")) {
            let delim = &rest[..2];
            r.find(delim)
                .filter(|&end| end > 0)
                .map(|end| (Inline::Bold(&r[..end]), end + 4))
        } else if let Some(r) = rest.strip_prefix('*').or_else(|| rest.strip_prefix('_')) {
            let delim = &rest[..1];
            r.find(delim)
                .filter(|&end| end > 0 && !r.starts_with(' '))
                .map(|end| (Inline::Italic(&r[..end]), end + 2))
        } else if at_word_start && (rest.starts_with("https://") || rest.starts_with("http://")) {
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            Some((Inline::Url(&rest[..end]), end))
        } else if at_word_start && rest.starts_with('#') {
            let end = rest[1..]
                .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-' || c == '/'))
                .map_or(rest.len(), |e| e + 1);
            (end > 1).then(|| (Inline::Tag(&rest[1..end]), end))
        } else {
            None
        };

        match span {
            Some((inline, len)) => {
                flush(out, plain_start, i);
                out.push(inline);
                i += len;
                plain_start = i;
            }
            None => i += rest.chars().next().map_or(1, char::len_utf8),
        }
    }
    flush(out, plain_start, text.len());
}

/// Toggles the task checkbox on `line` (0-based) and returns the new content.
/// Returns `None` if that line is not a task.
pub fn toggle_task(content: &str, line: usize) -> Option<String> {
    let mut out = String::with_capacity(content.len());
    let mut toggled = false;
    for (i, l) in content.split_inclusive('\n').enumerate() {
        if i == line {
            if let Some(pos) = l.find("[ ] ") {
                out.push_str(&l[..pos]);
                out.push_str("[x] ");
                out.push_str(&l[pos + 4..]);
                toggled = true;
                continue;
            }
            if let Some(pos) = l.find("[x] ").or_else(|| l.find("[X] ")) {
                out.push_str(&l[..pos]);
                out.push_str("[ ] ");
                out.push_str(&l[pos + 4..]);
                toggled = true;
                continue;
            }
        }
        out.push_str(l);
    }
    toggled.then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blocks(src: &str) -> Vec<Block<'_>> {
        parse_blocks(src).into_iter().map(|b| b.block).collect()
    }

    #[test]
    fn block_types() {
        let src = "# Title\ntext\n- item\n  - [ ] task\n1. first\n> quote\n---\n#tag line\n";
        assert_eq!(
            blocks(src),
            vec![
                Block::Heading { level: 1, text: "Title" },
                Block::Paragraph("text"),
                Block::ListItem { indent: 0, marker: ListMarker::Bullet, text: "item" },
                Block::ListItem {
                    indent: 1,
                    marker: ListMarker::Task { checked: false },
                    text: "task"
                },
                Block::ListItem {
                    indent: 0,
                    marker: ListMarker::Ordered("1".into()),
                    text: "first"
                },
                Block::Quote("quote"),
                Block::Rule,
                Block::Paragraph("#tag line"),
            ]
        );
    }

    #[test]
    fn code_blocks_and_line_numbers() {
        let src = "---\na: 1\n---\nintro\n```rust\nfn x() {}\n```\nafter";
        let parsed = parse_blocks(src);
        assert_eq!(parsed[0].line, 3);
        assert_eq!(
            parsed[1].block,
            Block::Code { lang: "rust", code: "fn x() {}\n".into() }
        );
        assert_eq!(parsed[2], LineBlock { line: 7, block: Block::Paragraph("after") });
    }

    #[test]
    fn inline_spans() {
        let spans = parse_inline("a **b** *c* `d` [[E|e]] https://x.y #tag");
        let kinds: Vec<String> = spans
            .iter()
            .map(|s| match s {
                Inline::Text(t) => format!("T({t})"),
                Inline::Bold(t) => format!("B({t})"),
                Inline::Italic(t) => format!("I({t})"),
                Inline::Code(t) => format!("C({t})"),
                Inline::Url(t) => format!("U({t})"),
                Inline::Tag(t) => format!("#({t})"),
                Inline::Link(l) => format!("L({})", l.target),
                Inline::Image { alt, url } => format!("IMG({alt}, {url})"),
            })
            .collect();
        assert_eq!(
            kinds,
            vec![
                "T(a )", "B(b)", "T( )", "I(c)", "T( )", "C(d)", "T( )", "L(E)", "T( )",
                "U(https://x.y)", "T( )", "#(tag)"
            ]
        );
    }

    #[test]
    fn inline_handles_unicode_and_lone_markers() {
        assert_eq!(parse_inline("café * 2"), vec![Inline::Text("café * 2")]);
    }

    #[test]
    fn task_toggling() {
        let src = "- [ ] a\n- [x] b\nplain";
        assert_eq!(toggle_task(src, 0).unwrap(), "- [x] a\n- [x] b\nplain");
        assert_eq!(toggle_task(src, 1).unwrap(), "- [ ] a\n- [ ] b\nplain");
        assert_eq!(toggle_task(src, 2), None);
    }

    #[test]
    fn image_parsing() {
        let spans = parse_inline("Here is ![A Cat](cat.png) inline");
        assert_eq!(
            spans,
            vec![
                Inline::Text("Here is "),
                Inline::Image { alt: "A Cat", url: "cat.png" },
                Inline::Text(" inline"),
            ]
        );
    }
}

