# Architecture

`vt-canvas` is one library crate: a grid of terminal cells to draw text, lines and boxes on, read back as styled text, never as ANSI escapes.
This page says how the code is laid out today.
When a module or a boundary changes, change this page in the same change.

## Overview

A caller makes a `Canvas<R>`, draws on it, then reads it back in a glyph set.

- Drawing stores what a cell holds (the sides of lines, a block, dots, a character), not always the final glyph.
- Reading picks the glyph for each cell in the glyph set asked for, so one drawing reads in every set.
- Each cell keeps a *role* of the caller's own type; the caller maps roles to colours and writes the terminal.

```text
 caller
   │  Canvas::new(width, height)              canvas        size limits, Error
   ▼
 draw ─┬─ text, text_cut                      canvas        text: which chars fit, their width
       ├─ horizontal, vertical, path, frame   canvas/lines  line: sides of a cell; frame: Corner
       ├─ shape, tip, symbol                  canvas/lines  shape, tip, symbol
       └─ fill_column, fill_row, stroke,      canvas/fine   block, wedge, pixel, styled
          wedge, dot, segment
   │
   ▼
 cells: a glyph or the sides of lines, a role, a background role
   │
   ▼
 read ─── rows(GlyphSet), lines(GlyphSet)     canvas        glyph: the character for each cell
   │
   ▼
 caller: maps roles to colours, sanitizes, writes the terminal
```

## Owns and never

The crate owns:

- The cell grid: each cell holds one glyph, its width, a colour role and the role behind it.
- Drawing on it: text, lines and boxes whose crossings merge into the right junction glyph, and node shapes.
- Drawing finer than a cell: blocks filling a cell by eighths, lines one eighth thick, wedges under or over a slanted edge, and dots on a grid that cuts each cell in 2 to 8.
- Glyph sets: which characters a drawing uses, avoiding East Asian ambiguous-width glyphs where Unicode has a choice.
- Measuring text: `text_width` says how many columns `Canvas::text` gives a string, so a caller sizes a box with the rule the canvas draws with.

The crate never:

- Reads files, the environment or the clock; prints; exits the process.
- Writes ANSI escapes or picks colours.
- Lets a dependency's type into a public signature.
- Knows a chart or a diagram: those live in vt-graph and vt-mermaid.

## Modules

| Module | Holds |
|---|---|
| `canvas` | `Canvas`, its cells and their backgrounds, `Run`, `Error` and the size limits |
| `canvas/lines` | Lines, paths, frames, shapes, tips and symbols on the canvas, merged into junctions |
| `canvas/fine` | Blocks, strokes, wedges and dots on the canvas, and the backgrounds they leave |
| `frame` | `Corner`: the corners of a frame |
| `role` | `Role`: what the canvas asks of the caller's role type, its ASCII texture |
| `line` | `LineKind` and the four sides a line leaves a cell through |
| `glyph` | `GlyphSet`: the character for a cell's sides or a filled cell, junction tables included |
| `block` | `EIGHTHS`, a cell filled by eighths from one side, a line one eighth thick, their glyphs, the cells a stretch covers, how much of a cell a glyph covers |
| `pixel` | `Dots`, `Pixel`: the dots of a cell and their glyph, the two halves of a half-block cell, the dots of a straight line |
| `wedge` | `Wedge`, `Fill`: a cell filled under or over a slanted edge, and its glyph |
| `shape` | `Shape`, `Inset`: each shape's outline in one place, which border cells are slants or brackets instead of lines, and their glyphs |
| `symbol` | `Symbol`: a node on one cell, and its glyph |
| `tip` | `Tip`: what an edge draws where it meets a node, and its glyph |
| `styled` | A cell as it is read back: its character, its role and the role behind it; what `canvas` asks of `block`, `pixel` and `wedge` |
| `text` | Which characters a cell can hold and how wide they are |
| `geometry` | `Point`, `Rect`, `Direction` |

## Canvas: the contract

### Size

`Canvas::new` refuses more than 1000 columns (`MAX_WIDTH`) or 1000000 cells (`MAX_CELLS`, width times height) with `Error::TooLarge`.
So a narrow drawing can be tall.

### Clipping

Drawing out of bounds is cut at the edges: never an error, never a panic.
A line or a segment only walks the cells on the canvas, so its cost is bounded by the canvas, not by its ends.

### Lines and paths

