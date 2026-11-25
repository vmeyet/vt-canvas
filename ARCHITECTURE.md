# Architecture

`vt-canvas` is one library crate.
A terminal cell grid: text, lines and boxes with junction merging, glyph sets; returns styled cells, never ANSI.
This page says how the code is laid out today; when a module or a boundary changes, change this page in the same change.

## Owns

- The cell grid: each cell holds one glyph, its width and a colour role.
- Drawing on it: text, lines and boxes whose crossings merge into the right junction glyph, and node shapes.
- Glyph sets: which characters a drawing uses, avoiding East Asian ambiguous-width glyphs where Unicode has a choice.
- Measuring text: `width` says how many columns `Canvas::text` gives a string, so a caller sizes a box with the rule the canvas draws with.

Sub-cell pixels and layers come with the charts.

## Never

- Reads files, the environment or the clock; prints; exits the process.
- Writes ANSI escapes or picks colours: it hands back cells with colour roles, the caller maps them and writes the terminal.
- Lets a dependency's type into a public signature.
- Knows a chart or a diagram: those live in vt-graph and vt-mermaid.

## Modules

| Module | Holds |
|---|---|
| `canvas` | `Canvas`, its cells, `Frame`, `Role`, `Run`, `Error` and the size limits |
| `line` | `LineKind` and the four sides a line leaves a cell through |
| `glyph` | `GlyphSet`: the character for a cell's sides, an arrow or a shape's mark, junction tables included |
| `shape` | `Shape`, `Inset`: which border cells of a node's outline are slants or brackets instead of lines; `Marker`, a node on one cell |
| `tip` | `Tip`: what an edge draws where it meets a node, a class relation's head or a crow's foot's part |
| `text` | Which characters a cell can hold and how wide they are |
| `geometry` | `Point`, `Rect`, `Direction` |

## Contract

- Size: `Canvas::new` refuses more than 1000 columns (`MAX_WIDTH`) or 1000000 cells (`MAX_CELLS`, width times height) with `Error::TooLarge`, so a narrow drawing can be tall.
- Clipping: drawing out of bounds is cut at the edges, never an error, never a panic.
- Lines are horizontal or vertical; a diagonal is `Error::Diagonal`.
  A line with both ends on one cell draws nothing.
  A path is a line from each point to the next, turning a corner at each point between; a diagonal step refuses the whole path before anything is drawn.
- Overwriting: the last drawing on a cell wins, except that a line drawn on a line merges with it, and a line drawn on a shape's mark leaves the mark.
  Writing over either half of a wide character blanks the other half; a wide character without room for both halves is not drawn.
- Text: control and zero-width characters are left out, so every row is exactly as wide as the canvas.
  `text_cut` cuts text to a width and ends it on `⋯` (`~` in ASCII) when cut; `…` would be East Asian ambiguous.
- Roles: `Plain`, `Node`, `Edge`, `Label`, `Group`; the caller maps each one to a colour.
- Output: `rows` yields each row as runs of text sharing a role, without `\n`.
- Cost at the cell limit, measured on an Apple M5 with `cargo bench --bench canvas`: a lattice of lines on every other row and column draws in 5 ms and reads back in 15 ms, in about 9 MB.

## Shapes

`Canvas::shape` draws one of Mermaid's flowchart shapes on the border of a rectangle: a light frame, some of its cells replaced by *marks*, slants `╱ ╲` and brackets `( ) < >`.

- Every border cell keeps a glyph, so a line ending on any side of the rectangle touches the shape.
  A line ending on a frame cell joins it (`┤`); one ending on a mark leaves it whole (`──<`).
- A shape is told apart by its corners and its sides: square or rounded corners, `╱ ╲` corners for a diamond, `< >` sides for a hexagon, `( )` for a stadium.
- A side that points out (hexagon, asymmetric) points at its middle row, or between its two middle rows when the count is even.
- A circle is a stadium with a blank column inside each bracket: three rows cannot curve more.
- `Shape::inset` gives the columns and rows the outline takes, so the caller sizes the rectangle around its content.

