//! Theme loading. A theme is a directory of `theme.toml` (tokens + grid +
//! templates + layouts) and `theme.css` (styling). The `default` theme — its
//! `base.css`, `runtime.js`, and manifest in `themes/default/` — is compiled into
//! the binary so `ondeck` works with zero external files.
//!
//! Model: a **block** is the one placed-region primitive (positioned by `at=`
//! cells). A block is *fixed* when the theme gives it content (`image`/`text`),
//! otherwise *editable* (the author fills it). A **template** is a named bundle
//! of fixed furniture blocks; a **layout** selects a template and owns its own
//! blocks. A slide picks a layout.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Deserialize;

use crate::grid::{parse_at, Rect, RepeatAlign, RepeatDir};

/// The engine's default stylesheet (tokens + grid vocabulary + default layout
/// styling), emitted before every theme. Themes override via [tokens]/theme.css.
/// Engine machinery (structural CSS), emitted first for every theme.
const BASE_CSS: &str = include_str!("../themes/default/base.css");
/// The default theme's look (palette + typography + per-layout styling), emitted
/// next for every theme as the base styling all themes inherit.
const DEFAULT_CSS: &str = include_str!("../themes/default/theme.css");
/// The default theme's manifest: the engine's layout/block vocabulary as data.
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
// Engine layout vocabulary — parsed from the embedded `default` theme.toml, so
// the layouts are data (discoverable + editable), not hardcoded. Every theme
// inherits these and overrides selectively. The default theme has no templates
// (furniture is theme-only), so these layouts carry no template.
// ---------------------------------------------------------------------------

fn default_layouts() -> BTreeMap<String, ResolvedLayout> {
    let file: ThemeFile =
        toml::from_str(DEFAULT_TOML).expect("embedded default theme.toml must parse");
    let mut m = BTreeMap::new();
    for (lname, lf) in &file.layout {
        let blocks = resolve_blocks(&lf.blocks, &format!("default layout '{lname}'"), None)
            .expect("embedded default layout blocks must resolve");
        m.insert(
            lname.clone(),
            ResolvedLayout {
                template: None,
                blocks,
            },
        );
    }
    m
}

// ---------------------------------------------------------------------------
// Raw deserialized `theme.toml`
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct ThemeFile {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    grid: GridCfg,
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
    #[serde(default)]
    blocks: BTreeMap<String, BlockFile>,
}

