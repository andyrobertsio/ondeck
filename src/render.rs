//! Renders a parsed [`Document`] into a single self-contained HTML string,
//! using a loaded [`Theme`] for the grid, layout slots, and styling.

use std::collections::{BTreeMap, HashMap};
use std::io::Write as IoWrite;
use std::path::Path;

use comrak::adapters::SyntaxHighlighterAdapter;
use comrak::{markdown_to_html_with_plugins, Options, Plugins};
use syntect::html::{ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

use crate::assets;

/// Token classes are prefixed (`syn-keyword`, …) to avoid clashing with deck
/// classes; they're coloured from theme tokens in base.css.
const SYN_STYLE: ClassStyle = ClassStyle::SpacedPrefixed { prefix: "syn-" };

/// A comrak highlighter that emits class-based spans (not inline styles), so the
/// theme's CSS controls code colours.
struct ClassedHighlighter {
    syntaxes: SyntaxSet,
}

impl ClassedHighlighter {
    fn new() -> Self {
        Self {
            syntaxes: SyntaxSet::load_defaults_newlines(),
        }
    }
}

impl SyntaxHighlighterAdapter for ClassedHighlighter {
    fn write_highlighted(
        &self,
        output: &mut dyn IoWrite,
        lang: Option<&str>,
        code: &str,
    ) -> std::io::Result<()> {
        let syntax = lang
            .and_then(|l| self.syntaxes.find_syntax_by_token(l))
            .unwrap_or_else(|| self.syntaxes.find_syntax_plain_text());
        let mut gen = ClassedHTMLGenerator::new_with_class_style(syntax, &self.syntaxes, SYN_STYLE);
        for line in LinesWithEndings::from(code) {
            // On the rare parse hiccup, skip the line rather than abort.
            let _ = gen.parse_html_for_line_which_includes_newline(line);
        }
        output.write_all(gen.finalize().as_bytes())
    }

    fn write_pre_tag(&self, output: &mut dyn IoWrite, _: HashMap<String, String>) -> std::io::Result<()> {
        output.write_all(b"<pre>")
    }

    fn write_code_tag(&self, output: &mut dyn IoWrite, _: HashMap<String, String>) -> std::io::Result<()> {
        output.write_all(b"<code>")
    }
}

use crate::fragments::{self, FragConfig};
use crate::grid::{parse_at, Rect};
use crate::parser::{Document, Slide};
use crate::theme::Theme;

const RUNTIME_JS: &str = include_str!("assets/runtime.js");

/// One occurrence of a `:::name [at="…"]` slot.
struct Instance {
    at: Option<Rect>,
    content: String,
}

struct Slots {
    named: BTreeMap<String, Vec<Instance>>,
    body: String,
    notes: Option<String>,
}

fn md(text: &str, plugins: &Plugins) -> String {
    let mut options = Options::default();
    options.render.unsafe_ = true; // allow the `raw` escape hatch
    options.extension.strikethrough = true;
    options.extension.table = true;
    markdown_to_html_with_plugins(text.trim(), &options, plugins)
}

fn extract_slots(body: &str) -> Slots {
    let mut named: BTreeMap<String, Vec<Instance>> = BTreeMap::new();
    let mut notes: Option<String> = None;
    let mut loose = String::new();

    // current open slot: (name, at, accumulated content)
    let mut current: Option<(String, Option<Rect>, String)> = None;
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some((name, at, acc)) = current.as_mut() {
            if trimmed == ":::" {
                let (n, at, content) = (name.clone(), *at, std::mem::take(acc));
                if n == "notes" {
                    notes = Some(content);
                } else {
                    named.entry(n).or_default().push(Instance { at, content });
                }
                current = None;
            } else {
                acc.push_str(line);
                acc.push('\n');
            }
        } else if let Some(rest) = trimmed.strip_prefix(":::") {
            let rest = rest.trim();
            if rest.is_empty() {
                loose.push_str(line);
                loose.push('\n');
            } else {
                let (name, at) = parse_slot_header(rest);
                current = Some((name, at, String::new()));
            }
        } else {
            loose.push_str(line);
            loose.push('\n');
        }
    }
    if let Some((name, _, acc)) = current {
        loose.push_str(&format!(":::{name}\n{acc}"));
    }

    Slots {
        named,
        body: loose,
        notes,
    }
}

