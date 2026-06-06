# ondeck

Turn a Markdown file into a polished, **self-contained** HTML presentation —
then export it to **PDF** or **PowerPoint**. One small Rust binary, no runtime,
no `node_modules`.

```bash
ondeck build talk.md          # → talk.html  (open it, present)
ondeck build talk.md --pdf    # → talk.html + talk.pdf
ondeck build talk.md --pptx   # → talk.html + talk.pptx
ondeck watch talk.md          # live-reloading preview server
```

You write content and pick a layout; `ondeck` and the theme own all the
boilerplate (the slide scaffolding, CSS, navigation). The output is a single
HTML file with images inlined, so it's trivial to share.

---

## Why

Hand-authoring HTML slides is mostly repetitive scaffolding. `ondeck` keeps the
source terse and content-first while still producing distinctive, designed
slides — via a small layout vocabulary, a themeable design system, and a
coordinate grid for the rare bespoke slide. The browser does all rendering;
`ondeck` just assembles HTML + CSS, and PDF/PPTX are produced by driving a
headless browser over that same output.

## Features

- **Markdown source**, slides split by `---`, with per-slide frontmatter.
- **Layout vocabulary**: title, section, bullets, statement, quote, two-col,
  media-split, stat (+ `stat-3`/`stat-4`), compare, code, image, raw.
- **Fixed-aspect stage** (16:9, 1920×1080) that scales to any window with
  letterbox bars — the on-screen view matches the export exactly.
- **32×18 grid** with a coordinate escape hatch (`layout: free`, `at="x… y…"`).
- **Theming**: token + CSS themes with inheritance (a theme can be a few tokens).
  Three built in: `midnight`, `paper`, `bold`.
- **Fragments** (incremental reveal) with a range of transitions, plus
  slide-to-slide transitions.
- **Self-contained output**: images embedded as data URIs.
- **Exports**: HTML, PDF (vector), PPTX (image-per-slide).
- **Live preview**: `ondeck watch` rebuilds on save and reloads the browser.

## Install / build

