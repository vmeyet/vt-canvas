# Architecture

`vt-canvas` is one library crate.
A terminal cell grid: text, lines and boxes with junction merging, glyph sets; returns styled cells, never ANSI.
This page says how the code is laid out today; when a module or a boundary changes, change this page in the same change.

## Owns

- The cell grid: each cell holds one glyph, its width, a colour role and the role behind it.
- Drawing on it: text, lines and boxes whose crossings merge into the right junction glyph, and node shapes.
- Drawing finer than a cell: blocks filling a cell by eighths, lines one eighth thick, wedges under or over a slanted edge, and dots on a grid of pixels that cuts each cell in 2 to 8.
- Glyph sets: which characters a drawing uses, avoiding East Asian ambiguous-width glyphs where Unicode has a choice.
- Measuring text: `text_width` says how many columns `Canvas::text` gives a string, so a caller sizes a box with the rule the canvas draws with.

## Never

- Reads files, the environment or the clock; prints; exits the process.
- Writes ANSI escapes or picks colours: it hands back cells with colour roles, the caller maps them and writes the terminal.
- Lets a dependency's type into a public signature.
- Knows a chart or a diagram: those live in vt-graph and vt-mermaid.

## Modules

| Module | Holds |
|---|---|
| `canvas` | `Canvas`, its cells and their backgrounds, `Run`, `Error` and the size limits |
| `frame` | `Corner`: the corners of a frame |
| `role` | `Role`: what the canvas asks of the caller's role type, its ASCII texture |
| `line` | `LineKind` and the four sides a line leaves a cell through |
| `glyph` | `GlyphSet`: the character for a cell's sides or a filled cell, junction tables included |
| `block` | A cell filled by eighths from one side and a line one eighth thick, their glyphs in each set, the cells a stretch covers, and how much of a cell a glyph covers |
| `pixel` | `Dots`, `Pixel`: the dots of a cell and their glyph in each set, the two halves of a half-block cell, and the dots of a straight line |
| `wedge` | `Wedge`, `Fill`: a cell filled under or over a slanted edge, its glyph in each set |
| `shape` | `Shape`, `Inset`: each shape's outline in one place, which border cells are slants or brackets instead of lines, and their glyphs |
| `symbol` | `Symbol`: a node on one cell, and its glyph |
| `tip` | `Tip`: what an edge draws where it meets a node, an arrow, a cross, a class relation's head or a crow's foot's part, and its glyph |
| `text` | Which characters a cell can hold and how wide they are |
| `geometry` | `Point`, `Rect`, `Direction` |

## Contract

- Size: `Canvas::new` refuses more than 1000 columns (`MAX_WIDTH`) or 1000000 cells (`MAX_CELLS`, width times height) with `Error::TooLarge`, so a narrow drawing can be tall.
- Clipping: drawing out of bounds is cut at the edges, never an error, never a panic.
- Lines are horizontal or vertical: `horizontal` and `vertical` cannot be given a diagonal.
  A line with both ends on one cell draws nothing.
  A path is a line from each point to the next, turning a corner at each point between; a diagonal step is `Error::Diagonal` and refuses the whole path before anything is drawn.
- Overwriting: the last drawing on a cell wins, except that a line drawn on a line merges with it, and a line drawn on a shape's mark leaves the mark.
  Writing over either half of a wide character blanks the other half; a wide character without room for both halves is not drawn.
- Text: control and zero-width characters are left out, so every row is exactly as wide as the canvas.
  `text_cut` cuts text to a width and ends it on `⋯` (`~` in ASCII) when cut; `…` would be East Asian ambiguous.
- Roles: the caller's own type, `Canvas<R: Role>`; the canvas keeps one per cell and hands it back, the caller maps each one to a colour.
  `Role` asks for `Copy + Eq + Default`, the default being the role of empty cells, and an ASCII `texture`, 0 unless told otherwise, so roles differ without colour; a `u8` is a role whose texture is itself.
- Output: `rows` yields each row as runs of text sharing a role and a background, without `\n`.
  A run's `background` is the role behind its characters, `None` where nothing is.
