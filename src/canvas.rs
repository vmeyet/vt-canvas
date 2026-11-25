use std::fmt;

use crate::geometry::{Direction, Point, Rect};
use crate::glyph::GlyphSet;
use crate::line::{LineKind, Sides};
use crate::shape::{Mark, Shape};
use crate::text;

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
}

/// A stretch of one row whose cells share a role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    /// The cells' characters, without `\n`.
    pub text: String,
    /// The role every cell of the run has.
    pub role: Role,
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
}

/// A grid of cells to draw text, lines, frames and arrows on, read back as rows of styled runs.
///
/// Drawing out of bounds is clipped; the last drawing on a cell wins, except that lines merge into junctions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Canvas {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
}

impl Canvas {
    /// An empty canvas, `width` columns by `height` rows, at most [`MAX_WIDTH`] columns and [`MAX_CELLS`] cells.
    pub fn new(width: u16, height: u16) -> Result<Self, Error> {
        let (columns, rows) = (usize::from(width), usize::from(height));
        if width > MAX_WIDTH || columns * rows > MAX_CELLS {
            return Err(Error::TooLarge { width, height });
        }
        Ok(Self { width: columns, height: rows, cells: vec![Cell::Empty; columns * rows] })
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

    /// Each row as runs of `(text, role)`, every row exactly as wide as the canvas and without `\n`.
    pub fn rows(&self, glyphs: GlyphSet) -> impl Iterator<Item = Vec<Run>> + '_ {
        (0..self.height).map(move |y| runs(&self.cells[y * self.width..(y + 1) * self.width], glyphs))
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

fn runs(cells: &[Cell], glyphs: GlyphSet) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    for cell in cells {
        let (char, role) = match *cell {
            Cell::Empty => (' ', Role::Plain),
            Cell::WideTail => continue,
            Cell::Text { char, role } => (char, role),
            Cell::Line { sides, rounded, role } => (glyphs.line(sides, rounded), role),
            Cell::Arrow { direction, role } => (glyphs.arrow(direction), role),
            Cell::Ellipsis { role } => (glyphs.ellipsis(), role),
            Cell::Mark { mark, role } => (glyphs.mark(mark), role),
        };
        match runs.last_mut() {
            Some(run) if run.role == role => run.text.push(char),
            _ => runs.push(Run { text: char.to_string(), role }),
        }
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let run = |text: &str, role| Run { text: text.to_owned(), role };
        assert_eq!(runs, [run(" ", Role::Plain), run("ab", Role::Label), run("──", Role::Edge), run(" ", Role::Plain)]);
    }
}
