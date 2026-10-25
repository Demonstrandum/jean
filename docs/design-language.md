# Design language

Every screen is **a grid of modules**. There are no free-floating cards.

## Anatomy of a module (`tile`)
- A mono, all-caps **label** top-left (what it is).
- One **visualisation** in the middle (a number, a dial, a chart, an icon, a list).
- A **count or status** bottom-left (the one fact to take away).
- A cell on the 6x3 `tilegrid`, placed with `place(tile(...), "row / col / rowEnd / colEnd")`.

## Tones (vary them; a screen should not be all one)
outlined (default) | **inverted** (white on black) | **sky** (dithered gradient) | **tank** (`levelTile`)
| **fill** (the visual takes the whole module) | **list** (compact rows with status dots).

## Visualisations (prefer these to plain bars and gauges)
dot-matrix clock, 24h `dayDial` / `dayStrip`, `timeline` lanes, stepped hatched `areaChart` with a
solid window, `waffle` cells, LED `dotBars`, hatched `stackBar`, `arcGauge`, `ruler` with marker and target,
`levelTile`, halftone `sky`.

## Type and line
Nunito only: light (200-300) for big numerals, 600-700 for text. Small labels in the system mono, 14px,
semibold, pure black (1-bit needs it). Lines are 2px outlines and 1px dividers. Radii 12-16px.
Contrast comes from scale (a 130px numeral next to a 14px label), inversion, hatching and halftone,
not from gray or colour. A colour accent (`ui.accent`) applies only on colour panels.

## Rules the screen enforces
Fit is automatic (scale, then drop parts marked optional, then clip). `ui.fits()` reports `crowded`
modules whose content is bigger than their cell. Pass `w:` to SVG charts so their text renders 1:1.
