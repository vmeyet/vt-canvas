use crate::frame::Corner;
use crate::geometry::{Border, Point};
use crate::glyph::GlyphSet;

/// The outline [`Canvas::shape`](crate::Canvas::shape) draws around a rectangle, named after Mermaid's flowchart shapes.
///
/// Every outline keeps a glyph on each cell of the rectangle's border, so a line ending on any side of it touches it.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shape {
    /// `┌─┐ │ │ └─┘`
    Rectangle,
    /// `╭─╮ │ │ ╰─╯`
    RoundedRectangle,
    /// `╭─╮ ( ) ╰─╯`
    Stadium,
    /// A stadium with a blank column inside each bracket: three rows cannot curve more.
    Circle,
    /// `╭──╮ (( )) ╰──╯`
    DoubleCircle,
    /// `┌┬─┬┐ ││ ││ └┴─┴┘`: a rectangle with an inner line down each side.
    Subroutine,
    /// `╭─╮ ├─┤ │ │ ╰─╯`: a rounded rectangle with a line under its top.
    Cylinder,
    /// `╱─╲ │ │ ╲─╱`
    Diamond,
    /// `╱─╲ < > ╲─╱`: each side points out at its middle.
    Hexagon,
    /// `╱─╱ ╱ ╱ ╱─╱`
    LeanRight,
    /// `╲─╲ ╲ ╲ ╲─╲`
    LeanLeft,
    /// `╱─╲ ╱ ╲ ╱─╲`: wider at the bottom.
    Trapezoid,
    /// `╲─╱ ╲ ╱ ╲─╱`: wider at the top.
    InvertedTrapezoid,
    /// `╲─┐ > │ ╱─┘`: a flag notched on the left.
    Asymmetric,
}

/// What an outline takes from the edges of its rectangle; what is left is free for its content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Inset {
    /// Columns on the left, and as many on the right.
    pub sides: u16,
    /// Rows at the top.
    pub top: u16,
    /// Rows at the bottom.
    pub bottom: u16,
}

/// A cell of an outline that is not a line: lines ending on it leave it whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mark {
    /// `╱`
    Rising,
    /// `╲`
    Falling,
    /// `(`
    LeftArc,
    /// `)`
    RightArc,
    /// `<`
    LeftPoint,
    /// `>`
    RightPoint,
}

impl Mark {
    /// Brackets and points are ASCII in every set; slants are `/ \` in ASCII.
    pub(crate) const fn glyph(self, glyphs: GlyphSet) -> char {
        match self {
            Self::Rising => glyphs.pick('╱', '/'),
            Self::Falling => glyphs.pick('╲', '\\'),
            Self::LeftArc => '(',
            Self::RightArc => ')',
            Self::LeftPoint => '<',
            Self::RightPoint => '>',
        }
    }
}

/// A mark and the cell it goes on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Placed {
    pub(crate) at: Point,
    pub(crate) mark: Mark,
}

/// Everything that tells a shape apart: its corners, its inset, its left and right columns and what it draws inside.
#[derive(Debug, Clone, Copy)]
struct Outline {
    corner: Corner,
    inset: Inset,
    left: Side,
    right: Side,
    inner: Inner,
}

/// What an outline draws on its left or right column, row by row; a row without a mark keeps the frame's line.
#[derive(Debug, Clone, Copy)]
enum Side {
    /// The frame's line on every row.
    Line,
    /// `mark` on every row but the top and the bottom one.
    Between(Mark),
    /// `top` on the top row, `bottom` on the bottom one.
    Ends { top: Mark, bottom: Mark },
    /// `upper` above the middle, `middle` on the middle row, `lower` below it: a side pointing at its middle.
    Pointed { upper: Mark, middle: Mark, lower: Mark },
    /// `mark` on every row.
    Every(Mark),
}

/// What an outline draws inside its border.
#[derive(Debug, Clone, Copy)]
enum Inner {
    Nothing,
    /// A line down each side, one column in.
    SideLines,
    /// A line across, one row under the top.
    LineUnderTop,
    /// A second bracket inside each side, between the top and the bottom row.
    SecondBrackets,
}