- `horizontal` and `vertical` cannot be given a diagonal.
- A line with both ends on one cell draws nothing.
- A path is a line from each point to the next, turning a corner at each point between.
- A diagonal step is `Error::Diagonal`, and it refuses the whole path before anything is drawn.

### Overwriting

- The last drawing on a cell wins.
- Except: a line drawn on a line merges with it, and a line drawn on a shape's mark, a tip or a symbol leaves it whole.
- Writing over either half of a wide character blanks the other half.
- A wide character without room for both halves is not drawn.

### Text

- Control and zero-width characters are left out, so every row is exactly as wide as the canvas.
- `text_cut` cuts text to a width, or at the right edge of the canvas.
  When cut, it ends on `⋯` (`~` in ASCII); `…` would be East Asian ambiguous.

### Roles

- A role is the caller's own type: `Canvas<R: Role>`.
  The canvas keeps one per cell and hands it back; the caller maps each one to a colour.
- `Role` asks for `Copy + Eq + Default`; the default is the role of empty cells.
- `Role::texture` gives an ASCII texture, 0 unless told otherwise, so roles differ without colour.
- A `u8` is a role whose texture is itself.

### Output

- `rows` yields each row as runs of text sharing a role and a background, without `\n`.
- `lines` yields each row's text alone.
- A run's `background` is the role behind its characters, `None` where nothing is.

### Cost

Medians at the cell limit, measured on an Apple M5 on 2026-11-26 with `cargo bench --bench canvas`:

| Drawing | Draw | Read back |
|---|---|---|
| A lattice of lines on every other row and column | 4.6 ms | 4.4 ms |
| A bar in every column under a line of octant dots across every row | 18 ms | 4.5 ms |
| A row of text on every row | 3.6 ms | |

With a role of 2 bytes, a cell takes 8 bytes and its background 2, so the canvas holds 10 MB.

## Lines: junctions

Each cell keeps, for each of its four sides, whether a line leaves through it and in which kind.
The glyph is chosen when the rows are read.

### Rules

Applied in order:

1. A side drawn twice takes the kind drawn last.
2. Dashed stays dashed only on a straight run of dashed sides (`╌ ╎`).
   Unicode has no dashed junction, so anywhere else dashed counts as light.
3. A side alone continues through its cell: a line end fills the cell (`─`, never the half `╴`).
4. A rounded corner is an arc (`╭`) only when its two sides are light.
   A heavy or double corner stays square, and a corner a line joins becomes a junction.
   The last frame drawn on a corner sets whether it is rounded; a frame one cell wide or tall has no corner.
5. Light and heavy mix freely: Unicode has every combination (`┽ ┿ ╀ ╆`).
6. Double mixes with light only axis by axis (`╪ ╫ ╞`).
   An axis with a double side is all double, and heavy elsewhere becomes light, as Unicode has no heavy and double glyph.

### Crossings

Two lines crossing draw `┼`, never a "pass over" where the line drawn first stays whole.
This was tried on the flowchart spike's output.
Frames are drawn before edges, so a pass over cut every edge where it left a group, which read as the edge ending at the frame.
Between two edges it broke one of them, which made it harder to follow.
Flowchart edges only meet at nodes, so a `┼` between nodes is never read as a join.

### Junction role

A junction has one role.

- It keeps the border's role when the new line only ends on the cell, adds a side the cell lacks, and the cell already has two sides or more (a side or a corner of a border).
  So a node's border stays whole in its colour where an edge joins it (`┤`, `├` on a corner).
- Otherwise it takes the role of the line drawn last.
  So an edge crossing a frame stays whole in its colour, and a frame or a shape redrawn in another role is wholly in that role, its corners included.

## Shapes, tips and symbols

### Shapes

`Canvas::shape` draws one of Mermaid's 14 flowchart shapes (`Shape::ALL`) on the border of a rectangle.
It is a light frame with some cells replaced by *marks*: slants `╱ ╲` and brackets `( ) < >`.

- Every border cell keeps a glyph, so a line ending on any side of the rectangle touches the shape.
- A line ending on a frame cell joins it (`┤`); one ending on a mark leaves it whole (`──<`).
- A shape is told apart by its corners and its sides: square or rounded corners, `╱ ╲` corners for a diamond, `< >` sides for a hexagon, `( )` for a stadium.
- A side that points out (hexagon, asymmetric) points at its middle row, or between its two middle rows when the count is even.
- A circle is a stadium with a blank column inside each bracket: three rows cannot curve more.
- A double circle draws its second brackets from four columns, so a narrower one is a stadium.
- `Shape::inset` gives the columns and rows the outline takes, so the caller sizes the rectangle around its content.

