# ondeck — format & architecture spec

`ondeck` converts a structured Markdown source into a self-contained HTML
presentation, styled by a theme. It can also export to PDF.

## Design principles

1. **The tool assembles, the browser renders.** `ondeck` emits HTML + CSS
   strings. All layout (flexbox/grid/fonts) happens in the browser. PDF export
   is "open the HTML in headless Chrome and print" — we never do PDF layout
   ourselves.
2. **The layout vocabulary lives in the engine; themes only restyle it.** The
   set of layout names and slot names is stable across every theme, so the
   authoring skill teaches one vocabulary. A theme overrides CSS (and optionally
   HTML structure), and may *add* bespoke layouts, but the core set is
   guaranteed.
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
ondeck watch <input.md> [--theme ...] [--port 7000] [--no-open]
  live-reload server: rebuilds on change (mtime polling), browser auto-reloads
```

## Source format

A document is an optional **deck frontmatter** block, followed by **slides**
separated by a line containing exactly `---`. A slide may begin with its own
**per-slide frontmatter** block.

```markdown
---
theme: midnight
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

### Slots

Multi-region layouts use fenced slot markers: `:::name` … `:::`. Content inside
is Markdown. Single-slot layouts (`bullets`, `statement`, `section`) need no
markers — the whole body is the implicit content.

`::: notes` is a reserved slot: embedded hidden in the HTML (for a future
presenter view), never shown on the slide.

## The stage

