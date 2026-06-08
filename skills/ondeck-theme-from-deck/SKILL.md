---
name: ondeck-theme-from-deck
description: >-
  Derive an `ondeck` slide theme that matches an EXISTING presentation or brand —
  a PowerPoint/Keynote/Google Slides deck, a PDF, a brand/template, or even
  screenshots of slides. Use this whenever the user wants ondeck decks to look
  like their company template or an existing deck, to "match this PowerPoint /
  our brand / these slides", or to reproduce a reference look as an ondeck theme.
  Extracts the palette and fonts from the reference, then builds the theme.
---

# Deriving an ondeck theme from an existing deck

Goal: extract the **design language** (palette, fonts, overall feel) from a
reference and turn it into an ondeck theme. This is an *extraction front-end* to
normal theme creation — once you have tokens, follow the **ondeck-theme** skill
for the mechanics, and `THEMING.md` (top level of the repo) for the complete
token/selector reference. This skill is about getting accurate tokens out of the
reference.

## Route by input type

How you extract depends on what the user gives you. Match the method:

### `.pptx` / `.potx` (PowerPoint, or Keynote/Google Slides exported to pptx) — precise
The OOXML embeds the exact theme. Unzip and read `ppt/theme/theme1.xml`:

```bash
unzip -p deck.pptx ppt/theme/theme1.xml > /tmp/theme1.xml
```

In `<a:clrScheme>` you'll find the brand colours; in `<a:fontScheme>` the fonts.
Map them to ondeck tokens:

| OOXML | ondeck token | Notes |
|---|---|---|
| `clrScheme` `lt1` (or `lt2`) | `bg` | usually `window`/`FFFFFF` for light decks |
| `clrScheme` `dk1` (or `tx1`/`dk2`) | `fg` | usually `windowText`/`000000` |
| `clrScheme` `accent1` | `accent` | the primary brand colour |
| `clrScheme` `accent2` | `accent-2` | |
| `dk1`/`lt1` lightened/darkened | `bg-2`, `muted`, `rule` | derive subtle surfaces from `bg`/`fg` |
| `fontScheme` `minorFont` latin | `font` | body font |
| `fontScheme` `majorFont` latin | (headings) | if it differs, note it; ondeck has one `font` token |

`sysClr` elements carry a `lastClr` attribute with the concrete hex. The slide
*background* may instead be set on the slide master (`ppt/slideMasters/…` `<p:bg>`)
— check there if `lt1`/`lt2` don't match what you see. The `anthropic-skills:pptx`
skill can help read the package if needed.

### PDF / images / screenshots — by eye
View the slides (the Read tool renders PDF pages and images) and read off the
design:
- **Background** and **text** colours (sample the dominant bg and the body text).
- The **accent** colour (buttons, highlights, bars, links, header rules).
- **Type:** serif vs sans, weight, any obvious typeface; pick the closest system
  or web-safe font, or a known lookalike, if you can't identify it exactly.
- Overall **feel** (minimal/dense, light/dark, flat/gradient) to guide `bg-2`,
  `frame`, and whether to add small `theme.css` touches.
Be honest that this is an approximation — confirm with the user once rendered.

### Google Slides / Keynote (native)
Ask the user to export to **`.pptx`** (precise route) or **PDF** (by-eye route),
then proceed as above.

## Fonts

- If the brand font file is available (`.woff2`/`.ttf`/`.otf`), **self-host it**:
  put it in the theme dir and `@font-face` it in `theme.css` — ondeck inlines it
  (see ondeck-theme). This reproduces the brand exactly and stays self-contained.
- If you only have a font *name* and it's a common/system font, reference it.
- From a screenshot alone you can only approximate — pick a close lookalike and
  flag it.

## Then: build and match

1. Create `themes/<name>/` with the extracted tokens (and a self-hosted font if
   you have one) per the **ondeck-theme** skill.
2. Build the demo deck with it: `ondeck build examples/demo.md --theme <name> -o /tmp/t.html`.
3. **Compare side by side with the reference.** Screenshot the demo's title,
   bullets, stat, and a content slide, and put them next to the source slides.
   Adjust tokens (especially `accent`, `bg`, `bg-2`, and the font) until the feel
   matches.
4. Show the user the comparison and refine — colour-matching by eye usually takes
   a round or two.

## Notes

- Aim for *faithful, not pixel-identical*: ondeck has its own layouts, so the
  result should feel like the same brand, not be a tracing of specific slides.
- Brand palettes sometimes only define one accent; it's fine to derive `accent-2`
  as a tint/shade of `accent` or a second brand colour.
- Capture what you extracted (hexes, font names, source) in a comment at the top
  of `theme.toml` so the theme is reproducible.
