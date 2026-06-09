---
name: ondeck-presentation
description: >-
  Author polished slide decks from Markdown with the `ondeck` CLI (self-contained
  HTML output, plus PDF and PPTX export). Use this whenever the user wants to
  create a presentation, slide deck, or slides — turning notes, an outline, a
  topic, a report, or raw content into slides; building a talk, pitch, lecture,
  or readout deck; or when they mention ondeck, Markdown slides, or want slides
  exported to HTML/PDF/PowerPoint. Prefer this over hand-writing HTML, reveal.js,
  or Marp decks.
---

# Authoring presentations with ondeck

`ondeck` turns a structured Markdown file into a self-contained HTML slide deck
(images inlined), and can export the same deck to PDF or PPTX. You write content
and pick a layout per slide; the tool and theme own all the scaffolding (slide
container, CSS, navigation). Your job is to produce a terse, well-composed `.md`
source and verify the rendered result.

Full format and architecture reference: `SPEC.md`; user docs: `README.md` (in the
ondeck repo). This skill is the practical authoring guide — consult those for
exhaustive detail.

## Setup: locate the CLI

Run `ondeck --version` first. If it's not on `PATH`:
- Inside the ondeck repo, use `cargo run --release -- <args>` or build once
  (`cargo build --release`) and call `./target/release/ondeck`.
- Otherwise build/install from https://github.com/andyrobertsio/ondeck.

PDF/PPTX export additionally needs a Chromium-family browser installed; plain
HTML does not.

## Workflow

1. **Understand the content & intent.** What's the material, the audience, the
   rough length? If the user handed you raw notes/a doc, extract the structure
   (sections → slides). If the ask is vague, make reasonable choices and refine.
2. **Pick a theme.** The built-in `default` (dark) is used if none is set;
   `paper` (light editorial) and `bold` (high-contrast keynote) also ship. Set it
   in deck frontmatter. To make a custom one, use the `ondeck-theme` skill.
3. **Draft the `.md`** — terse, one idea per slide, choosing a layout per slide
   (see *Choosing a layout*).
4. **Build:** `ondeck build talk.md` (writes `talk.html` next to the source; use
   `-o path.html` to choose the output).
5. **Verify visually.** Open `talk.html`, or screenshot slides headlessly. Don't
   trust the source alone — check that content fits and looks right.
6. **Iterate.** Tighten wording, fix overflow, vary layouts. Export to PDF/PPTX
   only when the user wants those (`--pdf` / `--pptx`).

For an authoring loop, `ondeck watch talk.md` serves a live-reloading preview.

## Source format

A document is optional **deck frontmatter**, then **slides** separated by a line
that is exactly `---`. A slide may start with its own per-slide frontmatter.
Frontmatter is flat `key: value` (quote values with spaces/`#`).

```markdown
---
theme: default
title: Q3 Review
slide-numbers: true
---

---
layout: title
---
# Q3 Review
## How we did, and what's next
June 2026 · All-hands

---
layout: bullets
reveal: true
---
# What changed
- Shipped the new onboarding flow
- Cut p95 latency by **40%**
- Migrated billing with zero downtime
```

> **Critical gotcha:** the *first* `---…---` block is always the deck
> frontmatter. So the first real slide needs its own `---`-fenced frontmatter
> block after it (see the title slide above). A slide body must also not *start*
> with a `word:` line, or it's mistaken for frontmatter — lead with a heading.

A slide is built from **blocks**. Single-block layouts (title, bullets, statement,
section, code, table, image) take the body directly — just write Markdown.
Multi-block layouts use fenced markers `:::name` … `:::` for every block.
`::: notes` holds speaker notes (hidden).

## Choosing a layout

The single biggest quality lever: **don't make every slide bullets.** Match the
layout to the content.

| Content you have | Layout | Notes |
|---|---|---|
| Deck opening | `title` | `#` title, `##` subtitle, then an optional plain line = meta (date/event) |
| A topic/section change | `section` | Big divider — use these to give structure |
| A list of points | `bullets` *(default)* | The workhorse; keep to ~3–6 items |
| One punchy idea / takeaway | `statement` | A big centered headline (`#`), optionally with one supporting line under it. Great for emphasis |
| A quotation | `quote` | `:::body` = quote; `:::cite` = attribution |
| A key metric (or 2–4) | `stat` | `:::head` + a repeatable `:::figure` per number (`**value**` + label) |
| Two things side by side | `two-col` | `:::head` / `:::left` / `:::right` |
| A vs B | `compare` | `:::head` / `:::left` / `:::right` rendered as cards |
| Image + explanation | `media-split` | `:::media` image one side, `:::body` text the other (`media: right` to flip) |
| An image *is* the point | `image` | body `![](src)`; `fit: full|contain` |
| Code | `code` | fenced code block, syntax-highlighted |
| Tabular data / a feature matrix | `table` | a Markdown table; keep it modest (≤~6 cols, ≤~8 rows) |
| Something bespoke | `raw` (raw HTML) or `free` (coordinate placement) | escape hatches; use rarely |

