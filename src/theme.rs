//! Theme loading. A theme is a directory of `theme.toml` (tokens + grid +
//! templates + layouts) and `theme.css` (styling).
//!
//! **The substrate is `base`** (`themes/base/`: `base.css` machinery +
//! agnostic look, `base.toml` neutral token contract + grid), compiled into the
//! binary and emitted beneath every deck. base ships no layouts. **Themes layer
//! on base**, and a theme may `extends = "<other>"` to inherit that theme's
//! tokens, layouts, templates, and CSS before applying its own. With no
//! `extends`, a theme builds straight on base. The bundled `default` theme owns
//! the core layout vocabulary; `bold`/`paper` extend it.
//!
//! Model: a **block** is the one placed-region primitive (positioned by `at=`
//! cells). A block is *fixed* when the theme gives it content (`image`/`text`),
//! otherwise *editable* (the author fills it). A **template** is a named bundle
//! of fixed furniture blocks; a **layout** selects a template and owns its own
//! blocks. A slide picks a layout.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::grid::{parse_at, Rect, RepeatAlign, RepeatDir};

/// The engine substrate's machinery + agnostic, token-driven look. Emitted first.
const BASE_CSS: &str = include_str!("../themes/base/base.css");
/// The substrate's manifest: the neutral token contract + default grid.
const BASE_TOML: &str = include_str!("../themes/base/base.toml");
/// The bundled `default` theme's per-layout look. Compiled in so it resolves
/// (and `extends = "default"`) with no `themes/` dir on disk.
const DEFAULT_CSS: &str = include_str!("../themes/default/theme.css");
/// The bundled `default` theme's manifest: palette tokens + the core layouts.
const DEFAULT_TOML: &str = include_str!("../themes/default/theme.toml");

// ---------------------------------------------------------------------------
// Resolved, ready-to-render types
// ---------------------------------------------------------------------------

/// Front-to-back band a block sits in, relative to the main (editable) content.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Layer {
    Behind,
    Main,
    Front,
}

/// Content sizing within a block's cell.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Fit {
    /// Natural flow; content is authored to fit and clips on overflow (default).
    Natural,
    /// Scale-to-fit: wrap in `.fit` and shrink uniformly until it fits (opt-in).
    Scale,
    Cover,
    Contain,
}

/// Alignment along one axis (left/top = Start … right/bottom = End).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Align {
    Start,
    Center,
    End,
}

/// Where a block's content comes from.
#[derive(Clone, PartialEq, Debug)]
pub enum BlockContent {
    /// A theme image (`url(...)`), inlined — fixed.
    Image(String),
    /// Theme Markdown text — fixed.
    Text(String),
    /// Author-filled — editable.
    Editable,
}

/// Repeatable-block flow parameters.
#[derive(Clone, Debug)]
pub struct Repeat {
    pub direction: RepeatDir,
    pub margin: u8,
    pub limit: Option<usize>,
    pub align: RepeatAlign,
}

/// A resolved block.
#[derive(Clone, Debug)]
pub struct Block {
    pub name: String,
    pub rect: Rect,
    pub content: BlockContent,
    pub layer: Layer,
    pub opacity: Option<f32>,
    pub align_x: Align,
    pub align_y: Align,
    pub fit: Fit,
    /// Explicit `background-size` for an `image` block (overrides `fit`).
    pub image_size: Option<String>,
    pub transition: Option<String>,
    pub repeat: Option<Repeat>,
}

impl Block {
    pub fn is_editable(&self) -> bool {
        self.content == BlockContent::Editable
    }
}

/// A resolved layout: the template it selects (if any) plus its own blocks.
#[derive(Clone, Debug)]
pub struct ResolvedLayout {
    pub template: Option<String>,
    pub blocks: Vec<Block>,
}

/// A resolved, ready-to-render theme.
#[derive(Debug)]
pub struct Theme {
    pub name: String,
    pub cols: u8,
    pub rows: u8,
    /// Token `:root` block + the theme's CSS.
    pub css: String,
    /// template name → its furniture blocks.
    pub templates: BTreeMap<String, Vec<Block>>,
    /// layout name → resolved layout.
    pub layouts: BTreeMap<String, ResolvedLayout>,
    /// Default fragment transition when none is named in the source.
    pub default_transition: String,
}