- Cost at the cell limit, measured on an Apple M5 with `cargo bench --bench canvas`: a lattice of lines on every other row and column draws in 4 ms and reads back in 8 ms; a bar in every column under a line of octant dots across every row draws in 17 ms and reads back in 5 ms.
  With a role of 2 bytes, a cell takes 8 bytes and its background 2, so the canvas holds 10 MB.

## Finer than a cell

`column` and `row` fill a stretch of one column or one row, its two ends counted in eighths of a cell from the top or the left of the canvas.
Each cell the stretch covers becomes a block filled from one of its sides: `▁▂▃▄▅▆▇█` from the bottom, `▏▎▍▌▋▊▉█` from the left, `▔🮂🮃▀🮄🮅🮆█` from the top, `▕🮇🮈▐🮉🮊🮋█` from the right.
A stretch that touches neither side of its cell has no glyph: it keeps its size and moves to the side it is nearer.

`stroke` draws a line one eighth of a row thick across one column, at any eighth of a row: `▔🭶🭷🭸🭹🭺🭻▁`.
It is how a line passes in front of a bar: the bar stays behind it as the cell's background.

`wedge` fills the part of a cell under or over an edge that runs from its left side to its right side, each end at 0 to 3 thirds of the cell's height: `🭈 🭊 🬽 🭆 🭄 🬿 🭑 🭂 🭏 🭍` below, `🭞 🭠 🭓 🭜 🭚 🭕 🭧 🭘 🭥 🭣` above, and the sextant rows `🬭 🬹 🬎 🬂` where the edge is level.
Legacy Computing has no edge from a bottom corner to the opposite top corner: `▌` or `▐`, the half on the filled side, stands in.

`dot` lights one dot of a grid of pixels over the cells and `segment` the dots of a straight line; `Dots` says how many dots a cell holds and which glyphs light them.

| `Dots` | In a cell | Glyphs | Drawn as, by the sets without them |
|---|---|---|---|
| `Braille` | 2 by 4 | `⠉ ⡠ ⣿` | `Quadrant` in ASCII |
| `Octant` | 2 by 4 | U+1CD00… (Unicode 16) | `Sextant` in `Unicode13`, `Quadrant` in `Unicode3` and ASCII |
| `Sextant` | 2 by 3 | U+1FB00… (Unicode 13) | `Quadrant` in `Unicode3` and ASCII |
| `Quadrant` | 2 by 2 | `▘ ▚ ▟` | `'` for the upper dots, `.` for the lower, `:` for both in ASCII; `" , ;` for an odd texture, so two lines differ without colour |
| `HalfBlock` | 1 by 2 | `▀ ▄ █` | `'`, `.` and the role's filled cell in ASCII |

A coarser grid lights the dot nearest each dot: of an octant's four rows the two in the middle share a sextant's middle row, and the upper two and lower two share a quadrant's rows; a sextant's middle row joins a quadrant's lower row, so a level line stays one dot thick.
A cell has one role for all its dots, the last drawn, so two roles crossing in a cell show as one there.
`HalfBlock` is the exception, and the reason to pick it: its upper dot is `▀` in its own role, drawn on the lower dot's role, two colours in the cell.
Dots of another kind replace the cell's.
A segment walks one dot per step of its longer way and only the steps inside the canvas, so its cost is bounded by the canvas, not by its ends.

## Layers

A terminal cell has one character, one colour for it and one colour behind it, so a cell shows two roles at most.
Each cell keeps a glyph with its role and, behind it, a *background* role.

1. A drawing replaces the cell's glyph; the last one wins, as for text and lines.
2. The glyph it replaces stays as the background when it is a block, a wedge or a half-block dot filling at least half of what the drawing leaves of the cell; a smaller one is dropped.
   Text, a line, a stroke or dots leave the whole cell, so a block stays behind them from half a cell up.
   A block or a wedge leaves what it does not fill: `▅` drawn over `▔` keeps it, as the upper quarter is most of the three eighths left, and the cell shows both roles.