/// Parse `name at="x2 y5 x8 y6"` from a slot header line.
fn parse_slot_header(header: &str) -> (String, Option<Rect>) {
    let name = header.split_whitespace().next().unwrap_or("").to_string();
    let at = header
        .split_once("at=")
        .map(|(_, v)| v.trim().trim_matches('"').trim_matches('\''))
        .and_then(parse_at);
    (name, at)
}

fn resolve_layout(slide: &Slide, index: usize) -> String {
    match slide.meta.get("layout") {
        Some(l) => l.clone(),
        None if index == 0 => "title".to_string(),
        None => "bullets".to_string(),
    }
}

fn slide_styling(slide: &Slide) -> (String, Vec<String>, Option<String>) {
    let mut style = String::new();
    let mut classes = Vec::new();
    let mut overlay = None;

    if let Some(bg) = slide.meta.get("background") {
        let bg = bg.trim();
        if bg.starts_with('#')
            || is_named_color(bg)
            || bg.starts_with("rgb")
            || bg.starts_with("hsl")
            || bg.starts_with("var(")
        // `var(--token)` lets a background follow the theme (theme-relative).
        {
            style.push_str(&format!("background-color:{bg};"));
        } else if bg.contains("gradient(") {
            style.push_str(&format!("background-image:{bg};"));
        } else {
            let fit = slide
                .meta
                .get("background-fit")
                .map(String::as_str)
                .unwrap_or("cover");
            style.push_str(&format!(
                "background-image:url('{bg}');background-size:{fit};background-position:center;"
            ));
        }
    }
    if let Some(scheme) = slide.meta.get("scheme") {
        classes.push(format!("scheme-{}", scheme.trim()));
    }
    if let Some(o) = slide.meta.get("background-overlay") {
        if let Ok(v) = o.trim().parse::<f32>() {
            if v > 0.0 {
                overlay = Some(format!(
                    "<div class=\"slide-overlay\" style=\"background:rgba(0,0,0,{v});\"></div>"
                ));
            }
        }
    }
    (style, classes, overlay)
}

/// Render a slot wrapper: positioned grid cell + scale-to-fit inner element.
fn slot(name: &str, rect: &Rect, inner: &str) -> String {
    format!(
        "<div class=\"slot slot-{name}\" style=\"{}\"><div class=\"fit\">{inner}</div></div>",
        rect.style()
    )
}

/// A slot whose content fills the cell (e.g. a cover image) — no scale-to-fit.
fn slot_cover(name: &str, rect: &Rect, inner: &str) -> String {
    format!(
        "<div class=\"slot slot-{name}\" style=\"{}\">{inner}</div>",
        rect.style()
    )
}

fn render_stat_grid(instances: &[Instance]) -> String {
    let mut cells = String::new();
    for inst in instances {
        let raw = inst.content.trim();
        let (value, label) = match raw.split_once('·') {
            Some((v, l)) => (v.trim().to_string(), l.trim().to_string()),
            None => match raw.split_once('\n') {
                Some((v, l)) => (v.trim().to_string(), l.trim().to_string()),
                None => (raw.to_string(), String::new()),
            },
        };
        cells.push_str(&format!(
            "<div class=\"stat\"><div class=\"stat-value\">{value}</div><div class=\"stat-label\">{label}</div></div>"
        ));
    }
    let count = instances.len().max(1);
    format!("<div class=\"stat-grid\" style=\"--stat-count:{count};\">{cells}</div>")
}

