# Theming ondeck

The complete reference for ondeck themes — every token, layout, CSS hook, and
syntax-token class you can style. For the *why* (architecture, the inheritance
model) see [SPEC.md](SPEC.md); for a guided workflow use the `ondeck-theme` skill.
This document is the exhaustive "what can I target" reference.

## Contents

- [How themes work](#how-themes-work)
- [theme.toml](#themetoml)
- [Tokens](#tokens)
- [The grid & stage](#the-grid--stage)
- [theme.css: the CSS vocabulary](#themecss-the-css-vocabulary)
  - [Structure](#structure-engine-stable)
  - [Typography defaults](#typography-defaults)
  - [Layouts & slots](#layouts--slots)
  - [Syntax-highlight tokens](#syntax-highlight-tokens-code)
  - [Deck chrome](#deck-chrome)
  - [Fragments & transitions](#fragments--transitions)
  - [Scheme overrides](#scheme-overrides)
- [Self-hosted fonts](#self-hosted-fonts)
- [Changing the aspect ratio](#changing-the-aspect-ratio)
- [Tips & gotchas](#tips--gotchas)

## How themes work

A theme is a directory:

```
themes/<name>/
  theme.toml   # tokens, grid, optional layout overrides   (required)
  theme.css    # CSS overrides on the base stylesheet       (optional)
```

The engine ships a **base stylesheet** (`src/assets/base.css`) and **default
layouts**. A theme *inherits all of it* and overrides only what it names — so a
theme can be as small as `name = "x"` plus a handful of tokens.

**Cascade order:** `base.css` → your `[tokens]` (override the `:root` defaults) →
your `theme.css` (overrides everything). **Layouts:** start from the engine
defaults; your `[layout.*]` overrides one or adds a new one.

**Resolution:** a theme is selected by `--theme <spec>` or a deck's `theme:`
frontmatter (flag wins). `<spec>` is a built-in name (`midnight`), a directory
path, or a name under `./themes/`. Built-ins: `midnight`, `paper`, `bold`.

## theme.toml

```toml
name = "my-theme"            # display name
transition = "fade-up"       # default fragment transition (optional)

[grid]                       # optional; defaults to 32×18
cols = 32
rows = 18

[tokens]                     # emitted as CSS variables (--bg, --accent, …)
bg = "#0d1017"
accent = "#7aa2f7"
# … see Tokens below

[layout.bullets]             # optional; override a layout's slot rectangles
body = "x4 y3 x28 y16"       # "x{c1} y{r1} x{c2} y{r2}", inclusive grid cells
```

Layout override values use the same coordinate syntax as the `at=` escape hatch:
cells are 1-indexed on the `cols`×`rows` grid, inclusive of both corners. Slot
names per layout are listed under [Layouts & slots](#layouts--slots).

## Tokens

Every token is a CSS variable consumed by the base stylesheet, so changing one
restyles every layout at once. Defaults below are the engine values (the
`midnight` look).

| Token | Default | Controls |
|---|---|---|
| `bg` | `#0d1017` | Slide background (and the `.stage`) |
| `bg-2` | `#131722` | Secondary surface: code blocks, compare cards, inline `code` |
| `fg` | `#e6e8ee` | Main text |
| `muted` | `#9aa5ce` | Secondary text: captions, labels, sub-bullets, comments |
| `accent` | `#7aa2f7` | Primary accent: links, bullet markers, section headings, stat gradient start, keyword tokens |
| `accent-2` | `#bb9af7` | Secondary accent: `em`, stat gradient end, string tokens |
| `rule` | `#232838` | Hairlines, the progress-bar track |
| `frame` | `#000` | Letterbox bar colour around the stage (also per-deck via `frame:` frontmatter) |
| `pad` | `8cqmin` | Padding unit used by media-split body and image captions |
| `font` | system sans stack | Main font (`font-family`) |
| `mono` | system mono stack | Code/monospace font |
| `stage-w` | `1920` | Design width — drives the stage aspect ratio |
| `stage-h` | `1080` | Design height |
| `fx-dur` | `0.45s` | Transition duration (fragments + slides + progress bar) |
| `fx-ease` | `cubic-bezier(0.2,0.7,0.2,1)` | Transition easing |

`--cols` / `--rows` are emitted from `[grid]` (defaults 32/18) and drive the
slide grid. `--stat-count` is set per-slide by the engine (you can read it but
not set it from a theme).

## The grid & stage

- Slides render on a **fixed-aspect stage** (default 1920×1080 = 16:9) that
  scales to fit the viewport, with letterbox bars (`--frame`).
- The slide is a **`cols`×`rows` CSS grid** (default 32×18). Cells are square
  when `cols:rows == stage aspect` — keep that ratio if you change either.
- **Sizing uses container-query units** (`cqmin`/`cqw`/`cqh`) resolved against
  the slide, so type and spacing scale with the stage. `1cqmin` = 1% of the
  slide's smaller dimension. Prefer `cqmin` for any sizes you add.

## theme.css: the CSS vocabulary

Everything below is engine-stable — these classes/elements are what `theme.css`
can target. Your rules layer on top of the base stylesheet (later + equal
specificity wins; add a class or `!important` only if you must).

### Structure (engine-stable)

| Selector | What it is |
|---|---|
| `.deck` | Viewport-filling letterbox container (background = `--frame`) |
| `.stage` | The fixed-aspect 16:9 box; container for chrome |
| `.slide` | One slide; fills the stage; container for content sizing |
| `.slide.active` / `.slide.leaving` | Current / outgoing-during-transition |
| `.slide-content` | The `cols`×`rows` grid |
| `.slot`, `.slot-<name>` | A placed grid region (e.g. `.slot-body`, `.slot-left`) |
| `.slot .fit` | Scale-to-fit wrapper inside each slot |
| `.slide-overlay` | Background-overlay scrim element (when `background-overlay` set) |

### Typography defaults

`h1` 7cqmin / `h2` 4cqmin (muted) / `h3` 3cqmin / `p` 2.6cqmin. `a` → `--accent`;
`strong` → `--fg` bold; `em` → `--accent-2` (not italic); inline `code` →
`--mono` on `--bg-2`. Override any of these globally or scoped to a layout.

### Layouts & slots

Each slide gets a `.layout-<name>` class. Slot names (the `:::name` regions, plus
the implicit `body`/`head`) per layout:

| Layout | `.layout-` class | Slots | Notable inner hooks |
|---|---|---|---|
| title | `layout-title` | `body` | `.layout-title h1/h2/p` (h2 & p are the subtitle/meta) |
| section | `layout-section` | `body` | `.layout-section h1` (accent, centered) |
| bullets | `layout-bullets` | `body` | `li`, `ul > li::before` (marker), `ol > li::before`, `li li` (sub-items) |
| statement | `layout-statement` | `body` | `.layout-statement h1` (centered) |
| quote | `layout-quote` | `body`, `cite` | `.slot-body::before` (opening “ mark), `.slot-cite p::before` (— dash) |
| two-col | `layout-two-col` | `head`, `left`, `right` | `.slot-head` (bottom-aligned) |
| media-split | `layout-media-split` | `media`, `body` | `.slot-media` (cover image), `.slot-body` (padded) |
| stat / stat-3 / stat-4 | `layout-stat` / `-stat-3` / `-stat-4` | `head`, `stats` | `.stat-grid` (`--stat-count` columns), `.stat-value`, `.stat-label` |
| compare | `layout-compare` | `head`, `left`, `right` | `.slot-left`/`.slot-right` are cards (`--bg-2`); `h3`, `li` |
| code | `layout-code` | `body` | `pre`, `pre code`, syntax tokens (below) |
| image | `layout-image` | `body` (+ `caption`) | `.image-fill`, `.layout-image img`, `.fit-contain img`, `.image-caption` |
| raw | `layout-raw` | `body` | author HTML passes through; `.slide-content` is `display:block` |
| free | `layout-free` | `block` (repeatable, `at=`) | `.slot-block` |

Notable shared structures worth styling:
- **Stat numbers** use a gradient (`--accent`→`--accent-2`) clipped to text via
  `.stat-value`. On light themes you often want a solid colour instead:
  `​.stat-value { background: none; -webkit-background-clip: border-box; color: var(--accent); }`
- **Quote mark**: `.layout-quote .slot-body::before` (the big “); recolour/resize
  or set `content: ""` to remove.
- **Compare cards**: `.layout-compare .slot-left, .layout-compare .slot-right`.

### Syntax-highlight tokens (code)

Code is highlighted into **class-based tokens** (prefix `syn-`), coloured from
theme tokens — no inline styles, so you can fully restyle them. The base colours:

| Class | Default | Typical meaning |
|---|---|---|
| `pre .syn-comment` | `--muted`, italic | comments |
| `pre .syn-keyword`, `pre .syn-storage` | `--accent` | keywords, storage/type words |
| `pre .syn-string` | `--accent-2` | strings |
| `pre .syn-constant` | `--accent-2` | numbers/constants |
| `pre .syn-entity`, `pre .syn-support` | `--accent` | function/type names, builtins |
| `pre .syn-variable` | `--fg` | variables |
| `pre .syn-punctuation` | `--muted` | punctuation |

The highlighter (syntect, `ClassStyle::SpacedPrefixed{"syn-"}`) emits a class per
scope atom, so finer scopes are also targetable (e.g. `.syn-numeric`,
`.syn-function`). The code block itself is `pre` (`--bg-2` background); the code
*layout* bumps size via `.layout-code pre`.

### Deck chrome

Shown only when enabled via deck frontmatter (`slide-numbers`, `progress`,
`footer`); hidden in print/PDF.

| Selector | Element |
|---|---|
| `.deck-number` | "n / N" (bottom-right) |
| `.deck-footer` | footer text (bottom-left) |
| `.deck-progress` | progress track (`--rule`) |
| `.deck-progress > i` | progress fill (`--accent`) |

### Fragments & transitions

**Fragments** (incremental reveal): `.fragment` (hidden), `.fragment.revealed`
(shown). Each transition is a "from" recipe — override or add your own:
`.fx-fade` (opacity only), `.fx-fade-up/-down/-left/-right`, `.fx-zoom`,
`.fx-zoom-out`, `.fx-blur`, `.fx-rise`, `.fx-none`. Duration/easing via
`--fx-dur`/`--fx-ease`. `prefers-reduced-motion` is honoured by the base.

**Slide transitions** are runtime-managed via `.slide.from-*` / `.to-*` /
`.leaving` / `.notrans` — themes rarely touch these; adjust feel via the `--fx-*`
tokens instead.

### Scheme overrides

`.slide.scheme-dark` / `.slide.scheme-light` (set per-slide via `scheme:`) remap
`--fg`/`--muted` for legible text over a custom background. Override these if your
palette needs different on-dark / on-light text colours.

## Self-hosted fonts

To use a non-system font and keep decks self-contained, put the font file in the
theme directory and `@font-face` it in `theme.css` — ondeck inlines the `url(...)`
as a data URI at build time:

```css
@font-face {
  font-family: "Brand Sans";
  src: url("BrandSans.woff2") format("woff2");
  font-weight: 400 700;
}
```

…then `font = '"Brand Sans", sans-serif'` in `[tokens]`. Supported: `woff2`,
`woff`, `ttf`, `otf` (correct MIME emitted). Remote `@import` / Google-Fonts URLs
are **not** fetched — self-host the file.

## Changing the aspect ratio

Set `stage-w`/`stage-h` tokens (and keep `[grid]` `cols:rows` matching for square
cells), e.g. 4:3:

```toml
[tokens]
stage-w = "1440"
stage-h = "1080"
[grid]
cols = 24
rows = 18
```

> **PDF caveat:** the print `@page` size is hardcoded `1920px 1080px` in the base
> stylesheet (CSS `@page` can't read variables). If you change the aspect, also
> override it in `theme.css` so PDF export matches:
> `@page { size: 1440px 1080px; margin: 0; }`

## Tips & gotchas

- **Test the whole demo deck, not one slide.** `ondeck build examples/demo.md
  --theme <name>` then screenshot title/bullets/stat/quote/code/compare/
  media-split. A theme is good when *every* layout looks coherent.
- **Light themes need care.** Re-check the **code block** (`--bg-2` + token
  colours) and the **stat gradient** — both are tuned for dark and often want
  overrides on light backgrounds.
- **Change tokens first.** Reach for `theme.css` only for genuine structural
  tweaks; the base stylesheet is already designed.
- **One or two accents.** `accent` carries the deck; `accent-2` is for
  gradients/emphasis. More competing hues read as noise.
- **`cqmin` for any sizes you add**, so they scale with the stage like the rest.
- Copy [`themes/paper`](themes/paper) (light, with overrides) or
  [`themes/bold`](themes/bold) (high-contrast) as a starting point.