Requires a [Rust toolchain](https://rustup.rs) (stable).

```bash
git clone https://github.com/andyrobertsio/ondeck
cd ondeck
cargo build --release
# binary at target/release/ondeck
```

Optionally put it on your `PATH`:

```bash
cp target/release/ondeck /usr/local/bin/    # or ~/.cargo/bin
```

**PDF and PPTX export** additionally need a Chromium-family browser
(Google Chrome, Chromium, Edge, or Brave) installed. `ondeck` auto-detects it; set
`DECK_CHROME=/path/to/browser` to override.

## Quick start

Create `talk.md`:

```markdown
---
theme: midnight
title: My Talk
slide-numbers: true
---

---
layout: title
---
# My Talk
## A short subtitle

---
layout: bullets
reveal: true
---
# What we'll cover

- The problem
- The idea
- The results

---
layout: stat
---
# By the numbers

:::stat
142% · of target
:::
:::stat
+18 · NPS
:::
```

Build and open it:

```bash
ondeck build talk.md && open talk.html    # macOS; or: ondeck build talk.md --open
```

Or develop with live reload:

```bash
ondeck watch talk.md
```

## The format

### Structure

A document is an optional **deck frontmatter** block, then **slides** separated
by a line containing exactly `---`. A slide may begin with its own per-slide
frontmatter.

> **Tip:** the *first* `---…---` block is always the deck frontmatter. So a
> single slide that has its own frontmatter needs a deck-frontmatter block above
> it (it can be empty-ish, e.g. just `theme:`).

Frontmatter is flat `key: value` pairs (values may be quoted).

### Slides & slots

Most layouts take the slide body directly. Multi-region layouts use fenced
**slots**:

```markdown
---
layout: two-col
---
# Heading
:::left
- left column
:::
:::right
- right column
:::

::: notes
Speaker notes — embedded hidden, never shown on the slide.
:::
```

### Layouts

| Layout | Content | Use |
|---|---|---|
| `title` | `#` title, `##` subtitle, rest meta | Opening slide |
| `section` | body | Section divider |
| `bullets` *(default)* | body | The workhorse |
| `statement` | body | Big centered idea |
| `quote` | body + `:::cite` | Attributed pull-quote |
| `two-col` | `:::left` / `:::right` (+ heading) | Side by side |
| `media-split` | `:::media` image + body; `media: right` | Image one side, text the other |
| `stat` / `stat-3` / `stat-4` | repeatable `:::stat` (`value · label`) | Big-number slides |
| `compare` | `:::left` / `:::right` | A vs B cards |
| `code` | fenced code block | Syntax-highlighted at build |
| `image` | `![](src)` + `:::caption`; `fit: full\|contain` | Image *is* the slide |
| `raw` | raw HTML | Escape hatch |
| `free` | `:::block at="x… y…"` | Coordinate placement |

The first slide defaults to `title`; others default to `bullets`.

### Per-slide frontmatter keys

| Key | Values | Notes |
|---|---|---|
| `layout` | a layout name | |
| `background` | `#hex` / named / `var(--token)` / `path.jpg` / `linear-gradient(…)` | Literal = theme-agnostic; `var(--…)` follows the theme. Images inlined |
| `background-fit` | `cover` \| `contain` | For image backgrounds |
| `background-overlay` | `0`–`1` | Darkening scrim (opt-in) |
| `scheme` | `light` \| `dark` | Manual text-colour override (never auto) |
| `fit` | `full` \| `contain` | `image` layout sizing |
| `media` | `right` | `media-split`: mirror image to the right |
| `transition` | a fragment fx name | Default fragment transition for the slide |
| `transition-speed` | e.g. `0.6s` | Sets `--fx-dur` for the slide |
| `slide-transition` | `none` \| `fade` \| `slide` | Transition used entering this slide |

### Deck frontmatter keys

| Key | Values | Notes |
|---|---|---|
| `theme` | name / path | See **Themes** |
| `title` | text | HTML `<title>` |
| `slide-transition` | `none` \| `fade` \| `slide` | Deck-wide default (default `none`) |
| `slide-numbers` | `true` | Show `n / N` |
| `progress` | `true` | Show a progress bar |
| `footer` | text | Footer text on every slide |
| `frame` | colour | Letterbox bar colour (also a theme token) |

### Fragments (incremental reveal)

- `{+}` at the end of a list item / block marks it as a reveal step.
- `{+n}` orders/groups (same `n` reveals together); `{+ fx}` / `{+n fx}` names
  the transition.
- `reveal: true` (slide frontmatter) auto-steps every top-level bullet.

```markdown
- always on screen
- fades in            {+}
- rises up            {+ fade-up}
- step 3, blurs in    {+3 blur}
```

Transitions: `fade` (default), `fade-up/down/left/right`, `zoom`, `zoom-out`,
`blur`, `rise`, `none`. Tune feel with `--fx-dur` / `--fx-ease` (theme- or
slide-overridable). `prefers-reduced-motion` is honoured.

### Coordinate escape hatch

Coordinates are a power tool, not the default surface. On a `layout: free` slide
(or any slot via `at=`), place blocks on the 32×18 grid:

```markdown
---
layout: free
---
:::block at="x2 y2 x16 y9"
# Top-left
:::
:::block at="x18 y10 x31 y17"
A precisely placed block.
:::
```

`at="x{c1} y{r1} x{c2} y{r2}"` spans cells (c1,r1) to (c2,r2) inclusive.

## CLI

```
ondeck build <input.md> [options]
  -o, --output <FILE>   output HTML (default: input with .html)
  -t, --theme <SPEC>    built-in name, directory, or name under ./themes/
      --pdf             also write <out>.pdf (needs a browser)
      --pptx            also write <out>.pptx (needs a browser)
      --no-inline       don't embed local images as data URIs
      --open            open the result in your default browser

ondeck watch <input.md> [options]
  -t, --theme <SPEC>    theme override
      --no-inline       don't embed images
  -p, --port <PORT>     server port (default 7000)
      --no-open         don't open a browser automatically
```

## Themes

A theme is a directory of `theme.toml` (tokens, grid, layout rects) and an
optional `theme.css` (styling). Everything is inherited from the engine's
defaults, so a theme can be as small as a name plus a few token overrides.

Select a theme with `--theme <name|path>` or the deck's `theme:` frontmatter.
Resolution order: `--theme` → frontmatter → `midnight`. A `<name>` is looked up
as a built-in, a directory, then `./themes/<name>/`.

```toml
# themes/mytheme/theme.toml
name = "mytheme"
transition = "fade-up"          # default fragment transition

[tokens]                        # emitted as CSS vars (--bg, --accent, …)
bg = "#101417"
accent = "#ff5470"
frame = "#000000"               # letterbox colour

[layout.bullets]                # override a layout's slot rectangles
body = "x4 y3 x28 y16"
```

```css
/* themes/mytheme/theme.css — overrides on top of the base stylesheet */
.layout-bullets ul > li::before { border-radius: 50%; }
```

See [`themes/paper`](themes/paper) (light) and [`themes/bold`](themes/bold)
(high-contrast) for worked examples. The full token/layout/inheritance contract
is in [SPEC.md](SPEC.md).

## Development

```bash
cargo build            # debug build
cargo build --release  # optimised binary
cargo test             # unit + render tests
cargo run -- build examples/demo.md --open   # build & view the demo deck
```

`examples/demo.md` exercises every layout, fragments, transitions, and the deck
chrome — a good smoke test and reference.

### Project layout

| Path | Role |
|---|---|
| `src/parser.rs` | Markdown → deck frontmatter + slides |
| `src/render.rs` | Slides → self-contained HTML |
| `src/theme.rs` | Theme loading + inheritance |
| `src/grid.rs` | Grid `Rect` + `at=` parsing |
| `src/fragments.rs` | Fragment marker post-processing |
| `src/assets.rs` | Image → data-URI inlining |
| `src/pdf.rs` | PDF export (headless browser) |
| `src/pptx.rs` | PPTX export (image-per-slide OOXML) |
| `src/watch.rs` | Live-reload dev server |
| `src/assets/base.css` | Engine stylesheet (the grid/layout vocabulary) |
| `src/assets/runtime.js` | Navigation, fragments, transitions, scale-to-fit |
| `themes/` | Built-in themes (embedded at build time) |
| `SPEC.md` | Format & architecture reference |

## Notes & limitations

- PDF/PPTX require a Chromium-family browser. PPTX is **image-per-slide**
  (pixel-perfect, not editable) and does one browser launch per slide, so large
  decks export slowly.
- Local images and theme assets (including self-hosted fonts referenced from
  `theme.css`) are inlined; **remote** assets (e.g. a Google Fonts `@import`) are
  not — self-host them for a fully offline, self-contained deck.

## License

MIT — see [LICENSE](LICENSE).
