# vt-canvas

A grid of terminal cells to draw text, lines and boxes on, read back as styled text, never as ANSI escapes.

[![crates.io](https://img.shields.io/crates/v/vt-canvas.svg)](https://crates.io/crates/vt-canvas)
[![docs.rs](https://img.shields.io/docsrs/vt-canvas)](https://docs.rs/vt-canvas)

```text
      ┌─────────┐
      │ Request │◂╌╌╌╌┐
      └────┬────┘     ╎
           │          ╎
╭─ cache ──┼──────────┼─────────────────╮
│          ▾          ╎                 │
│     ┌─────────┐ no  ╎    ┌─────────┐  │
│     │ Cached? ├─────┼───▸│  Fetch  │  │
│     └─────────┘     ╎    └────┬────┘  │
│                     ╎         ╎       │
│                     ╎         ╎       │
│                     └╌ retry ╌┘       │
│                                       │
╰───────────────────────────────────────╯
```

This is the output of `cargo run --example demo`, read in the `Unicode` glyph set.
The same drawing reads in plain ASCII too, with `+ - | . :`.

## Install

```sh
cargo add vt-canvas
```

## Example

Draw two boxes and an arrow, then read the rows back.

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

The line that starts on box `A` merges into its border as `├`.

To add colour, read the rows with `Canvas::rows` instead.
Each row comes as runs of text that share a role and a background role.
You map each role to a colour of your palette.

```rust
# use vt_canvas::{Canvas, GlyphSet, Point};
# let mut canvas = Canvas::<u8>::new(4, 1)?;
# canvas.text(Point::new(0, 0), "ab", 1);
for runs in canvas.rows(GlyphSet::Unicode) {
    for run in runs {
        let colour = match run.role {
            1 => "green",
            _ => "default",
        };
        // Write run.text in `colour`, through your own sanitizer.
    }
}
# Ok::<(), vt_canvas::Error>(())
```

## Features

| Feature | What you get |
|---|---|
| Text | Cut at a width with an ellipsis (`⋯`); wide characters take two cells; `text_width` measures text the way the canvas draws it. |
| Lines and boxes | Horizontal and vertical lines, paths and frames in four kinds: light, heavy, double, dashed. |
| Junction merging | Where lines cross or meet, the cell gets the right junction: `┼ ╋ ╪ ┤`. |
| Shapes | 14 node shapes from Mermaid's flowcharts: rectangle, stadium, diamond, hexagon, cylinder… |
| Tips and symbols | Arrows, triangles, diamonds and crow's foot parts at an edge's end; nodes on one cell. |
| Finer than a cell | Bars by eighths of a cell, lines one eighth thick, wedges under a slanted edge, and dots on braille, octant, sextant, quadrant or half-block grids. |
| Glyph sets | `Unicode`, `Unicode13`, `Unicode3`, `Ascii`: each glyph falls back to the closest one the set holds. |
| Roles | Each cell keeps a role of your own type, plus a background role where a cell shows two colours. |

Run the examples to see more:

- `cargo run --example demo`: a small flowchart in every glyph set.
- `cargo run --example shapes`: every shape.
- `cargo run --example pixels`: the drawings finer than a cell, in every glyph set.

## Limits and errors

- A canvas holds at most 1000 columns (`MAX_WIDTH`) and 1000000 cells (`MAX_CELLS`).
  Past that, `Canvas::new` returns `Error::TooLarge`.
- A path with a diagonal step returns `Error::Diagonal`, and none of it is drawn.
- Drawing past the edges is clipped: it is never an error or a panic.
- A cell holds one character.
  Combining marks and other zero-width characters are left out, so a decomposed `é` (`e` then U+0301) draws as `e`.
  Normalize text to NFC first.
  Scripts that need shaping or joining across cells are not drawn right.
- Emoji modifier and ZWJ sequences draw one emoji per character, without the joiner.
  `👍🏽` takes 4 columns and `👨‍👩‍👧` takes 6, where a terminal that joins them draws 2, so the rest of the row shifts left.
  Leave modifiers and joiners out of text you draw on a canvas.
- No glyph of the Unicode sets is East Asian ambiguous, outside box drawing and block elements.

## Safety

The library is pure: it does no I/O, never prints, never reads the environment and never exits the process.
It never writes ANSI escapes.
Your code maps roles to colours, sanitizes text that comes from untrusted input, and writes the terminal.

## Used by

- [vt-mermaid](https://github.com/vmeyet/vt-mermaid): Mermaid diagrams drawn as text on a vt-canvas grid.

## Learn more

- [`ARCHITECTURE.md`](https://github.com/vmeyet/vt-canvas/blob/main/ARCHITECTURE.md): the modules, the contract, and the rules for junctions and layers.
- [`AGENTS.md`](https://github.com/vmeyet/vt-canvas/blob/main/AGENTS.md): how to work on the code.

## Minimum Rust version

Rust 1.92, as set by `rust-version` in `Cargo.toml`.

## License

[MIT](https://github.com/vmeyet/vt-canvas/blob/main/LICENSE)
