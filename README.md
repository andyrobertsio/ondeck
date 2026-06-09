# ondeck

[![CI](https://github.com/andyrobertsio/ondeck/actions/workflows/ci.yml/badge.svg)](https://github.com/andyrobertsio/ondeck/actions/workflows/ci.yml)

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
  media-split, stat, compare, code, table, image, raw, free.
- **Fixed-aspect stage** (16:9, 1920×1080) that scales to any window with
  letterbox bars — the on-screen view matches the export exactly.
- **32×18 grid** of **blocks** (the one placed-region primitive) with a
  coordinate escape hatch (`layout: free`, `at="x… y…"`).
- **Theming**: token + CSS themes with inheritance (a theme can be a few tokens),
  plus **templates** for fixed furniture (logo, watermark). Three built in:
  `default`, `paper`, `bold`.
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
theme: default
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
:::head
# By the numbers
:::
:::figure
**142%**

of target
:::
:::figure
**+18**

NPS
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

### Slides & blocks

A **block** is a placed region on the slide. If a layout has a single editable
block, just write Markdown — it flows in (the **single-sink** rule). Layouts with
several blocks address each with a fenced `:::name` … `:::` marker:

```markdown
---
layout: two-col
---
:::head
# Heading
:::
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

A **repeatable** block (like `stat`'s `figure`) is filled by repeating its
marker — one copy per entry.

### Layouts

Single-block layouts take the body directly; multi-block layouts (marked ✳) need
a `:::name` per block.

| Layout | Blocks | Use |
|---|---|---|
| `title` | `#` title, `##` subtitle, rest meta | Opening slide |
| `section` | body | Section divider |
| `bullets` *(default)* | body | The workhorse |
| `statement` | body | Big centered idea |
| `quote` ✳ | `:::body` + `:::cite` | Attributed pull-quote |
| `two-col` ✳ | `:::head` / `:::left` / `:::right` | Side by side |
| `media-split` ✳ | `:::media` image + `:::body`; `media: right` | Image one side, text the other |
| `stat` ✳ | `:::head` + repeatable `:::figure` (`**value**` + label) | Big-number slides |
| `compare` ✳ | `:::head` / `:::left` / `:::right` | A vs B cards |
| `code` | fenced code block | Syntax-highlighted at build |
| `table` | Markdown table | Themed; `highlight-col`/`-row`/`row-headers` emphasis |
| `image` | `![](src)`; `fit: full\|contain` | Image *is* the slide |
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
  -p, --port <PORT>     server port (default 7321; falls back if busy)
      --no-open         don't open a browser automatically

ondeck present <input.md | input.html> [options]
  -t, --theme <SPEC>    theme override (Markdown input only)
      --no-inline       don't embed images (Markdown input only)
  -p, --port <PORT>     server port (default 7321; falls back if busy)
      --no-open         don't open browser windows automatically
```

> **macOS note:** ports 7000/5000 are held by the AirPlay Receiver (it answers
> `403`). `watch`/`present` default to 7321 and auto-fall back to the next free
> port, so this shouldn't bite — but if you force `-p 7000`, expect the conflict.

## Presenting (speaker notes)

Add per-slide notes with a `:::notes` block (hidden on the slide):

```markdown
---
layout: statement
---
# Make it boringly reliable
:::notes
Land the reliability message — this is the emotional pivot.
:::
```

Then open a **two-window presenter view** — audience deck in one, your notes +
current/next previews + timer/clock in the other, kept in sync (navigating either
moves both):

```bash
ondeck present talk.md      # opens audience + presenter windows (http, synced)
ondeck present talk.html    # also works on a prebuilt deck
```

Or, from any deck (even a double-clicked `.html`), press **`P`** to open the
presenter window and **`F`** to fullscreen. `present` is just the convenience that
serves over http and opens both windows for you.

## Themes

A theme is a directory of `theme.toml` (tokens, grid, templates, layout blocks)
and an optional `theme.css` (styling). Everything is inherited from the engine's
defaults, so a theme can be as small as a name plus a few token overrides.

Select a theme with `--theme <name|path>` or the deck's `theme:` frontmatter.
Resolution order: `--theme` → frontmatter → `default`. A `<name>` is looked up
as a built-in, a directory, then `./themes/<name>/`.

```toml
# themes/mytheme/theme.toml
name = "mytheme"
transition = "fade-up"          # default fragment transition

[tokens]                        # emitted as CSS vars (--bg, --accent, …)
bg = "#101417"
accent = "#ff5470"
frame = "#000000"               # letterbox colour

[template.brand]                # fixed furniture, applied to every slide
default = true
[template.brand.blocks]
logo = { at = "x26 y2 x28 y3", image = "url('logo.svg')" }   # inlined; or layer="behind" for a watermark

[layout.bullets.blocks]         # override a layout's blocks (placement, etc.)
body = { at = "x4 y3 x28 y16" }
```

```css
/* themes/mytheme/theme.css — overrides on top of the base stylesheet */
.layout-bullets ul > li::before { border-radius: 50%; }
```

See [`themes/paper`](themes/paper) (light) and [`themes/bold`](themes/bold)
(high-contrast) for worked examples. **[THEMING.md](THEMING.md) is the complete
theming reference** — every token, the block model, templates, layout/block
selectors, syntax-token class, and chrome/fragment hook you can style.

## Authoring with Claude

This repo ships [Claude](https://claude.com/claude-code) skills (in [`skills/`](skills/))
that teach Claude to drive `ondeck`:

- **`ondeck-presentation`** — turn notes/an outline/a topic into a deck.
- **`ondeck-theme`** — create a theme from scratch.
- **`ondeck-theme-from-deck`** — derive a theme matching an existing presentation
  or brand (PowerPoint/PDF/images).

Point Claude Code at this repo (or install the skills) and ask it to, e.g., "make
a deck from these notes" or "create an ondeck theme like our brand template."

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
| `themes/default/base.css` | Engine machinery (stage, grid, `.block`, transitions, print) |
| `themes/default/theme.css` | The default look (palette, typography, per-layout styling) |
| `themes/default/theme.toml` | The engine's layout/block vocabulary, as data |
| `src/assets/runtime.js` | Deck runtime: navigation, fragments, transitions, scale-to-fit |
| `themes/` | Built-in themes; `default` is compiled into the binary |
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