impl Shape {
    /// Every shape, in the order the documentation lists them.
    pub const ALL: &[Self] = &[
        Self::Rectangle,
        Self::RoundedRectangle,
        Self::Stadium,
        Self::Circle,
        Self::DoubleCircle,
        Self::Subroutine,
        Self::Cylinder,
        Self::Diamond,
        Self::Hexagon,
        Self::LeanRight,
        Self::LeanLeft,
        Self::Trapezoid,
        Self::InvertedTrapezoid,
        Self::Asymmetric,
    ];

    /// The columns and rows the outline takes inside its rectangle.
    pub fn inset(self) -> Inset {
        self.outline().inset
    }

    pub(crate) fn corner(self) -> Corner {
        self.outline().corner
    }

    fn outline(self) -> Outline {
        const THIN: Inset = Inset { sides: 1, top: 1, bottom: 1 };
        const WIDE: Inset = Inset { sides: 2, top: 1, bottom: 1 };
        let square = |left, right| Outline { corner: Corner::Square, inset: THIN, left, right, inner: Inner::Nothing };
        let arcs = Side::Between(Mark::LeftArc);
        let closing_arcs = Side::Between(Mark::RightArc);
        let rounded = Outline { corner: Corner::Rounded, inset: THIN, left: Side::Line, right: Side::Line, inner: Inner::Nothing };
        match self {
            Self::Rectangle => square(Side::Line, Side::Line),
            Self::RoundedRectangle => rounded,
            Self::Stadium => Outline { left: arcs, right: closing_arcs, ..rounded },
            Self::Circle => Outline { inset: WIDE, left: arcs, right: closing_arcs, ..rounded },
            Self::DoubleCircle => Outline { inset: WIDE, left: arcs, right: closing_arcs, inner: Inner::SecondBrackets, ..rounded },
            Self::Subroutine => Outline { inset: WIDE, inner: Inner::SideLines, ..square(Side::Line, Side::Line) },
            Self::Cylinder => Outline { inset: Inset { top: 2, ..THIN }, inner: Inner::LineUnderTop, ..rounded },
            Self::Diamond => {
                square(Side::Ends { top: Mark::Rising, bottom: Mark::Falling }, Side::Ends { top: Mark::Falling, bottom: Mark::Rising })
            }
            Self::Hexagon => square(
                Side::Pointed { upper: Mark::Rising, middle: Mark::LeftPoint, lower: Mark::Falling },
                Side::Pointed { upper: Mark::Falling, middle: Mark::RightPoint, lower: Mark::Rising },
            ),
            Self::LeanRight => square(Side::Every(Mark::Rising), Side::Every(Mark::Rising)),
            Self::LeanLeft => square(Side::Every(Mark::Falling), Side::Every(Mark::Falling)),
            Self::Trapezoid => square(Side::Every(Mark::Rising), Side::Every(Mark::Falling)),
            Self::InvertedTrapezoid => square(Side::Every(Mark::Falling), Side::Every(Mark::Rising)),
            Self::Asymmetric => square(Side::Pointed { upper: Mark::Falling, middle: Mark::RightPoint, lower: Mark::Rising }, Side::Line),
        }
    }

    /// Lines inside the border: down each side of a subroutine, under the top of a cylinder.
    pub(crate) fn inner_lines(self, border: Border) -> Vec<[Point; 2]> {
        let Border { left, top, right, bottom, width, height } = border;
        match self.outline().inner {
            Inner::SideLines if width > 2 => vec![
                [Point::new(left.saturating_add(1), top), Point::new(left.saturating_add(1), bottom)],
                [Point::new(right - 1, top), Point::new(right - 1, bottom)],
            ],
            Inner::LineUnderTop if height > 2 => vec![[Point::new(left, top.saturating_add(1)), Point::new(right, top.saturating_add(1))]],
            Inner::Nothing | Inner::SideLines | Inner::LineUnderTop | Inner::SecondBrackets => Vec::new(),
        }
    }

