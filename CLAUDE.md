# CLAUDE.md — working notes for ondeck

`ondeck` is a Rust CLI that turns structured Markdown into self-contained HTML
slide decks, with PDF and PPTX export. Architecture & format live in
[SPEC.md](SPEC.md); user docs in [README.md](README.md); the complete theming
vocabulary is in [THEMING.md](THEMING.md). Keep these up to date when behaviour
changes — in particular, **`THEMING.md` must track `themes/base/` + `themes/default/`**
(`base/base.css` machinery + agnostic look, `base/base.toml` neutral token
contract, `default/theme.css` per-layout look + `default/theme.toml` palette +
core layout vocabulary): if you add/rename a token, layout/template class,
`block`/`block-*` hook, `syn-*` token, a core layout's blocks, or a chrome/fragment
hook, update THEMING.md (and the theming skills reference it, so don't duplicate it).

## Before committing (always)

Run all three and make sure they're green:

```bash
cargo fmt --all -- --check          # formatting
cargo clippy --all-targets -- -D warnings   # lint (zero warnings)
cargo test                          # the test suite
```

Prefer real clippy fixes over `#[allow]`. Only commit/push when the user asks.
End commit messages with the standard `Co-Authored-By` line.

## Build / test / run

```bash
cargo build                         # debug
cargo build --release               # release binary at target/release/ondeck
cargo test
cargo run -- build examples/demo.md -o examples/index.html   # build the demo
cargo run -- watch examples/demo.md                          # live-reload server
```

`examples/demo.md` exercises every layout, fragments, transitions, and chrome —
use it as the smoke test / reference deck.

## Gotchas (learned the hard way)

- **`cargo test` does NOT rebuild `target/debug/ondeck`.** After editing source,
  run `cargo build` *before* regenerating the demo or screenshotting — otherwise
  you verify against a stale binary. This has caused misleading results multiple
  times.
- **Verify rendering/visual changes via computed styles, not just screenshots.**
  Class/DOM assertions and screenshots can both pass while the CSS is wrong (the
  fragment-transition bug had correct classes but a losing cascade). Use the
  preview's `getComputedStyle` (e.g. check `transform`, `color`, `background`).
  Note screenshots are letterboxed/scaled, so don't read pixel positions off them.
- **Don't blind-replace "deck".** The tool/command is `ondeck`; but "deck" /
  "slide deck" is the generic term and the internal CSS classes are `.deck`,
  `.deck-number`, etc. Only rename actual tool/command references.
- **PDF/PPTX need a Chromium-family browser** (`DECK_CHROME` overrides). The test
  suite does not launch a browser; don't add tests that require one.

## Design principles

- **The tool assembles; the browser renders.** We emit HTML + CSS; layout happens
  in the browser. PDF/PPTX drive headless Chrome over that same HTML — never
  reimplement layout for an export target.
- **Themes layer on a base substrate.** `themes/base/` is the engine substrate
  emitted beneath every deck: `base.css` (machinery + a layout-agnostic,
  token-driven look — typography, code, tables) + `base.toml` (a neutral token
  contract + grid). base ships **no layouts**. A theme layers on base and may
  `extends = "<other>"` to inherit that theme's tokens/layouts/templates/CSS;
  with no `extends` it builds straight on base (owns its whole vocabulary). The
  bundled `default` theme owns the core layout set; `bold`/`paper` extend it.
  Layouts and templates are data (named **blocks** = grid rects + hints); themes
  override via tokens + CSS. A block is *fixed* (theme `image`/`text`) or
  *editable* (author-filled). Add a layout/template as data + CSS, not a new code
  path where avoidable.
- **Keep dependencies light.** Don't add crates without good reason.
- **Output stays self-contained** (images/fonts inlined). Don't introduce remote
  runtime dependencies in generated decks.

## Where things are

`src/parser.rs` (Markdown→slides) · `render.rs` (slides→HTML, block rendering) ·
`theme.rs` (themes/templates/blocks + inheritance) · `grid.rs` (rects, `at=`,
repeat math) · `fragments.rs` ·
`assets.rs` (data-URI inlining) · `pdf.rs` · `pptx.rs` · `watch.rs` (live-reload
`watch` + two-window `present`) · `assets/runtime.js` (deck runtime: nav,
fragments, transitions, opt-in scale-to-fit, presenter view — engine plumbing,
not themeable) · `themes/` (bundled themes). The substrate is `themes/base/`
(`base.css` machinery + agnostic look, `base.toml` token contract + grid) and the
bundled `themes/default/` (`theme.css` per-layout look, `theme.toml` palette +
core layouts); both compiled in via `include_str!`. Themes layer on base and opt
into a parent via `extends`.
