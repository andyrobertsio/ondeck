# ondeck — format & architecture spec

`ondeck` converts a structured Markdown source into a self-contained HTML
presentation, styled by a theme. It can also export to PDF.

## Design principles

1. **The tool assembles, the browser renders.** `ondeck` emits HTML + CSS
   strings. All layout (flexbox/grid/fonts) happens in the browser. PDF export
   is "open the HTML in headless Chrome and print" — we never do PDF layout
   ourselves.
2. **The layout vocabulary lives in the engine; themes only restyle it.** The
   set of layout names and block names is stable across every theme, so the
   authoring skill teaches one vocabulary. A theme overrides CSS, may *add*
   bespoke layouts, and may own fixed furniture via **templates**, but the core
   set is guaranteed.
3. **Explicit over magic.** Layout is chosen explicitly (with inference only for
   the obvious). We never second-guess the author — e.g. we give contrast knobs
   but never auto-adjust colors.
4. **Deterministic output.** Same source + theme → byte-identical HTML, so we
   can golden-file snapshot test it.

## CLI

```
ondeck build <input.md> [-o out.html] [--theme <name|path>] [--no-inline] [--pdf] [--pptx] [--open]
  --pdf        also write <out>.pdf via headless Chrome over ?mode=print
  --pptx       also write <out>.pptx (one full-bleed PNG per slide, via ?shot=N)
  --no-inline  don't embed local images as data URIs
  --open       open the built HTML in the default browser
  --theme      built-in name, directory, or name under ./themes/
ondeck watch <input.md> [--theme ...] [--port 7321] [--no-open]
  live-reload server: rebuilds on change (mtime polling), browser auto-reloads;
  if the port is already in use (incl. a service that bind alone can't detect,
  e.g. macOS AirPlay on 7000), falls back to the next free port
ondeck present <input.md | input.html> [--theme ...] [--port 7321] [--no-open]
  like watch, but opens two synced windows — audience + presenter dashboard
  (?present=1); accepts a Markdown source (built + watched) or a prebuilt .html
```

## Presenter view

`:::notes` content (hidden per-slide) powers a two-window presenter experience:
an audience window (the normal deck) and a **presenter** window
(`?present=1`) showing the current slide, a next-slide preview, the slide's
notes, an elapsed timer, and a wall clock. Navigation in either window drives
both. Sync transport: **`window.open` + `postMessage`** when the deck opens its
own presenter window (works from `file://`, e.g. pressing **`P`**), plus
**`BroadcastChannel`** when served over http (used by `ondeck present`). **`F`**
toggles fullscreen. It's all client-side JS in the self-contained output — the
deck can present itself from a bare file; `present` is the http convenience.

## Source format

A document is an optional **deck frontmatter** block, followed by **slides**
separated by a line containing exactly `---`. A slide may begin with its own
**per-slide frontmatter** block.

```markdown
---
theme: default
title: Q3 Review
---

---
layout: title
---
# Q3 Review
## How we did

---
layout: two-col
background: "#0a0a0a"
scheme: dark
---
# What changed
:::left
- shipped X
- killed Y
:::
:::right
![](chart.png)
:::

::: notes
Speaker notes — embedded hidden, never rendered to the slide.
:::
```

### Parsing rules

- A line that is exactly `---` is a **slide boundary**.
- The block between a boundary and the next `---` is **frontmatter** *if and only
  if* its first non-blank line looks like a key (`^[A-Za-z][\w-]*\s*:`).
  Otherwise the boundary just begins a slide whose body starts immediately.
- The first frontmatter block in the document (before any slide content) is the
  **deck frontmatter**.
- Frontmatter is **flat `key: value` pairs** (not full YAML) — simpler and
  predictable. Values may be quoted.
- Caveat: a slide body must not *start* with a `word:` line (it would be read as
  frontmatter). Lead with a heading or blank line.

### Blocks

A **block** is the one placed-region primitive. Editable blocks are filled by the
author with fenced markers: `:::name` … `:::` (content is Markdown).