    /// The marks replacing border cells, with a double circle's second bracket.
    pub(crate) fn marks(self, border: Border) -> Vec<Placed> {
        let outline = self.outline();
        let second = matches!(outline.inner, Inner::SecondBrackets) && border.width > 2;
        (0..border.height)
            .flat_map(|row| {
                let y = border.top.saturating_add(row);
                let on = |x, side: Side| side.mark(row, border.height).map(|mark| Placed { at: Point::new(x, y), mark });
                let outer = [on(border.left, outline.left), on(border.right, outline.right)];
                let inner = second.then(|| {
                    [on(border.left.saturating_add(1), Side::Between(Mark::LeftArc)), on(border.right - 1, Side::Between(Mark::RightArc))]
                });
                outer.into_iter().chain(inner.into_iter().flatten()).flatten()
            })
            .collect()
    }
}

impl Side {
    /// The mark at `row` of a side `height` rows tall, `None` where the side is the frame's line.
    fn mark(self, row: u16, height: u16) -> Option<Mark> {
        let inside = row > 0 && row + 1 < height;
        match self {
            Self::Line => None,
            Self::Between(mark) => inside.then_some(mark),
            Self::Ends { top, .. } if row == 0 => Some(top),
            Self::Ends { bottom, .. } if row + 1 == height => Some(bottom),
            Self::Ends { .. } => None,
            Self::Pointed { upper, middle, lower } => Some(pointed(row, height, [upper, middle, lower])),
            Self::Every(mark) => Some(mark),
        }
    }
}

