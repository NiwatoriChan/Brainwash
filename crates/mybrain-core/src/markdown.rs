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
    Quote(String),
    Code { lang: &'a str, code: String },
    Table { headers: Vec<&'a str>, rows: Vec<Vec<&'a str>> },
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
    Strikethrough(&'a str),
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

        // Group consecutive `>` blockquote / callout lines into a single Block::Quote
        if let Some(first_quote) = trimmed.strip_prefix('>') {
            let mut quote_text = String::from(first_quote.trim_start());
            while let Some(&(_, next_line)) = lines.peek() {
                if let Some(next_quote) = next_line.trim_start().strip_prefix('>') {
                    lines.next();
                    quote_text.push('\n');
                    quote_text.push_str(next_quote.trim_start());
                } else {
                    break;
                }
            }
            blocks.push(LineBlock {
                line: line_no,
                block: Block::Quote(quote_text),
            });
            continue;
        }

        // Parse Markdown pipe tables (`| Header |` followed by `| --- |`)
        if is_table_row(trimmed) {
            if let Some(&(_, next_line)) = lines.peek() {
                if is_table_separator(next_line.trim()) {
                    lines.next(); // consume separator row
                    let headers = parse_table_cells(trimmed);
                    let mut rows = Vec::new();
                    while let Some(&(_, row_line)) = lines.peek() {
                        let row_trimmed = row_line.trim();
                        if is_table_row(row_trimmed) && !is_table_separator(row_trimmed) {
                            lines.next();
                            rows.push(parse_table_cells(row_trimmed));
                        } else {
                            break;
                        }
                    }
                    blocks.push(LineBlock {
                        line: line_no,
                        block: Block::Table { headers, rows },
                    });
                    continue;
                }
            }
        }

        blocks.push(LineBlock {
            line: line_no,
            block: parse_line(line),
        });
    }
    blocks
}

fn is_table_row(trimmed: &str) -> bool {
    let t = trimmed.trim();
    t.starts_with('|') && t.ends_with('|') && t.len() > 2
}

fn is_table_separator(trimmed: &str) -> bool {
    if !is_table_row(trimmed) {
        return false;
    }
    let cells = parse_table_cells(trimmed);
    !cells.is_empty()
        && cells.iter().all(|c| {
            let c = c.trim();
            !c.is_empty()
                && c.chars().any(|ch| ch == '-')
                && c.chars().all(|ch| ch == '-' || ch == ':')
        })
}

fn parse_table_cells(trimmed: &str) -> Vec<&str> {
    let inner = trimmed.trim().trim_start_matches('|').trim_end_matches('|');
    inner.split('|').map(str::trim).collect()
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
        return Block::Quote(rest.trim_start().to_owned());
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
    normalize_paired_bold_across_links(&mut out);
    out
}

/// Cleans up `**...` before a `[[WikiLink]]` and `...**` after it so patterns like
/// `**Double-brackets [[Wiki Links]]**` render as bold text + link without raw `**`.
fn normalize_paired_bold_across_links<'a>(spans: &mut Vec<Inline<'a>>) {
    let mut open_idx: Option<(usize, &'a str, &'a str)> = None;
    let mut pairs = Vec::new();

    for (idx, span) in spans.iter().enumerate() {
        if let Inline::Text(txt) = span {
            if open_idx.is_none() {
                if let Some(pos) = txt.find("**") {
                    let before = &txt[..pos];
                    let after = &txt[pos + 2..];
                    if !after.contains("**") {
                        open_idx = Some((idx, before, after));
                    }
                }
            } else if let Some(pos) = txt.find("**") {
                let (start_i, before_open, after_open) = open_idx.take().unwrap();
                let before_close = &txt[..pos];
                let after_close = &txt[pos + 2..];
                pairs.push((start_i, before_open, after_open, idx, before_close, after_close));
            }
        }
    }

    for (start_i, before_open, after_open, end_i, before_close, after_close) in pairs.into_iter().rev() {
        // Replace end_i first so indices at start_i stay valid
        let mut end_repl = Vec::new();
        if !before_close.is_empty() {
            end_repl.push(Inline::Bold(before_close));
        }
        if !after_close.is_empty() {
            end_repl.push(Inline::Text(after_close));
        }
        spans.splice(end_i..=end_i, end_repl);

        // Promote any intermediate plain text spans to Bold
        for mid in (start_i + 1)..end_i {
            if let Inline::Text(mid_txt) = spans[mid] {
                spans[mid] = Inline::Bold(mid_txt);
            }
        }

        let mut start_repl = Vec::new();
        if !before_open.is_empty() {
            start_repl.push(Inline::Text(before_open));
        }
        if !after_open.is_empty() {
            start_repl.push(Inline::Bold(after_open));
        }
        spans.splice(start_i..=start_i, start_repl);
    }
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
        } else if let Some(r) = rest.strip_prefix("~~") {
            r.find("~~")
                .filter(|&end| end > 0)
                .map(|end| (Inline::Strikethrough(&r[..end]), end + 4))
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
                Block::Quote("quote".into()),
                Block::Rule,
                Block::Paragraph("#tag line"),
            ]
        );
    }

    #[test]
    fn multiline_quotes_and_tables() {
        let src = "> [!NOTE]\n> Line 1\n> Line 2\n\n| Col A | Col B |\n| --- | --- |\n| 1 | 2 |\n";
        assert_eq!(
            blocks(src),
            vec![
                Block::Quote("[!NOTE]\nLine 1\nLine 2".into()),
                Block::Blank,
                Block::Table {
                    headers: vec!["Col A", "Col B"],
                    rows: vec![vec!["1", "2"]],
                },
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
                Inline::Strikethrough(t) => format!("S({t})"),
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

