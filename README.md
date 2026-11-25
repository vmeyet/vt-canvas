# vt-canvas

A terminal cell grid: text, lines and boxes with junction merging, glyph sets; returns styled cells, never ANSI.
It is a pure library: it returns styled cells with colour roles and never writes to the terminal, so the caller (mrk first) applies the theme, sanitizes and prints.
See [`ARCHITECTURE.md`](ARCHITECTURE.md) for what it owns and [`AGENTS.md`](AGENTS.md) for how to work on it.

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

`cargo run --example demo` draws a small flowchart with both glyph sets.
`cargo run --example shapes` draws every node shape with both glyph sets.
`cargo run --example pixels` draws blocks, a stroke, wedges and each kind of dots with every glyph set.
