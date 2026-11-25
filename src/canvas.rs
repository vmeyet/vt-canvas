use std::fmt;
use std::ops::Range;

use crate::block::{self, Block, Stroke};
use crate::geometry::{Direction, Point, Rect};
use crate::glyph::GlyphSet;
use crate::line::{LineKind, Sides};
use crate::shape::{Mark, Marker, Shape};
use crate::text;
use crate::tip::Tip;
use crate::wedge::Wedge;

/// Widest canvas [`Canvas::new`] accepts, in columns.
pub const MAX_WIDTH: u16 = 1_000;
/// Most cells [`Canvas::new`] accepts, width times height: a narrow canvas can be tall.
pub const MAX_CELLS: usize = 1_000_000;

/// What a cell is drawn for; the caller maps each role to a colour.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Role {
    /// Empty cells and anything without a role of its own.
    #[default]
    Plain,
    /// The border of a node.
    Node,
    /// A line between nodes and its arrow.
    Edge,
    /// Text: a node's name, an edge's label, a frame's title.
    Label,
    /// The frame around a group of nodes.
    Group,
    /// A chart's axis and its ticks.
    Axis,
    /// The marks of a chart's series, counted from 0; the caller maps each number to a colour of its palette.
    Series(u8),
}

/// A stretch of one row whose cells share a role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    /// The cells' characters, without `\n`.
    pub text: String,
    /// The role every cell of the run has.
    pub role: Role,
    /// The role behind the cells' characters, `None` where nothing is: a block a later drawing left as its background.
    pub background: Option<Role>,
}

/// The corners of a [`Frame`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Corner {
    /// `┌ ┐ └ ┘`
    #[default]
    Square,
    /// `╭ ╮ ╰ ╯`, square when the frame is not light or dashed, as Unicode has no other arc.
    Rounded,
}

/// How [`Canvas::frame`] draws a rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame<'a> {
    /// The kind of its four lines.
    pub kind: LineKind,
    /// Its corners.
    pub corner: Corner,
    /// Text set in the top border, cut to fit.
    pub title: Option<&'a str>,
    /// The role of the border and the title.
    pub role: Role,
}

/// A drawing the canvas refuses.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The canvas would be wider than [`MAX_WIDTH`] or hold more than [`MAX_CELLS`] cells.
    TooLarge {
        /// Columns asked for.
        width: u16,
        /// Rows asked for.
        height: u16,
    },
    /// A line's ends share neither a row nor a column.
    Diagonal {
        /// Where the line starts.
        from: Point,
        /// Where the line ends.
        to: Point,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge { width, height } => {
                write!(f, "a {width}x{height} canvas is over the limit of {MAX_WIDTH} columns or {MAX_CELLS} cells")
            }
            Self::Diagonal { from, to } => {
                write!(f, "a line from ({}, {}) to ({}, {}) is neither horizontal nor vertical", from.x, from.y, to.x, to.y)
            }
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Cell {
    #[default]
    Empty,
    Text {
        char: char,
        role: Role,
    },
    /// The right half of the wide character in the cell to its left.
    WideTail,
    /// Marks where [`Canvas::text_cut`] cut its text.
    Ellipsis {
        role: Role,
    },
    Line {
        sides: Sides,
        rounded: bool,
        role: Role,
    },
    Arrow {
        direction: Direction,
        role: Role,
    },
    /// A cell of a shape's outline that is not a line.
    Mark {
        mark: Mark,
        role: Role,
    },
    Block {
        block: Block,
        role: Role,
    },
    Wedge {
        wedge: Wedge,
        role: Role,
    },
    Stroke {
        stroke: Stroke,
        role: Role,
    },
}

impl Cell {
    /// The role a drawing over this cell keeps behind it: a block's, when it fills at least half the cell.
    const fn behind(self) -> Option<Role> {
        match self {
            Self::Block { block, role } if block.fills_half() => Some(role),
            Self::Wedge { wedge, role } if wedge.fills_half() => Some(role),
            _ => None,
        }
    }

    const fn fills_cell(self) -> bool {
        match self {
            Self::Block { block, .. } => block.fills_cell(),
            Self::Wedge { wedge, .. } => wedge.fills_cell(),
            _ => false,
        }
    }
}

/// A grid of cells to draw text, lines, frames and arrows on, read back as rows of styled runs.
///
/// Drawing out of bounds is clipped; the last drawing on a cell wins, except that lines merge into junctions
/// and a block at least half a cell stays behind what is drawn over it, as the cell's background.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Canvas {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
    /// The role behind each cell's glyph.
    grounds: Vec<Option<Role>>,
}

impl Canvas {
    /// An empty canvas, `width` columns by `height` rows, at most [`MAX_WIDTH`] columns and [`MAX_CELLS`] cells.
    pub fn new(width: u16, height: u16) -> Result<Self, Error> {
        let (columns, rows) = (usize::from(width), usize::from(height));
        if width > MAX_WIDTH || columns * rows > MAX_CELLS {
            return Err(Error::TooLarge { width, height });
        }
        Ok(Self { width: columns, height: rows, cells: vec![Cell::Empty; columns * rows], grounds: vec![None; columns * rows] })
    }

    /// Writes `text` on one row from `at`, cut at the right edge; see [`width`](crate::width) for what it leaves out.
    pub fn text(&mut self, at: Point, text: &str, role: Role) {
        self.write(usize::from(at.x), usize::from(at.y), text, role, self.width);
    }