/// `image` layout: the image fills the stage; an optional `:::caption` overlays.
fn render_image(slots: &Slots, plugins: &Plugins) -> String {
    let img = format!("<div class=\"image-fill\">{}</div>", md(&slots.body, plugins));
    let caption = slots
        .named
        .get("caption")
        .and_then(|v| v.first())
        .map(|i| format!("<div class=\"image-caption\">{}</div>", md(&i.content, plugins)))
        .unwrap_or_default();
    format!("{img}{caption}")
}

/// Render Markdown, then apply fragment markers (shared slide-wide counter).
fn md_frag(text: &str, plugins: &Plugins, cfg: &FragConfig, counter: &mut u32) -> String {
    fragments::apply(&md(text, plugins), cfg, counter)
}

/// Build the inner HTML of `.slide-content` for a given layout.
fn build_cells(
    layout: &str,
    slots: &Slots,
    theme: &Theme,
    plugins: &Plugins,
    cfg: &FragConfig,
    counter: &mut u32,
    media_right: bool,
) -> String {
    // raw: the author owns the markup entirely.
    if layout == "raw" {
        return slots.body.clone();
    }
    if layout == "image" {
        return render_image(slots, plugins);
    }

    let defs = theme.layouts.get(layout);
    let mut cells = String::new();
    let mut body_used = false;

    match defs {
        // `free` / unknown layout: place every authored slot by its `at=` rect.
        None => {
            for (name, instances) in &slots.named {
                for inst in instances {
                    if let Some(rect) = inst.at {
                        cells.push_str(&slot(name, &rect, &md_frag(&inst.content, plugins, cfg, counter)));
                    }
                }
            }
        }
        Some(defs) => {
            for (name, default_rect) in defs {
                // media-split `media: right` mirrors the column placement.
                let default_rect = if media_right {
                    default_rect.mirror_cols(theme.cols)
                } else {
                    *default_rect
                };

                // Special slot: stat grid built from repeatable :::stat blocks.
                if name == "stats" {
                    if let Some(stats) = slots.named.get("stat") {
                        let rect = stats.first().and_then(|i| i.at).unwrap_or(default_rect);
                        cells.push_str(&slot(name, &rect, &render_stat_grid(stats)));
                    }
                    continue;
                }

                // Special slot: a cover image (fills its cell, no scale-to-fit).
                if name == "media" {
                    if let Some(inst) = slots.named.get("media").and_then(|v| v.first()) {
                        let rect = inst.at.unwrap_or(default_rect);
                        cells.push_str(&slot_cover(name, &rect, &md(&inst.content, plugins)));
                    }
                    continue;
                }

                // Content: named slot if present, else loose body for body/head.
                let (content, rect) = match slots.named.get(name).and_then(|v| v.first()) {
                    Some(inst) => (
                        md_frag(&inst.content, plugins, cfg, counter),
                        inst.at.unwrap_or(default_rect),
                    ),
                    None if (name == "body" || name == "head") && !body_used => {
                        body_used = true;
                        (md_frag(&slots.body, plugins, cfg, counter), default_rect)
                    }
                    None => continue,
                };
                if !content.trim().is_empty() {
                    cells.push_str(&slot(name, &rect, &content));
                }
            }
        }
    }
    cells
}

