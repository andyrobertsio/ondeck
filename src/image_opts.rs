//! Inline image options: `![alt](src){ cover|contain|natural  top/…/center  N% }`.
//!
//! A post-process over rendered Markdown (same shape as `fragments`): the `{…}`
//! that comrak leaves as literal text right after an `<img>` is folded into a
//! marker class + an inline `style`. Inline style is deliberate — it overrides
//! the block's `fit` rules, so an author's `{…}` wins.
//!
//! - **fit**: `cover` | `contain` | `natural` → `object-fit` (+ sizing).
//! - **position**: `top`/`bottom`/`left`/`right`/`center` → both `object-position`
//!   (crop framing, for cover/contain) and `justify-self`/`align-self`
//!   (placement of a scaled/natural image). `center` centres; a following axis
//!   keyword overrides that axis (`center left`, `center top`).
//! - **scale**: `<n>%` → `max-width:n%`, aspect-locked (shrink to a slot fraction).
//! - **decoration**: `border` / `round` / `shadow` → token-driven classes
//!   (`img-bordered` / `img-round` / `img-shadow`) the theme styles.
//!
//! A brace group starting with `+` is a fragment marker (`{+ fx}`) — left for
//! `fragments::apply`. A group with no recognised token is left untouched.

use regex::{Captures, Regex};

/// Fold `![](src){…}` image options into the rendered `<img>` tags.
pub fn apply(html: &str) -> String {
    // An <img …> immediately followed by a `{…}` whose first char isn't `+`.
    let re = Regex::new(r"(<img\b[^>]*>)\s*\{([^+}][^}]*)\}").unwrap();
    re.replace_all(html, |caps: &Captures| {
        rewrite(&caps[1], &caps[2]).unwrap_or_else(|| caps[0].to_string())
    })
    .into_owned()
}

/// Rewrite one `<img>` tag from its option string, or `None` if no option was
/// recognised (so the original `{…}` is left in place).
fn rewrite(tag: &str, opts: &str) -> Option<String> {
    let mut fit: Option<&str> = None;
    let mut pos_x: Option<&str> = None;
    let mut pos_y: Option<&str> = None;
    let mut any_pos = false;
    let mut scale: Option<&str> = None;
    let mut border = false;
    let mut round = false;
    let mut shadow = false;
    let mut valid = false;

    for tok in opts.split_whitespace() {
        match tok {
            "cover" | "contain" | "natural" => fit = Some(tok),
            "left" | "right" => pos_x = Some(tok),
            "top" | "bottom" => pos_y = Some(tok),
            "center" => any_pos = true,
            "border" => border = true,
            "round" => round = true,
            "shadow" => shadow = true,
            t if is_pct(t) => scale = Some(t),
            _ => continue, // ignore unknown tokens
        }
        valid = true;
        if matches!(tok, "left" | "right" | "top" | "bottom") {
            any_pos = true;
        }
    }
    if !valid {
        return None;
    }

    let mut classes = String::from("img-opt");
    let mut style = String::new();
    match fit {
        Some("cover") => {
            classes.push_str(" imgfit-cover");
            style.push_str("object-fit:cover;width:100%;height:100%;");
        }
        Some("contain") => {
            classes.push_str(" imgfit-contain");
            style.push_str("object-fit:contain;width:100%;height:100%;");
        }
        Some("natural") => {
            style.push_str("object-fit:initial;width:auto;height:auto;max-width:100%;")
        }
        _ => {}
    }
    // Decorations — class only; base.css styles them from tokens (themeable).
    if border {
        classes.push_str(" img-bordered");
    }
    if round {
        classes.push_str(" img-round");
    }
    if shadow {
        classes.push_str(" img-shadow");
    }
    if any_pos {
        // Same keywords drive the crop (object-position, for cover/contain) AND
        // the placement (justify-self/align-self, for a scaled/natural image).
        // `img-placed` opts the wrapper into a full-block grid to place within.
        classes.push_str(" img-placed");
        let x = pos_x.unwrap_or("center");
        let y = pos_y.unwrap_or("center");
        let jx = match x {
            "left" => "start",
            "right" => "end",
            _ => "center",
        };
        let jy = match y {
            "top" => "start",
            "bottom" => "end",
            _ => "center",
        };
        style.push_str(&format!(
            "object-position:{x} {y};justify-self:{jx};align-self:{jy};"
        ));
    }
    if let Some(s) = scale {
        style.push_str(&format!("max-width:{s};height:auto;"));
    }

    // Insert class + style before the tag's closing `/>` / `>`.
    let body = tag
        .trim_end()
        .trim_end_matches('>')
        .trim_end_matches('/')
        .trim_end();
    let style_attr = if style.is_empty() {
        String::new()
    } else {
        format!(" style=\"{style}\"")
    };
    Some(format!("{body} class=\"{classes}\"{style_attr} />"))
}

/// A 1–3 digit percentage, e.g. `60%`.
fn is_pct(t: &str) -> bool {
    let Some(n) = t.strip_suffix('%') else {
        return false;
    };
    !n.is_empty() && n.len() <= 3 && n.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::apply;

    #[test]
    fn scale_only() {
        let out = apply(r#"<img src="a.png" alt="" />{60%}"#);
        assert!(out.contains(r#"class="img-opt""#));
        assert!(out.contains("max-width:60%;height:auto;"));
        assert!(!out.contains('{')); // marker stripped
    }

    #[test]
    fn fit_and_position() {
        let out = apply(r#"<img src="a.png" alt="x" />{cover top left}"#);
        assert!(out.contains("imgfit-cover"));
        assert!(out.contains("object-fit:cover;width:100%;height:100%;"));
        assert!(out.contains("object-position:left top;"));
        assert!(out.contains("justify-self:start;align-self:start;"));
    }

    #[test]
    fn position_defaults_unset_axis_to_center() {
        let out = apply(r#"<img src="a.png" />{top}"#);
        assert!(out.contains("object-position:center top;"));
        assert!(out.contains("justify-self:center;align-self:start;"));
    }

    #[test]
    fn center_with_axis_override_places_image() {
        // `center left` → vertical centre, horizontal left.
        let out = apply(r#"<img src="a.png" />{60% center left}"#);
        assert!(out.contains("max-width:60%;"));
        assert!(out.contains("justify-self:start;align-self:center;"));
        assert!(out.contains("object-position:left center;"));
    }

    #[test]
    fn contain_with_scale() {
        let out = apply(r#"<img src="a.png" />{contain 75%}"#);
        assert!(out.contains("imgfit-contain"));
        assert!(out.contains("max-width:75%;height:auto;"));
    }

    #[test]
    fn decorations_add_classes() {
        let out = apply(r#"<img src="a.png" />{round border shadow}"#);
        assert!(out.contains("img-bordered"));
        assert!(out.contains("img-round"));
        assert!(out.contains("img-shadow"));
        // decorations are class-only, no inline style
        assert!(!out.contains("style="));
    }

    #[test]
    fn fragment_marker_is_left_alone() {
        let out = apply(r#"<img src="a.png" />{+ fade-up}"#);
        assert_eq!(out, r#"<img src="a.png" />{+ fade-up}"#);
    }

    #[test]
    fn unknown_tokens_left_untouched() {
        let src = r#"<img src="a.png" />{see fig 2}"#;
        assert_eq!(apply(src), src); // no valid token → not rewritten
    }

    #[test]
    fn leaves_non_image_braces() {
        let src = "<p>a value {x} here</p>";
        assert_eq!(apply(src), src);
    }
}