### Symbols

`Canvas::symbol` draws a node too small for an outline on one cell.
Like a mark, it stays whole under a line drawn later.

| Symbol | Unicode | ASCII |
|---|---|---|
| `Symbol::Dot` | `⬤` | `*` |
| `Symbol::RingedDot` | `◉` | `@` |
| `Symbol::FilledDiamond` | `⬥` | `?` |

`●` and `◆` would be East Asian ambiguous.

### Tips

`Canvas::tip` draws an edge's end on one cell, facing the node it goes into.
Like a mark, it stays whole under a line drawn later, so an edge drawn after it does not cut it.

| Tip | Unicode | ASCII |
|---|---|---|
| `Tip::Arrow` | `▸ ◂ ▴ ▾` | `> < ^ v` |
| `Tip::Triangle` | `▹ ◃ ▵ ▿` | `> < ^ v` |
| `Tip::Diamond` | `⬦` | `o` |
| `Tip::FilledDiamond` | `⬥` | `#` (not `*`, a multiplicity) |
| `Tip::Ring` | `o` | `o` |
| `Tip::Fork` | `< > Ʌ V`, opening onto the node | `< > ^ v` |
| `Tip::Cross` | `╳` | `x` |
| `Tip::Bar` | a line across the edge, merged into `┼` | `+` |

`Tip::Bar` takes the kind of the line it crosses: `╋` on a heavy line, `╬` on a double one.
On a symbol, a mark or a tip, it replaces them, as every tip does.
`△ ▷ ◇ ◆ ○` would be East Asian ambiguous.

## Finer than a cell

### Blocks

`fill_column` and `fill_row` fill a stretch of one column or one row.
Its two ends count in eighths of a cell (`EIGHTHS`) from the top or the left of the canvas.
Each cell the stretch covers becomes a block filled from one of its sides:

| From | Glyphs |
|---|---|
| Bottom | `▁▂▃▄▅▆▇█` |
| Left | `▏▎▍▌▋▊▉█` |
| Top | `▔🮂🮃▀🮄🮅🮆█` |
| Right | `▕🮇🮈▐🮉🮊🮋█` |

A stretch that touches neither side of its cell has no glyph: it keeps its size and moves to the side it is nearer.

### Strokes

`stroke` draws a line one eighth of a row thick across one column, at any eighth of a row: `▔🭶🭷🭸🭹🭺🭻▁`.
It is how a line passes in front of a bar: the bar stays behind it as the cell's background.

### Wedges

`wedge` fills the part of a cell under or over an edge that runs from its left side to its right side.
Each end is 0 to 3 thirds of the cell's height below its top, as every coordinate counts down.

- Below: `🭈 🭊 🬽 🭆 🭄 🬿 🭑 🭂 🭏 🭍`.
- Above: `🭞 🭠 🭓 🭜 🭚 🭕 🭧 🭘 🭥 🭣`.
- Level edge: the sextant rows `🬭 🬹 🬎 🬂`.

Legacy Computing has no edge from a bottom corner to the opposite top corner: `▌` or `▐`, the half on the filled side, stands in.

### Dots

`dot` lights one dot of a grid of pixels over the cells, and `segment` the dots of a straight line.
`Dots` says how many dots a cell holds and which glyphs light them.

| `Dots` | In a cell | Glyphs | Drawn as, by the sets without them |
|---|---|---|---|
| `Braille` | 2 by 4 | `⠉ ⡠ ⣿` | `Quadrant` in ASCII |
| `Octant` | 2 by 4 | U+1CD00… (Unicode 16) | `Sextant` in `Unicode13`, `Quadrant` in `Unicode3` and ASCII |
| `Sextant` | 2 by 3 | U+1FB00… (Unicode 13) | `Quadrant` in `Unicode3` and ASCII |
| `Quadrant` | 2 by 2 | `▘ ▚ ▟` | `'` for the upper dots, `.` for the lower, `:` for both in ASCII; `" , ;` for an odd texture, so two lines differ without colour |
| `HalfBlock` | 1 by 2 | `▀ ▄ █` | `'`, `.` and the role's filled cell in ASCII |

- A coarser grid lights the dot nearest each dot.
  Of an octant's four rows, the two in the middle share a sextant's middle row, and the upper two and lower two share a quadrant's rows.
  A sextant's middle row joins a quadrant's lower row, so a level line stays one dot thick.
