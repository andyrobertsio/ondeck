//! Post-processes rendered HTML to mark fragment elements for incremental
//! reveal. Markers in the source (`{+}`, `{+2}`, `{+ fade-up}`, `{+2 zoom}`)
//! survive Markdown rendering as literal text; here we attach
//! `class="fragment fx-<name>" data-step="<n>"` to the enclosing block element
//! and strip the marker. `reveal: true` auto-steps every top-level `<li>`.

pub struct FragConfig {
    /// `reveal: true` — every top-level list item becomes a step.
    pub auto_li: bool,
    /// Transition used when a marker names none (slide/theme/engine default).
    pub default_fx: String,
}

#[derive(PartialEq, Clone, Copy)]
enum Kind {
    Open,
    Close,
    Void,
    Text,
}

struct Tok {
    raw: String,
    name: String,
    kind: Kind,
}

fn is_block(name: &str) -> bool {
    matches!(
        name,
        "p" | "li"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "blockquote"
            | "pre"
            | "figure"
            | "table"
    )
}

fn is_void(name: &str) -> bool {
    matches!(
        name,
        "img" | "br" | "hr" | "input" | "col" | "area" | "source"
    )
}

fn tokenize(html: &str) -> Vec<Tok> {
    let mut toks = Vec::new();
    let bytes = html.as_bytes();
    let n = bytes.len();
    let mut i = 0;
    while i < n {
        if bytes[i] == b'<' {
            let start = i;
            let mut j = i + 1;
            while j < n && bytes[j] != b'>' {
                j += 1;
            }
            let end = if j < n { j + 1 } else { n };
            let raw = &html[start..end];
            let rb = raw.as_bytes();
            let close = rb.get(1) == Some(&b'/');
            let name_start = if close { 2 } else { 1 };
            let mut m = name_start;
            while m < raw.len() && raw.as_bytes()[m].is_ascii_alphanumeric() {
                m += 1;
            }
            let name = raw[name_start..m].to_ascii_lowercase();
            let kind = if close {
                Kind::Close
            } else if raw.ends_with("/>") || is_void(&name) {
                Kind::Void
            } else {
                Kind::Open
            };
            toks.push(Tok {
                raw: raw.to_string(),
                name,
                kind,
            });
            i = end;
        } else {
            let start = i;
            while i < n && bytes[i] != b'<' {
                i += 1;
            }
            toks.push(Tok {
                raw: html[start..i].to_string(),
                name: String::new(),
                kind: Kind::Text,
            });
        }
    }
    toks
}

/// Find the first `{+...}` marker in text. Returns (cleaned text, step, fx).
fn extract_marker(text: &str) -> Option<(String, Option<u32>, Option<String>)> {
    let start = text.find("{+")?;
    let rest = &text[start + 2..];
    let end_rel = rest.find('}')?;
    let inner = rest[..end_rel].trim();

    let digits: String = inner.chars().take_while(|c| c.is_ascii_digit()).collect();
    let step = digits.parse().ok();
    let fx_str = inner[digits.len()..].trim();
    let fx = (!fx_str.is_empty()).then(|| fx_str.to_string());

    let cleaned = format!("{}{}", &text[..start], &text[start + 2 + end_rel + 1..]);
    Some((cleaned, step, fx))
}

fn assign(
    frag: &mut [Option<(u32, String)>],
    idx: usize,
    counter: &mut u32,
    default_fx: &str,
    step: Option<u32>,
    fx: Option<String>,
) {
    let s = match step {
        Some(n) => {
            if n + 1 > *counter {
                *counter = n + 1;
            }
            n
        }
        None => {
            let c = *counter;
            *counter += 1;
            c
        }
    };
    frag[idx] = Some((s, fx.unwrap_or_else(|| default_fx.to_string())));
}

