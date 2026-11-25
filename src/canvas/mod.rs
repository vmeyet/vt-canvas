mod fine;
mod lines;

use std::fmt;

use crate::block::{Block, Cover, Stroke};
use crate::geometry::{Direction, Point};
use crate::glyph::GlyphSet;
use crate::line::Sides;
use crate::pixel::{Dots, Halves};
use crate::role::Role;
use crate::shape::Mark;
use crate::symbol::Symbol;
use crate::text;
use crate::tip::Tip;
use crate::wedge::Wedge;

/// Widest canvas [`Canvas::new`] accepts, in columns.
pub const MAX_WIDTH: u16 = 1_000;
/// Most cells [`Canvas::new`] accepts, width times height: a narrow canvas can be tall.
pub const MAX_CELLS: usize = 1_000_000;

/// A stretch of one row whose cells share a role and a background.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run<R> {
    /// The cells' characters, without `\n`.
    pub text: String,
    /// The role every cell of the run has.
    pub role: R,
    /// The role behind the cells' characters, `None` where nothing is: a block a later drawing left as its background.
    pub background: Option<R>,
}

/// A drawing the canvas refuses.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The canvas would be wider than [`MAX_WIDTH`] or hold more than [`MAX_CELLS`] cells.
    TooLarge {
        /// Columns asked for.
        width: u16,
        /// Rows asked for.
        height: u16,
    },
    /// Two consecutive points of a path share neither a row nor a column.
    Diagonal {
        /// Where the step starts.
        from: Point,
        /// Where the step ends.
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
                write!(f, "a path's step from ({}, {}) to ({}, {}) is neither horizontal nor vertical", from.x, from.y, to.x, to.y)
            }
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Cell<R> {
    #[default]
    Empty,
    Text {
        char: char,
        role: R,
    },
    /// The right half of the wide character in the cell to its left.
    WideTail,
    /// Marks where [`Canvas::text_cut`] cut its text.
    Ellipsis {
        role: R,
    },
    Line {
        sides: Sides,
        rounded: bool,
        role: R,
    },
    /// A cell of a shape's outline that is not a line.
    Mark {
        mark: Mark,
        role: R,
    },
    Tip {
        tip: Tip,
        direction: Direction,
        role: R,
    },
    Symbol {
        symbol: Symbol,
        role: R,
    },
    Block {
        block: Block,
        role: R,
    },
    Wedge {
        wedge: Wedge,
        role: R,
    },
    Stroke {
        stroke: Stroke,
        role: R,
    },
    /// The dots lit in a cell, one bit each, row by row from the top left.
    Dots {
        dots: Dots,
        lit: u8,
        role: R,
    },
    Halves(Halves<R>),
}

impl<R: Role> Cell<R> {
    /// How much of the cell this glyph fills and from which side, when it is one that fills.
    fn cover(self) -> Option<Cover> {
        match self {
            Self::Block { block, .. } => Some(block.cover()),
            Self::Wedge { wedge, .. } => Some(wedge.cover()),
            Self::Halves(halves) => halves.cover(),
            _ => None,
        }
    }

    /// The role a glyph that fills is drawn in.
    fn filled_in(self) -> Option<R> {
        match self {
            Self::Block { role, .. } | Self::Wedge { role, .. } => Some(role),
            Self::Halves(halves) => halves.lowest(),
            _ => None,
        }
    }

    /// The role `over`, drawn on this cell, keeps behind it: this cell's, when it fills at least half of what `over` leaves.
    fn behind(self, over: Self) -> Option<R> {
        self.cover().filter(|cover| cover.shows_behind(over.cover())).and(self.filled_in())
    }

    fn fills_cell(self) -> bool {
        matches!(self.cover(), Some(cover) if cover.parts >= Cover::WHOLE)
    }
}

/// A grid of cells to draw text, lines, frames, shapes, tips and finer than a cell on, read back as rows of styled runs.
///
/// Drawing out of bounds is clipped; the last drawing on a cell wins, except that lines merge into junctions
/// and a block filling at least half of what a drawing leaves of its cell stays behind it, as the cell's background.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Canvas<R> {
    width: u16,
    height: u16,
    cells: Vec<Cell<R>>,
    /// The role behind each cell's glyph.
    grounds: Vec<Option<R>>,
}