**Single-sink rule:** if a layout has exactly *one* editable block, loose
(unslotted) Markdown fills it — so `bullets`, `statement`, `section`, `title`,
`code`, `table`, `image` need no markers. A layout with two or more editable
blocks (`two-col`, `compare`, `quote`, `media-split`, `stat`) requires every one
to be addressed with `:::name`.

A **repeatable** block is filled by writing its `:::name` block multiple times
(e.g. `stat`'s `:::figure`); each entry renders a copy (see Grid & layout).

`::: notes` is reserved: embedded hidden in the HTML (for a future presenter
view), never shown on the slide. It is not a block.

### Image options

An inline `{…}` *after* a Markdown image sets how it fits, crops, and scales —
mirroring the `{+ fx}` fragment marker. All parts optional, order-independent:

```markdown
![](photo.jpg){cover top}    # cover-crop, framed to the top
![](logo.png){60%}           # shrink to 60% of the slot, aspect locked
![](map.png){contain 75%}    # combine
```

- **fit**: `cover` \| `contain` \| `natural` → `object-fit` (overrides the block's
  `fit` for this image).
- **position**: `top`/`bottom`/`left`/`right`/`center` → the image's crop
  (`object-position`, for `cover`/`contain`) **and** its placement
  (`justify-self`/`align-self`, for a scaled/natural image). `center` centres
  both axes; a following axis keyword overrides it (`center left` = left +
  vertically centred; `center top` = horizontally centred + top).
- **scale**: `<n>%` → `max-width:n%`, aspect-locked (shrink to a fraction of the
  slot; never upscales).
- **decoration**: `border` / `round` / `shadow` → token-driven classes the theme
  styles (`--img-border`, `--radius`, `--img-shadow`).

A post-process folds the `{…}` into a class + inline style on the `<img>`. A
group starting with `+` is a fragment marker, not image options; an unrecognised
group is left as literal text. The theme-side default for an image's crop is the
block's `align-x`/`align-y` (see THEMING); the author's `{…}` overrides it.

## The stage

Slides render on a **fixed-aspect stage** (default 1920×1080, 16:9) centered in
the viewport with letterbox bars whose colour is the `--frame` token (set by a
theme's `[tokens]`, or per-deck via the `frame:` frontmatter key). The stage
scales to fit — the
on-screen view *is* the design, just scaled. Sizing uses **container-query units**
(`cqmin`/`cqw`/`cqh`) resolved against the slide (a `container-type: size`
element), so type and spacing track the stage rather than the raw window. This is
pure CSS (no JS resize handler), so a theme can override the aspect/size/frame.
The design size also drives the PDF `@page`. (A block can opt into JS
scale-to-fit via `fit: scale`; it is no longer the default — see Overflow.)

## Grid & layout engine

Every slide is a **64×36 CSS grid** — square cells on a 16:9 stage. (Keep
`cols:rows == aspect` for square cells; both are theme-overridable.) A *layout* is
**data, not code**: a set of named blocks, each a grid rectangle plus styling
hints (layer, fit, alignment, repeat), inherited from the engine defaults and
overridable per theme. Margins come from the rects, not slide padding. Rendering
one generic grid means adding a layout is data, not a new code path.

A block is **fixed** when the theme gives it content (`image`/`text`) and
**editable** otherwise (the author fills it). Blocks default to **top-left**
(`align-y: top`, `align-x: left`), overridable per block. A **template** is a
named bundle of fixed furniture blocks (logo, watermark); a layout selects a
template (or the deck's `default` one) and renders its furniture *plus* the
layout's own blocks.

```css
.slide-content { display:grid;
  grid-template-columns:repeat(64,1fr); grid-template-rows:repeat(36,1fr); }
.block { grid-column: 4 / 29; grid-row: 6 / 14; }   /* a block's rectangle */
```

A **repeatable** block (e.g. `stat`'s `figure`) stamps one copy per authored
entry, flowing from its anchor along `repeatable-direction` by (extent +
`repeatable-margin`), capped at `repeatable-limit`, positioned within that track
by `repeatable-align`.

**Coordinate escape hatch.** Coordinates are a theme-author / power tool, *not*
the default authoring surface — the deck author picks a `layout` and fills slots.
But for a bespoke slide:

- `layout: free` — no predefined blocks; the author places every block explicitly.
- `:::block at="x3 y9 x16 y12"` — place a block from cell (col 3,row 9) to
  (col 16,row 12), inclusive, on the 64×36 grid.

**Overflow policy: author to fit; clip the rest.** By default (`fit: none`) a
block's content flows naturally and is **clipped** (`overflow:hidden`) if it
exceeds the cell — the engine never auto-scales or reflows. A block can opt into
scale-to-fit with **`fit: scale`** (wrap in `.fit`, transform-scale down until it
fits — the JS scaler still ships for this), or `cover`/`contain` for media. This
keeps a fixed fine grid honest and predictable; sizing in `cqmin` tracks the
stage. We do not second-guess placement.

## Layout vocabulary (core set)

Layouts are defined as named blocks (above). Single-block layouts take loose
Markdown (single-sink); multi-block layouts need `:::name` on each block.

| Layout | Blocks | Use |
|---|---|---|
| `title` | `body` (`#` title, `##` subtitle, rest meta) | Opening slide |
| `section` | `body` | Section divider |
| `bullets` *(default)* | `body` | The workhorse |
| `lede` | `head`, `body` | Heading + body in a logical column — a long `head` grows and pushes `body` down |
| `two-col` | `head`, `left`, `right` | Side by side |
| `media-split` | `media` (cover image, `fit:cover`), `body`; `media: right` mirrors | Image one side (full-bleed), text the other |
| `statement` | `body` | Big centered idea |
| `quote` | `body`, `cite` | Attributed pull-quote |
| `stat` | `head`, `figure` (repeatable) | Big-number slide(s); the theme styles each figure's Markdown (`**value**` = the number) |
| `image` | `body` (`![](src)`); `fit: full\|contain` | Image *is* the content (full-bleed) |
| `code` | `body` (fenced code) | Code, highlighted at build (syntect) |
| `table` | `body` (Markdown table) | Themed table; `highlight-col`/`-row`/`row-headers` emphasis |
| `compare` | `head`, `left`, `right` | A vs B |
| `raw` | `body` (raw HTML) | Escape hatch |
| `free` | author-placed via `:::block at="…"` | Coordinate escape hatch |

`stat` replaces the former `stat`/`stat-3`/`stat-4` presets: the `figure` block
is repeatable (`limit` 4, centred), so 2–4 figures Just Work.

## Per-slide frontmatter keys

| Key | Values | Notes |
|---|---|---|
| `layout` | layout name | Defaults to `bullets`; `title` inferred for first slide if unset |
| `background` | `#hex` / named color / `var(--token)` / `path.jpg` / `linear-gradient(...)` | Type inferred from the string. A literal (`#0a0a0a`) is theme-agnostic; `var(--bg-2)` or a token-based gradient is theme-relative and follows the active theme. Images inlined as data URIs. Behind content — distinct from the `image` layout |
| `background-fit` | `cover` \| `contain` | For image backgrounds |
| `background-overlay` | `0`–`1` | Optional darkening scrim. **Off by default.** Opt-in only |
| `scheme` | `light` \| `dark` | Text treatment override. **Manual only** — we never auto-detect contrast |
| `highlight-col` / `highlight-row` | `1`–`8` | Emphasize a table column/row (tinted + accent) |
| `row-headers` | `true` | Style a table's first column as labels |
| `table-spacing` | `default` \| `compact` \| `comfortable` | Table cell density (remaps cell-pad tokens) |
| `table-style` | `lines` \| `stripes` \| `borders` \| `none` | Table row/border treatment |

## Fragments (incremental reveal)

**Authoring.** One atom plus sugar:

- `{+}` at the end of a list item / block marks it as a reveal step.
- `{+n}` sets/groups the step number (same `n` = revealed together; ascending
  `n` = order). `{+ fx}` / `{+n fx}` names the transition for that fragment.
- `reveal: true` (slide frontmatter) auto-applies `{+}` to every top-level list
  item — the common "build this whole list" case with no inline markers.
- Slide `transition:` sets the default transition; `transition-speed:` sets
  `--fx-dur` for the slide.

```markdown
- always on screen
- fades in            {+}
- rises up            {+ fade-up}
- step 3, blurs in    {+3 blur}
```

**Transitions** are named CSS recipes on `.fragment.fx-<name>` — each defines only
the hidden "from" state; `.revealed` resets to neutral. Built-in: `fade`
(default), `fade-up/down/left/right`, `zoom`, `zoom-out`, `blur`, `rise`, `none`.
Feel is tuned by `--fx-dur` / `--fx-ease` (theme- or slide-overridable); themes
add transitions by adding CSS. Default resolution: per-fragment `{+ fx}` →
slide `transition:` → theme `transition` → engine `fade`. `prefers-reduced-motion`
is honored. The same `fx-*` vocabulary can later drive slide-to-slide transitions.

**Mechanics.** Build emits `class="fragment fx-<fx>" data-step="n"`; the runtime
reveals steps within a slide before advancing. Hidden fragments keep their layout
space (no reflow on reveal). Stepping back animates out for free.

- **Additive only** in v1: final state is always fully in the DOM. Replace-style
  fragments are reserved for later.
- **`?mode=print`** adds a `print` class to `<body>` that force-reveals all
  fragments to final state. The PDF path loads `…?mode=print` — no separate render
  path.

## Slide transitions

Slide-to-slide transitions reuse the `--fx-dur` / `--fx-ease` feel.

- **`slide-transition:`** in deck frontmatter sets the deck-wide default; a
  per-slide `slide-transition:` overrides it (the transition used when *entering*
  that slide). Default: `none`.
- **Set:** `none` (instant), `fade` (dissolve — the outgoing slide fades out over
  the opaque incoming, so the background never gaps to the stage/frame), `slide`
  (directional push — forward enters from the right, backward reverses).
- **Mechanics:** both slides are `display:block` during the animation; the
  incoming starts at a `from-*` state and animates to rest, the outgoing animates
  to a `to-*` state and drops to `display:none` on `transitionend` (with a
  timeout fallback). Rapid navigation snaps any in-flight transition first.
- `.deck` is positioned + `overflow:hidden` so a slide pushed off-screen during
  a transition doesn't create a transient scrollbar (print overrides this to
  `overflow:visible` for pagination).
- Screen-only; print/PDF is one page per slide, unaffected.

The same `from-*`/`to-*` CSS vocabulary can host more transitions (zoom, etc.)
without runtime changes.

## Theme contract

> Complete, exhaustive theming reference (every token, layout selector, slot,
> code-token class, chrome/fragment hook): **[THEMING.md](THEMING.md)**. This
> section is the architectural overview.

A theme is a directory — **tokens + grid + templates + layouts (TOML) and styling
(CSS)**, not just a palette.

```
themes/<name>/
  theme.toml      # tokens, grid size, templates (furniture), layout blocks
  theme.css       # styling, references the tokens as CSS variables
```

```toml
# theme.toml
name = "my-theme"
extends = "default"           # inherit a theme first (optional; omitted = base-only)
[grid]
cols = 64
rows = 36
[tokens]                      # emitted as :root CSS vars (--bg, --accent, …)
bg = "#0d1017"
accent = "#7aa2f7"
[template.brand]              # named furniture; one may be the deck-wide default
default = true
[template.brand.blocks]
logo = { at = "x51 y3 x56 y6", image = "url('logo.svg')" }   # fixed (inlined)
[layout.two-col.blocks]       # a layout's blocks; `at` is "x{c1} y{r1} x{c2} y{r2}"
head  = { at = "x5 y5 x56 y12" }
left  = { at = "x5 y15 x30 y36" }
right = { at = "x31 y15 x56 y36" }
```

The grid vocabulary (`.slide` / `.slide-content` / `.block` / `.block-<name>`,
plus `.layout-<name>` / `.template-<name>` on the slide) is engine-stable, so
themes restyle one vocabulary. A block is fixed when it has `image`/`text`, else
editable; `repeatable = true` makes it a per-entry stamp.

**Block properties:** `at` (required), `image`/`text` (content → fixed),
`layer` (`front`|`behind`), `opacity`, `align-x`/`align-y` (default top-left),
`fit` (`none`*default*|`scale`|`cover`|`contain`), `background-color`,
`transition`, `column` + `expandable-y`/`fill` (see below), and `repeatable` +
`repeatable-direction`/`-margin`/`-limit`/`-align`. An `image` block renders as
a CSS background, positioned by `align-x`/`align-y` and sized by `fit` (or an
explicit `image-size`). A `background-color` paints the block as a decorative
colour panel: it renders even when empty and is excluded from the loose-Markdown
sink (address it by name to put content over the fill). A layout selects furniture via `template = "<name>"` or
`template = "none"`; with neither, it inherits the deck's `default` template. A
template or layout may also carry a `[…tokens]` table — token overrides scoped to
slides using it (`.template-<name>`/`.layout-<name>`), so a dark-mode template
flips `bg`/`fg` as data instead of a CSS rule (layout tokens win over template).

**Logical columns.** Blocks are placed at fixed rects, so overflow clips. Blocks
sharing a `column = "<name>"` instead **flow** in a flex column (`.block-column`)
placed at their bounding-box rect, ordered by `at`: each is fixed-height by
default, `expandable-y` grows with content (pushing siblings), `fill` absorbs the
remainder. Members that share a **starting row** (aligned tops) form a horizontal
**band** (`.block-band`) laid out side by side, so a heading can push a
two-column body down as a unit (a boundary touch from inclusive `at` still
stacks). A gap between members' `at` rows is kept as a fixed spacer (so it
survives expansion). Short content is pixel-identical to the fixed grid; long
content
pushes within the column rect (clipping if even that overflows). This trades
determinism for content-adaptive layout — the long-title-pushes-content case —
and is opt-in per block.

**Validation:** at most one `default` template per theme; `at` required; a block
can't be both fixed and `repeatable`; a layout can't name a template that doesn't
exist; a block name can't appear in both a layout and its template.

**Substrate + inheritance — a theme writes overrides on a base.** The substrate
is **`base`** (`themes/base/`, compiled in), emitted beneath every deck:
- `base.css` — engine **machinery** (stage, slide, grid, the `.block` primitive,
  transitions, fragments, print) **plus a layout-agnostic, token-driven look**
  (typography, inline code, code blocks + `syn-*`, tables). Structural.
- `base.toml` — a **neutral token contract** (every token, neutral values) + the
  default **grid**. base ships **no layouts**.

A theme layers on base, and may **`extends = "<other>"`** to inherit that theme's
tokens, layouts, templates, and CSS first. The CSS cascade is `base.css` → base
`[tokens]` → for each theme in the `extends` chain (root → leaf) its `[tokens]`
then its `theme.css`. Layouts/templates/tokens merge by name down the chain
(child wins); grid + default transition take the last set. With no `extends` a
theme builds straight on base and owns its **whole** layout vocabulary; with
`extends = "default"` it inherits the core layouts and overrides selectively.
`theme.css` is optional, so a minimal theme is `name` + `extends` + a few tokens.

Reference themes: the substrate **`base`**; the bundled **`default`** (palette +
the core layout vocabulary + per-layout look; the common `extends` target);
**`paper`** (`extends = "default"`, token overrides + a 3-rule `theme.css` →
light editorial); **`bold`** (`extends = "default"`, electric high-contrast
keynote).

**Theme resolution** (`Theme::load`): a built-in name (`default`/`base`, embedded
in the binary), a directory path, or a name under `./themes/`. Selected by the
`--theme` flag, else the deck's `theme:` frontmatter, else `default`. `extends`
targets resolve the same way; chains are followed with cycle detection.

Slide-content images (content + backgrounds) are inlined as base64 data URIs
(`--no-inline` opts out). A theme's own CSS assets (background images, self-hosted
fonts via `theme.css` `url()`) are inlined against the theme directory too.
Remote assets (e.g. a Google Fonts `@import`) are not fetched.

## Architecture (Rust)

- `comrak` — Markdown → HTML
- `syntect` — build-time code highlighting (no client-side JS)
- `minijinja` / string templates — layout assembly
- `clap` — CLI
- `axum` + `notify` — `watch` server (later)
- PDF — `chromiumoxide` or shell out to a detected Chrome/Chromium/Edge, loading
  `?mode=print`. Behind an optional feature; core stays dependency-light.

## Status

**Built:**
- Grid engine (size from theme); layouts as data-driven slot tables.
- **Themes as TOML** (tokens + grid + layout rects) + CSS; loaded from a built-in
  name, a directory path, or `./themes/`; selected via `--theme` or frontmatter.
- **Substrate + `extends` inheritance**: the built-in `base` substrate
  (`themes/base/`: `base.css` machinery + agnostic look, `base.toml` neutral
  token contract + grid; no layouts) is compiled in and emitted beneath every
  deck. A theme layers on base and may `extends = "<other>"` to inherit its
  tokens/layouts/templates/CSS (chains followed, cycle-detected). The bundled
  `default` theme owns the core layouts; `bold`/`paper` extend it.
- Layouts: `title`, `section`, `bullets`, `lede`, `statement`, `quote`,
  `two-col`, `media-split`, `stat` (repeatable figures), `compare`, `code`,
  `table`, `image`, `raw`, `free`. Themed Markdown tables with
  column/row/row-header emphasis. Logical columns (`expandable-y`/`fill`).
- **Templates** (theme furniture) + **blocks** (the unified placed-region
  primitive): fixed vs editable by content, single-sink authoring, repeatables.
- Fixed-aspect **stage** (1920×1080 / 16:9 default) with letterbox; pure-CSS
  container-unit scaling; square 64×36 grid; theme-overridable aspect/size.
- Deck chrome (frontmatter toggles): `slide-numbers`, `progress`, `footer`.
- Reference themes: substrate `base`; bundled `default` (core layouts); `paper`
  and `bold` (both `extends = "default"`).
- `ondeck watch` (live-reload server) + `ondeck build --open`.
- **Presenter view**: two-window audience + notes/preview dashboard, synced via
  `postMessage`/`BroadcastChannel`; `P` opens it, `F` fullscreen; `ondeck present`
  serves both windows over http (Markdown or prebuilt `.html`).
- Test suite (`cargo test`): parser, grid, fragments, theme, render (40 tests).
- `free` layout + `at="…"` coordinate escape hatch (any slot may override).
- Code highlighting at build via syntect, emitted as **theme-coloured CSS
  classes** (`syn-*`) — no client JS, and overridable per theme.
- **Fragments** (`{+}`, `{+n fx}`, `reveal: true`) with a range of transitions
  (`fade`/`fade-*`/`zoom`/`blur`/`rise`/`none`); within-slide runtime stepping;
  print force-reveal.
- **Slide transitions** (`slide-transition:` deck/per-slide; `none`/`fade`/`slide`,
  directional, default `none`).
- Overflow: natural flow + clip by default; opt-in `fit: scale` scale-to-fit
  (runs on slide activation), `cover`/`contain` for media.
- Per-slide `background` (color/gradient/`var()`/image path), `scheme`,
  `background-overlay`, `fit` (image); hidden `::: notes`.
- **Image inlining**: content (`<img>`) and background (`url(…)`) images are
  embedded as base64 data URIs (paths relative to the source file; remote/`data:`
  left alone). `--no-inline` opts out. Output is fully self-contained.
- Self-contained HTML; keyboard nav + `?mode=print`.
- **PDF export** (`--pdf`): headless Chrome/Chromium/Edge/Brave over `?mode=print`;
  page size from CSS `@page` (1920×1080px → 1440×810pt landscape); fragments
  force-revealed; `DECK_CHROME` overrides browser detection.
- **PPTX export** (`--pptx`): one full-bleed PNG per slide (captured via
  `?shot=N` in headless Chrome) packed into a minimal OOXML `.pptx`.
  Pixel-identical to the HTML/PDF; **not editable** (a distribution format). One
  Chrome launch per slide, so large decks are slow.

**Not yet built:** remote asset fetching (a Google Fonts `@import` isn't
inlined — self-host instead).