## Junctions

Each cell keeps, for each of its four sides, whether a line leaves through it and in which kind; the glyph is chosen when the rows are read.
The rules, applied in order:

1. A side drawn twice takes the kind drawn last.
2. Dashed stays dashed only on a straight run of dashed sides (`╌ ╎`); Unicode has no dashed junction, so anywhere else dashed counts as light.
3. A side alone continues through its cell: a line end fills the cell (`─`, never the half `╴`).
4. A rounded corner is an arc (`╭`) only when its two sides are light; a heavy or double corner stays square, and a corner a line joins becomes a junction.
5. Light and heavy mix freely: Unicode has every combination (`┽ ┿ ╀ ╆`).
6. Double mixes with light only axis by axis (`╪ ╫ ╞`): an axis with a double side is all double, and heavy elsewhere becomes light, as Unicode has no heavy and double glyph.

Two lines crossing draw `┼`, never a "pass over" where the line drawn first stays whole.
Tried on the flowchart spike's output: frames are drawn before edges, so a pass over cut every edge where it left a group, which read as the edge ending at the frame; between two edges it broke one of them, which made it harder to follow.
Flowchart edges only meet at nodes, so a `┼` between nodes is never read as a join.

The ASCII set draws any junction as `+`, a horizontal run as `-`, `=` when heavy or double, `.` when dashed, and a vertical run as `|`, `:` when dashed.
A rounded corner is `.` at the top and `'` at the bottom, so a rounded box still differs from a square one.
Slants are `/ \`; brackets and points are the same in both sets.
Arrows are `▴ ▸ ▾ ◂` in Unicode, `^ > v <` in ASCII.
`Canvas::cross` draws `╳`, `x` in ASCII, and keeps it under a line as a shape's mark is kept.
`Canvas::marker` draws a node too small for an outline on one cell, kept under a line the same way: `Marker::Dot` `⬤`, `Marker::RingedDot` `◉` and `Marker::Diamond` `⬥`, `*`, `@` and `?` in ASCII; `●` and `◇` would be East Asian ambiguous.
`Canvas::tip` draws an edge's end on one cell, facing the node it goes into, kept under a line the same way: `Tip::Hollow` `▹ ◃ ▵ ▿`, `Tip::Diamond` `⬦`, `Tip::FilledDiamond` `⬥`, `Tip::Ring` `o` and `Tip::Fork` `< > Ʌ V`, opening onto the node; in ASCII `> < ^ v`, `o`, `*`, `o` and `< > ^ v`.
`Tip::Bar` is a line across the edge instead, merged with it into `┼` (`+`).
`△ ▷ ◇ ◆ ○` would be East Asian ambiguous.

## Glyph width

Every glyph of the Unicode set is one column wide, tested.
Outside the box drawing block none is East Asian ambiguous: arrows are `▴ ▸ ▾ ◂`, not `▲ ▶ ▼ ◀`, and slants are `╱ ╲` from the block, not `◢ ◣`.
Box drawing itself (U+2500 to U+257F) is ambiguous in Unicode; it has no replacement and terminals draw it one column wide, so the test exempts that block.

## Dependency direction

```
vt-canvas ← vt-graph ← vt-mermaid
```

vt-canvas is the bottom: it depends on `unicode-width` only, called directly from `text`.
mrk measures text with its own helper over the same crate; vt-canvas exposes `width` so its callers measure the way it draws.

## Fuzzing and properties

- `fuzz/` is a cargo-fuzz crate, its own workspace, out of the library's build and of `scripts/check`: `cargo +nightly fuzz run draw` draws arbitrary text, lines, paths, frames, arrows and shapes, then checks that every row is exactly the canvas's width.
- The proptest properties in `canvas` keep the same check in `cargo test`, plus a line drawn from either end, a line drawn twice, and a path drawn as its lines.