### Block examples

```markdown
---
layout: stat
---
:::head
# By the numbers
:::
:::figure
**142%**

of revenue target
:::
:::figure
**+18**

NPS points
:::
:::figure
**40%**

faster p95
:::
```
The `figure` block is repeatable — write one `:::figure` per number (up to 4,
auto-centred). Bold (`**…**`) is the big number; the rest is the label.

```markdown
---
layout: quote
---
:::body
The best way to predict the future is to invent it.
:::
:::cite
Alan Kay
:::
```

```markdown
---
layout: media-split
media: right
---
:::body
# Built for the field
Crews see the next job, route, and parts before they leave the depot.
:::
:::media
![Depot](depot.jpg)
:::
```

```markdown
---
layout: table
highlight-col: 3      # tint a column (1–8) to call it out; or highlight-row: N
---
# Plans
| Feature | Free | Pro  | Team |
| ------- | :--: | :--: | ---: |
| Seats   | 1    | 5    | ∞    |
| SSO     | —    | —    | ✓    |
```
Tables are plain Markdown (column alignment via `:--`/`--:`/`:--:` is respected)
and styled by the theme. Keep them modest — a slide can't show a spreadsheet; for
big data, show a chart image or split it. `row-headers: true` styles the first
column as labels. For `colspan`/`rowspan`, drop to a `raw` HTML table.

## Fragments (incremental reveal)

Reveal content step-by-step on click — good for builds and walkthroughs.

- `{+}` at the end of a list item / block marks it as a reveal step.
- `{+n}` orders/groups (same `n` reveals together); `{+ fx}` names the transition.
- `reveal: true` in slide frontmatter auto-steps every top-level bullet (the
  common case — prefer this over marking each line).

```markdown
- on screen immediately
- appears next        {+}
- then this, rising   {+ fade-up}
```

Transitions: `fade` (default), `fade-up/down/left/right`, `zoom`, `zoom-out`,
`blur`, `rise`, `none`. Use motion sparingly — it should aid focus, not distract.

## Backgrounds, scheme, transitions, chrome

Per-slide frontmatter:
- `background:` — `#hex` / named / `var(--bg-2)` (theme-relative, preferred) /
  `path.jpg` / `linear-gradient(...)`. `background-overlay: 0–1` adds a scrim;
  `scheme: light|dark` overrides text colour over a custom background.
- `slide-transition: none|fade|slide` — entering this slide (deck-wide default
  via deck frontmatter).

Deck frontmatter: `slide-numbers: true`, `progress: true`, `footer: "…"`,
`frame: "#…"` (letterbox colour).

## Coordinate escape hatch (rare)

For a genuinely bespoke slide, `layout: free` places blocks on a 32×18 grid:

```markdown
---
layout: free
---
:::block at="x2 y2 x16 y9"
# Top-left
:::
```
`at="x{c1} y{r1} x{c2} y{r2}"` spans cells (c1,r1)→(c2,r2) inclusive. Reach for
this only when no standard layout fits — the named layouts carry the design.

## Composition principles

- **One idea per slide.** If a slide has two ideas, split it.
- **Open with `title`, structure with `section` dividers, close with a
  `statement`.** Give the deck a shape.
- **Vary the layouts.** A deck that's all bullets is a wall of text; reach for
  `stat`, `quote`, `statement`, `media-split`, `compare` to create rhythm.
- **Keep text terse.** Slides are not a document — short phrases, not sentences.
  Headlines should state the point, not label the topic ("p95 dropped 40%", not
  "Performance").
- **Let content overflow guide you.** If a slide doesn't fit (see verify step),
  cut words or split the slide rather than shrinking everything.

## Verifying

Output must be checked, not assumed. After building, either open the HTML
(`ondeck build … --open`) or capture slides headlessly with the platform's
Chromium binary (it's usually not on `PATH` as bare `chrome`):

```bash
# macOS
CHROME="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
# Linux: google-chrome / chromium ; Windows: the chrome.exe path
"$CHROME" --headless=new --screenshot=s.png --window-size=1920,1080 \
  "file://$PWD/talk.html?shot=3"
```

`?shot=N` renders only slide N full-size; `?mode=print` reveals all fragments.
Check: content fits each slide, layouts read well, fragments/transitions behave,
the theme looks coherent. (Don't put an `&` after `?shot=N`/`?mode=print` —
they're single-param URLs.)

If you edited the CLI itself, rebuild before regenerating — a stale binary
produces misleading output.

## Exporting

```bash
ondeck build talk.md --pdf      # → talk.pdf  (vector, crisp)
ondeck build talk.md --pptx     # → talk.pptx (image-per-slide; not editable)
ondeck build talk.md --open     # open the HTML when done
```
Both `--pdf` and `--pptx` need a Chromium-family browser. PPTX does one browser
launch per slide, so it's slow on large decks.