fn inject(raw: &str, class: &str, step: u32) -> String {
    let mut i = 1;
    while i < raw.len() && raw.as_bytes()[i].is_ascii_alphanumeric() {
        i += 1;
    }
    format!(
        "{} class=\"{class}\" data-step=\"{step}\"{}",
        &raw[..i],
        &raw[i..]
    )
}

/// Apply fragment markers to a rendered HTML fragment. `counter` is shared
/// across a slide so step numbers are global to the slide.
pub fn apply(html: &str, cfg: &FragConfig, counter: &mut u32) -> String {
    let mut toks = tokenize(html);
    let mut frag: Vec<Option<(u32, String)>> = vec![None; toks.len()];
    let mut stack: Vec<usize> = Vec::new(); // open block element indices
    let mut list_depth = 0u32;

    for idx in 0..toks.len() {
        match toks[idx].kind {
            Kind::Open => {
                let name = toks[idx].name.clone();
                if name == "ul" || name == "ol" {
                    list_depth += 1;
                }
                if is_block(&name) {
                    if name == "li" && cfg.auto_li && list_depth == 1 {
                        assign(&mut frag, idx, counter, &cfg.default_fx, None, None);
                    }
                    stack.push(idx);
                }
            }
            Kind::Close => {
                let name = toks[idx].name.clone();
                if (name == "ul" || name == "ol") && list_depth > 0 {
                    list_depth -= 1;
                }
                if is_block(&name) {
                    if let Some(pos) = stack.iter().rposition(|&t| toks[t].name == name) {
                        stack.truncate(pos);
                    }
                }
            }
            Kind::Text => {
                if let Some((cleaned, step, fx)) = extract_marker(&toks[idx].raw) {
                    if let Some(&owner) = stack.last() {
                        assign(&mut frag, owner, counter, &cfg.default_fx, step, fx);
                    }
                    toks[idx].raw = cleaned;
                }
            }
            Kind::Void => {}
        }
    }

    let mut out = String::with_capacity(html.len() + 64);
    for (idx, t) in toks.iter().enumerate() {
        match &frag[idx] {
            Some((step, fx)) => out.push_str(&inject(&t.raw, &format!("fragment fx-{fx}"), *step)),
            None => out.push_str(&t.raw),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(fx: &str, auto: bool) -> FragConfig {
        FragConfig {
            auto_li: auto,
            default_fx: fx.to_string(),
        }
    }

    #[test]
    fn explicit_marker_uses_default_fx() {
        let mut c = 1;
        let out = apply(
            "<ul><li>a</li><li>b {+}</li></ul>",
            &cfg("fade", false),
            &mut c,
        );
        assert!(out.contains("<li>a</li>")); // unmarked item untouched
        assert!(out.contains("class=\"fragment fx-fade\" data-step=\"1\""));
        assert!(!out.contains("{+}")); // marker stripped
    }

    #[test]
    fn named_transition_and_explicit_step() {
        let mut c = 1;
        let out = apply("<p>x {+2 zoom}</p>", &cfg("fade", false), &mut c);
        assert!(out.contains("fragment fx-zoom"));
        assert!(out.contains("data-step=\"2\""));
    }

    #[test]
    fn reveal_auto_steps_top_level_items() {
        let mut c = 1;
        let out = apply(
            "<ul><li>a</li><li>b</li></ul>",
            &cfg("fade-up", true),
            &mut c,
        );
        assert_eq!(out.matches("class=\"fragment fx-fade-up\"").count(), 2);
        assert!(out.contains("data-step=\"1\""));
        assert!(out.contains("data-step=\"2\""));
    }

    #[test]
    fn nested_items_not_auto_stepped() {
        let mut c = 1;
        let out = apply(
            "<ul><li>a<ul><li>inner</li></ul></li></ul>",
            &cfg("fade", true),
            &mut c,
        );
        // only the outer (depth-1) li is a fragment
        assert_eq!(out.matches("fragment").count(), 1);
    }
}