#[derive(Deserialize)]
struct LayoutFile {
    /// A template name, or "none" to opt out of the default template.
    #[serde(default)]
    template: Option<String>,
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

#[derive(Deserialize)]
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
        None | Some("center") => Align::Center,
        Some("left") => Align::Start,
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
        None | Some("scale") => Fit::Scale,
        Some("cover") => Fit::Cover,
        Some("contain") => Fit::Contain,
        Some(o) => return Err(err(format!("bad fit '{o}' (scale|cover|contain)"))),
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

impl Theme {
    /// Resolve a theme spec: a built-in name, a directory path, or a name under
    /// `./themes/`.
    pub fn load(spec: &str) -> Result<Theme, String> {
        if spec == "default" {
            // The built-in baseline is compiled in, so it resolves with no
            // `themes/` dir on disk.
            return Theme::from_parts(DEFAULT_TOML, "", None);
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
        Theme::from_parts(&toml, &css, Some(&dir))
    }

    /// `asset_base`, when set, is the theme directory: `url(…)` references in the
    /// theme's CSS (fonts, background images) are inlined as data URIs against it.
    fn from_parts(
        toml_src: &str,
        css_src: &str,
        asset_base: Option<&Path>,
    ) -> Result<Theme, String> {
        let file: ThemeFile =
            toml::from_str(toml_src).map_err(|e| format!("parsing theme.toml: {e}"))?;

        // CSS cascade: engine machinery, then the default look (the base styling
        // every theme inherits), then this theme's [tokens] (override the :root
        // defaults), then its theme.css (overrides everything).
        let mut css = String::from(BASE_CSS);
        css.push_str(DEFAULT_CSS);
        css.push_str("\n:root{");
        for (k, v) in &file.tokens {
            css.push_str(&format!("--{k}:{v};"));
        }
        css.push_str("}\n");
        css.push_str(css_src);

        // Inline the theme's own CSS assets (fonts, background images, block
        // images referenced from theme.css) so the deck stays self-contained.
        if let Some(base) = asset_base {
            css = crate::assets::inline(&css, base);
        }

        // Templates, and the (at most one) default.
        let mut templates: BTreeMap<String, Vec<Block>> = BTreeMap::new();
        let mut default_template: Option<String> = None;
        let mut default_count = 0u32;
        for (tname, tf) in &file.template {
            if tf.default {
                default_count += 1;
                default_template = Some(tname.clone());
            }
            templates.insert(
                tname.clone(),
                resolve_blocks(&tf.blocks, &format!("template '{tname}'"), asset_base)?,
            );
        }
        if default_count > 1 {
            return Err("more than one template marked `default = true`".into());
        }

        // Layouts: engine defaults, then theme overrides (template and/or blocks
        // independently).
        let mut layouts = default_layouts();
        let mut explicit_template: BTreeSet<String> = BTreeSet::new();
        for (lname, lf) in &file.layout {
            let entry = layouts
                .entry(lname.clone())
                .or_insert_with(|| ResolvedLayout {
                    template: None,
                    blocks: vec![],
                });
            if let Some(t) = &lf.template {
                explicit_template.insert(lname.clone());
                entry.template = if t == "none" {
                    None
                } else if templates.contains_key(t) {
                    Some(t.clone())
                } else {
                    return Err(format!("layout '{lname}': unknown template '{t}'"));
                };
            }
            if !lf.blocks.is_empty() {
                entry.blocks =
                    resolve_blocks(&lf.blocks, &format!("layout '{lname}'"), asset_base)?;
            }
        }
        // Layouts that didn't name a template inherit the default template.
        for (lname, rl) in layouts.iter_mut() {
            if !explicit_template.contains(lname) {
                rl.template = default_template.clone();
            }
        }
        // A block named in both a layout and its template is ambiguous.
        for (lname, rl) in &layouts {
            if let Some(tn) = &rl.template {
                if let Some(tb) = templates.get(tn) {
                    for b in &rl.blocks {
                        if tb.iter().any(|x| x.name == b.name) {
                            return Err(format!(
                                "layout '{lname}' block '{}' collides with a same-named block in template '{tn}'",
                                b.name
                            ));
                        }
                    }
                }
            }
        }

        Ok(Theme {
            name: file.name.unwrap_or_else(|| "theme".to_string()),
            cols: file.grid.cols,
            rows: file.grid.rows,
            css,
            templates,
            layouts,
            default_transition: file.transition.unwrap_or_else(|| "fade".to_string()),
        })
    }
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
    fn overrides_layer_over_defaults() {
        let toml = concat!(
            "name=\"x\"\ntransition=\"zoom\"\n",
            "[tokens]\naccent=\"#abc\"\n",
            "[layout.bullets.blocks]\nbody = { at = \"x4 y3 x27 y18\" }\n"
        );
        let t = Theme::from_parts(toml, ".custom{color:red}", None).unwrap();
        assert_eq!(t.default_transition, "zoom");
        assert!(t.css.contains("--accent:#abc;"));
        assert!(t.css.contains(".custom{color:red}"));
        assert_eq!(t.layouts["bullets"].blocks[0].rect.col_start, 4);
        assert!(t.layouts.contains_key("stat")); // others still inherited
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
            "[layout.title]\ntemplate = \"bare\"\n",
        );
        let t = Theme::from_parts(toml, "", None).unwrap();
        // brand is the default → bullets (untouched) inherits it.
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
