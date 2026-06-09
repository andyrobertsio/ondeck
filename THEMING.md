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
- [Blocks & templates](#blocks--templates)
- [theme.css: the CSS vocabulary](#themecss-the-css-vocabulary)
  - [Structure](#structure-engine-stable)
  - [Typography defaults](#typography-defaults)
  - [Layouts & blocks](#layouts--blocks)
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

The baseline is the built-in **`default` theme** in `themes/default/`, compiled
into the binary. Every theme inherits it:
- `base.css` — engine **machinery** (stage, slide, grid, the `.block` primitive,
  transitions, fragments, print). Structural; you rarely touch it.
- `theme.css` — the default **look** (palette tokens, typography, per-layout
  styling). This is the kind of thing your theme overrides.
- `theme.toml` — the engine's **layout/block vocabulary** as data.

A theme overrides only what it names — so it can be as small as `name = "x"` plus
a handful of tokens.

**Cascade order:** `base.css` (machinery) → `default/theme.css` (default look) →
your `[tokens]` (override the `:root` defaults) → your `theme.css` (overrides
everything). **Layouts:** start from `default/theme.toml`'s definitions; your
`[layout.*]` overrides a layout's `template` and/or its `blocks`.

**Resolution:** a theme is selected by `--theme <spec>` or a deck's `theme:`
frontmatter (flag wins). `<spec>` is a built-in name (`default`), a directory
path, or a name under `./themes/`. Built-ins: `default`, `paper`, `bold`.

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

[template.brand]             # fixed furniture; one template may be the default
default = true
[template.brand.blocks]
logo = { at = "x27 y1 x31 y3", image = "url('logo.svg')" }

[layout.bullets.blocks]      # override a layout's blocks
body = { at = "x4 y3 x28 y16" }   # `at` = "x{c1} y{r1} x{c2} y{r2}", inclusive cells
```

`at` uses the same coordinate syntax as the `at=` escape hatch: cells are
1-indexed on the `cols`×`rows` grid, inclusive of both corners. The block model
(fixed vs editable, templates, repeatables) is described under
[Blocks & templates](#blocks--templates); block names per layout under
[Layouts & blocks](#layouts--blocks).

## Tokens

Every token is a CSS variable consumed by the base stylesheet, so changing one
restyles every layout at once. Defaults below are the engine values (the
`default` look).

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
| `pad` | `8cqmin` | Padding unit used by the media-split body |
| `font` | system sans stack | Main font (`font-family`) |
| `mono` | system mono stack | Code/monospace font |
| `stage-w` | `1920` | Design width — drives the stage aspect ratio |
| `stage-h` | `1080` | Design height |
| `fx-dur` | `0.45s` | Transition duration (fragments + slides + progress bar) |
| `fx-ease` | `cubic-bezier(0.2,0.7,0.2,1)` | Transition easing |

`--cols` / `--rows` are emitted from `[grid]` (defaults 32/18) and drive the
slide grid.

## The grid & stage

- Slides render on a **fixed-aspect stage** (default 1920×1080 = 16:9) that
  scales to fit the viewport, with letterbox bars (`--frame`).
- The slide is a **`cols`×`rows` CSS grid** (default 32×18). Cells are square
  when `cols:rows == stage aspect` — keep that ratio if you change either.
- **Sizing uses container-query units** (`cqmin`/`cqw`/`cqh`) resolved against
  the slide, so type and spacing scale with the stage. `1cqmin` = 1% of the
  slide's smaller dimension. Prefer `cqmin` for any sizes you add.

## Blocks & templates

A **block** is the one placed-region primitive. A block is **fixed** when the
theme gives it content (`image`/`text`) and **editable** otherwise (the author
fills it). Editable blocks come from layouts; fixed furniture comes from
templates.

### Block properties

Every block needs `at`; the rest are optional. Set on any block in a
`[…blocks.<name>]` table (inline `{ … }` or a full sub-table).

| Property | Values | Default | Meaning |
|---|---|---|---|
| `at` | `"x{c1} y{r1} x{c2} y{r2}"` | **required** | Grid placement, inclusive cells |
| `image` | `url('…')` | — | Image content (inlined against the theme dir) → **fixed** |
| `text` | Markdown string | — | Text content → **fixed** |
| `layer` | `front` \| `behind` | `front` (fixed) | Stack vs main content (`.block.layer-*`) |
| `opacity` | `0`–`1` | `1` | Block opacity (e.g. a faint watermark) |
| `align-x` | `left` \| `center` \| `right` | `center` | Horizontal alignment (`.block.ax-*`) |
| `align-y` | `top` \| `center` \| `bottom` | `center` | Vertical alignment (`.block.ay-*`) |
| `fit` | `scale` \| `cover` \| `contain` | `scale` | Content sizing (`.block.fit-*`) |
| `transition` | a fragment fx name | theme/slide default | Fragment transition for this block's content |
| `repeatable` | `true` \| `false` | `false` | A per-entry stamp (editable only) |
| `repeatable-direction` | `up`\|`down`\|`left`\|`right` | `down` | Flow direction of copies |
| `repeatable-margin` | integer (cells) | `0` | Gap between copies |
| `repeatable-limit` | integer | — | Max copies; extras dropped |
| `repeatable-align` | `start`\|`center`\|`end` | `start` | Position copies within the limit-sized track |

### Templates (fixed furniture)

A **template** is a named bundle of fixed blocks. One may be `default` (applied
to every layout that doesn't name its own). A layout selects furniture with
`template = "<name>"`, or opts out with `template = "none"`.

```toml
[template.brand]
default = true                 # every layout gets this unless it says otherwise
[template.brand.blocks]
logo      = { at = "x27 y1 x31 y3", image = "url('logo.svg')" }                       # top-right, front
watermark = { at = "x1 y14 x10 y17", image = "url('mark.svg')", layer = "behind", opacity = 0.12 }

[template.bare]                # a furniture-free look for cover/section slides
[template.bare.blocks]

[layout.title]
template = "bare"              # title opts out of the logo/watermark
```

Furniture is **inlined** (image `url()` resolved against the theme dir, like
fonts), so the deck stays self-contained. A block image can't be re-coloured by
tokens; for a mark that follows `--accent`, use a monochrome SVG and the mask
trick on its `.block-<name>`:

```css
.block-logo {
  background: var(--accent);
  -webkit-mask: url('logo.svg') center / contain no-repeat;
          mask: url('logo.svg') center / contain no-repeat;
}
```

> **A layout can't reposition or hide an individual template block** — furniture
> is all-or-nothing per layout (select a different template, or `none`). Vary the
> look by defining a second template, as `bare` above. You *can* still restyle
> furniture via `.block-<name>` / `.template-<name> .block-<name>` in `theme.css`.

### Repeatable blocks

A `repeatable` editable block stamps one copy per authored `:::name` entry. Copies
flow from the anchor `at` along `repeatable-direction` by *(block extent +
`repeatable-margin`)*, capped at `repeatable-limit`, and positioned within that
limit-sized track by `repeatable-align` (`center` centres a partial count). This
is how `stat`'s `figure` works — define your own for timelines, logo walls, etc.

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
| `.block`, `.block-<name>` | A placed grid region (e.g. `.block-body`, `.block-left`) |
| `.block .fit` | Scale-to-fit wrapper inside a `fit:scale` block |
| `.block.layer-behind` / `.layer-front` | Stack band vs main content |
| `.block.ax-*` / `.block.ay-*` | Horizontal / vertical alignment hooks |
| `.block.fit-cover` / `.fit-contain` | Media-sizing hooks (size the inner `<img>`) |
| `.layout-<name>` / `.template-<name>` | On the slide: its layout and selected template |
| `.slide-overlay` | Background-overlay scrim element (when `background-overlay` set) |

### Typography defaults

`h1` 7cqmin / `h2` 4cqmin (muted) / `h3` 3cqmin / `p` 2.6cqmin. `a` → `--accent`;
`strong` → `--fg` bold; `em` → `--accent-2` (not italic); inline `code` →
`--mono` on `--bg-2`. Override any of these globally or scoped to a layout.

### Layouts & blocks

Each slide gets a `.layout-<name>` class. Block names (the `:::name` regions, plus
the implicit single-sink `body`/`head`) per layout, each rendered as
`.block-<name>`:

| Layout | `.layout-` class | Blocks | Notable inner hooks |
|---|---|---|---|
| title | `layout-title` | `body` | `.layout-title h1/h2/p` (h2 & p are the subtitle/meta) |
| section | `layout-section` | `body` | `.layout-section h1` (accent, centered) |
| bullets | `layout-bullets` | `body` | `li`, `ul > li::before` (marker), `ol > li::before`, `li li` (sub-items) |
| statement | `layout-statement` | `body` | `.layout-statement h1` (centered) |
| quote | `layout-quote` | `body`, `cite` | `.block-body::before` (opening “ mark), `.block-cite p::before` (— dash) |
| two-col | `layout-two-col` | `head`, `left`, `right` | `.block-head` (bottom-aligned) |
| media-split | `layout-media-split` | `media`, `body` | `.block-media` (cover image, `fit:cover`), `.block-body` (padded) |
| stat | `layout-stat` | `head`, `figure` (repeatable) | `.block-figure strong` (the big number), `.block-figure p` (label) |
| compare | `layout-compare` | `head`, `left`, `right` | `.block-left`/`.block-right` are cards (`--bg-2`); `h3`, `li` |
| code | `layout-code` | `body` | `pre`, `pre code`, syntax tokens (below) |
| table | `layout-table` | `body` | `table`/`thead th`/`tbody td`; emphasis classes (see [Tables](#tables--emphasis)) |
| image | `layout-image` | `body` | `.layout-image img`, `.fit-contain img`; full-bleed (`.slide-content` is `display:block`) |
| raw | `layout-raw` | `body` | author HTML passes through; `.slide-content` is `display:block` |
| free | `layout-free` | author-placed `:::name at="…"` | `.block-<name>` per authored block |

Notable shared structures worth styling:
- **Stat numbers**: each `figure` is author Markdown — `**value**` becomes
  `.block-figure strong`, styled with a gradient (`--accent`→`--accent-2`)
  clipped to text. On light themes use a solid colour:
  `​.layout-stat .block-figure strong { background: none; -webkit-background-clip: border-box; color: var(--accent); }`
- **Quote mark**: `.layout-quote .block-body::before` (the big “); recolour/resize
  or set `content: ""` to remove.
- **Compare cards**: `.layout-compare .block-left, .layout-compare .block-right`.

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

### Tables & emphasis

Markdown tables render to `<table>` and are styled from tokens, so they work in
any layout (the `table` layout just frames one as the slide's focus). Hooks:

| Selector | Styled |
|---|---|
| `table` | full width, collapsed borders, `2.4cqmin` |
| `thead th` | bold, `--rule` underline |
| `tbody td` | `--rule` row hairlines |
| `th[align=…]`, `td[align=…]` | Markdown column alignment (`:--`, `--:`, `:--:`) is respected |

**Emphasis** is opt-in via per-slide frontmatter (the engine adds a class to the
slide; base.css styles it). Column/row indices are 1-based and supported 1–8
(keep tables modest):

| Frontmatter | Class added | Effect |
|---|---|---|
| `highlight-col: N` | `.hl-col-N` | tints column N (`--bg-2`) + accent header |
| `highlight-row: N` | `.hl-row-N` | tints body row N (`--bg-2`) |
| `row-headers: true` | `.row-headers` | first column styled as bold labels |

A theme can restyle these (e.g. a stronger highlight) — they're plain CSS
classes. For `colspan`/`rowspan` or per-cell control, use a `raw` HTML `<table>`;
it inherits the same table styling.

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
