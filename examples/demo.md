---
theme: midnight
title: Q3 Review
slide-transition: slide
slide-numbers: true
progress: true
footer: Acme · Q3 2026
---

---
layout: title
---
# Q3 Review
## How we did, and what's next

A quick walk through the quarter — *June 2026*

---
layout: bullets
---
# What changed this quarter

- Shipped the new onboarding flow
- Killed three legacy services
- Cut p95 latency by **40%**
- Moved billing to the new platform
  - migrated 12k accounts
  - zero downtime

---
layout: stat
---
# By the numbers

:::stat
142% · of revenue target
:::
:::stat
+18 · NPS points
:::
:::stat
40% · faster p95
:::

---
layout: statement
background: "var(--bg-2)"
slide-transition: fade
---
# The bet for Q4: make it *boringly reliable*.

::: notes
Land the reliability message here. This is the emotional pivot of the deck.
:::

---
layout: free
background: "linear-gradient(135deg, var(--bg-2), var(--bg))"
---
:::block at="x2 y2 x14 y8"
# Top-left
Placed by coordinates on the 30×20 grid.
:::
:::block at="x16 y4 x29 y10"
## Right block
A second region — `x16 y4 x29 y10`.
:::
:::block at="x6 y13 x25 y19"
### Centered-ish footer block
This is the escape hatch: explicit placement when a slide needs it.
:::

---
layout: bullets
reveal: true
transition: fade-up
---
# Revealed one at a time

- First, the problem
- Then the constraint
- Then the insight
- Finally, the fix

---
layout: bullets
---
# Mixed transitions

- Always here
- Fades in {+}
- Rises up {+ fade-up}
- Zooms in {+ zoom}
- Blurs into focus {+ blur}

---
layout: compare
---
# Before vs after
:::left
### Before
- 3 services to deploy
- p95 ~ 800ms
- manual billing
:::
:::right
### After
- 1 service
- p95 ~ 480ms
- automated billing
:::

---
layout: code
---
# The whole deploy

```bash
deck build deck.md --pdf out.pdf
rsync -a out.pdf release@host:/srv/decks/
echo "shipped $(git rev-parse --short HEAD)"
```

---
layout: stat-3
---
# Reliability, in three numbers

:::stat
99.98% · uptime
:::
:::stat
0 · paging incidents
:::
:::stat
12m · mean time to deploy
:::

---
layout: image
fit: full
---
![Quarterly growth](chart.svg)

:::caption
Revenue, last five quarters.
:::

---
layout: quote
---
The best way to predict the future is to invent it.
:::cite
Alan Kay
:::

---
layout: media-split
---
# Built for the field

Crews see the next job, the route, and the parts list — before they leave the depot.

:::media
![Depot](chart.svg)
:::

---
layout: media-split
media: right
---
# And on the right

Same layout, `media: right` — the image mirrors to the other side.

:::media
![Depot](chart.svg)
:::

---
layout: raw
---
<div style="display:flex;height:100%;align-items:center;justify-content:center;gap:2rem;font-family:sans-serif">
  <div style="font-size:8vmin;font-weight:800;color:var(--accent)">100%</div>
  <div style="font-size:3vmin;max-width:14em;color:var(--fg)">
    A <code>raw</code> slide: when you need full control, you write the HTML directly.
  </div>
</div>
