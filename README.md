# vt-canvas

A terminal cell grid: text, lines and boxes with junction merging, glyph sets; returns styled cells, never ANSI.
It is a pure library: it returns styled cells with colour roles and never writes to the terminal, so the caller (mrk first) applies the theme, sanitizes and prints.
See [`ARCHITECTURE.md`](ARCHITECTURE.md) for what it owns and [`AGENTS.md`](AGENTS.md) for how to work on it.
