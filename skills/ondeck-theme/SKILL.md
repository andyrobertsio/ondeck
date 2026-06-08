---
name: ondeck-theme
description: >-
  Create or customize a theme for the `ondeck` slide-deck tool — the colours,
  fonts, and styling that decks render with. Use this whenever the user wants a
  new ondeck theme, to restyle/rebrand their slides, change the deck palette or
  typography, build a light/dark or branded slide theme, or tweak how ondeck
  decks look. (To match an *existing* deck/brand, use ondeck-theme-from-deck; to
  author slide content, use ondeck-presentation.)
---

# Creating ondeck themes

An ondeck theme is a directory of two files:

```
themes/<name>/
  theme.toml   # tokens (colours/fonts), grid size, optional layout overrides
  theme.css    # optional CSS overrides on top of the engine's base stylesheet
```

Everything is **inherited** from the engine's base stylesheet and default
layouts, so a theme can be as small as a name plus a few tokens — you override
only what you want to change.

**Read `THEMING.md` (top level of the repo) for the complete reference** — every
token, layout selector, slot name, syntax-token class, and chrome/fragment hook
you can target. Don't reverse-engineer it from `base.css`; `THEMING.md` is the
source of truth. `themes/paper` (light) and `themes/bold` (high-contrast) are
worked examples to copy from.

## Tokens (theme.toml)

Tokens are emitted as CSS variables (`--bg`, `--accent`, …) that the base
stylesheet uses, so changing a token restyles every layout at once.

| Token | Controls |
|---|---|
| `bg` | slide background |
| `bg-2` | secondary surface (code blocks, compare cards, raised panels) |
| `fg` | main text |
| `muted` | secondary text (captions, labels, sub-bullets) |
| `accent` | primary accent (headings' highlights, bullet markers, links, stat numbers) |
| `accent-2` | secondary accent (gradients, emphasis, strings in code) |
| `rule` | hairlines / progress track |
| `frame` | letterbox bar colour around the 16:9 stage |
| `font` | main font stack |
| `mono` | code font stack |
| `pad` | global padding unit (a `cqmin` length, e.g. `8cqmin`) |

```toml
name = "midnight-rose"
transition = "fade-up"          # default fragment transition (optional)

[tokens]
bg = "#14101a"
bg-2 = "#1e1726"
fg = "#f3eef7"
muted = "#a99bb5"
accent = "#ff5d8f"
accent-2 = "#c9a2ff"
frame = "#000000"
font = '"Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
```

### Grid

The grid defaults to 32×18 (square cells on the 16:9 stage). If you change it,
keep `cols:rows == 16:9` for square cells:

```toml
[grid]
cols = 32
rows = 18
```

### Overriding layouts (optional)

Layout slot rectangles are inherited; override one by name with `at`-style
rects (`x{c1} y{r1} x{c2} y{r2}`, inclusive cells):

```toml
[layout.bullets]
body = "x4 y3 x28 y16"
```

## theme.css (optional overrides)

Anything you can't express with tokens goes here, layered over the base
stylesheet. The vocabulary is engine-stable: `.slide`, `.slide-content`, `.slot`
/ `.slot-<name>`, `.layout-<name>`, and the code token classes `pre .syn-*`. See
`THEMING.md` for the full list of selectors per layout.

```css
/* round bullet markers instead of squares */
.layout-bullets ul > li::before { border-radius: 50%; }

/* uppercase section dividers */
.layout-section h1 { text-transform: uppercase; letter-spacing: 0.04em; }

/* recolour code tokens (otherwise they follow --accent/--accent-2) */
pre .syn-keyword { color: var(--accent); }
pre .syn-string  { color: var(--accent-2); }
```

### Self-hosted fonts

To use a non-system font and keep decks self-contained, drop the font file in
the theme directory and `@font-face` it from `theme.css` — ondeck inlines the
`url(...)` as a data URI at build time:

```css
@font-face {
  font-family: "Brand Sans";
  src: url("BrandSans.woff2") format("woff2");
  font-weight: 400 700;
}
```
…then set `font = '"Brand Sans", sans-serif'` in `[tokens]`. (Remote
`@import`/Google-Fonts URLs are *not* fetched — self-host the file.)

## Workflow

1. Create `themes/<name>/theme.toml` with your tokens (start from `themes/paper`
   or `themes/bold` if useful).
2. **Test against the demo deck** — it exercises every layout, so it's the right
   harness: `ondeck build examples/demo.md --theme <name> -o /tmp/t.html`.
3. **Screenshot across layouts** and check coherence — at minimum title, bullets,
   stat, quote, code, media-split, compare. The demo covers these; capture a few
   with the platform browser (see *Verifying* in the ondeck-presentation skill,
   or `--open`).
4. Iterate the tokens/CSS until every layout looks right (not just one slide).

## Design guidance

- **Contrast first.** `bg`↔`fg` must be comfortably readable; `muted` should be
  legible but clearly secondary.
- **One or two accents.** `accent` does most of the work; `accent-2` is for
  gradients/emphasis. More than two competing hues reads as noise.
- **Pick `bg-2` close to `bg`** (a subtle raised surface), not a jarring third
  colour — it backs code blocks and cards.
- **Type:** one good font stack via `font`; the base scale is fluid (`cqmin`),
  so you rarely need to touch sizes — adjust via tokens/overrides only if needed.
- **Test light themes carefully:** code blocks (`bg-2`) and the stat gradient
  read very differently on light backgrounds — verify them explicitly.
- **Restraint.** The base stylesheet is already designed; change tokens first,
  reach for `theme.css` only for genuine structural tweaks.

A theme is good when the *whole demo deck* looks coherent and intentional, not
just the title slide.