impl<R: Role> Canvas<R> {
    /// An empty canvas, `width` columns by `height` rows, at most [`MAX_WIDTH`] columns and [`MAX_CELLS`] cells.
    pub fn new(width: u16, height: u16) -> Result<Self, Error> {
        let cells = usize::from(width) * usize::from(height);
        if width > MAX_WIDTH || cells > MAX_CELLS {
            return Err(Error::TooLarge { width, height });
        }
        Ok(Self { width, height, cells: vec![Cell::Empty; cells], grounds: vec![None; cells] })
    }

    /// Columns, as given to [`new`](Self::new).
    pub const fn width(&self) -> u16 {
        self.width
    }

    /// Rows, as given to [`new`](Self::new).
    pub const fn height(&self) -> u16 {
        self.height
    }

    /// Writes `text` on one row from `at`, cut at the right edge; see [`text_width`](crate::text_width) for what it leaves out.
    pub fn text(&mut self, at: Point, text: &str, role: R) {
        self.write(at, text, self.width, role);
    }

    /// Writes `text` like [`text`](Self::text), cut to `max_width` columns, its last column an ellipsis when cut.
    ///
    /// The right edge of the canvas cuts it the same way, so text running off the canvas still ends on an ellipsis.
    pub fn text_cut(&mut self, at: Point, text: &str, max_width: u16, role: R) {
        let max_width = max_width.min(self.width.saturating_sub(at.x));
        if text::text_width(text) <= usize::from(max_width) {
            self.write(at, text, at.x + max_width, role);
        } else if max_width > 0 {
            let end = self.write(at, text, at.x + max_width - 1, role);
            self.put(Point::new(end, at.y), Cell::Ellipsis { role });
        }
    }

    /// Each row as runs of text sharing a role and a background, every row exactly as wide as the canvas and without `\n`.
    pub fn rows(&self, glyphs: GlyphSet) -> impl ExactSizeIterator<Item = Vec<Run<R>>> {
        let width = usize::from(self.width);
        (0..usize::from(self.height)).map(move |y| {
            let row = y * width..(y + 1) * width;
            runs(&self.cells[row.clone()], &self.grounds[row], glyphs)
        })
    }

    /// Each row as its text alone, without `\n`: [`rows`](Self::rows) for a caller that draws no colour.
    pub fn lines(&self, glyphs: GlyphSet) -> impl ExactSizeIterator<Item = String> {
        self.rows(glyphs).map(|runs| runs.into_iter().map(|run| run.text).collect())
    }

    /// Writes `text` from `at` up to column `end`, excluded, and returns the column after the last character.
    ///
    /// `end` stops at the right edge, so a wide character on the last column is left out rather than drawn without its right half.
    fn write(&mut self, at: Point, text: &str, end: u16, role: R) -> u16 {
        let end = end.min(self.width);
        let mut x = at.x;
        for letter in text::letters(text) {
            if usize::from(x) + usize::from(letter.width) > usize::from(end) {
                break;
            }
            self.put(Point::new(x, at.y), Cell::Text { char: letter.char, role });
            if letter.width == 2 {
                self.put(Point::new(x + 1, at.y), Cell::WideTail);
            }
            x += u16::from(letter.width);
        }
        x
    }

    /// Writing over either half of a wide character blanks the other half.
    fn put(&mut self, at: Point, cell: Cell<R>) {
        let Some(index) = self.index(at) else { return };
        if self.cells[index] == Cell::WideTail {
            self.cells[index - 1] = Cell::Empty;
        }
        if at.x + 1 < self.width && self.cells[index + 1] == Cell::WideTail {
            self.cells[index + 1] = Cell::Empty;
        }
        self.grounds[index] = if cell.fills_cell() { None } else { self.cells[index].behind(cell).or(self.grounds[index]) };
        self.cells[index] = cell;
    }

    fn index(&self, at: Point) -> Option<usize> {
        (at.x < self.width && at.y < self.height).then(|| usize::from(at.y) * usize::from(self.width) + usize::from(at.x))
    }
}

/// A cell as it is written: its character, the role it is drawn in and the role behind it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Styled<R> {
    pub(crate) char: char,
    pub(crate) role: R,
    pub(crate) background: Option<R>,
}

