//! Minimal YAML-like front-matter support.
//!
//! Only flat `key: value` pairs are supported, which covers everything MyBrain
//! itself writes (event metadata, tags, ...) without pulling in a full YAML
//! parser. Unknown lines are preserved verbatim when re-serialising is not
//! needed, because we never rewrite user front-matter we do not own.
//!
//! ```text
//! ---
//! type: event
//! date: 2026-10-25
//! ---
//! # Body starts here
//! ```

use std::collections::BTreeMap;

const DELIMITER: &str = "---";

/// Ordered `key -> value` map parsed from a front-matter block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Frontmatter {
    entries: BTreeMap<String, String>,
}

impl Frontmatter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Splits `content` into its front-matter and body.
    ///
    /// If the content does not start with a `---` block, an empty front-matter
    /// and the whole content are returned.
    pub fn parse(content: &str) -> (Self, &str) {
        let Some(rest) = strip_delimiter_line(content) else {
            return (Self::default(), content);
        };

        let mut entries = BTreeMap::new();
        let mut offset = 0;
        for line in rest.split_inclusive('\n') {
            offset += line.len();
            let trimmed = line.trim();
            if trimmed == DELIMITER {
                return (Self { entries }, &rest[offset..]);
            }
            if let Some((key, value)) = trimmed.split_once(':') {
                let value = value.trim().trim_matches('"').trim_matches('\'');
                entries.insert(key.trim().to_owned(), value.to_owned());
            }
        }

        // Unterminated block: treat the whole thing as body.
        (Self::default(), content)
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.entries.insert(key.into(), value.into());
        self
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Serialises the front-matter including the surrounding `---` lines.
    /// Returns an empty string when there are no entries.
    pub fn render(&self) -> String {
        if self.entries.is_empty() {
            return String::new();
        }
        let mut out = String::from("---\n");
        for (key, value) in &self.entries {
            out.push_str(key);
            out.push_str(": ");
            out.push_str(value);
            out.push('\n');
        }
        out.push_str("---\n");
        out
    }
}

fn strip_delimiter_line(content: &str) -> Option<&str> {
    let first_line_end = content.find('\n')?;
    (content[..first_line_end].trim_end() == DELIMITER).then(|| &content[first_line_end + 1..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_frontmatter_and_body() {
        let src = "---\ntype: event\ndate: \"2026-10-25\"\n---\n# Title\n";
        let (fm, body) = Frontmatter::parse(src);
        assert_eq!(fm.get("type"), Some("event"));
        assert_eq!(fm.get("date"), Some("2026-10-25"));
        assert_eq!(body, "# Title\n");
    }

    #[test]
    fn no_frontmatter() {
        let (fm, body) = Frontmatter::parse("hello\n---\n");
        assert!(fm.is_empty());
        assert_eq!(body, "hello\n---\n");
    }

    #[test]
    fn unterminated_block_is_body() {
        let src = "---\nkey: value\n";
        let (fm, body) = Frontmatter::parse(src);
        assert!(fm.is_empty());
        assert_eq!(body, src);
    }

    #[test]
    fn render_roundtrip() {
        let mut fm = Frontmatter::new();
        fm.set("a", "1").set("b", "two");
        let rendered = format!("{}body", fm.render());
        let (parsed, body) = Frontmatter::parse(&rendered);
        assert_eq!(parsed, fm);
        assert_eq!(body, "body");
    }
}