/// `[upper, middle, lower]`: the side slants out to a point at its middle row, between two rows when there is none.
const fn pointed(row: u16, height: u16, [upper, middle, lower]: [Mark; 3]) -> Mark {
    if height % 2 == 1 && row == height / 2 {
        middle
    } else if row < height / 2 {
        upper
    } else {
        lower
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::role::tests::Part;
    use crate::{Canvas, Direction, GlyphSet, LineKind, Rect, Tip};

    fn render(canvas: &Canvas<Part>, glyphs: GlyphSet) -> String {
        canvas.rows(glyphs).map(|runs| runs.into_iter().map(|run| run.text).collect::<String>()).collect::<Vec<_>>().join("\n")
    }

    /// The shape around a `Label` with a space on each side, as a flowchart node.
    fn node(shape: Shape, label_rows: u16) -> Canvas<Part> {
        let inset = shape.inset();
        let (width, height) = (5 + 2 + 2 * inset.sides, label_rows + inset.top + inset.bottom);
        let mut canvas = Canvas::<Part>::new(width, height).unwrap();
        canvas.shape(Rect::new(0, 0, width, height), shape, Part::Node);
        for row in 0..label_rows {
            canvas.text(Point::new(inset.sides + 1, inset.top + row), "Label", Part::Label);
        }
        canvas
    }

    /// Each shape with a line joining the middle of each side, the left and right ones ending on the border.
    fn joined(shape: Shape) -> Canvas<Part> {
        let inset = shape.inset();
        let rect = Rect::new(3, 2, 5 + 2 + 2 * inset.sides, 1 + inset.top + inset.bottom);
        let (middle_x, middle_y) = (rect.x + rect.width / 2, rect.y + inset.top);
        let (right, bottom) = (rect.x + rect.width - 1, rect.y + rect.height - 1);
        let mut canvas = Canvas::<Part>::new(rect.width + 6, rect.height + 4).unwrap();
        canvas.shape(rect, shape, Part::Node);
        for (from, to) in [
            (Point::new(middle_x, 0), Point::new(middle_x, rect.y)),
            (Point::new(middle_x, bottom + 2), Point::new(middle_x, bottom)),
            (Point::new(0, middle_y), Point::new(rect.x, middle_y)),
            (Point::new(right + 3, middle_y), Point::new(right, middle_y)),
        ] {
            canvas.path(&[from, to], LineKind::Light, Part::Edge).unwrap();
        }
        canvas
    }

    /// Fails to build when a shape is added, so that it is added to `Shape::ALL` too.
    #[test]
    fn every_shape_is_listed_once_in_order() {
        let place = |shape| match shape {
            Shape::Rectangle => 0,
            Shape::RoundedRectangle => 1,
            Shape::Stadium => 2,
            Shape::Circle => 3,
            Shape::DoubleCircle => 4,
            Shape::Subroutine => 5,
            Shape::Cylinder => 6,
            Shape::Diamond => 7,
            Shape::Hexagon => 8,
            Shape::LeanRight => 9,
            Shape::LeanLeft => 10,
            Shape::Trapezoid => 11,
            Shape::InvertedTrapezoid => 12,
            Shape::Asymmetric => 13,
        };

        assert_eq!(Shape::ALL.iter().map(|&shape| place(shape)).collect::<Vec<_>>(), (0..14).collect::<Vec<_>>());
    }

    fn every_shape(draw: impl Fn(Shape) -> Canvas<Part>, glyphs: GlyphSet) -> String {
        Shape::ALL.iter().map(|&shape| format!("{shape:?}\n{}", render(&draw(shape), glyphs))).collect::<Vec<_>>().join("\n\n")
    }

    /// Found by fuzzing: an inner line or a second bracket one cell inside a rectangle at the last column overflowed.
    #[test]
    fn a_shape_past_the_last_cell_draws_nothing() {
        for &shape in Shape::ALL {
            let mut canvas = Canvas::<Part>::new(2, 2).unwrap();

            canvas.shape(Rect::new(u16::MAX, u16::MAX, 5, 5), shape, Part::Node);

            assert_eq!(render(&canvas, GlyphSet::Unicode), "  \n  ", "{shape:?}");
        }
    }

    #[test]
    fn every_shape_draws_in_unicode() {
        insta::assert_snapshot!(every_shape(|shape| node(shape, 1), GlyphSet::Unicode));
    }

    #[test]
    fn every_shape_draws_in_ascii() {
        insta::assert_snapshot!(every_shape(|shape| node(shape, 1), GlyphSet::Ascii));
    }

    #[test]
    fn a_shape_taller_than_three_rows_points_between_its_middle_rows() {
        let shapes = [Shape::Hexagon, Shape::Asymmetric, Shape::Stadium, Shape::DoubleCircle];

        let drawn: Vec<String> = shapes.iter().map(|&shape| render(&node(shape, 2), GlyphSet::Unicode)).collect();

        insta::assert_snapshot!(drawn.join("\n\n"));
    }

    #[test]
    fn lines_join_every_side_of_every_shape_in_unicode() {
        insta::assert_snapshot!(every_shape(joined, GlyphSet::Unicode));
    }

    #[test]
    fn lines_join_every_side_of_every_shape_in_ascii() {
        insta::assert_snapshot!(every_shape(joined, GlyphSet::Ascii));
    }

    #[test]
    fn a_line_ending_on_a_slant_leaves_it_whole() {
        let mut canvas = Canvas::<Part>::new(8, 3).unwrap();
        canvas.shape(Rect::new(3, 0, 5, 3), Shape::Hexagon, Part::Node);

        canvas.horizontal(1, 0, 3, LineKind::Light, Part::Edge);
        canvas.tip(Point::new(2, 1), Direction::Right, Tip::Arrow, Part::Edge);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "   ╱───╲\n──▸<   >\n   ╲───╱");
    }

    #[test]
    fn every_shape_keeps_a_glyph_on_each_border_cell() {
        for &shape in Shape::ALL {
            let canvas = node(shape, 1);
            let rows: Vec<Vec<char>> = render(&canvas, GlyphSet::Unicode).lines().map(|row| row.chars().collect()).collect();
            let (right, bottom) = (rows[0].len() - 1, rows.len() - 1);

            let blank: Vec<(usize, usize)> = (0..=bottom)
                .flat_map(|y| (0..=right).map(move |x| (x, y)))
                .filter(|&(x, y)| (x == 0 || y == 0 || x == right || y == bottom) && rows[y][x] == ' ')
                .collect();

            assert_eq!(blank, [], "{shape:?}");
        }
    }

    #[test]
    fn an_empty_rectangle_draws_nothing() {
        let mut canvas = Canvas::<Part>::new(2, 2).unwrap();

        canvas.shape(Rect::new(0, 0, 0, 2), Shape::Hexagon, Part::Node);
        canvas.shape(Rect::new(0, 0, 2, 0), Shape::DoubleCircle, Part::Node);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "  \n  ");
    }

    #[test]
    fn a_shape_leaving_the_canvas_is_clipped() {
        let mut canvas = Canvas::<Part>::new(3, 2).unwrap();

        canvas.shape(Rect::new(1, 1, u16::MAX, u16::MAX), Shape::Trapezoid, Part::Node);

        assert_eq!(render(&canvas, GlyphSet::Ascii), "   \n /-");
    }
}
