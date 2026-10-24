//! Parsing of Obsidian-style `[[wiki links]]`.
//!
//! Supported forms:
//!
//! * `[[Note]]`
//! * `[[Folder/Note]]`
//! * `[[Note|Displayed alias]]`
//! * `[[Note#Heading]]` and `[[Note#Heading|alias]]`
//!
//! Links inside fenced code blocks (```` ``` ````) and inline code (`` ` ``) are ignored.

use std::ops::Range;

/// A single `[[wiki link]]` found in a piece of text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiLink {
    /// Link target as written, e.g. `"Folder/Note"`.
    pub target: String,
    /// Optional heading after `#`.
    pub heading: Option<String>,
    /// Optional alias after `|`.
    pub alias: Option<String>,
    /// Byte range of the whole link (including the brackets) in the source.
    pub range: Range<usize>,
}

impl WikiLink {
    /// Text to show to the user: the alias if present, otherwise the target
    /// (plus heading).
    pub fn display_text(&self) -> String {
        if let Some(alias) = &self.alias {
            return alias.clone();
        }
        match &self.heading {
            Some(h) => format!("{} > {}", self.target, h),
            None => self.target.clone(),
        }
    }
}

/// Extracts all wiki links from `text`.
pub fn parse_links(text: &str) -> Vec<WikiLink> {
    let mut links = Vec::new();
    let mut in_fence = false;
    let mut line_start = 0;

    for line in text.split_inclusive('\n') {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
        } else if !in_fence {
            parse_line(line, line_start, &mut links);
        }
        line_start += line.len();
    }
    links
}

fn parse_line(line: &str, offset: usize, out: &mut Vec<WikiLink>) {
    let bytes = line.as_bytes();
    let mut i = 0;
    let mut in_code = false;

    while i < bytes.len() {
        match bytes[i] {
            b'`' => {
                in_code = !in_code;
                i += 1;
            }
            b'[' if !in_code && bytes.get(i + 1) == Some(&b'[') => {
                let inner_start = i + 2;
                match line[inner_start..].find("]]") {
                    Some(rel_end) => {
                        let inner = &line[inner_start..inner_start + rel_end];
                        let end = inner_start + rel_end + 2;
                        if let Some(link) = build_link(inner, offset + i..offset + end) {
                            out.push(link);
                        }
                        i = end;
                    }
                    None => return,
                }
            }
            _ => i += 1,
        }
    }
}

fn build_link(inner: &str, range: Range<usize>) -> Option<WikiLink> {
    if inner.contains('[') || inner.contains('\n') {
        return None;
    }
    let (target_part, alias) = match inner.split_once('|') {
        Some((t, a)) => (t, Some(a.trim().to_owned()).filter(|a| !a.is_empty())),
        None => (inner, None),
    };
    let (target, heading) = match target_part.split_once('#') {
        Some((t, h)) => (t, Some(h.trim().to_owned()).filter(|h| !h.is_empty())),
        None => (target_part, None),
    };
    let target = target.trim();
    if target.is_empty() {
        return None;
    }
    Some(WikiLink {
        target: target.to_owned(),
        heading,
        alias,
        range,
    })
}

/// If the cursor (byte offset) sits right after an unclosed `[[`, returns the
/// byte offset where the partial link text starts and the text typed so far.
///
/// Used by the editor to drive link auto-completion.
pub fn open_link_at(text: &str, cursor: usize) -> Option<(usize, &str)> {
    let before = text.get(..cursor)?;
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    let line = &before[line_start..];
    let open = line.rfind("[[")?;
    let partial = &line[open + 2..];
    if partial.contains("]]") || partial.contains('|') {
        return None;
    }
    Some((line_start + open + 2, partial))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn targets(text: &str) -> Vec<String> {
        parse_links(text).into_iter().map(|l| l.target).collect()
    }

    #[test]
    fn simple_links() {
        assert_eq!(targets("see [[A]] and [[B/C]]"), vec!["A", "B/C"]);
    }

    #[test]
    fn alias_and_heading() {
        let links = parse_links("[[Note#Intro|read this]]");
        assert_eq!(links.len(), 1);
        let l = &links[0];
        assert_eq!(l.target, "Note");
        assert_eq!(l.heading.as_deref(), Some("Intro"));
        assert_eq!(l.alias.as_deref(), Some("read this"));
        assert_eq!(l.range, 0..24);
        assert_eq!(l.display_text(), "read this");
    }

    #[test]
    fn ignores_code() {
        assert!(targets("`[[no]]`").is_empty());
        assert!(targets("```\n[[no]]\n```\n").is_empty());
        assert_eq!(targets("```\n[[no]]\n```\n[[yes]]"), vec!["yes"]);
    }

    #[test]
    fn ignores_malformed() {
        assert!(targets("[[]] [[ ]] [[unclosed").is_empty());
    }

    #[test]
    fn ranges_are_correct_on_multiline() {
        let text = "line one\nsee [[Target]]";
        let link = &parse_links(text)[0];
        assert_eq!(&text[link.range.clone()], "[[Target]]");
    }

    #[test]
    fn open_link_detection() {
        let text = "hello [[Pro";
        assert_eq!(open_link_at(text, text.len()), Some((8, "Pro")));
        assert_eq!(open_link_at("[[done]] x", 10), None);
        assert_eq!(open_link_at("no link", 7), None);
    }
}