fn runs<R: Role>(cells: &[Cell<R>], grounds: &[Option<R>], glyphs: GlyphSet) -> Vec<Run<R>> {
    let mut runs: Vec<Run<R>> = Vec::new();
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
fn styled<R: Role>(cell: Cell<R>, ground: Option<R>, glyphs: GlyphSet) -> Option<Styled<R>> {
    let (char, role) = match cell {
        Cell::Empty => (' ', R::default()),
        Cell::WideTail => return None,
        Cell::Text { char, role } => (char, role),
        Cell::Line { sides, rounded, role } => (glyphs.line(sides, rounded), role),
        Cell::Ellipsis { role } => (glyphs.ellipsis(), role),
        Cell::Mark { mark, role } => (mark.glyph(glyphs), role),
        Cell::Tip { tip, direction, role } => (tip.glyph(direction, glyphs), role),
        Cell::Symbol { symbol, role } => (symbol.glyph(glyphs), role),
        Cell::Block { block, role } => return Some(block.styled(role, ground, glyphs)),
        Cell::Wedge { wedge, role } => return Some(wedge.styled(role, ground, glyphs)),
        Cell::Stroke { stroke, role } => (stroke.glyph(glyphs), role),
        Cell::Dots { dots, lit, role } => (dots.glyph(lit, role.texture(), glyphs), role),
        Cell::Halves(halves) => return Some(halves.styled(ground, glyphs)),
    };
    Some(Styled { char, role, background: ground })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::line::LineKind;
    use crate::role::tests::Part;

    /// Also checks the invariant every drawing keeps: each row is exactly as wide as the canvas.
    pub(crate) fn render(canvas: &Canvas<Part>, glyphs: GlyphSet) -> String {
        let rows: Vec<String> = canvas.lines(glyphs).collect();
        for row in &rows {
            assert_eq!(crate::text_width(row), usize::from(canvas.width()), "row {row:?}");
        }
        rows.join("\n")
    }

    pub(crate) fn canvas(width: u16, height: u16) -> Canvas<Part> {
        Canvas::new(width, height).unwrap()
    }

    pub(crate) fn line(canvas: &mut Canvas<Part>, from: (u16, u16), to: (u16, u16), kind: LineKind) {
        line_between(canvas, Point::new(from.0, from.1), Point::new(to.0, to.1), kind);
    }

    /// The line between two points sharing a row or a column.
    pub(crate) fn line_between(canvas: &mut Canvas<Part>, from: Point, to: Point, kind: LineKind) {
        if from.y == to.y {
            canvas.horizontal(from.y, from.x, to.x, kind, Part::Edge);
        } else {
            assert_eq!(from.x, to.x, "a diagonal from {from:?} to {to:?}");
            canvas.vertical(from.x, from.y, to.y, kind, Part::Edge);
        }
    }

    fn tallest_full_width() -> u16 {
        u16::try_from(MAX_CELLS / usize::from(MAX_WIDTH)).unwrap()
    }

    /// The memory `ARCHITECTURE.md` states for a canvas at the cell limit.
    #[test]
    fn a_cell_and_its_background_take_ten_bytes() {
        assert_eq!((size_of::<Cell<Part>>(), size_of::<Option<Part>>()), (8, 2));
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
            Canvas::<Part>::new(MAX_WIDTH, tallest_full_width() + 1),
            Err(Error::TooLarge { width: MAX_WIDTH, height: tallest_full_width() + 1 })
        );
    }

    #[test]
    fn a_canvas_over_the_width_limit_is_refused() {
        assert_eq!(Canvas::<Part>::new(MAX_WIDTH + 1, 1), Err(Error::TooLarge { width: MAX_WIDTH + 1, height: 1 }));
    }

    #[test]
    fn text_is_cut_at_the_right_edge() {
        let mut canvas = canvas(5, 1);

        canvas.text(Point::new(2, 0), "abcdef", Part::Label);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "  abc");
    }

    #[test]
    fn drawing_outside_the_canvas_draws_nothing() {
        let mut canvas = canvas(3, 2);

        canvas.text(Point::new(0, 2), "below", Part::Label);
        canvas.text(Point::new(3, 0), "right", Part::Label);
        canvas.tip(Point::new(u16::MAX, u16::MAX), Direction::Up, Tip::Arrow, Part::Edge);
        line(&mut canvas, (5, 0), (5, 9), LineKind::Light);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "   \n   ");
    }

    #[test]
    fn a_wide_character_takes_two_cells() {
        let mut canvas = canvas(5, 1);

        canvas.text(Point::new(0, 0), "漢x", Part::Label);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "漢x  ");
    }

    #[test]
    fn a_wide_character_without_room_for_both_halves_is_not_drawn() {
        let mut canvas = canvas(3, 1);

        canvas.text(Point::new(1, 0), "a漢", Part::Label);

        assert_eq!(render(&canvas, GlyphSet::Unicode), " a ");
    }

    #[test]
    fn overwriting_the_left_half_of_a_wide_character_blanks_the_right_half() {
        let mut canvas = canvas(4, 1);
        canvas.text(Point::new(1, 0), "漢", Part::Label);

        canvas.text(Point::new(1, 0), "a", Part::Label);

        assert_eq!(render(&canvas, GlyphSet::Unicode), " a  ");
    }

    #[test]
    fn overwriting_the_right_half_of_a_wide_character_blanks_the_left_half() {
        let mut canvas = canvas(4, 1);
        canvas.text(Point::new(1, 0), "漢", Part::Label);

        canvas.tip(Point::new(2, 0), Direction::Left, Tip::Arrow, Part::Edge);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "  ◂ ");
    }

    #[test]
    fn a_wide_character_over_two_others_blanks_their_outer_halves() {
        let mut canvas = canvas(6, 1);
        canvas.text(Point::new(0, 0), "漢字", Part::Label);

        canvas.text(Point::new(1, 0), "中", Part::Label);

        assert_eq!(render(&canvas, GlyphSet::Unicode), " 中   ");
    }

    #[test]
    fn errors_say_what_was_refused() {
        let too_large = Error::TooLarge { width: 2000, height: 1 };
        let diagonal = Error::Diagonal { from: Point::new(0, 0), to: Point::new(2, 3) };

        assert_eq!(
            [too_large, diagonal].map(|error| error.to_string()),
            [
                "a 2000x1 canvas is over the limit of 1000 columns or 1000000 cells",
                "a path's step from (0, 0) to (2, 3) is neither horizontal nor vertical"
            ]
        );
    }

    #[test]
    fn text_cut_keeps_text_that_fits() {
        let mut canvas = canvas(6, 1);

        canvas.text_cut(Point::new(0, 0), "abcd", 4, Part::Label);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "abcd  ");
    }

    #[test]
    fn text_cut_ends_cut_text_on_an_ellipsis() {
        let mut canvas = canvas(6, 1);

        canvas.text_cut(Point::new(0, 0), "abcdef", 4, Part::Label);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "abc⋯  ");
        assert_eq!(render(&canvas, GlyphSet::Ascii), "abc~  ");
    }

    #[test]
    fn text_cut_puts_the_ellipsis_after_a_wide_character_that_fits() {
        let mut canvas = canvas(5, 1);

        canvas.text_cut(Point::new(0, 0), "a漢字", 4, Part::Label);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "a漢⋯ ");
    }

    #[test]
    fn text_cut_to_no_width_draws_nothing() {
        let mut canvas = canvas(2, 1);

        canvas.text_cut(Point::new(0, 0), "ab", 0, Part::Label);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "  ");
    }

    #[test]
    fn text_cut_by_the_right_edge_ends_on_an_ellipsis() {
        let mut canvas = canvas(5, 1);

        canvas.text_cut(Point::new(2, 0), "abcdef", 10, Part::Label);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "  ab⋯");
    }

    #[test]
    fn text_cut_by_the_right_edge_leaves_out_a_wide_character_on_the_last_column() {
        let mut canvas = canvas(3, 1);

        canvas.text_cut(Point::new(2, 0), "漢", 5, Part::Label);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "  ⋯");
    }

    #[test]
    fn rows_group_cells_by_role_without_newlines() {
        let mut canvas = canvas(6, 1);
        canvas.text(Point::new(1, 0), "ab", Part::Label);
        line(&mut canvas, (3, 0), (4, 0), LineKind::Light);

        let runs: Vec<Run<Part>> = canvas.rows(GlyphSet::Unicode).flatten().collect();

        let run = |text: &str, role| Run { text: text.to_owned(), role, background: None };
        assert_eq!(runs, [run(" ", Part::Plain), run("ab", Part::Label), run("──", Part::Edge), run(" ", Part::Plain)]);
    }

    /// Each run as its text, its role and its background, so a test sees which role a cell kept.
    pub(crate) fn styles(canvas: &Canvas<Part>, glyphs: GlyphSet) -> Vec<String> {
        canvas
            .rows(glyphs)
            .map(|runs| runs.into_iter().map(|run| format!("[{} {:?} on {:?}]", run.text, run.role, run.background)).collect())
            .collect()
    }

    mod properties {
        use std::ops::Range;

        use proptest::prelude::*;
        use unicode_width::UnicodeWidthChar;

        use super::*;
        use crate::frame::Corner;
        use crate::geometry::Rect;
        use crate::pixel::Pixel;
        use crate::shape::Shape;
        use crate::wedge::Fill;

        /// Points reach past the canvas, so every property also holds while clipping.
        const REACH: u16 = 24;

        fn point() -> impl Strategy<Value = Point> {
            (0..REACH, 0..REACH).prop_map(|(x, y)| Point::new(x, y))
        }

        fn kind() -> impl Strategy<Value = LineKind> {
            prop::sample::select(LineKind::ALL)
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

        fn dots() -> impl Strategy<Value = Dots> {
            prop::sample::select(Dots::ALL)
        }

        /// Dots and eighths reach past the canvas too, as far as `u32::MAX`.
        fn fine() -> impl Strategy<Value = u32> {
            prop_oneof![9 => 0..u32::from(REACH) * 8, 1 => Just(u32::MAX)]
        }

        /// A drawing on whole cells other than text, lines and frames.
        #[derive(Debug, Clone)]
        enum Figure {
            Shape(Rect, Shape),
            Path(Vec<Point>, LineKind),
            Tip(Point, Direction, Tip),
            Symbol(Point, Symbol),
        }

        fn figure() -> impl Strategy<Value = Figure> {
            let direction = prop::sample::select(Direction::ALL.to_vec());
            prop_oneof![
                (point(), point(), prop::sample::select(Shape::ALL))
                    .prop_map(|(at, size, shape)| Figure::Shape(Rect::new(at.x, at.y, size.x, size.y), shape)),
                (path(), kind()).prop_map(|(points, kind)| Figure::Path(points, kind)),
                (point(), direction, prop::sample::select(Tip::ALL)).prop_map(|(at, direction, tip)| Figure::Tip(at, direction, tip)),
                (point(), prop::sample::select(Symbol::ALL)).prop_map(|(at, symbol)| Figure::Symbol(at, symbol)),
            ]
        }

        fn draw_figure(canvas: &mut Canvas<Part>, figure: Figure) {
            match figure {
                Figure::Shape(rect, shape) => canvas.shape(rect, shape, Part::Node),
                Figure::Path(points, kind) => canvas.path(&points, kind, Part::Edge).unwrap(),
                Figure::Tip(at, direction, tip) => canvas.tip(at, direction, tip, Part::Edge),
                Figure::Symbol(at, symbol) => canvas.symbol(at, symbol, Part::Node),
            }
        }

        #[derive(Debug, Clone)]
        enum Fine {
            Column(u16, Range<u32>),
            Row(u16, Range<u32>),
            Stroke(u16, u32),
            Wedge(Point, Wedge),
            Segment(Pixel, Pixel, Dots),
        }

        fn finer_than_a_cell() -> impl Strategy<Value = Fine> {
            let fill = prop_oneof![Just(Fill::Below), Just(Fill::Above)];
            let pixel = || (fine(), fine()).prop_map(|(x, y)| Pixel::new(x, y));
            prop_oneof![
                (0..REACH, fine(), fine()).prop_map(|(x, from, to)| Fine::Column(x, from..to)),
                (0..REACH, fine(), fine()).prop_map(|(y, from, to)| Fine::Row(y, from..to)),
                (0..REACH, fine()).prop_map(|(x, eighth)| Fine::Stroke(x, eighth)),
                (point(), 0..5u8, 0..5u8, fill).prop_map(|(at, left, right, fill)| Fine::Wedge(at, Wedge { left, right, fill })),
                (pixel(), pixel(), dots()).prop_map(|(from, to, dots)| Fine::Segment(from, to, dots)),
            ]
        }

        fn draw(canvas: &mut Canvas<Part>, fine: Fine, role: Part) {
            match fine {
                Fine::Column(x, eighths) => canvas.column(x, eighths, role),
                Fine::Row(y, eighths) => canvas.row(y, eighths, role),
                Fine::Stroke(x, eighth) => canvas.stroke(x, eighth, role),
                Fine::Wedge(at, wedge) => canvas.wedge(at, wedge, role),
                Fine::Segment(from, to, dots) => canvas.segment(from, to, dots, role),
            }
        }

        proptest! {
            #[test]
            fn every_row_is_as_wide_as_the_canvas(
                (width, height) in (0..16u16, 0..16u16),
                texts in prop::collection::vec((point(), "[a漢\u{301}\u{200b}\t\u{1b} ]{0,8}", 0..12u16), 0..6),
                lines in prop::collection::vec((straight(), kind()), 0..6),
                frames in prop::collection::vec((point(), point(), kind()), 0..3),
                figures in prop::collection::vec(figure(), 0..6),
                fines in prop::collection::vec((finer_than_a_cell(), 0..3u8), 0..8),
                glyphs in prop_oneof![Just(GlyphSet::Unicode), Just(GlyphSet::Unicode13), Just(GlyphSet::Unicode3), Just(GlyphSet::Ascii)],
            ) {
                let mut canvas = canvas(width, height);
                for (fine, series) in fines.iter().take(4).cloned() {
                    draw(&mut canvas, fine, Part::Series(series));
                }
                for ((from, to), kind) in lines {
                    line_between(&mut canvas, from, to, kind);
                }
                for (corner, size, kind) in frames {
                    canvas.frame(Rect::new(corner.x, corner.y, size.x, size.y), kind, Corner::Rounded, Part::Node);
                }
                for figure in figures {
                    draw_figure(&mut canvas, figure);
                }
                for (at, text, max_width) in &texts {
                    canvas.text_cut(*at, text, *max_width, Part::Label);
                }
                for (fine, series) in fines.into_iter().skip(4) {
                    draw(&mut canvas, fine, Part::Series(series));
                }

                for row in canvas.lines(glyphs) {
                    let columns: usize = row.chars().map(|char| char.width().unwrap_or(0)).sum();
                    prop_assert_eq!(columns, usize::from(width), "row {:?}", row);
                }
            }

            #[test]
            fn a_whole_block_covers_whatever_was_drawn(fines in prop::collection::vec(finer_than_a_cell(), 0..8), text in "[a漢 ]{0,8}") {
                let mut canvas = canvas(8, 2);
                canvas.text(Point::new(0, 0), &text, Part::Label);
                for fine in fines {
                    draw(&mut canvas, fine, Part::Series(1));
                }

                canvas.row(0, 0..64, Part::Series(0));
                canvas.row(1, 0..64, Part::Series(0));

                let whole = || vec![Run { text: "█".repeat(8), role: Part::Series(0), background: None }];
                prop_assert_eq!(canvas.rows(GlyphSet::Unicode).collect::<Vec<_>>(), [whole(), whole()]);
            }

            #[test]
            fn a_tip_or_a_symbol_stays_whole_under_the_lines_drawn_after_it(
                (at, direction) in (point(), prop::sample::select(Direction::ALL.to_vec())),
                tip in prop::sample::select(Tip::ALL).prop_filter("a bar is a line", |tip| *tip != Tip::Bar),
                symbol in prop::sample::select(Symbol::ALL),
                lines in prop::collection::vec(path(), 1..4),
            ) {
                let mut tipped = canvas(16, 16);
                let mut marked = canvas(16, 16);
                tipped.tip(at, direction, tip, Part::Edge);
                marked.symbol(at, symbol, Part::Node);
                let (tip_alone, symbol_alone) = (tipped.clone(), marked.clone());

                for points in lines {
                    tipped.path(&points, LineKind::Heavy, Part::Edge).unwrap();
                    marked.path(&points, LineKind::Heavy, Part::Edge).unwrap();
                }

                let cell = |canvas: &Canvas<Part>| canvas.index(at).map(|index| canvas.cells[index]);
                prop_assert_eq!(cell(&tipped), cell(&tip_alone));
                prop_assert_eq!(cell(&marked), cell(&symbol_alone));
            }

            #[test]
            fn a_line_draws_the_same_from_either_end(((from, to), kind) in (straight(), kind())) {
                let mut forward = canvas(16, 16);
                let mut backward = canvas(16, 16);

                line_between(&mut forward, from, to, kind);
                line_between(&mut backward, to, from, kind);

                prop_assert_eq!(forward, backward);
            }

            #[test]
            fn drawing_a_line_twice_changes_nothing(((from, to), kind) in (straight(), kind()), path in path()) {
                let mut once = canvas(16, 16);
                once.path(&path, LineKind::Light, Part::Edge).unwrap();
                line_between(&mut once, from, to, kind);
                let mut twice = once.clone();

                line_between(&mut twice, from, to, kind);

                prop_assert_eq!(once, twice);
            }

            #[test]
            fn a_path_draws_a_line_from_each_point_to_the_next(points in path(), kind in kind()) {
                let mut path = canvas(16, 16);
                let mut lines = canvas(16, 16);

                path.path(&points, kind, Part::Edge).unwrap();
                for pair in points.windows(2) {
                    line_between(&mut lines, pair[0], pair[1], kind);
                }

                prop_assert_eq!(path, lines);
            }
        }
    }
}
