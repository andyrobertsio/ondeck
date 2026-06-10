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

    fn write_pre_tag(
        &self,
        output: &mut dyn IoWrite,
        _: HashMap<String, String>,
    ) -> std::io::Result<()> {
        output.write_all(b"<pre>")
    }

    fn write_code_tag(
        &self,
        output: &mut dyn IoWrite,
        _: HashMap<String, String>,
    ) -> std::io::Result<()> {
        output.write_all(b"<code>")
    }
}

use crate::fragments::{self, FragConfig};
use crate::grid::{parse_at, repeat_rects, Rect};
use crate::parser::{Document, Slide};
use crate::theme::{Align, Block, BlockContent, Fit, Layer, ResolvedLayout, Theme};

const RUNTIME_JS: &str = include_str!("assets/runtime.js");

/// One occurrence of a `:::name [at="…"]` block in the author's Markdown.
struct Instance {
    at: Option<Rect>,
    content: String,
}

/// What the author wrote: named `:::block`s, loose (unslotted) Markdown, notes.
struct Authored {
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

fn extract_blocks(body: &str) -> Authored {
    let mut named: BTreeMap<String, Vec<Instance>> = BTreeMap::new();
    let mut notes: Option<String> = None;
    let mut loose = String::new();

    // current open block: (name, at, accumulated content)
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
                let (name, at) = parse_block_header(rest);
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

    Authored {
        named,
        body: loose,
        notes,
    }
}

/// Parse `name at="x2 y5 x8 y6"` from a `:::` header line.
fn parse_block_header(header: &str) -> (String, Option<Rect>) {
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

/// Render Markdown, then apply fragment markers (shared slide-wide counter).
fn md_frag(text: &str, plugins: &Plugins, cfg: &FragConfig, counter: &mut u32) -> String {
    fragments::apply(&md(text, plugins), cfg, counter)
}

/// CSS classes for a block: name + layer/fit/alignment hooks (Main/Center/Scale
/// are the defaults and emit no class).
fn block_classes(b: &Block) -> String {
    let mut c = format!("block block-{}", b.name);
    match b.layer {
        Layer::Behind => c.push_str(" layer-behind"),
        Layer::Front => c.push_str(" layer-front"),
        Layer::Main => {}
    }
    match b.fit {
        Fit::Cover => c.push_str(" fit-cover"),
        Fit::Contain => c.push_str(" fit-contain"),
        Fit::Scale => {}
    }
    match b.align_x {
        Align::Start => c.push_str(" ax-start"),
        Align::End => c.push_str(" ax-end"),
        Align::Center => {}
    }
    match b.align_y {
        Align::Center => c.push_str(" ay-center"),
        Align::End => c.push_str(" ay-end"),
        Align::Start => {} // top — the default
    }
    c
}

fn block_style(b: &Block, rect: &Rect, extra: &str) -> String {
    let mut s = rect.style();
    if let Some(o) = b.opacity {
        s.push_str(&format!("opacity:{o};"));
    }
    s.push_str(extra);
    s
}

/// A content block (editable or fixed text). `fit:scale` wraps the content in a
/// scale-to-fit `.fit`; `cover`/`contain` size the content via CSS instead.
fn emit_block(b: &Block, rect: &Rect, inner: &str) -> String {
    let body = if b.fit == Fit::Scale {
        format!("<div class=\"fit\">{inner}</div>")
    } else {
        inner.to_string()
    };
    format!(
        "<div class=\"{}\" style=\"{}\">{}</div>",
        block_classes(b),
        block_style(b, rect, ""),
        body
    )
}

/// A fixed image block: the (already-inlined) image fills the cell via
/// background, sized by `fit` (`cover` crops, anything else contains).
fn emit_image_block(b: &Block, rect: &Rect, url: &str) -> String {
    let size = if b.fit == Fit::Cover {
        "cover"
    } else {
        "contain"
    };
    let bg = format!("background:{url} center/{size} no-repeat;");
    format!(
        "<div class=\"{}\" style=\"{}\"></div>",
        block_classes(b),
        block_style(b, rect, &bg)
    )
}

/// Render one resolved block. `sink` is the name of the layout's sole editable
/// block, which receives loose (unslotted) Markdown.
fn render_one_block(
    b: &Block,
    rect: Rect,
    authored: &Authored,
    plugins: &Plugins,
    cfg: &FragConfig,
    counter: &mut u32,
    sink: Option<&str>,
) -> String {
    match &b.content {
        BlockContent::Image(url) => emit_image_block(b, &rect, url),
        BlockContent::Text(t) => emit_block(b, &rect, &md(t, plugins)),
        BlockContent::Editable => {
            // Per-block fragment transition overrides the slide/theme default.
            let bcfg = FragConfig {
                auto_li: cfg.auto_li,
                default_fx: b
                    .transition
                    .clone()
                    .unwrap_or_else(|| cfg.default_fx.clone()),
            };
            if let Some(rep) = &b.repeat {
                let insts = authored
                    .named
                    .get(&b.name)
                    .map(|v| v.as_slice())
                    .unwrap_or(&[]);
                if insts.is_empty() {
                    return String::new();
                }
                let limit = rep.limit.unwrap_or(insts.len());
                let rects = repeat_rects(
                    &rect,
                    rep.direction,
                    rep.margin,
                    insts.len(),
                    limit,
                    rep.align,
                );
                let mut out = String::new();
                for (inst, r) in insts.iter().zip(rects.iter()) {
                    let inner = md_frag(&inst.content, plugins, &bcfg, counter);
                    out.push_str(&emit_block(b, r, &inner));
                }
                out
            } else {
                let content =
                    if let Some(inst) = authored.named.get(&b.name).and_then(|v| v.first()) {
                        md_frag(&inst.content, plugins, &bcfg, counter)
                    } else if sink == Some(b.name.as_str()) {
                        md_frag(&authored.body, plugins, &bcfg, counter)
                    } else {
                        String::new()
                    };
                if content.trim().is_empty() {
                    String::new()
                } else {
                    emit_block(b, &rect, &content)
                }
            }
        }
    }
}

/// Build `.slide-content`: the union of the layout's selected template furniture
/// and the layout's own blocks.
fn build_blocks(
    layout: &ResolvedLayout,
    theme: &Theme,
    authored: &Authored,
    plugins: &Plugins,
    cfg: &FragConfig,
    counter: &mut u32,
    media_right: bool,
) -> String {
    let mut out = String::new();

    // Template furniture (fixed) — not affected by `media: right`.
    for b in theme.template_blocks(layout) {
        out.push_str(&render_one_block(
            b, b.rect, authored, plugins, cfg, counter, None,
        ));
    }

    // Single-sink: loose Markdown fills the sole editable block.
    let editable: Vec<&Block> = layout.blocks.iter().filter(|b| b.is_editable()).collect();
    let sink = if editable.len() == 1 {
        Some(editable[0].name.as_str())
    } else {
        None
    };

    for b in &layout.blocks {
        let rect = if media_right {
            b.rect.mirror_cols(theme.cols)
        } else {
            b.rect
        };
        out.push_str(&render_one_block(
            b, rect, authored, plugins, cfg, counter, sink,
        ));
    }
    out
}

/// `free` (and unknown) layouts: place every authored `:::name at="…"` block.
fn build_free(
    authored: &Authored,
    plugins: &Plugins,
    cfg: &FragConfig,
    counter: &mut u32,
) -> String {
    let mut out = String::new();
    for (name, instances) in &authored.named {
        for inst in instances {
            if let Some(rect) = inst.at {
                out.push_str(&format!(
                    "<div class=\"block block-{name}\" style=\"{}\"><div class=\"fit\">{}</div></div>",
                    rect.style(),
                    md_frag(&inst.content, plugins, cfg, counter)
                ));
            }
        }
    }
    out
}

fn render_slide(slide: &Slide, index: usize, theme: &Theme, plugins: &Plugins) -> String {
    let layout = resolve_layout(slide, index);
    let authored = extract_blocks(&slide.body);

    let (mut style, mut classes, overlay) = slide_styling(slide);
    classes.insert(0, format!("layout-{layout}"));
    // Mark the selected template so themes can style per-template.
    if let Some(t) = theme
        .layouts
        .get(&layout)
        .and_then(|l| l.template.as_deref())
    {
        classes.push(format!("template-{t}"));
    }

    // Fragment config: reveal flag + default transition (slide → theme → fade).
    let default_fx = slide
        .meta
        .get("transition")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| theme.default_transition.clone());
    let cfg = FragConfig {
        auto_li: slide
            .meta
            .get("reveal")
            .map(|v| v.trim() == "true")
            .unwrap_or(false),
        default_fx,
    };
    let mut counter = 1u32;
    if let Some(speed) = slide.meta.get("transition-speed") {
        style.push_str(&format!("--fx-dur:{};", speed.trim()));
    }
    // media-split: `media: right` mirrors the layout.
    let media_right = slide
        .meta
        .get("media")
        .map(|v| v.trim() == "right")
        .unwrap_or(false);
    // `image` carries a fit modifier: fit-full (cover, edge-to-edge) or fit-contain.
    if layout == "image" {
        let fit = slide.meta.get("fit").map(|s| s.trim()).unwrap_or("full");
        classes.push(format!("fit-{fit}"));
    }
    // Table emphasis (any slide with a table): highlight a column/row (1-based,
    // 1–8) or treat the first column as row labels. Styled via base.css classes.
    if let Some(n) = slide
        .meta
        .get("highlight-col")
        .and_then(|v| v.trim().parse::<u32>().ok())
    {
        classes.push(format!("hl-col-{n}"));
    }
    if let Some(n) = slide
        .meta
        .get("highlight-row")
        .and_then(|v| v.trim().parse::<u32>().ok())
    {
        classes.push(format!("hl-row-{n}"));
    }
    if slide
        .meta
        .get("row-headers")
        .map(|v| v.trim() == "true")
        .unwrap_or(false)
    {
        classes.push("row-headers".to_string());
    }

    let cells = if layout == "raw" {
        // raw: the author owns the markup entirely.
        authored.body.clone()
    } else {
        match theme.layouts.get(&layout) {
            Some(rl) => build_blocks(
                rl,
                theme,
                &authored,
                plugins,
                &cfg,
                &mut counter,
                media_right,
            ),
            None => build_free(&authored, plugins, &cfg, &mut counter),
        }
    };

    let class_attr = classes.join(" ");
    let style_attr = if style.is_empty() {
        String::new()
    } else {
        format!(" style=\"{style}\"")
    };
    let overlay = overlay.unwrap_or_default();
    let notes = authored
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
        "black",
        "white",
        "red",
        "green",
        "blue",
        "yellow",
        "orange",
        "purple",
        "gray",
        "grey",
        "cyan",
        "magenta",
        "pink",
        "brown",
        "navy",
        "teal",
        "maroon",
        "olive",
        "lime",
        "aqua",
        "silver",
        "gold",
        "transparent",
    ];
    NAMED.contains(&s.to_ascii_lowercase().as_str())
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
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
    let flag = |k: &str| {
        doc.frontmatter
            .get(k)
            .map(|v| v.trim() == "true")
            .unwrap_or(false)
    };
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
        chrome.push_str(&format!(
            "<div class=\"deck-footer\">{}</div>",
            escape_html(footer)
        ));
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
        let theme = Theme::load("default").unwrap();
        render(&doc, &theme, Path::new("."), false) // inline=false: no fs access
    }

    #[test]
    fn layouts_emit_classes_and_blocks() {
        let html = build("---\nlayout: title\n---\n# Hi\n---\nlayout: bullets\n---\n- a\n- b\n");
        assert!(html.contains("class=\"slide layout-title"));
        assert!(html.contains("class=\"slide layout-bullets"));
        assert!(html.contains("block block-body"));
    }

    #[test]
    fn repeatable_figures_render_per_entry() {
        // The first `---…---` block is deck frontmatter, so a single slide with
        // its own frontmatter needs a leading deck-frontmatter block. `stat` has
        // a head + a repeatable figure (two editable blocks → both need `:::`).
        let html = build(
            "---\ntheme: default\n---\n\n---\nlayout: stat\n---\n:::head\n# Numbers\n:::\n:::figure\n**42%**\n:::\n:::figure\n**+18**\n:::\n",
        );
        assert_eq!(html.matches("block block-figure").count(), 2);
        assert!(html.contains("42%"));
        assert!(html.contains("+18"));
    }

    #[test]
    fn free_layout_places_by_coordinates() {
        let html = build("---\ntheme: default\n---\n\n---\nlayout: free\n---\n:::block at=\"x2 y2 x10 y8\"\nHi\n:::\n");
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
            "---\ntheme: default\n---\n\n---\nlayout: media-split\nmedia: right\n---\n:::media\n![](x.png)\n:::\n:::body\n# H\nText\n:::\n",
        );
        // 64-col grid: media (cols 1/33) mirrors to 33/65; body (37/61) to 5/29.
        assert!(
            html.contains("grid-column:33/65"),
            "media should mirror to the right"
        );
        assert!(
            html.contains("grid-column:5/29"),
            "body should mirror to the left"
        );
    }

    #[test]
    fn background_var_is_color_not_image() {
        let html = build("---\ntheme: default\n---\n\n---\nlayout: statement\nbackground: var(--bg-2)\n---\n# x\n");
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
        let html = build(
            "---\ntheme: default\n---\n\n---\nlayout: code\n---\n```rust\nfn main() {}\n```\n",
        );
        assert!(html.contains("syn-")); // class-based tokens, theme-coloured
        assert!(html.contains("<pre>"));
    }

    #[test]
    fn table_layout_and_emphasis() {
        let html = build("---\ntheme: default\n---\n\n---\nlayout: table\nhighlight-col: 2\nrow-headers: true\n---\n# T\n\n| a | b |\n| - | - |\n| 1 | 2 |\n");
        assert!(html.contains("layout-table"));
        assert!(html.contains("hl-col-2"));
        assert!(html.contains("row-headers"));
        assert!(html.contains("<table>"));
    }

    #[test]
    fn first_slide_defaults_to_title() {
        let html = build("# Just a heading\n");
        assert!(html.contains("class=\"slide layout-title"));
    }
}
