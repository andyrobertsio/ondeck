//! Theme loading. A theme is a directory of `theme.toml` (tokens + grid +
//! layouts) and `theme.css` (styling). `midnight` is embedded in the binary so
//! `deck` works with zero external files.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::grid::{parse_at, Rect};

/// The engine's default stylesheet (tokens + grid vocabulary + default layout
/// styling), emitted before every theme. Themes override via [tokens]/theme.css.
const BASE_CSS: &str = include_str!("assets/base.css");
const MIDNIGHT_TOML: &str = include_str!("../themes/midnight/theme.toml");

/// Default layout slot rectangles, inherited by every theme. A theme's
/// [layout.*] entries override an existing layout or add a new one.
fn default_layouts() -> BTreeMap<String, Vec<(String, Rect)>> {
    fn layout(slots: &[(&str, [u8; 4])]) -> Vec<(String, Rect)> {
        slots
            .iter()
            .map(|(n, [c1, r1, c2, r2])| (n.to_string(), Rect::cells(*c1, *r1, *c2, *r2)))
            .collect()
    }
    // Rects on the default 32×18 grid (inclusive cells). Margins come from the
    // insets here, not slide padding.
    let mut m = BTreeMap::new();
    m.insert("title".into(), layout(&[("body", [4, 6, 28, 13])]));
    m.insert("section".into(), layout(&[("body", [4, 7, 29, 12])]));
    m.insert("bullets".into(), layout(&[("body", [4, 3, 29, 16])]));
    m.insert("statement".into(), layout(&[("body", [5, 5, 28, 14])]));
    m.insert(
        "two-col".into(),
        layout(&[("head", [4, 3, 29, 5]), ("left", [4, 7, 15, 16]), ("right", [18, 7, 29, 16])]),
    );
    let stat = layout(&[("head", [4, 3, 29, 5]), ("stats", [4, 7, 29, 13])]);
    m.insert("stat".into(), stat.clone());
    // stat-N presets share stat's rendering; CSS can tune them per count.
    m.insert("stat-3".into(), stat.clone());
    m.insert("stat-4".into(), stat);
    m.insert(
        "compare".into(),
        layout(&[("head", [4, 3, 29, 5]), ("left", [4, 7, 15, 16]), ("right", [18, 7, 29, 16])]),
    );
    m.insert("code".into(), layout(&[("body", [4, 3, 29, 16])]));
    m.insert("quote".into(), layout(&[("body", [5, 5, 28, 12]), ("cite", [5, 13, 28, 15])]));
    // media-split: a cover image one side, text the other (`media: right`
    // mirrors). Full-bleed because the slide has no padding.
    m.insert(
        "media-split".into(),
        layout(&[("media", [1, 1, 16, 18]), ("body", [19, 4, 30, 15])]),
    );
    // image and raw fill the whole stage; their content is rendered specially.
    m.insert("image".into(), layout(&[("body", [1, 1, 32, 18])]));
    m.insert("raw".into(), layout(&[("body", [1, 1, 32, 18])]));
    m
}

/// Raw deserialized `theme.toml`.
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
    /// layout name → (slot name → rect string, e.g. "x3 y7 x27 y14")
    #[serde(default)]
    layout: BTreeMap<String, BTreeMap<String, String>>,
}

#[derive(Deserialize)]
struct GridCfg {
    cols: u8,
    rows: u8,
}

impl Default for GridCfg {
    fn default() -> Self {
        // 32×18 over a 16:9 stage → square cells.
        GridCfg { cols: 32, rows: 18 }
    }
}

/// A resolved, ready-to-render theme.
pub struct Theme {
    pub name: String,
    pub cols: u8,
    pub rows: u8,
    /// Token `:root` block + the theme's CSS.
    pub css: String,
    /// layout name → ordered list of (slot name, rect).
    pub layouts: BTreeMap<String, Vec<(String, Rect)>>,
    /// Default fragment transition when none is named in the source.
    pub default_transition: String,
}

impl Theme {
    /// Resolve a theme spec: a built-in name, a directory path, or a name under
    /// `./themes/`.
    pub fn load(spec: &str) -> Result<Theme, String> {
        if spec == "midnight" {
            // The built-in default theme inherits everything from base.css.
            return Theme::from_parts(MIDNIGHT_TOML, "", None);
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
    fn from_parts(toml_src: &str, css_src: &str, asset_base: Option<&Path>) -> Result<Theme, String> {
        let file: ThemeFile =
            toml::from_str(toml_src).map_err(|e| format!("parsing theme.toml: {e}"))?;

        // CSS cascade: base defaults, then theme tokens (override the :root
        // defaults), then the theme's own CSS (overrides everything).
        let mut css = String::from(BASE_CSS);
        css.push_str("\n:root{");
        for (k, v) in &file.tokens {
            css.push_str(&format!("--{k}:{v};"));
        }
        css.push_str("}\n");
        css.push_str(css_src);

        // Inline the theme's own CSS assets (fonts, background images) so a
        // theme with self-hosted assets still produces a self-contained deck.
        if let Some(base) = asset_base {
            css = crate::assets::inline(&css, base);
        }

        // Layouts: start from the inherited defaults, then override/add.
        let mut layouts = default_layouts();
        for (lname, slots) in &file.layout {
            let mut resolved = Vec::with_capacity(slots.len());
            for (sname, rectstr) in slots {
                let rect = parse_at(rectstr).ok_or_else(|| {
                    format!("layout '{lname}' slot '{sname}': bad rect '{rectstr}'")
                })?;
                resolved.push((sname.clone(), rect));
            }
            layouts.insert(lname.clone(), resolved);
        }

        Ok(Theme {
            name: file.name.unwrap_or_else(|| "theme".to_string()),
            cols: file.grid.cols,
            rows: file.grid.rows,
            css,
            layouts,
            default_transition: file.transition.unwrap_or_else(|| "fade".to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_midnight_inherits_defaults() {
        let t = Theme::load("midnight").unwrap();
        assert_eq!(t.name, "midnight");
        assert_eq!((t.cols, t.rows), (32, 18));
        assert_eq!(t.default_transition, "fade");
        // inherited default layouts present
        for l in ["title", "bullets", "stat", "media-split", "quote"] {
            assert!(t.layouts.contains_key(l), "missing layout {l}");
        }
        assert!(t.css.contains("--bg")); // base tokens emitted
    }

    #[test]
    fn overrides_layer_over_defaults() {
        let toml = "name=\"x\"\ntransition=\"zoom\"\n[tokens]\naccent=\"#abc\"\n[layout.bullets]\nbody=\"x4 y3 x27 y18\"\n";
        let t = Theme::from_parts(toml, ".custom{color:red}", None).unwrap();
        assert_eq!(t.default_transition, "zoom");
        assert!(t.css.contains("--accent:#abc;")); // token override emitted
        assert!(t.css.contains(".custom{color:red}")); // theme.css appended
        assert_eq!(t.layouts["bullets"][0].1.col_start, 4); // layout overridden
        assert!(t.layouts.contains_key("stat")); // other layouts still inherited
    }

    #[test]
    fn theme_css_assets_inlined_against_theme_dir() {
        // examples/chart.svg exists; a theme CSS url() should inline against base.
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
    fn unknown_theme_errors() {
        assert!(Theme::load("definitely-not-a-theme-xyz").is_err());
    }
}
