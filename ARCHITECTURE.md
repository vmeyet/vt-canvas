# Architecture

`vt-canvas` is one library crate.
A terminal cell grid: text, lines and boxes with junction merging, glyph sets; returns styled cells, never ANSI.
This page says how the code is laid out today; when a module or a boundary changes, change this page in the same change.

## Owns

- The cell grid: each cell holds one glyph, its width and a colour role.
- Drawing on it: text, lines and boxes whose crossings merge into the right junction glyph, sub-cell pixels, layers.
- Glyph sets: which characters a drawing uses, avoiding East Asian ambiguous-width glyphs by default.

## Never

- Reads files, the environment or the clock; prints; exits the process.
- Writes ANSI escapes or picks colours: it hands back cells with colour roles, the caller maps them and writes the terminal.
- Lets a dependency's type into a public signature.
- Knows a chart or a diagram: those live in vt-graph and vt-mermaid.

## Dependency direction

```
vt-canvas ← vt-graph ← vt-mermaid
```

vt-canvas is the bottom: it depends on `unicode-width` only, added by the first code that measures text.