    /// Writes `text` like [`text`](Self::text), cut to `max_width` columns, its last column an ellipsis when cut.
    pub fn text_cut(&mut self, at: Point, text: &str, max_width: u16, role: Role) {
        let (x, y, max_width) = (usize::from(at.x), usize::from(at.y), usize::from(max_width));
        if text::width(text) <= max_width {
            self.write(x, y, text, role, x + max_width);
        } else if max_width > 0 {
            let end = self.write(x, y, text, role, x + max_width - 1);
            self.put(end, y, Cell::Ellipsis { role });
        }
    }

    /// Draws a horizontal or vertical line, both ends included; a cell it shares with another line becomes a junction.
    pub fn line(&mut self, from: Point, to: Point, kind: LineKind, role: Role) -> Result<(), Error> {
        if from.x != to.x && from.y != to.y {
            return Err(Error::Diagonal { from, to });
        }
        self.straight(from, to, kind, role);
        Ok(())
    }

    /// Draws a line from each point to the next, so they turn a corner at every point between the ends.
    ///
    /// A pair of consecutive points sharing neither a row nor a column is refused before anything is drawn.
    pub fn path(&mut self, points: &[Point], kind: LineKind, role: Role) -> Result<(), Error> {
        if let Some(&[from, to]) = points.windows(2).find(|pair| pair[0].x != pair[1].x && pair[0].y != pair[1].y) {
            return Err(Error::Diagonal { from, to });
        }
        for pair in points.windows(2) {
            self.straight(pair[0], pair[1], kind, role);
        }
        Ok(())
    }

    /// Draws the border of `rect`, with its title in the top border after the first corner.
    pub fn frame(&mut self, rect: Rect, frame: Frame<'_>) {
        if rect.width == 0 || rect.height == 0 {
            return;
        }
        let (left, top) = (rect.x, rect.y);
        let right = left.saturating_add(rect.width - 1);
        let bottom = top.saturating_add(rect.height - 1);
        let corners = [Point::new(left, top), Point::new(right, top), Point::new(right, bottom), Point::new(left, bottom)];
        for (index, from) in corners.into_iter().enumerate() {
            self.straight(from, corners[(index + 1) % corners.len()], frame.kind, frame.role);
        }
        if frame.corner == Corner::Rounded {
            for corner in corners {
                self.round(corner);
            }
        }
        if let Some(title) = frame.title {
            self.title(rect, title, frame.role);
        }
    }

    /// Draws the outline of `shape` on the border of `rect`; content goes inside its [`inset`](Shape::inset).
    ///
    /// A line drawn onto a slant or a bracket of the outline leaves it whole, so an edge ending on a side touches it.
    pub fn shape(&mut self, rect: Rect, shape: Shape, role: Role) {
        if rect.width == 0 || rect.height == 0 {
            return;
        }
        self.frame(rect, Frame { kind: LineKind::Light, corner: shape.corner(), title: None, role });
        for [from, to] in shape.inner_lines(rect) {
            self.straight(from, to, LineKind::Light, role);
        }
        for placed in shape.marks(rect) {
            self.put(usize::from(placed.at.x), usize::from(placed.at.y), Cell::Mark { mark: placed.mark, role });
        }
    }

    /// Draws an arrow head pointing to `direction`.
    pub fn arrow(&mut self, at: Point, direction: Direction, role: Role) {
        self.put(usize::from(at.x), usize::from(at.y), Cell::Arrow { direction, role });
    }

    /// Draws a cross, `╳` (`x` in ASCII): a crossed arrow head, or where a line is struck out.
    ///
    /// Like a shape's mark, a line drawn onto it later leaves it whole.
    pub fn cross(&mut self, at: Point, role: Role) {
        self.put(usize::from(at.x), usize::from(at.y), Cell::Mark { mark: Mark::Cross, role });
    }

    /// Draws `marker` on one cell: a node too small for an outline.
    ///
    /// Like a shape's mark, a line drawn onto it later leaves it whole, so an edge starting on it keeps it.
    pub fn marker(&mut self, at: Point, marker: Marker, role: Role) {
        self.put(usize::from(at.x), usize::from(at.y), Cell::Mark { mark: Mark::Marker(marker), role });
    }

    /// Draws `tip` on one cell, facing the node `heading` points at.
    ///
    /// A bar merges with the lines through its cell into `┼`; any other tip is kept under a line, as a shape's mark is.
    pub fn tip(&mut self, at: Point, heading: Direction, tip: Tip, role: Role) {
        let (x, y) = (usize::from(at.x), usize::from(at.y));
        if tip == Tip::Bar {
            let across = Direction::ALL.into_iter().fold(Sides::default(), |sides, side| sides.with(side, Some(LineKind::Light)));
            self.draw_line(x, y, across, role);
        } else {
            self.put(x, y, Cell::Mark { mark: Mark::Tip(tip, heading), role });
        }
    }

    /// Fills column `x` between two heights, each counted in eighths of a row from the top of the canvas.
    ///
    /// A bar standing on the bottom of row 9 and 2.5 rows tall is `column(x, 60..80, role)`.
    /// A stretch that touches neither end of its cell has no glyph, so it moves to the end it is nearer.
    pub fn column(&mut self, x: u16, eighths: Range<u32>, role: Role) {
        for (y, block) in block::span(eighths, self.height, Direction::Up) {
            self.put(usize::from(x), y, Cell::Block { block, role });
        }
    }