3. Otherwise the background stays as it was, until a drawing that fills the whole cell clears it.

So text, a line, a stroke or dots over a bar are drawn on the bar's colour, a block drawn over a taller one shows both, and two bands meeting in a cell fill it between them.
Where a third role would be needed (a line through the cell a bar ends in), the bar gains or loses under half a cell.

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

## Glyph sets

`rows` takes the set a drawing is read in; the drawing is the same for every set, and each glyph falls back to the closest one the set holds.

| Set | Holds | Falls back |
|---|---|---|
| `Unicode` | Everything, octants (Unicode 16) included | |
| `Unicode13` | Symbols for Legacy Computing: sextants, the eighth blocks from the top and the right, strokes, wedges | Octants are sextants |
| `Unicode3` | Box drawing, block elements, braille | A block from the top or the right is the nearest of `▔ ▀ █`, `▕ ▐ █`; a stroke the nearest of `▔ ─ ▁`; a wedge the block filling as much from the bottom or the top; octants and sextants are quadrants |
| `Ascii` | ASCII | A block at least half a cell is `#`, a smaller one `_`, `"` or `|` on the side it starts from; a stroke the nearest of `" - _`; a wedge `/` or `\`, a block when its edge is level; dots are `' . :` |

In ASCII a filled cell is `# % * o @ x` for textures 0 to 5, then again from `#`, so roles differ without colour.
A cell holding a block under half on a background shows the background's character, the one filling more of it.

The ASCII set draws any junction as `+`, a horizontal run as `-`, `=` when heavy or double, `.` when dashed, and a vertical run as `|`, `:` when dashed.
A rounded corner is `.` at the top and `'` at the bottom, so a rounded box still differs from a square one.
Slants are `/ \`; brackets and points are the same in both sets.
`Canvas::symbol` draws a node too small for an outline on one cell, kept under a line the same way: `Symbol::Dot` `⬤`, `Symbol::RingedDot` `◉` and `Symbol::FilledDiamond` `⬥`, `*`, `@` and `?` in ASCII; `●` and `◆` would be East Asian ambiguous.
`Canvas::tip` draws an edge's end on one cell, facing the node it goes into, kept under a line the same way, so an edge drawn later does not cut it: `Tip::Arrow` `▸ ◂ ▴ ▾`, `Tip::Triangle` `▹ ◃ ▵ ▿`, `Tip::Diamond` `⬦`, `Tip::FilledDiamond` `⬥`, `Tip::Ring` `o`, `Tip::Fork` `< > Ʌ V`, opening onto the node, and `Tip::Cross` `╳`; in ASCII `> < ^ v` for both triangles, `o`, `#` (not `*`, a multiplicity), `o`, `< > ^ v` and `x`.
`Tip::Bar` is a line across the edge instead, merged with it into `┼` (`+`).
`△ ▷ ◇ ◆ ○` would be East Asian ambiguous.

## Glyph width

Every glyph of the Unicode set is one column wide, tested.
Outside the box drawing block none is East Asian ambiguous: arrows are `▴ ▸ ▾ ◂`, not `▲ ▶ ▼ ◀`, and slants are `╱ ╲` from the block, not `◢ ◣`.
Box drawing and block elements (U+2500 to U+259F) are ambiguous in Unicode; they have no replacement and terminals draw them one column wide, so the test exempts those two blocks.

## Dependency direction

```
vt-canvas ← vt-graph ← vt-mermaid
```

vt-canvas is the bottom: it depends on `unicode-width` only, called directly from `text`.
vt-canvas exposes `text_width` so its callers measure text the way it draws it.

## Fuzzing and properties

- `fuzz/` is a cargo-fuzz crate, its own workspace, out of the library's build and of `scripts/check`: `cargo +nightly fuzz run draw` draws arbitrary text, lines, paths, frames, tips and shapes, then checks that every row is exactly the canvas's width.
- The proptest properties in `canvas` keep the same check in `cargo test` for every glyph set, plus a line drawn from either end, a line drawn twice, a path drawn as its lines, and a whole block covering whatever was drawn.