- A cell has one role for all its dots, the last drawn, so two roles crossing in a cell show as one there.
- `HalfBlock` is the exception, and the reason to pick it: its upper dot is `▀` in its own role, drawn on the lower dot's role, two colours in the cell.
- Dots of another kind replace the cell's.
- A segment walks one dot per step of its longer way, and only the steps inside the canvas, so its cost is bounded by the canvas, not by its ends.

### Layers

A terminal cell has one character, one colour for it and one colour behind it, so a cell shows two roles at most.
Each cell keeps a glyph with its role and, behind it, a *background* role.

1. A drawing replaces the cell's glyph; the last one wins, as for text and lines.
2. The glyph it replaces stays as the background when it is a block, a wedge or a half-block dot filling at least half of what the drawing leaves of the cell; a smaller one is dropped.
   Text, a line, a stroke or dots leave the whole cell, so a block stays behind them from half a cell up.
   A block or a wedge leaves what it does not fill: `▅` drawn over `▔` keeps it, as the upper quarter is most of the three eighths left, and the cell shows both roles.
3. Otherwise the background stays as it was, until a drawing that fills the whole cell clears it.

A wide character is drawn on its left cell's background, as a terminal paints one colour behind both its columns.

So text, a line, a stroke or dots over a bar are drawn on the bar's colour.
A block drawn over a taller one shows both, and two bands meeting in a cell fill it between them.
Where a third role would be needed (a line through the cell a bar ends in), the bar gains or loses under half a cell.

## Reading back: glyph sets

`rows` and `lines` take the set a drawing is read in.
The drawing is the same for every set, and each glyph falls back to the closest one the set holds.

| Set | Holds | Falls back |
|---|---|---|
| `Unicode` | Everything, octants (Unicode 16) included | |
| `Unicode13` | Symbols for Legacy Computing: sextants, the eighth blocks from the top and the right, strokes, wedges | Octants are sextants |
| `Unicode3` | Box drawing, block elements, braille | A block from the top or the right is the nearest of `▔ ▀ █`, `▕ ▐ █`; a stroke the nearest of `▔ ─ ▁`; a wedge the block filling as much from the bottom or the top; octants and sextants are quadrants |
| `Ascii` | ASCII | A block at least half a cell is `#`, a smaller one `_`, `"` or `|` on the side it starts from; a stroke the nearest of `" - _`; a wedge `/` or `\`, a block when its edge is level; dots are `' . :` |

### ASCII

- A filled cell is `# % * o @ x` for textures 0 to 5, then again from `#`, so roles differ without colour.
- A cell holding a block under half on a background shows the background's character, the one filling more of it.
- Any junction is `+`.
- A horizontal run is `-`, `=` when heavy or double, `.` when dashed.
- A vertical run is `|`, `:` when dashed.
- A rounded corner is `.` at the top and `'` at the bottom, so a rounded box still differs from a square one.
- Slants are `/ \`; brackets and points are the same as in Unicode.

### Glyph width

- Every glyph of the Unicode set is one column wide, tested.
- Outside the box drawing block, none is East Asian ambiguous: arrows are `▴ ▸ ▾ ◂`, not `▲ ▶ ▼ ◀`, and slants are `╱ ╲` from the block, not `◢ ◣`.
- Box drawing and block elements (U+2500 to U+259F) are ambiguous in Unicode.
  They have no replacement and terminals draw them one column wide, so the test exempts those two blocks.

## Dependencies

```text
          ┌── vt-graph    (charts, not published)
vt-canvas ┤
          └── vt-mermaid  (Mermaid diagrams)
```

vt-graph and vt-mermaid each depend on vt-canvas directly; neither depends on the other.
vt-canvas is the bottom: it depends on `unicode-width` only, called directly from `text`.
It exposes `text_width` so its callers measure text the way it draws it.

## Fuzzing and properties

- `fuzz/` is a cargo-fuzz crate in its own workspace, out of the library's build and of `scripts/check`.
  `cargo +nightly fuzz run draw` draws arbitrary text, lines, paths, frames, shapes, tips, symbols and drawings finer than a cell.
  It then checks that every row, in every glyph set, is exactly the canvas's width.
- The proptest properties in `canvas/mod.rs` keep the same width check in `cargo test`, for every glyph set and every kind of drawing.
  They also check that a line draws the same from either end, a line drawn twice changes nothing, a path draws as its lines, a tip or a symbol stays whole under the lines drawn after it, and a whole block covers whatever was drawn.