    /// Fills row `y` between two offsets, each counted in eighths of a column from the left of the canvas.
    pub fn row(&mut self, y: u16, eighths: Range<u32>, role: Role) {
        for (x, block) in block::span(eighths, self.width, Direction::Left) {
            self.put(x, usize::from(y), Cell::Block { block, role });
        }
    }

    /// Fills the part of the cell `at` that is on one side of a slanted edge; a wedge filling nothing draws nothing.
    pub fn wedge(&mut self, at: Point, wedge: Wedge, role: Role) {
        if !wedge.fills_nothing() {
            self.put(usize::from(at.x), usize::from(at.y), Cell::Wedge { wedge, role });
        }
    }

    /// Draws a line one eighth of a row thick across column `x`, `eighth` eighths of a row below the top of the canvas.
    pub fn stroke(&mut self, x: u16, eighth: u32, role: Role) {
        let (y, stroke) = Stroke::at(eighth);
        self.put(usize::from(x), y as usize, Cell::Stroke { stroke, role });
    }

    /// Each row as runs of text sharing a role and a background, every row exactly as wide as the canvas and without `\n`.
    pub fn rows(&self, glyphs: GlyphSet) -> impl Iterator<Item = Vec<Run>> + '_ {
        (0..self.height).map(move |y| {
            let row = y * self.width..(y + 1) * self.width;
            runs(&self.cells[row.clone()], &self.grounds[row], glyphs)
        })
    }

    /// `from` and `to` share a row or a column.
    fn straight(&mut self, from: Point, to: Point, kind: LineKind, role: Role) {
        let horizontal = from.y == to.y;
        let (low, high, before) = if horizontal {
            (from.x.min(to.x), from.x.max(to.x), Direction::Left)
        } else {
            (from.y.min(to.y), from.y.max(to.y), Direction::Up)
        };
        for step in low..=high {
            let (x, y) = if horizontal { (step, from.y) } else { (from.x, step) };
            self.draw_line(usize::from(x), usize::from(y), segment(step, low, high, kind, before), role);
        }
    }

    /// The title sits between `┌─ ` and ` ─┐`, cut to fit.
    fn title(&mut self, rect: Rect, title: &str, role: Role) {
        let (left, y) = (usize::from(rect.x), usize::from(rect.y));
        let (start, end) = (left + 3, (left + usize::from(rect.width)).saturating_sub(3));
        if start >= end {
            return;
        }
        let title_end = self.write(start, y, title, role, end);
        self.put(start - 1, y, Cell::Text { char: ' ', role });
        self.put(title_end, y, Cell::Text { char: ' ', role });
    }

    /// Writes `text` from column `x` up to column `end`, excluded, and returns the column after the last character.
    ///
    /// `end` stops at the right edge, so a wide character on the last column is left out rather than drawn without its right half.
    fn write(&mut self, x: usize, y: usize, text: &str, role: Role, end: usize) -> usize {
        let end = end.min(self.width);
        let mut x = x;
        for glyph in text::glyphs(text) {
            if x + glyph.width > end {
                break;
            }
            self.put(x, y, Cell::Text { char: glyph.char, role });
            if glyph.width == 2 {
                self.put(x + 1, y, Cell::WideTail);
            }
            x += glyph.width;
        }
        x
    }

    /// A line with both ends on one cell leaves it through no side, so it draws nothing.
    fn draw_line(&mut self, x: usize, y: usize, sides: Sides, role: Role) {
        let Some(index) = self.index(x, y).filter(|_| sides.count() > 0) else { return };
        match self.cells[index] {
            Cell::Line { sides: below, rounded, .. } => self.cells[index] = Cell::Line { sides: below.merge(sides), rounded, role },
            Cell::Mark { .. } => {}
            _ => self.put(x, y, Cell::Line { sides, rounded: false, role }),
        }
    }

    fn round(&mut self, at: Point) {
        if let Some(index) = self.index(usize::from(at.x), usize::from(at.y))
            && let Cell::Line { sides, role, .. } = self.cells[index]
        {
            self.cells[index] = Cell::Line { sides, rounded: true, role };
        }
    }

    /// Writing over either half of a wide character blanks the other half.
    fn put(&mut self, x: usize, y: usize, cell: Cell) {
        let Some(index) = self.index(x, y) else { return };
        if self.cells[index] == Cell::WideTail {
            self.cells[index - 1] = Cell::Empty;
        }
        if x + 1 < self.width && self.cells[index + 1] == Cell::WideTail {
            self.cells[index + 1] = Cell::Empty;
        }
        self.grounds[index] = if cell.fills_cell() { None } else { self.cells[index].behind().or(self.grounds[index]) };
        self.cells[index] = cell;
    }

    fn index(&self, x: usize, y: usize) -> Option<usize> {
        (x < self.width && y < self.height).then_some(y * self.width + x)
    }
}

/// The sides a line from `low` to `high` leaves the cell at `at` through; `before` is the side facing `low`.
fn segment(at: u16, low: u16, high: u16, kind: LineKind, before: Direction) -> Sides {
    Sides::default().with(before, (at > low).then_some(kind)).with(before.opposite(), (at < high).then_some(kind))
}

/// A cell as it is written: its character, the role it is drawn in and the role behind it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Styled {
    pub(crate) char: char,
    pub(crate) role: Role,
    pub(crate) background: Option<Role>,
}