Slides render on a **fixed-aspect stage** (default 1920×1080, 16:9) centered in
the viewport with letterbox bars whose colour is the `--frame` token (set by a
theme's `[tokens]`, or per-deck via the `frame:` frontmatter key). The stage
scales to fit — the
on-screen view *is* the design, just scaled. Sizing uses **container-query units**
(`cqmin`/`cqw`/`cqh`) resolved against the slide (a `container-type: size`
element), so type and spacing track the stage rather than the raw window. This is
pure CSS (no JS resize handler), so a theme can override the aspect/size/frame.
The design size also drives the PDF `@page`. The `.fit` overflow scaler still
runs as a JS safety net.

## Grid & layout engine

Every slide is a **32×18 CSS grid** — square cells on a 16:9 stage. (Keep
`cols:rows == aspect` for square cells; both are theme-overridable.) A *layout* is
**data, not code**: a table of named slots → grid rectangles, inherited from the
engine defaults and overridable per theme. Margins come from the rects, not slide
padding. Rendering one generic grid means adding a layout is data, not a new code
path.

```css
.slide-content { display:grid;
  grid-template-columns:repeat(32,1fr); grid-template-rows:repeat(18,1fr); }
.slot { grid-column: 4 / 29; grid-row: 6 / 14; }   /* a slot's rectangle */
```

**Coordinate escape hatch.** Coordinates are a theme-author / power tool, *not*
the default authoring surface — the deck author picks a `layout` and fills slots.
But for a bespoke slide:

- `layout: free` — no predefined slots; every block is placed explicitly.
- `:::block at="x2 y5 x8 y6"` — place a block from cell (col 2,row 5) to
  (col 8,row 6), inclusive, on the 32×18 grid.
- Any slot in *any* layout may carry `at="…"` to override its default rectangle
  on a single slide.

**Overflow policy: scale-to-fit with a clip backstop.** A slot's content is
uniformly transform-scaled down until it fits its cell (works regardless of font
unit); `overflow:hidden` clips only if it hits the floor. A fixed fine grid stays
pleasant instead of brittle, and content is never silently destroyed. We do not
auto-reflow or second-guess placement.

## Layout vocabulary (core set)

Layouts are defined as grid-slot rectangles (above). Slots:

| Layout | Slots | Use |
|---|---|---|
| `title` | (body: `#` title, `##` subtitle, rest meta) | Opening slide |
| `section` | (body) | Section divider |
| `bullets` *(default)* | (body) | The workhorse |
| `two-col` | `left`, `right` (+ body heading) | Side by side |
| `media-split` | `media` (cover image) + body; `media: right` | Image one side (full-bleed), text the other; `media: right` mirrors |
| `statement` | (body) | Big centered idea |
| `quote` | (body) + `:::cite` | Attributed pull-quote |
| `stat` | repeatable `:::stat` (`value · label`) | Big-number slide(s) |
| `stat-3` / `stat-4` | as `stat`, fixed count | Tuned N-column grids (presets over `stat`) |
| `image` | body `![](src)` + optional `:::caption`; `fit: full\|contain` | Image *is* the content (full-bleed by default) |
| `code` | (body: fenced code) | Code, highlighted at build (syntect) |
| `table` | (body: Markdown table) | Themed table; `highlight-col`/`-row`/`row-headers` emphasis |
| `compare` | `left`, `right` | A vs B |
| `raw` | (body: raw HTML) | Escape hatch |

`stat-N` shares `stat`'s rendering path; the variants only differ in grid hints.

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

A theme is a directory — **tokens + grid + layouts (TOML) and styling (CSS)**,
not just a palette.

```
themes/<name>/
  theme.toml      # tokens, grid size, layout slot rectangles
  theme.css       # styling, references the tokens as CSS variables
```

```toml
# theme.toml
name = "midnight"
[grid]
cols = 30
rows = 20
[tokens]                      # emitted as :root CSS vars (--bg, --accent, …)
bg = "#0d1017"
accent = "#7aa2f7"
[layout.two-col]              # layouts are grid-slot rects ("x{c1} y{r1} x{c2} y{r2}")
head  = "x3 y3 x28 y6"
left  = "x3 y8 x15 y18"
right = "x16 y8 x28 y18"
```

The grid vocabulary (`.slide` / `.slide-content` / `.slot` / `.slot-<name>`) is
engine-stable, so themes restyle one vocabulary. Slot names `body`/`head`
receive the loose body; `stats` is the special repeatable-`:::stat` grid.

**Inheritance — a theme only writes overrides.** The engine ships:
- `base.css` (embedded): default `:root` tokens, the grid vocabulary, and
  token-driven default styling for every core layout.
- Default layout rectangles for the core set.

A theme inherits all of it. The CSS cascade is `base.css` → theme `[tokens]`
(override the `:root` defaults) → theme `theme.css` (overrides everything).
Layouts start from the defaults; a theme's `[layout.*]` overrides one or adds a
new one. `theme.css` is optional. So a minimal theme is `name = "…"` plus a few
tokens. Reference themes: **`midnight`** (`name` only — pure inheritance),
**`paper`** (token overrides + a 3-rule `theme.css` → light editorial), and
**`bold`** (electric high-contrast keynote).

**Theme resolution** (`Theme::load`): a built-in name (`midnight`, embedded in
the binary), a directory path, or a name under `./themes/`. Selected by the
`--theme` flag, else the deck's `theme:` frontmatter, else `midnight`.

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
- **Theme inheritance**: `base.css` + default layouts in the engine; a theme only
  writes overrides. Two reference themes: `midnight` (pure inheritance) + `paper`.
- Layouts: `title`, `section`, `bullets`, `statement`, `quote`, `two-col`,
  `media-split`, `stat`, `stat-3`/`stat-4`, `compare`, `code`, `table`, `image`,
  `raw`. Themed Markdown tables with column/row/row-header emphasis.
- Fixed-aspect **stage** (1920×1080 / 16:9 default) with letterbox; pure-CSS
  container-unit scaling; square 32×18 grid; theme-overridable aspect/size.
- Deck chrome (frontmatter toggles): `slide-numbers`, `progress`, `footer`.
- Three reference themes: `midnight`, `paper`, `bold`.
- `ondeck watch` (live-reload server) + `ondeck build --open`.
- Test suite (`cargo test`): parser, grid, fragments, theme, render (19 tests).
- `free` layout + `at="…"` coordinate escape hatch (any slot may override).
- Code highlighting at build via syntect, emitted as **theme-coloured CSS
  classes** (`syn-*`) — no client JS, and overridable per theme.
- **Fragments** (`{+}`, `{+n fx}`, `reveal: true`) with a range of transitions
  (`fade`/`fade-*`/`zoom`/`blur`/`rise`/`none`); within-slide runtime stepping;
  print force-reveal.
- **Slide transitions** (`slide-transition:` deck/per-slide; `none`/`fade`/`slide`,
  directional, default `none`).
- Scale-to-fit overflow with clip backstop (runs on slide activation).
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
inlined — self-host instead); presenter view (notes are embedded but unused);
the authoring skill.
