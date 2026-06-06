//! Inlines local image references as base64 data URIs so output HTML is fully
//! self-contained. Handles `<img src="…">` and `url(…)` (background images),
//! resolving paths relative to the source Markdown file. Remote (`http(s)://`,
//! `//`) and already-inlined (`data:`) references are left untouched, as are
//! files that can't be read.

use std::path::Path;

use regex::{Captures, Regex};

fn base64_encode(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 { T[((n >> 6) & 63) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { T[(n & 63) as usize] as char } else { '=' });
    }
    out
}

fn mime_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .as_deref()
    {
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("avif") => "image/avif",
        Some("bmp") => "image/bmp",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        Some("ttf") => "font/ttf",
        Some("otf") => "font/otf",
        _ => "application/octet-stream",
    }
}

/// Turn a local path into a data URI, or None to leave the reference as-is.
fn data_uri(raw: &str, base: &Path) -> Option<String> {
    let p = raw.trim();
    if p.is_empty()
        || p.starts_with("data:")
        || p.starts_with("http://")
        || p.starts_with("https://")
        || p.starts_with("//")
    {
        return None;
    }
    let full = base.join(p);
    let bytes = std::fs::read(&full).ok()?;
    Some(format!("data:{};base64,{}", mime_for(&full), base64_encode(&bytes)))
}

/// Inline image references in `html`, resolving relative paths against `base`.
pub fn inline(html: &str, base: &Path) -> String {
    let src_re = Regex::new(r#"(src\s*=\s*)(["'])([^"']*)(["'])"#).unwrap();
    let url_re = Regex::new(r#"url\(\s*(['"]?)([^'")]*)(['"]?)\s*\)"#).unwrap();

    let html = src_re.replace_all(html, |c: &Captures| match data_uri(&c[3], base) {
        Some(uri) => format!("{}{}{}{}", &c[1], &c[2], uri, &c[4]),
        None => c[0].to_string(),
    });
    let html = url_re.replace_all(&html, |c: &Captures| match data_uri(&c[2], base) {
        Some(uri) => format!("url({}{}{})", &c[1], uri, &c[3]),
        None => c[0].to_string(),
    });
    html.into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn inlines_url_against_base() {
        // examples/chart.svg exists relative to the crate root (test cwd).
        let out = inline("a{background:url('chart.svg')}", Path::new("examples"));
        assert!(out.contains("url('data:image/svg+xml;base64,"));
    }

    #[test]
    fn leaves_remote_and_data_untouched() {
        let out = inline("<img src=\"https://x/y.png\"><img src=\"data:image/png;base64,AA\">", Path::new("."));
        assert!(out.contains("https://x/y.png"));
        assert!(out.contains("data:image/png;base64,AA"));
    }

    #[test]
    fn font_mime_types() {
        assert_eq!(mime_for(Path::new("Inter.woff2")), "font/woff2");
        assert_eq!(mime_for(Path::new("x.ttf")), "font/ttf");
    }
}
