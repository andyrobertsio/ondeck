//! Parses a deck source document into deck-level frontmatter + a list of slides.
//!
//! Grammar (see SPEC.md):
//!  - A line that is exactly `---` is a slide boundary.
//!  - The block between a boundary and the next `---` is *frontmatter* iff its
//!    first non-blank line looks like a key (`^[A-Za-z][\w-]*\s*:`).
//!  - The first frontmatter block in the document is the deck frontmatter.
//!  - Frontmatter is flat `key: value` pairs (values may be quoted).

use std::collections::BTreeMap;

pub type Frontmatter = BTreeMap<String, String>;

#[derive(Debug)]
pub struct Document {
    pub frontmatter: Frontmatter,
    pub slides: Vec<Slide>,
}

#[derive(Debug)]
pub struct Slide {
    pub meta: Frontmatter,
    pub body: String,
}

/// A segment of the source between `---` boundary lines.
struct Segment {
    text: String,
}

impl Segment {
    fn is_blank(&self) -> bool {
        self.text.trim().is_empty()
    }

    /// True if the first non-blank line looks like a frontmatter key.
    fn looks_like_frontmatter(&self) -> bool {
        self.text
            .lines()
            .find(|l| !l.trim().is_empty())
            .map(is_key_line)
            .unwrap_or(false)
    }
}

fn is_key_line(line: &str) -> bool {
    let line = line.trim_start();
    let mut chars = line.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    // key = [A-Za-z][\w-]* followed by ':'
    let key: String = std::iter::once(line.chars().next().unwrap())
        .chain(
            line.chars()
                .skip(1)
                .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-'),
        )
        .collect();
    let rest = &line[key.len()..];
    rest.trim_start().starts_with(':')
}

fn parse_frontmatter(text: &str) -> Frontmatter {
    let mut map = Frontmatter::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim().to_string();
            let val = unquote(v.trim());
            if !key.is_empty() {
                map.insert(key, val);
            }
        }
    }
    map
}

fn unquote(s: &str) -> String {
    let bytes = s.as_bytes();
    if s.len() >= 2
        && ((bytes[0] == b'"' && bytes[s.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[s.len() - 1] == b'\''))
    {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

/// Split source into segments on lines that are exactly `---`.
fn split_segments(source: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut current = String::new();
    for line in source.lines() {
        if line.trim() == "---" {
            segments.push(Segment {
                text: std::mem::take(&mut current),
            });
        } else {
            current.push_str(line);
            current.push('\n');
        }
    }
    segments.push(Segment { text: current });
    segments
}

pub fn parse(source: &str) -> Document {
    let segments = split_segments(source);
    let mut iter = segments.into_iter().peekable();

    // Drop leading blank segments (whitespace before the first `---`).
    while iter.peek().map(|s| s.is_blank()).unwrap_or(false) {
        iter.next();
    }

    // First non-blank segment that looks like frontmatter is the deck frontmatter.
    let frontmatter = match iter.peek() {
        Some(s) if s.looks_like_frontmatter() => parse_frontmatter(&iter.next().unwrap().text),
        _ => Frontmatter::new(),
    };

    let mut slides = Vec::new();
    while let Some(seg) = iter.next() {
        if seg.is_blank() {
            continue;
        }
        if seg.looks_like_frontmatter() {
            // This segment is a slide's frontmatter; its body is the next
            // non-blank, non-frontmatter segment (if any).
            let meta = parse_frontmatter(&seg.text);
            let body = match iter.peek() {
                Some(next) if !next.is_blank() && !next.looks_like_frontmatter() => {
                    iter.next().unwrap().text
                }
                _ => String::new(),
            };
            slides.push(Slide { meta, body });
        } else {
            // Bare body, no frontmatter.
            slides.push(Slide {
                meta: Frontmatter::new(),
                body: seg.text,
            });
        }
    }

    Document { frontmatter, slides }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deck_frontmatter_and_per_slide() {
        let doc = parse(
            "---\ntheme: midnight\ntitle: T\n---\n\n---\nlayout: title\n---\n# Hi\n---\nlayout: bullets\n---\n- a\n",
        );
        assert_eq!(doc.frontmatter.get("theme").map(String::as_str), Some("midnight"));
        assert_eq!(doc.slides.len(), 2);
        assert_eq!(doc.slides[0].meta.get("layout").map(String::as_str), Some("title"));
        assert!(doc.slides[0].body.contains("# Hi"));
        assert_eq!(doc.slides[1].meta.get("layout").map(String::as_str), Some("bullets"));
    }

    #[test]
    fn bare_body_without_frontmatter() {
        let doc = parse("# A\nbody a\n---\n# B\n");
        assert!(doc.frontmatter.is_empty());
        assert_eq!(doc.slides.len(), 2);
        assert!(doc.slides[0].meta.is_empty());
        assert!(doc.slides[0].body.contains("# A"));
    }

    #[test]
    fn quoted_value_unquoted() {
        let doc = parse("---\nfooter: \"Acme · 2026\"\n---\n# x\n");
        assert_eq!(doc.frontmatter.get("footer").map(String::as_str), Some("Acme · 2026"));
    }
}