fn render_slide(slide: &Slide, index: usize, theme: &Theme, plugins: &Plugins) -> String {
    let layout = resolve_layout(slide, index);
    let slots = extract_slots(&slide.body);

    let (mut style, mut classes, overlay) = slide_styling(slide);
    classes.insert(0, format!("layout-{layout}"));

    // Fragment config: reveal flag + default transition (slide → theme → fade).
    let default_fx = slide
        .meta
        .get("transition")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| theme.default_transition.clone());
    let cfg = FragConfig {
        auto_li: slide.meta.get("reveal").map(|v| v.trim() == "true").unwrap_or(false),
        default_fx,
    };
    let mut counter = 1u32;
    if let Some(speed) = slide.meta.get("transition-speed") {
        style.push_str(&format!("--fx-dur:{};", speed.trim()));
    }
    // media-split: `media: right` mirrors the layout.
    let media_right = slide.meta.get("media").map(|v| v.trim() == "right").unwrap_or(false);
    // `image` carries a fit modifier: fit-full (cover, edge-to-edge) or fit-contain.
    if layout == "image" {
        let fit = slide.meta.get("fit").map(|s| s.trim()).unwrap_or("full");
        classes.push(format!("fit-{fit}"));
    }

    let cells = build_cells(&layout, &slots, theme, plugins, &cfg, &mut counter, media_right);

    let class_attr = classes.join(" ");
    let style_attr = if style.is_empty() {
        String::new()
    } else {
        format!(" style=\"{style}\"")
    };
    let overlay = overlay.unwrap_or_default();
    let notes = slots
        .notes
        .map(|n| format!("<aside class=\"notes\" hidden>{}</aside>", md(&n, plugins)))
        .unwrap_or_default();
    // Per-slide transition override (used when entering this slide).
    let tx = slide
        .meta
        .get("slide-transition")
        .map(|t| format!(" data-transition=\"{}\"", t.trim()))
        .unwrap_or_default();

    format!(
        "<section class=\"slide {class_attr}\" data-index=\"{index}\"{tx}{style_attr}>{overlay}<div class=\"slide-content\">{cells}</div>{notes}</section>"
    )
}