impl Theme {
    /// Furniture blocks for a layout's selected template (empty if none).
    pub fn template_blocks(&self, layout: &ResolvedLayout) -> &[Block] {
        layout
            .template
            .as_ref()
            .and_then(|t| self.templates.get(t))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

// ---------------------------------------------------------------------------
// Raw deserialized `theme.toml`
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct ThemeFile {
    #[serde(default)]
    name: Option<String>,
    /// Inherit another theme's tokens/layouts/templates/CSS before our own.
    /// A built-in name, a path, or a name under `./themes/`. Omitted (or
    /// "base") = build straight on the substrate.
    #[serde(default)]
    extends: Option<String>,
    /// Grid size; when omitted, inherited (ultimately from base's 64×36).
    #[serde(default)]
    grid: Option<GridCfg>,
    /// Default fragment transition (e.g. "fade", "fade-up", "zoom").
    #[serde(default)]
    transition: Option<String>,
    #[serde(default)]
    tokens: BTreeMap<String, String>,
    /// template name → furniture bundle.
    #[serde(default)]
    template: BTreeMap<String, TemplateFile>,
    /// layout name → template selection + blocks.
    #[serde(default)]
    layout: BTreeMap<String, LayoutFile>,
}

#[derive(Deserialize)]
struct TemplateFile {
    #[serde(default)]
    default: bool,
    /// Token overrides for slides using this template (a "mode" bundle:
    /// background/foreground/etc.), emitted as `.template-<name> { --… }`.
    #[serde(default)]
    tokens: BTreeMap<String, String>,
    #[serde(default)]
    blocks: BTreeMap<String, BlockFile>,
}

#[derive(Deserialize)]
struct LayoutFile {
    /// A template name, or "none" to opt out of the default template.
    #[serde(default)]
    template: Option<String>,
    /// Token overrides scoped to this layout (`.layout-<name> { --… }`);
    /// override the template's tokens.
    #[serde(default)]
    tokens: BTreeMap<String, String>,
    #[serde(default)]
    blocks: BTreeMap<String, BlockFile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BlockFile {
    at: String,
    #[serde(default)]
    image: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    layer: Option<String>,
    #[serde(default)]
    opacity: Option<f32>,
    #[serde(default, rename = "align-x")]
    align_x: Option<String>,
    #[serde(default, rename = "align-y")]
    align_y: Option<String>,
    #[serde(default)]
    fit: Option<String>,
    #[serde(default, rename = "image-size")]
    image_size: Option<String>,
    #[serde(default)]
    transition: Option<String>,
    #[serde(default)]
    repeatable: bool,
    #[serde(default, rename = "repeatable-direction")]
    repeatable_direction: Option<String>,
    #[serde(default, rename = "repeatable-margin")]
    repeatable_margin: Option<u8>,
    #[serde(default, rename = "repeatable-limit")]
    repeatable_limit: Option<usize>,
    #[serde(default, rename = "repeatable-align")]
    repeatable_align: Option<String>,
}

#[derive(Deserialize, Clone, Copy)]
struct GridCfg {
    cols: u8,
    rows: u8,
}

impl Default for GridCfg {
    fn default() -> Self {
        // 64×36 over a 16:9 stage → 30×30px square cells (64:36 == 16:9).
        GridCfg { cols: 64, rows: 36 }
    }
}

// ---------------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------------

fn resolve_block(
    name: &str,
    f: &BlockFile,
    scope: &str,
    asset_base: Option<&Path>,
) -> Result<Block, String> {
    let err = |m: String| format!("{scope} block '{name}': {m}");
    let rect = parse_at(&f.at).ok_or_else(|| err(format!("bad at '{}'", f.at)))?;

    let content = match (&f.image, &f.text) {
        (Some(_), Some(_)) => return Err(err("has both `image` and `text`".into())),
        // A block image lives in the theme dir, so inline it now (against the
        // theme), not at render time (which resolves against the deck dir).
        (Some(img), None) => BlockContent::Image(match asset_base {
            Some(base) => crate::assets::inline(img, base),
            None => img.clone(),
        }),
        (None, Some(txt)) => BlockContent::Text(txt.clone()),
        (None, None) => BlockContent::Editable,
    };
    let is_fixed = content != BlockContent::Editable;

    if f.repeatable && is_fixed {
        return Err(err(
            "repeatable blocks are author-filled — remove `image`/`text`".into(),
        ));
    }
    let has_repeat_props = f.repeatable_direction.is_some()
        || f.repeatable_margin.is_some()
        || f.repeatable_limit.is_some()
        || f.repeatable_align.is_some();
    if has_repeat_props && !f.repeatable {
        return Err(err("`repeatable-*` set without `repeatable = true`".into()));
    }

    let layer = match f.layer.as_deref() {
        None => {
            if is_fixed {
                Layer::Front
            } else {
                Layer::Main
            }
        }
        Some("front") => Layer::Front,
        Some("behind") => Layer::Behind,
        Some(o) => return Err(err(format!("bad layer '{o}' (front|behind)"))),
    };

    let align_x = match f.align_x.as_deref() {
        None | Some("left") => Align::Start, // default is left
        Some("center") => Align::Center,
        Some("right") => Align::End,
        Some(o) => return Err(err(format!("bad align-x '{o}' (left|center|right)"))),
    };
    let align_y = match f.align_y.as_deref() {
        None | Some("top") => Align::Start, // default is top
        Some("center") => Align::Center,
        Some("bottom") => Align::End,
        Some(o) => return Err(err(format!("bad align-y '{o}' (top|center|bottom)"))),
    };
    let fit = match f.fit.as_deref() {
        None | Some("none") => Fit::Natural, // natural flow + clip (default)
        Some("scale") => Fit::Scale,
        Some("cover") => Fit::Cover,
        Some("contain") => Fit::Contain,
        Some(o) => return Err(err(format!("bad fit '{o}' (none|scale|cover|contain)"))),
    };

    let repeat = if f.repeatable {
        let direction = match f.repeatable_direction.as_deref() {
            None | Some("down") => RepeatDir::Down,
            Some("up") => RepeatDir::Up,
            Some("left") => RepeatDir::Left,
            Some("right") => RepeatDir::Right,
            Some(o) => {
                return Err(err(format!(
                    "bad repeatable-direction '{o}' (up|down|left|right)"
                )))
            }
        };
        let align = match f.repeatable_align.as_deref() {
            None | Some("start") => RepeatAlign::Start,
            Some("center") => RepeatAlign::Center,
            Some("end") => RepeatAlign::End,
            Some(o) => {
                return Err(err(format!(
                    "bad repeatable-align '{o}' (start|center|end)"
                )))
            }
        };
        Some(Repeat {
            direction,
            margin: f.repeatable_margin.unwrap_or(0),
            limit: f.repeatable_limit,
            align,
        })
    } else {
        None
    };

    Ok(Block {
        name: name.to_string(),
        rect,
        content,
        layer,
        opacity: f.opacity,
        align_x,
        align_y,
        fit,
        image_size: f.image_size.clone(),
        transition: f.transition.clone(),
        repeat,
    })
}

fn resolve_blocks(
    blocks: &BTreeMap<String, BlockFile>,
    scope: &str,
    asset_base: Option<&Path>,
) -> Result<Vec<Block>, String> {
    blocks
        .iter()
        .map(|(name, bf)| resolve_block(name, bf, scope, asset_base))
        .collect()
}

/// One theme's parsed manifest + raw CSS + the dir its assets resolve against.
struct RawTheme {
    file: ThemeFile,
    css: String,
    asset_base: Option<PathBuf>,
}

/// A layout under construction as the inheritance chain is folded in.
#[derive(Default)]
struct LayoutAcc {
    /// `Some(Some(name))` selects a template, `Some(None)` opts out ("none");
    /// `None` means no theme in the chain set one (→ inherit the default).
    template: Option<Option<String>>,
    blocks: Vec<Block>,
}

impl Theme {
    /// Resolve a theme spec: a built-in name (`default`/`base`), a directory
    /// path, or a name under `./themes/`. Follows `extends` to build the
    /// inheritance chain, then folds it onto the `base` substrate.
    pub fn load(spec: &str) -> Result<Theme, String> {
        let mut seen = Vec::new();
        let chain = resolve_chain(spec, &mut seen)?;
        assemble(&chain)
    }

    /// Assemble a single in-memory theme (base substrate + this one, no
    /// `extends` resolution). Test helper.
    #[cfg(test)]
    fn from_parts(
        toml_src: &str,
        css_src: &str,
        asset_base: Option<&Path>,
    ) -> Result<Theme, String> {
        let file: ThemeFile =
            toml::from_str(toml_src).map_err(|e| format!("parsing theme.toml: {e}"))?;
        let raw = RawTheme {
            file,
            css: css_src.to_string(),
            asset_base: asset_base.map(Path::to_path_buf),
        };
        assemble(std::slice::from_ref(&raw))
    }
}

/// Read one theme's manifest + CSS for a spec, without following `extends`.
/// `default` is compiled in; everything else is a directory or `./themes/<name>`.
fn read_raw(spec: &str) -> Result<RawTheme, String> {
    if spec == "default" {
        let file = toml::from_str(DEFAULT_TOML).map_err(|e| format!("parsing default: {e}"))?;
        return Ok(RawTheme {
            file,
            css: DEFAULT_CSS.to_string(),
            asset_base: None,
        });
    }
    let direct = Path::new(spec);
    let under_themes = Path::new("themes").join(spec);
    let dir = if direct.is_dir() {
        direct.to_path_buf()
    } else if under_themes.is_dir() {
        under_themes
    } else {
        return Err(format!(
            "unknown theme '{spec}' (not a built-in, a directory, or themes/{spec})"
        ));
    };
    let toml = std::fs::read_to_string(dir.join("theme.toml"))
        .map_err(|e| format!("reading {}: {e}", dir.join("theme.toml").display()))?;
    let css = std::fs::read_to_string(dir.join("theme.css")).unwrap_or_default();
    let file: ThemeFile = toml::from_str(&toml).map_err(|e| format!("parsing theme.toml: {e}"))?;
    Ok(RawTheme {
        file,
        css,
        asset_base: Some(dir),
    })
}

/// Follow `extends` to produce the chain root-ancestor → … → `spec` (the leaf).
/// `base` (or omitted `extends`) terminates the chain — base is the substrate,
/// folded in separately, not a link here.
fn resolve_chain(spec: &str, seen: &mut Vec<String>) -> Result<Vec<RawTheme>, String> {
    if seen.iter().any(|s| s == spec) {
        return Err(format!(
            "theme `extends` cycle: {} -> {spec}",
            seen.join(" -> ")
        ));
    }
    seen.push(spec.to_string());
    let raw = read_raw(spec)?;
    let mut chain = match raw.file.extends.as_deref() {
        None | Some("base") => Vec::new(),
        Some(parent) => resolve_chain(parent, seen)?,
    };
    chain.push(raw);
    Ok(chain)
}

/// Emit a token map as a CSS custom-property rule for `selector`.
fn emit_token_rule(out: &mut String, selector: &str, tokens: &BTreeMap<String, String>) {
    if tokens.is_empty() {
        return;
    }
    out.push_str(&format!("\n{selector}{{"));
    for (k, v) in tokens {
        out.push_str(&format!("--{k}:{v};"));
    }
    out.push_str("}\n");
}

/// Fold the `base` substrate + the resolved chain (root → leaf) into a Theme.
fn assemble(chain: &[RawTheme]) -> Result<Theme, String> {
    let base: ThemeFile = toml::from_str(BASE_TOML).expect("embedded base.toml must parse");

    // CSS cascade: base machinery + agnostic look, base tokens, then each theme
    // in chain order — its tokens (override) then its CSS (override), with its
    // own assets inlined so the deck stays self-contained.
    let mut css = String::from(BASE_CSS);
    emit_token_rule(&mut css, ":root", &base.tokens);

    let mut grid = base.grid.unwrap_or_default();
    let mut transition: Option<String> = None;
    let mut name: Option<String> = None;
    let mut templates: BTreeMap<String, Vec<Block>> = BTreeMap::new();
    let mut default_template: Option<String> = None;
    let mut layouts: BTreeMap<String, LayoutAcc> = BTreeMap::new();
    // Per-template / per-layout token overrides, merged by name down the chain.
    let mut template_tokens: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut layout_tokens: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();

    for raw in chain {
        let asset_base = raw.asset_base.as_deref();
        emit_token_rule(&mut css, ":root", &raw.file.tokens);
        let theme_css = match asset_base {
            Some(base) => crate::assets::inline(&raw.css, base),
            None => raw.css.clone(),
        };
        css.push_str(&theme_css);

        if let Some(g) = raw.file.grid {
            grid = g;
        }
        if let Some(t) = &raw.file.transition {
            transition = Some(t.clone());
        }
        if let Some(n) = &raw.file.name {
            name = Some(n.clone());
        }

        // Templates: child overrides by name; at most one `default` per theme.
        let mut default_count = 0u32;
        for (tname, tf) in &raw.file.template {
            if tf.default {
                default_count += 1;
                default_template = Some(tname.clone());
            }
            templates.insert(
                tname.clone(),
                resolve_blocks(&tf.blocks, &format!("template '{tname}'"), asset_base)?,
            );
            if !tf.tokens.is_empty() {
                template_tokens.insert(tname.clone(), tf.tokens.clone());
            }
        }
        if default_count > 1 {
            return Err("more than one template marked `default = true`".into());
        }

        // Layouts: child overrides a layout's template and/or blocks by name.
        for (lname, lf) in &raw.file.layout {
            let entry = layouts.entry(lname.clone()).or_default();
            if let Some(t) = &lf.template {
                entry.template = Some(if t == "none" { None } else { Some(t.clone()) });
            }
            if !lf.blocks.is_empty() {
                entry.blocks =
                    resolve_blocks(&lf.blocks, &format!("layout '{lname}'"), asset_base)?;
            }
            if !lf.tokens.is_empty() {
                layout_tokens.insert(lname.clone(), lf.tokens.clone());
            }
        }
    }

    // Per-template, then per-layout token overrides (equal specificity, so the
    // later `.layout-*` rules win over `.template-*`). Emitted after the chain
    // CSS so they override `:root`.
    for (tname, toks) in &template_tokens {
        emit_token_rule(&mut css, &format!(".template-{tname}"), toks);
    }
    for (lname, toks) in &layout_tokens {
        emit_token_rule(&mut css, &format!(".layout-{lname}"), toks);
    }

    // Resolve each layout's template: explicit selection wins; otherwise inherit
    // the chain's default template. Validate names + block/furniture collisions.
    let mut resolved: BTreeMap<String, ResolvedLayout> = BTreeMap::new();
    for (lname, acc) in layouts {
        let template = match acc.template {
            Some(Some(t)) => {
                if !templates.contains_key(&t) {
                    return Err(format!("layout '{lname}': unknown template '{t}'"));
                }
                Some(t)
            }
            Some(None) => None, // explicit "none"
            None => default_template.clone(),
        };
        if let (Some(tn), tb) = (&template, &templates) {
            if let Some(tb) = tb.get(tn) {
                for b in &acc.blocks {
                    if tb.iter().any(|x| x.name == b.name) {
                        return Err(format!(
                            "layout '{lname}' block '{}' collides with a same-named block in template '{tn}'",
                            b.name
                        ));
                    }
                }
            }
        }
        resolved.insert(
            lname,
            ResolvedLayout {
                template,
                blocks: acc.blocks,
            },
        );
    }

    Ok(Theme {
        name: name.unwrap_or_else(|| "theme".to_string()),
        cols: grid.cols,
        rows: grid.rows,
        css,
        templates,
        layouts: resolved,
        default_transition: transition.unwrap_or_else(|| "fade".to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_default_inherits_defaults() {
        let t = Theme::load("default").unwrap();
        assert_eq!(t.name, "default");
        assert_eq!((t.cols, t.rows), (64, 36));
        assert_eq!(t.default_transition, "fade");
        for l in ["title", "bullets", "stat", "media-split", "quote"] {
            assert!(t.layouts.contains_key(l), "missing layout {l}");
        }
        assert!(t.css.contains("--bg")); // base tokens emitted
        assert!(t.templates.is_empty()); // engine ships no furniture
    }

    #[test]
    fn image_block_parses_image_size() {
        let toml = concat!(
            "[template.brand]\ndefault = true\n[template.brand.blocks]\n",
            "logo = { at = \"x1 y1 x4 y4\", image = \"url('l.png')\", image-size = \"80%\" }\n",
        );
        let t = Theme::from_parts(toml, "", None).unwrap();
        assert_eq!(t.templates["brand"][0].image_size.as_deref(), Some("80%"));
    }

    #[test]
    fn template_and_layout_token_overrides_emit_scoped_css() {
        let toml = concat!(
            "[template.dark]\n[template.dark.tokens]\nbg = \"#182534\"\nfg = \"#fff\"\n",
            "[template.dark.blocks]\n",
            "[layout.bullets]\ntemplate = \"dark\"\n",
            "[layout.bullets.tokens]\naccent = \"#abc\"\n",
            "[layout.bullets.blocks]\nbody = { at = \"x4 y3 x27 y18\" }\n",
        );
        let t = Theme::from_parts(toml, "", None).unwrap();
        assert!(t.css.contains(".template-dark{--bg:#182534;"));
        assert!(t.css.contains(".layout-bullets{--accent:#abc;"));
        // The layout rule must come after the template rule (equal specificity →
        // layout wins).
        let ti = t.css.find(".template-dark{").unwrap();
        let li = t.css.find(".layout-bullets{").unwrap();
        assert!(li > ti, "layout token rule must follow template token rule");
    }

    #[test]
    fn single_theme_is_base_only() {
        // With no `extends`, a theme builds straight on base: it owns ONLY the
        // layouts it defines — no `default` vocabulary leaks in.
        let toml = concat!(
            "name=\"x\"\ntransition=\"zoom\"\n",
            "[tokens]\naccent=\"#abc\"\n",
            "[layout.bullets.blocks]\nbody = { at = \"x4 y3 x27 y18\" }\n"
        );
        let t = Theme::from_parts(toml, ".custom{color:red}", None).unwrap();
        assert_eq!(t.default_transition, "zoom");
        assert!(t.css.contains("--accent:#abc;")); // theme tokens override base
        assert!(t.css.contains(".custom{color:red}"));
        assert_eq!(t.layouts["bullets"].blocks[0].rect.col_start, 4);
        assert!(!t.layouts.contains_key("stat")); // no implicit default inheritance
    }

    #[test]
    fn extends_inherits_parent_layouts_and_tokens() {
        // paper `extends = "default"` → inherits default's core layouts and the
        // base machinery, while its own tokens win.
        let t = Theme::load("paper").unwrap();
        assert_eq!(t.name, "paper");
        for l in ["title", "bullets", "stat", "quote", "compare"] {
            assert!(t.layouts.contains_key(l), "missing inherited layout {l}");
        }
        assert!(t.css.contains("--accent:#b4341c;")); // paper's token overrides default's
        assert!(t.css.contains(".slide-content")); // base machinery present
    }

    #[test]
    fn theme_css_assets_inlined_against_theme_dir() {
        let t = Theme::from_parts(
            "name=\"t\"",
            ".x{background:url('chart.svg')}",
            Some(Path::new("examples")),
        )
        .unwrap();
        assert!(t.css.contains("url('data:image/svg+xml;base64,"));
        assert!(!t.css.contains("url('chart.svg')"));
    }

    #[test]
    fn template_default_applies_and_layout_selects() {
        let toml = concat!(
            "name=\"x\"\n",
            "[template.brand]\ndefault = true\n",
            "[template.brand.blocks]\nlogo = { at = \"x2 y2 x5 y3\", image = \"url('l.png')\" }\n",
            "[template.bare]\n[template.bare.blocks]\n",
            "[layout.bullets.blocks]\nbody = { at = \"x4 y3 x27 y18\" }\n",
            "[layout.title]\ntemplate = \"bare\"\n",
        );
        let t = Theme::from_parts(toml, "", None).unwrap();
        // brand is the default → bullets (defined, untouched template) inherits it.
        assert_eq!(t.layouts["bullets"].template.as_deref(), Some("brand"));
        // title explicitly picked bare.
        assert_eq!(t.layouts["title"].template.as_deref(), Some("bare"));
        // logo resolved as a fixed image block.
        assert!(matches!(
            t.templates["brand"][0].content,
            BlockContent::Image(_)
        ));
        assert_eq!(t.templates["brand"][0].layer, Layer::Front);
    }

    #[test]
    fn template_none_opts_out() {
        let toml = concat!(
            "[template.brand]\ndefault = true\n[template.brand.blocks]\n",
            "logo = { at = \"x2 y2 x5 y3\", image = \"url('l.png')\" }\n",
            "[layout.bullets.blocks]\nbody = { at = \"x4 y3 x27 y18\" }\n",
            "[layout.section]\ntemplate = \"none\"\n",
        );
        let t = Theme::from_parts(toml, "", None).unwrap();
        assert_eq!(t.layouts["section"].template, None);
        assert_eq!(t.layouts["bullets"].template.as_deref(), Some("brand"));
    }

    #[test]
    fn repeatable_block_parses() {
        let toml = concat!(
            "[layout.stat.blocks]\n",
            "figure = { at = \"x4 y7 x9 y13\", repeatable = true, repeatable-direction = \"right\", repeatable-margin = 1, repeatable-limit = 4, repeatable-align = \"center\" }\n",
        );
        let t = Theme::from_parts(toml, "", None).unwrap();
        let fig = &t.layouts["stat"].blocks[0];
        let r = fig.repeat.as_ref().unwrap();
        assert_eq!(r.direction, RepeatDir::Right);
        assert_eq!(r.margin, 1);
        assert_eq!(r.limit, Some(4));
        assert_eq!(r.align, RepeatAlign::Center);
    }

    #[test]
    fn validation_errors() {
        let two_defaults = concat!(
            "[template.a]\ndefault = true\n[template.a.blocks]\n",
            "[template.b]\ndefault = true\n[template.b.blocks]\n",
        );
        assert!(Theme::from_parts(two_defaults, "", None)
            .unwrap_err()
            .contains("default"));

        let unknown_tpl = "[layout.title]\ntemplate = \"ghost\"\n";
        assert!(Theme::from_parts(unknown_tpl, "", None)
            .unwrap_err()
            .contains("unknown template"));

        let fixed_repeat = concat!(
            "[layout.stat.blocks]\n",
            "x = { at = \"x1 y1 x2 y2\", image = \"url('a')\", repeatable = true }\n",
        );
        assert!(Theme::from_parts(fixed_repeat, "", None).is_err());

        let both = concat!(
            "[layout.title.blocks]\n",
            "x = { at = \"x1 y1 x2 y2\", image = \"url('a')\", text = \"hi\" }\n",
        );
        assert!(Theme::from_parts(both, "", None)
            .unwrap_err()
            .contains("both"));

        let collide = concat!(
            "[template.brand]\ndefault = true\n[template.brand.blocks]\n",
            "logo = { at = \"x1 y1 x2 y2\", image = \"url('a')\" }\n",
            "[layout.title.blocks]\nlogo = { at = \"x3 y3 x4 y4\" }\n",
        );
        assert!(Theme::from_parts(collide, "", None)
            .unwrap_err()
            .contains("collides"));

        let bad_rect = "[layout.title.blocks]\nbody = { at = \"nope\" }\n";
        assert!(Theme::from_parts(bad_rect, "", None).is_err());
    }

    #[test]
    fn unknown_theme_errors() {
        assert!(Theme::load("definitely-not-a-theme-xyz").is_err());
    }
}
