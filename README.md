# vt-canvas

A terminal cell grid: text, lines and boxes with junction merging, drawings finer than a cell, glyph sets; returns styled cells, never ANSI.

It is a pure library.
Each cell carries a *role* of the caller's own type, so the caller maps roles to colours, sanitizes and writes the terminal; vt-canvas never prints, reads the environment or knows what a diagram is.

```sh
cargo add vt-canvas
```

The minimum supported Rust version is 1.92.

## Example

```rust
use vt_canvas::{Canvas, Corner, Direction, GlyphSet, LineKind, Point, Rect, Role, Tip};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Part {
    #[default]
    Plain,
    Node,
    Edge,
    Label,
}

impl Role for Part {}

let mut canvas = Canvas::new(14, 3)?;
canvas.frame(Rect::new(0, 0, 5, 3), LineKind::Light, Corner::Rounded, Part::Node);
canvas.text(Point::new(2, 1), "A", Part::Label);
canvas.horizontal(1, 4, 7, LineKind::Light, Part::Edge);
canvas.tip(Point::new(8, 1), Direction::Right, Tip::Arrow, Part::Edge);
canvas.frame(Rect::new(9, 0, 5, 3), LineKind::Light, Corner::Rounded, Part::Node);
canvas.text(Point::new(11, 1), "B", Part::Label);

let lines: Vec<String> = canvas.lines(GlyphSet::Unicode).collect();
assert_eq!(lines, ["╭───╮    ╭───╮", "│ A ├───▸│ B │", "╰───╯    ╰───╯"]);
# Ok::<(), vt_canvas::Error>(())
```

`Canvas::rows` gives the same rows as runs of text sharing a role and a background, for the caller to colour.

## What it draws

- Text, cut at a width with an ellipsis, wide characters taking two cells.
- Horizontal and vertical lines and paths, light, heavy, double or dashed, whose crossings merge into the right junction (`┼ ╋ ╪ ┤`).
- Frames and 14 node shapes (rectangle, stadium, diamond, hexagon, cylinder…), tips at an edge's end (arrows, triangles, diamonds, a crow's foot's parts) and one-cell symbols.
- Finer than a cell: bars by eighths of a cell, lines one eighth thick, wedges under a slanted edge, and dots on braille, octant, sextant, quadrant or half-block grids, with two roles per cell where the terminal allows it.
- Every drawing in four glyph sets, from `Unicode` (octants of Unicode 16) down to `Ascii`, each glyph falling back to the closest one the set holds; no glyph of the Unicode sets is East Asian ambiguous outside box drawing and block elements.

## Limits

- A canvas holds at most 1000 columns and 1000000 cells; drawing past its edges is clipped, never an error.
- A cell holds one character: combining marks and other zero-width characters are left out, so a decomposed `é` (`e` then U+0301) draws as `e`.
  Normalize text to NFC first; scripts that need shaping or joining across cells are not drawn right.
- Emoji modifier and ZWJ sequences are drawn one emoji per character, the joiner left out: `👍🏽` takes 4 columns and `👨‍👩‍👧` 6, where a terminal that joins them draws 2, so the rest of the row shifts left.
  Leave modifiers and joiners out of text drawn on a canvas.

## Learn more

- [`ARCHITECTURE.md`](https://github.com/vmeyet/vt-canvas/blob/main/ARCHITECTURE.md): what the crate owns, its contract, the junction and layer rules.
- [`AGENTS.md`](https://github.com/vmeyet/vt-canvas/blob/main/AGENTS.md): how to work on it.
- `cargo run --example demo`, `--example shapes` and `--example pixels` draw a small flowchart, every shape, and the drawings finer than a cell in every glyph set.

## License

MIT