fn is_named_color(s: &str) -> bool {
    const NAMED: &[&str] = &[
        "black", "white", "red", "green", "blue", "yellow", "orange", "purple", "gray", "grey",
        "cyan", "magenta", "pink", "brown", "navy", "teal", "maroon", "olive", "lime", "aqua",
        "silver", "gold", "transparent",
    ];
    NAMED.contains(&s.to_ascii_lowercase().as_str())
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

pub fn render(doc: &Document, theme: &Theme, asset_base: &Path, inline: bool) -> String {
    let title = doc
        .frontmatter
        .get("title")
        .cloned()
        .unwrap_or_else(|| "Presentation".to_string());

    // Deck-wide default slide transition (per-slide may override).
    let deck_transition = doc
        .frontmatter
        .get("slide-transition")
        .map(|t| t.trim().to_string())
        .unwrap_or_else(|| "none".to_string());

    // Deck chrome: slide numbers, progress bar, footer (frontmatter toggles).
    let flag = |k: &str| doc.frontmatter.get(k).map(|v| v.trim() == "true").unwrap_or(false);
    let mut deck_classes = String::from("deck");
    let mut chrome = String::new();
    if flag("slide-numbers") {
        deck_classes.push_str(" has-numbers");
        chrome.push_str("<div class=\"deck-number\"></div>");
    }
    if flag("progress") {
        deck_classes.push_str(" has-progress");
        chrome.push_str("<div class=\"deck-progress\"><i></i></div>");
    }
    if let Some(footer) = doc.frontmatter.get("footer") {
        chrome.push_str(&format!("<div class=\"deck-footer\">{}</div>", escape_html(footer)));
    }
    // Per-deck letterbox colour override (themes set --frame via [tokens]).
    let deck_style = doc
        .frontmatter
        .get("frame")
        .map(|f| format!(" style=\"--frame:{}\"", f.trim()))
        .unwrap_or_default();

    // Code fences are highlighted at build time into theme-coloured CSS classes.
    let highlighter = ClassedHighlighter::new();
    let mut plugins = Plugins::default();
    plugins.render.codefence_syntax_highlighter = Some(&highlighter);

    let mut slides: String = doc
        .slides
        .iter()
        .enumerate()
        .map(|(i, s)| render_slide(s, i, theme, &plugins))
        .collect();

    // Inline local images (content + backgrounds) as data URIs for portability.
    if inline {
        slides = assets::inline(&slides, asset_base);
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
:root {{ --cols:{cols}; --rows:{rows}; }}
{css}
</style>
</head>
<body>
<div class="{deck_classes}"{deck_style}>
<div class="stage" data-transition="{deck_transition}">
{slides}
{chrome}
</div>
</div>
<script>
{js}
</script>
</body>
</html>
"#,
        title = escape_html(&title),
        cols = theme.cols,
        rows = theme.rows,
        css = theme.css,
        deck_classes = deck_classes,
        deck_style = deck_style,
        deck_transition = deck_transition,
        slides = slides,
        chrome = chrome,
        js = RUNTIME_JS,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use crate::theme::Theme;
    use std::path::Path;

    fn build(src: &str) -> String {
        let doc = parser::parse(src);
        let theme = Theme::load("midnight").unwrap();
        render(&doc, &theme, Path::new("."), false) // inline=false: no fs access
    }

    #[test]
    fn layouts_emit_classes_and_slots() {
        let html = build("---\nlayout: title\n---\n# Hi\n---\nlayout: bullets\n---\n- a\n- b\n");
        assert!(html.contains("class=\"slide layout-title"));
        assert!(html.contains("class=\"slide layout-bullets"));
        assert!(html.contains("slot slot-body"));
    }

    #[test]
    fn stat_grid_renders_value() {
        // The first `---…---` block is deck frontmatter, so a single slide with
        // its own frontmatter needs a leading deck-frontmatter block.
        let html = build("---\ntheme: midnight\n---\n\n---\nlayout: stat\n---\n:::stat\n42% · target\n:::\n");
        assert!(html.contains("stat-value"));
        assert!(html.contains("42%"));
        assert!(html.contains("target"));
    }

    #[test]
    fn free_layout_places_by_coordinates() {
        let html = build("---\ntheme: midnight\n---\n\n---\nlayout: free\n---\n:::block at=\"x2 y2 x10 y8\"\nHi\n:::\n");
        assert!(html.contains("grid-column:2/11;grid-row:2/9;"));
    }

    #[test]
    fn chrome_toggles() {
        // Check for the actual elements — the runtime JS always mentions the
        // class names, so a bare substring check would false-positive.
        let on = build("---\nslide-numbers: true\nprogress: true\nfooter: F\n---\n# x\n");
        assert!(on.contains("<div class=\"deck-number\">"));
        assert!(on.contains("<div class=\"deck-progress\">"));
        assert!(on.contains("<div class=\"deck-footer\">"));
        assert!(on.contains("has-numbers"));

        let off = build("---\ntitle: x\n---\n# y\n");
        assert!(!off.contains("<div class=\"deck-number\">"));
        assert!(!off.contains("<div class=\"deck-progress\">"));
    }

    #[test]
    fn media_split_right_mirrors_columns() {
        let html = build(
            "---\ntheme: midnight\n---\n\n---\nlayout: media-split\nmedia: right\n---\n# H\nText\n:::media\n![](x.png)\n:::\n",
        );
        assert!(html.contains("grid-column:17/33"), "media should mirror to the right");
        assert!(html.contains("grid-column:3/15"), "body should mirror to the left");
    }

    #[test]
    fn background_var_is_color_not_image() {
        let html = build("---\ntheme: midnight\n---\n\n---\nlayout: statement\nbackground: var(--bg-2)\n---\n# x\n");
        assert!(html.contains("background-color:var(--bg-2);"));
        assert!(!html.contains("url('var"));
    }

    #[test]
    fn frame_frontmatter_overrides_letterbox() {
        let html = build("---\nframe: \"#123456\"\n---\n# x\n");
        assert!(html.contains("style=\"--frame:#123456\""));
    }

    #[test]
    fn code_is_class_highlighted_not_inline() {
        let html = build("---\ntheme: midnight\n---\n\n---\nlayout: code\n---\n```rust\nfn main() {}\n```\n");
        assert!(html.contains("syn-")); // class-based tokens, theme-coloured
        assert!(html.contains("<pre>"));
    }

    #[test]
    fn first_slide_defaults_to_title() {
        let html = build("# Just a heading\n");
        assert!(html.contains("class=\"slide layout-title"));
    }
}