fn runs(cells: &[Cell], grounds: &[Option<Role>], glyphs: GlyphSet) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    for (cell, ground) in cells.iter().zip(grounds) {
        let Some(Styled { char, role, background }) = styled(*cell, *ground, glyphs) else { continue };
        match runs.last_mut() {
            Some(run) if run.role == role && run.background == background => run.text.push(char),
            _ => runs.push(Run { text: char.to_string(), role, background }),
        }
    }
    runs
}

/// `None` for the right half of a wide character, written with its left half.
fn styled(cell: Cell, ground: Option<Role>, glyphs: GlyphSet) -> Option<Styled> {
    let (char, role) = match cell {
        Cell::Empty => (' ', Role::Plain),
        Cell::WideTail => return None,
        Cell::Text { char, role } => (char, role),
        Cell::Line { sides, rounded, role } => (glyphs.line(sides, rounded), role),
        Cell::Arrow { direction, role } => (glyphs.arrow(direction), role),
        Cell::Ellipsis { role } => (glyphs.ellipsis(), role),
        Cell::Mark { mark, role } => (glyphs.mark(mark), role),
        Cell::Block { block, role } => return Some(block.styled(role, ground, glyphs)),
        Cell::Wedge { wedge, role } => return Some(wedge.styled(role, ground, glyphs)),
        Cell::Stroke { stroke, role } => (stroke.glyph(glyphs), role),
    };
    Some(Styled { char, role, background: ground })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wedge::Fill;

    const NODE: Frame<'static> = Frame { kind: LineKind::Light, corner: Corner::Square, title: None, role: Role::Node };

    /// Also checks the invariant every drawing keeps: each row is exactly as wide as the canvas.
    fn render(canvas: &Canvas, glyphs: GlyphSet) -> String {
        let rows: Vec<String> = canvas.rows(glyphs).map(|runs| runs.into_iter().map(|run| run.text).collect()).collect();
        for row in &rows {
            assert_eq!(crate::width(row), canvas.width, "row {row:?}");
        }
        rows.join("\n")
    }

    fn canvas(width: u16, height: u16) -> Canvas {
        Canvas::new(width, height).unwrap()
    }

    fn line(canvas: &mut Canvas, from: (u16, u16), to: (u16, u16), kind: LineKind) {
        canvas.line(Point::new(from.0, from.1), Point::new(to.0, to.1), kind, Role::Edge).unwrap();
    }

    fn tallest_full_width() -> u16 {
        u16::try_from(MAX_CELLS / usize::from(MAX_WIDTH)).unwrap()
    }

    #[test]
    fn a_canvas_at_the_cell_limit_is_accepted() {
        let canvas = canvas(MAX_WIDTH, tallest_full_width());

        assert_eq!(canvas.rows(GlyphSet::Unicode).count(), usize::from(tallest_full_width()));
    }

    #[test]
    fn a_narrow_canvas_can_be_tens_of_thousands_of_rows_tall() {
        let canvas = canvas(10, u16::MAX);

        assert_eq!(canvas.rows(GlyphSet::Unicode).count(), usize::from(u16::MAX));
    }

    #[test]
    fn a_canvas_over_the_cell_limit_is_refused() {
        assert_eq!(
            Canvas::new(MAX_WIDTH, tallest_full_width() + 1),
            Err(Error::TooLarge { width: MAX_WIDTH, height: tallest_full_width() + 1 })
        );
    }

    #[test]
    fn a_canvas_over_the_width_limit_is_refused() {
        assert_eq!(Canvas::new(MAX_WIDTH + 1, 1), Err(Error::TooLarge { width: MAX_WIDTH + 1, height: 1 }));
    }

    #[test]
    fn text_is_cut_at_the_right_edge() {
        let mut canvas = canvas(5, 1);

        canvas.text(Point::new(2, 0), "abcdef", Role::Label);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "  abc");
    }

    #[test]
    fn drawing_outside_the_canvas_draws_nothing() {
        let mut canvas = canvas(3, 2);

        canvas.text(Point::new(0, 2), "below", Role::Label);
        canvas.text(Point::new(3, 0), "right", Role::Label);
        canvas.arrow(Point::new(u16::MAX, u16::MAX), Direction::Up, Role::Edge);
        line(&mut canvas, (5, 0), (5, 9), LineKind::Light);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "   \n   ");
    }

    #[test]
    fn a_line_leaving_the_canvas_is_clipped() {
        let mut canvas = canvas(4, 2);

        line(&mut canvas, (1, 0), (u16::MAX, 0), LineKind::Heavy);
        line(&mut canvas, (2, 0), (2, 9), LineKind::Light);

        assert_eq!(render(&canvas, GlyphSet::Unicode), " ━┯━\n  │ ");
    }

    #[test]
    fn a_wide_character_takes_two_cells() {
        let mut canvas = canvas(5, 1);

        canvas.text(Point::new(0, 0), "漢x", Role::Label);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "漢x  ");
    }

    #[test]
    fn a_wide_character_without_room_for_both_halves_is_not_drawn() {
        let mut canvas = canvas(3, 1);

        canvas.text(Point::new(1, 0), "a漢", Role::Label);

        assert_eq!(render(&canvas, GlyphSet::Unicode), " a ");
    }

    #[test]
    fn overwriting_the_left_half_of_a_wide_character_blanks_the_right_half() {
        let mut canvas = canvas(4, 1);
        canvas.text(Point::new(1, 0), "漢", Role::Label);

        canvas.text(Point::new(1, 0), "a", Role::Label);

        assert_eq!(render(&canvas, GlyphSet::Unicode), " a  ");
    }

    #[test]
    fn overwriting_the_right_half_of_a_wide_character_blanks_the_left_half() {
        let mut canvas = canvas(4, 1);
        canvas.text(Point::new(1, 0), "漢", Role::Label);

        canvas.arrow(Point::new(2, 0), Direction::Left, Role::Edge);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "  ◂ ");
    }

    #[test]
    fn a_wide_character_over_two_others_blanks_their_outer_halves() {
        let mut canvas = canvas(6, 1);
        canvas.text(Point::new(0, 0), "漢字", Role::Label);

        canvas.text(Point::new(1, 0), "中", Role::Label);

        assert_eq!(render(&canvas, GlyphSet::Unicode), " 中   ");
    }

    #[test]
    fn text_replaces_a_line() {
        let mut canvas = canvas(5, 1);
        line(&mut canvas, (0, 0), (4, 0), LineKind::Light);

        canvas.text(Point::new(1, 0), "ab", Role::Label);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "─ab──");
    }

    #[test]
    fn a_line_replaces_text_and_arrows() {
        let mut canvas = canvas(4, 1);
        canvas.text(Point::new(0, 0), "ab", Role::Label);
        canvas.arrow(Point::new(3, 0), Direction::Right, Role::Edge);

        line(&mut canvas, (1, 0), (3, 0), LineKind::Double);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "a═══");
    }

    #[test]
    fn a_cross_ends_a_line_and_stays_whole_under_another() {
        let mut canvas = canvas(4, 1);
        canvas.cross(Point::new(3, 0), Role::Edge);

        line(&mut canvas, (0, 0), (3, 0), LineKind::Light);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "───╳");
        assert_eq!(render(&canvas, GlyphSet::Ascii), "---x");
    }

    #[test]
    fn tips_face_the_node_their_edge_goes_into() {
        let mut canvas = canvas(8, 3);
        line(&mut canvas, (0, 1), (7, 1), LineKind::Light);
        let tips = [
            (0, Direction::Left, Tip::Fork),
            (1, Direction::Left, Tip::Bar),
            (2, Direction::Left, Tip::Hollow),
            (3, Direction::Up, Tip::Diamond),
        ];
        let more = [(4, Direction::Down, Tip::FilledDiamond), (5, Direction::Right, Tip::Ring), (6, Direction::Right, Tip::Fork)];

        for (x, heading, tip) in tips.into_iter().chain(more) {
            canvas.tip(Point::new(x, 1), heading, tip, Role::Edge);
        }
        canvas.tip(Point::new(7, 0), Direction::Down, Tip::Fork, Role::Edge);
        canvas.tip(Point::new(7, 2), Direction::Up, Tip::Fork, Role::Edge);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "       Ʌ\n>┼◃⬦⬥o<─\n       V");
        assert_eq!(render(&canvas, GlyphSet::Ascii), "       ^\n>+<o#o<-\n       v");
    }

    #[test]
    fn a_bar_crosses_the_end_of_a_line() {
        let mut canvas = canvas(3, 1);
        line(&mut canvas, (0, 0), (1, 0), LineKind::Light);

        canvas.tip(Point::new(1, 0), Direction::Right, Tip::Bar, Role::Edge);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "─┼ ");
    }

    #[test]
    fn a_marker_fills_its_cell_and_stays_whole_under_a_line_leaving_it() {
        let mut canvas = canvas(3, 3);
        canvas.marker(Point::new(0, 0), Marker::Dot, Role::Node);
        canvas.marker(Point::new(2, 0), Marker::RingedDot, Role::Node);
        canvas.marker(Point::new(1, 2), Marker::Diamond, Role::Node);

        line(&mut canvas, (0, 0), (0, 2), LineKind::Light);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "⬤ ◉\n│  \n│⬥ ");
        assert_eq!(render(&canvas, GlyphSet::Ascii), "* @\n|  \n|? ");
    }

    #[test]
    fn a_line_with_both_ends_on_one_cell_draws_nothing() {
        let mut canvas = canvas(1, 1);
        canvas.text(Point::new(0, 0), "a", Role::Label);

        line(&mut canvas, (0, 0), (0, 0), LineKind::Light);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "a");
    }

    #[test]
    fn text_cut_keeps_text_that_fits() {
        let mut canvas = canvas(6, 1);

        canvas.text_cut(Point::new(0, 0), "abcd", 4, Role::Label);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "abcd  ");
    }

    #[test]
    fn text_cut_ends_cut_text_on_an_ellipsis() {
        let mut canvas = canvas(6, 1);

        canvas.text_cut(Point::new(0, 0), "abcdef", 4, Role::Label);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "abc⋯  ");
        assert_eq!(render(&canvas, GlyphSet::Ascii), "abc~  ");
    }

    #[test]
    fn text_cut_puts_the_ellipsis_after_a_wide_character_that_fits() {
        let mut canvas = canvas(5, 1);

        canvas.text_cut(Point::new(0, 0), "a漢字", 4, Role::Label);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "a漢⋯ ");
    }

    #[test]
    fn text_cut_to_no_width_draws_nothing() {
        let mut canvas = canvas(2, 1);

        canvas.text_cut(Point::new(0, 0), "ab", 0, Role::Label);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "  ");
    }

    #[test]
    fn text_cut_wider_than_the_canvas_leaves_out_a_wide_character_on_the_last_column() {
        let mut canvas = canvas(3, 1);

        canvas.text_cut(Point::new(2, 0), "漢", 5, Role::Label);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "   ");
    }

    #[test]
    fn a_path_turns_a_corner_at_each_inner_point() {
        let mut canvas = canvas(4, 3);
        let points = [Point::new(0, 0), Point::new(3, 0), Point::new(3, 2), Point::new(1, 2)];

        canvas.path(&points, LineKind::Light, Role::Edge).unwrap();

        assert_eq!(render(&canvas, GlyphSet::Unicode), "───┐\n   │\n ──┘");
    }

    #[test]
    fn a_path_with_a_diagonal_step_draws_nothing() {
        let mut canvas = canvas(3, 3);
        let points = [Point::new(0, 0), Point::new(2, 0), Point::new(0, 2)];

        let result = canvas.path(&points, LineKind::Light, Role::Edge);

        assert_eq!(result, Err(Error::Diagonal { from: points[1], to: points[2] }));
        assert_eq!(render(&canvas, GlyphSet::Unicode), "   \n   \n   ");
    }

    #[test]
    fn a_diagonal_line_is_refused() {
        let mut canvas = canvas(3, 3);
        let (from, to) = (Point::new(0, 0), Point::new(2, 2));

        assert_eq!(canvas.line(from, to, LineKind::Light, Role::Edge), Err(Error::Diagonal { from, to }));
    }

    #[test]
    fn crossing_lines_merge_into_a_junction() {
        let mut canvas = canvas(3, 3);

        line(&mut canvas, (0, 1), (2, 1), LineKind::Light);
        line(&mut canvas, (1, 2), (1, 0), LineKind::Heavy);

        assert_eq!(render(&canvas, GlyphSet::Unicode), " ┃ \n─╂─\n ┃ ");
    }

    #[test]
    fn lines_meeting_at_their_ends_turn_a_corner() {
        let mut canvas = canvas(3, 3);

        line(&mut canvas, (0, 0), (2, 0), LineKind::Dashed);
        line(&mut canvas, (2, 0), (2, 2), LineKind::Dashed);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "╌╌┐\n  ╎\n  ╎");
    }

    #[test]
    fn a_line_ending_on_a_frame_joins_it() {
        let mut canvas = canvas(5, 5);
        canvas.frame(Rect::new(0, 2, 5, 3), NODE);

        line(&mut canvas, (2, 0), (2, 2), LineKind::Light);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "  │  \n  │  \n┌─┴─┐\n│   │\n└───┘");
    }

    #[test]
    fn frames_draw_each_line_kind_and_corner() {
        let mut canvas = canvas(17, 3);
        let kinds = [LineKind::Light, LineKind::Heavy, LineKind::Double, LineKind::Dashed];

        for (x, kind) in (0..).step_by(4).zip(kinds) {
            canvas.frame(Rect::new(x, 0, 4, 3), Frame { kind, corner: Corner::Rounded, ..NODE });
        }
        canvas.frame(Rect::new(16, 0, 1, 3), NODE);

        insta::assert_snapshot!(render(&canvas, GlyphSet::Unicode));
    }

    #[test]
    fn a_frame_title_is_cut_to_leave_both_corners() {
        let mut canvas = canvas(12, 6);

        canvas.frame(Rect::new(0, 0, 12, 3), Frame { title: Some("Group"), ..NODE });
        canvas.frame(Rect::new(0, 3, 9, 3), Frame { title: Some("Group"), ..NODE });

        assert_eq!(
            render(&canvas, GlyphSet::Unicode),
            "┌─ Group ──┐\n│          │\n└──────────┘\n┌─ Gro ─┐   \n│       │   \n└───────┘   "
        );
    }

    #[test]
    fn a_frame_too_narrow_for_a_title_draws_none() {
        let mut canvas = canvas(6, 2);

        canvas.frame(Rect::new(0, 0, 6, 2), Frame { title: Some("Group"), ..NODE });

        assert_eq!(render(&canvas, GlyphSet::Unicode), "┌────┐\n└────┘");
    }

    #[test]
    fn a_frame_title_past_the_canvas_leaves_out_a_wide_character_on_the_last_column() {
        let mut canvas = canvas(7, 1);

        canvas.frame(Rect::new(0, 0, 20, 3), Frame { title: Some("abc漢"), ..NODE });

        assert_eq!(render(&canvas, GlyphSet::Unicode), "┌─ abc ");
    }

    #[test]
    fn the_ascii_set_draws_lines_junctions_and_arrows() {
        let mut canvas = canvas(9, 3);
        canvas.frame(Rect::new(0, 0, 4, 3), Frame { corner: Corner::Rounded, ..NODE });
        line(&mut canvas, (3, 1), (8, 1), LineKind::Dashed);
        line(&mut canvas, (6, 0), (6, 2), LineKind::Heavy);

        canvas.arrow(Point::new(8, 1), Direction::Right, Role::Edge);

        assert_eq!(render(&canvas, GlyphSet::Ascii), ".--.  |  \n|  +..+.>\n'--'  |  ");
    }

    #[test]
    fn rows_group_cells_by_role_without_newlines() {
        let mut canvas = canvas(6, 1);
        canvas.text(Point::new(1, 0), "ab", Role::Label);
        line(&mut canvas, (3, 0), (4, 0), LineKind::Light);

        let runs: Vec<Run> = canvas.rows(GlyphSet::Unicode).flatten().collect();

        let run = |text: &str, role| Run { text: text.to_owned(), role, background: None };
        assert_eq!(runs, [run(" ", Role::Plain), run("ab", Role::Label), run("──", Role::Edge), run(" ", Role::Plain)]);
    }

    /// Each run as its text, its role and its background, so a test sees which role a cell kept.
    fn styles(canvas: &Canvas, glyphs: GlyphSet) -> Vec<String> {
        canvas
            .rows(glyphs)
            .map(|runs| runs.into_iter().map(|run| format!("[{} {:?} on {:?}]", run.text, run.role, run.background)).collect())
            .collect()
    }

    #[test]
    fn a_column_stands_on_the_bottom_of_a_row_in_eighths() {
        let mut canvas = canvas(3, 3);

        canvas.column(0, 4..24, Role::Series(0));
        canvas.column(1, 21..24, Role::Series(0));
        canvas.column(2, 0..3, Role::Series(0));

        assert_eq!(render(&canvas, GlyphSet::Unicode), "▄ 🮃\n█  \n█▃ ");
        assert_eq!(render(&canvas, GlyphSet::Ascii), "# \"\n#  \n#_ ");
    }

    #[test]
    fn a_row_starts_at_the_left_in_eighths() {
        let mut canvas = canvas(3, 2);

        canvas.row(0, 0..19, Role::Series(0));
        canvas.row(1, 21..80, Role::Series(1));

        assert_eq!(render(&canvas, GlyphSet::Unicode), "██▍\n  🮈");
        assert_eq!(render(&canvas, GlyphSet::Ascii), "##|\n  |");
    }

    #[test]
    fn a_drawing_over_a_block_at_least_half_a_cell_keeps_it_as_background() {
        let mut canvas = canvas(3, 1);
        canvas.column(0, 0..8, Role::Series(0));
        canvas.column(1, 4..8, Role::Series(0));
        canvas.column(2, 5..8, Role::Series(0));

        canvas.text(Point::new(0, 0), "abc", Role::Label);

        assert_eq!(styles(&canvas, GlyphSet::Unicode), ["[ab Label on Some(Series(0))][c Label on None]"]);
    }

    #[test]
    fn a_line_over_a_block_keeps_it_as_background() {
        let mut canvas = canvas(3, 1);
        canvas.column(1, 0..8, Role::Series(2));

        line(&mut canvas, (0, 0), (2, 0), LineKind::Light);

        assert_eq!(styles(&canvas, GlyphSet::Unicode), ["[─ Edge on None][─ Edge on Some(Series(2))][─ Edge on None]"]);
    }

    #[test]
    fn a_stroke_over_a_bar_is_drawn_on_the_bar() {
        let mut canvas = canvas(3, 2);
        canvas.column(1, 3..16, Role::Series(0));

        for x in 0..3 {
            canvas.stroke(x, 5, Role::Series(1));
        }

        assert_eq!(
            styles(&canvas, GlyphSet::Unicode),
            [
                "[🭺 Series(1) on None][🭺 Series(1) on Some(Series(0))][🭺 Series(1) on None]",
                "[  Plain on None][█ Series(0) on None][  Plain on None]"
            ]
        );
        assert_eq!(render(&canvas, GlyphSet::Unicode3), "───\n █ ");
        assert_eq!(render(&canvas, GlyphSet::Ascii), "---\n # ");
    }

    #[test]
    fn a_stroke_below_the_canvas_draws_nothing() {
        let mut canvas = canvas(1, 1);

        canvas.stroke(0, 8, Role::Series(0));
        canvas.stroke(0, u32::MAX, Role::Series(0));

        assert_eq!(render(&canvas, GlyphSet::Unicode), " ");
    }

    #[test]
    fn a_wedge_over_a_wedge_at_least_half_a_cell_keeps_it_as_background() {
        let mut canvas = canvas(2, 1);
        let rising = |fill| Wedge { left: 1, right: 2, fill };
        canvas.wedge(Point::new(0, 0), rising(Fill::Below), Role::Series(0));
        canvas.wedge(Point::new(1, 0), Wedge { left: 0, right: 1, fill: Fill::Below }, Role::Series(0));

        canvas.wedge(Point::new(0, 0), rising(Fill::Above), Role::Series(1));
        canvas.wedge(Point::new(1, 0), rising(Fill::Above), Role::Series(1));

        assert_eq!(styles(&canvas, GlyphSet::Unicode), ["[🭜 Series(1) on Some(Series(0))][🭜 Series(1) on None]"]);
        assert_eq!(render(&canvas, GlyphSet::Ascii), "//");
    }

    #[test]
    fn a_wedge_filling_nothing_leaves_the_cell_as_it_was() {
        let mut canvas = canvas(1, 1);
        canvas.text(Point::new(0, 0), "a", Role::Label);

        canvas.wedge(Point::new(0, 0), Wedge { left: 0, right: 0, fill: Fill::Below }, Role::Series(0));

        assert_eq!(render(&canvas, GlyphSet::Unicode), "a");
    }

    #[test]
    fn a_block_over_a_block_keeps_the_first_as_background() {
        let mut canvas = canvas(1, 2);
        canvas.column(0, 0..16, Role::Series(1));

        canvas.column(0, 11..16, Role::Series(0));

        assert_eq!(styles(&canvas, GlyphSet::Unicode), ["[█ Series(1) on None]", "[▅ Series(0) on Some(Series(1))]"]);
    }

    #[test]
    fn a_background_stays_under_every_drawing_that_follows() {
        let mut canvas = canvas(1, 1);
        canvas.column(0, 0..8, Role::Series(1));
        canvas.column(0, 6..8, Role::Series(0));

        canvas.text(Point::new(0, 0), "a", Role::Label);

        assert_eq!(styles(&canvas, GlyphSet::Unicode), ["[a Label on Some(Series(1))]"]);
    }

    #[test]
    fn a_whole_block_leaves_no_background() {
        let mut canvas = canvas(1, 1);
        canvas.column(0, 0..8, Role::Series(1));

        canvas.column(0, 0..8, Role::Series(0));

        assert_eq!(styles(&canvas, GlyphSet::Unicode), ["[█ Series(0) on None]"]);
    }

    #[test]
    fn cells_with_different_backgrounds_are_different_runs() {
        let mut canvas = canvas(4, 1);
        canvas.row(0, 8..16, Role::Series(0));
        canvas.row(0, 16..24, Role::Series(1));

        canvas.text(Point::new(0, 0), "abcd", Role::Label);

        assert_eq!(
            styles(&canvas, GlyphSet::Unicode),
            ["[a Label on None][b Label on Some(Series(0))][c Label on Some(Series(1))][d Label on None]"]
        );
    }

    mod properties {
        use proptest::prelude::*;
        use unicode_width::UnicodeWidthChar;

        use super::*;

        /// Points reach past the canvas, so every property also holds while clipping.
        const REACH: u16 = 24;

        fn point() -> impl Strategy<Value = Point> {
            (0..REACH, 0..REACH).prop_map(|(x, y)| Point::new(x, y))
        }

        fn kind() -> impl Strategy<Value = LineKind> {
            prop_oneof![Just(LineKind::Light), Just(LineKind::Heavy), Just(LineKind::Double), Just(LineKind::Dashed)]
        }

        fn straight() -> impl Strategy<Value = (Point, Point)> {
            (point(), 0..REACH, any::<bool>())
                .prop_map(|(from, to, horizontal)| if horizontal { (from, Point::new(to, from.y)) } else { (from, Point::new(from.x, to)) })
        }

        /// A path without a diagonal step: each point shares a row or a column with the one before.
        fn path() -> impl Strategy<Value = Vec<Point>> {
            (point(), prop::collection::vec((0..REACH, any::<bool>()), 0..6)).prop_map(|(start, turns)| {
                turns.into_iter().fold(vec![start], |mut points, (to, horizontal)| {
                    let last = points[points.len() - 1];
                    points.push(if horizontal { Point::new(to, last.y) } else { Point::new(last.x, to) });
                    points
                })
            })
        }

        fn rows(canvas: &Canvas) -> Vec<String> {
            canvas.rows(GlyphSet::Unicode).map(|runs| runs.into_iter().map(|run| run.text).collect()).collect()
        }

        proptest! {
            #[test]
            fn every_row_is_as_wide_as_the_canvas(
                (width, height) in (0..16u16, 0..16u16),
                texts in prop::collection::vec((point(), "[a漢\u{301}\u{200b}\t\u{1b} ]{0,8}", 0..12u16), 0..6),
                lines in prop::collection::vec((straight(), kind()), 0..6),
                frames in prop::collection::vec((point(), point(), "[b漢 ]{0,6}"), 0..3),
            ) {
                let mut canvas = canvas(width, height);
                for ((from, to), kind) in lines {
                    canvas.line(from, to, kind, Role::Edge).unwrap();
                }
                for (corner, size, title) in &frames {
                    canvas.frame(Rect::new(corner.x, corner.y, size.x, size.y), Frame { title: Some(title), ..NODE });
                }
                for (at, text, max_width) in &texts {
                    canvas.text_cut(*at, text, *max_width, Role::Label);
                }

                for row in rows(&canvas) {
                    let columns: usize = row.chars().map(|char| char.width().unwrap_or(0)).sum();
                    prop_assert_eq!(columns, usize::from(width), "row {:?}", row);
                }
            }

            #[test]
            fn a_line_draws_the_same_from_either_end(((from, to), kind) in (straight(), kind())) {
                let mut forward = canvas(16, 16);
                let mut backward = canvas(16, 16);

                forward.line(from, to, kind, Role::Edge).unwrap();
                backward.line(to, from, kind, Role::Edge).unwrap();

                prop_assert_eq!(forward, backward);
            }

            #[test]
            fn drawing_a_line_twice_changes_nothing(((from, to), kind) in (straight(), kind()), path in path()) {
                let mut once = canvas(16, 16);
                once.path(&path, LineKind::Light, Role::Edge).unwrap();
                once.line(from, to, kind, Role::Edge).unwrap();
                let mut twice = once.clone();

                twice.line(from, to, kind, Role::Edge).unwrap();

                prop_assert_eq!(once, twice);
            }

            #[test]
            fn a_path_draws_a_line_from_each_point_to_the_next(points in path(), kind in kind()) {
                let mut path = canvas(16, 16);
                let mut lines = canvas(16, 16);

                path.path(&points, kind, Role::Edge).unwrap();
                for pair in points.windows(2) {
                    lines.line(pair[0], pair[1], kind, Role::Edge).unwrap();
                }

                prop_assert_eq!(path, lines);
            }
        }
    }
}
