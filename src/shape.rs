use crate::canvas::Corner;
use crate::geometry::{Point, Rect};

/// The outline [`Canvas::shape`](crate::Canvas::shape) draws around a rectangle, named after Mermaid's flowchart shapes.
///
/// Every outline keeps a glyph on each cell of the rectangle's border, so a line ending on any side of it touches it.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shape {
    /// `┌─┐ │ │ └─┘`
    Rectangle,
    /// `╭─╮ │ │ ╰─╯`
    Rounded,
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
    pub side: u16,
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

/// A mark and the cell it goes on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Placed {
    pub(crate) at: Point,
    pub(crate) mark: Mark,
}

impl Shape {
    /// Every shape, in the order the documentation lists them.
    pub const ALL: [Self; 14] = [
        Self::Rectangle,
        Self::Rounded,
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
        let side = if matches!(self, Self::Circle | Self::DoubleCircle | Self::Subroutine) { 2 } else { 1 };
        let top = if self == Self::Cylinder { 2 } else { 1 };
        Inset { side, top, bottom: 1 }
    }

    pub(crate) fn corner(self) -> Corner {
        match self {
            Self::Rounded | Self::Stadium | Self::Circle | Self::DoubleCircle | Self::Cylinder => Corner::Rounded,
            _ => Corner::Square,
        }
    }

    /// Lines inside the border: down each side of a subroutine, under the top of a cylinder.
    pub(crate) fn inner_lines(self, rect: Rect) -> Vec<[Point; 2]> {
        let edges = Edges::of(rect);
        match self {
            Self::Subroutine if rect.width > 2 => vec![
                [Point::new(edges.left + 1, edges.top), Point::new(edges.left + 1, edges.bottom)],
                [Point::new(edges.right - 1, edges.top), Point::new(edges.right - 1, edges.bottom)],
            ],
            Self::Cylinder if rect.height > 2 => vec![[Point::new(edges.left, edges.top + 1), Point::new(edges.right, edges.top + 1)]],
            _ => Vec::new(),
        }
    }

    /// The marks replacing border cells, with a double circle's second bracket.
    pub(crate) fn marks(self, rect: Rect) -> Vec<Placed> {
        let edges = Edges::of(rect);
        let inner = (self == Self::DoubleCircle && rect.width > 2).then(|| Edges { left: edges.left + 1, right: edges.right - 1, ..edges });
        (0..rect.height)
            .flat_map(|row| {
                let y = edges.top.saturating_add(row);
                let outer = [
                    self.left(row, rect.height).map(|mark| Placed { at: Point::new(edges.left, y), mark }),
                    self.right(row, rect.height).map(|mark| Placed { at: Point::new(edges.right, y), mark }),
                ];
                let second = inner.filter(|_| is_inside(row, rect.height)).map(|inner| {
                    [
                        Placed { at: Point::new(inner.left, y), mark: Mark::LeftArc },
                        Placed { at: Point::new(inner.right, y), mark: Mark::RightArc },
                    ]
                });
                outer.into_iter().flatten().chain(second.into_iter().flatten())
            })
            .collect()
    }

    /// The mark on the left border at `row` of `height`, `None` where the border is a line.
    fn left(self, row: u16, height: u16) -> Option<Mark> {
        match self {
            Self::Rectangle | Self::Rounded | Self::Subroutine | Self::Cylinder => None,
            Self::Stadium | Self::Circle | Self::DoubleCircle => is_inside(row, height).then_some(Mark::LeftArc),
            Self::Diamond if row == 0 => Some(Mark::Rising),
            Self::Diamond if row + 1 == height => Some(Mark::Falling),
            Self::Diamond => None,
            Self::Hexagon => Some(pointed(row, height, [Mark::Rising, Mark::LeftPoint, Mark::Falling])),
            Self::Asymmetric => Some(pointed(row, height, [Mark::Falling, Mark::RightPoint, Mark::Rising])),
            Self::LeanRight | Self::Trapezoid => Some(Mark::Rising),
            Self::LeanLeft | Self::InvertedTrapezoid => Some(Mark::Falling),
        }
    }

    /// The left border mirrored, except where the shape is not symmetric.
    fn right(self, row: u16, height: u16) -> Option<Mark> {
        match self {
            Self::LeanRight => Some(Mark::Rising),
            Self::LeanLeft => Some(Mark::Falling),
            Self::Asymmetric => None,
            _ => self.left(row, height).map(Mark::mirrored),
        }
    }
}

impl Mark {
    const fn mirrored(self) -> Self {
        match self {
            Self::Rising => Self::Falling,
            Self::Falling => Self::Rising,
            Self::LeftArc => Self::RightArc,
            Self::RightArc => Self::LeftArc,
            Self::LeftPoint => Self::RightPoint,
            Self::RightPoint => Self::LeftPoint,
        }
    }
}

/// The columns and rows of a rectangle's border.
#[derive(Debug, Clone, Copy)]
struct Edges {
    left: u16,
    top: u16,
    right: u16,
    bottom: u16,
}

impl Edges {
    fn of(rect: Rect) -> Self {
        Self {
            left: rect.x,
            top: rect.y,
            right: rect.x.saturating_add(rect.width.saturating_sub(1)),
            bottom: rect.y.saturating_add(rect.height.saturating_sub(1)),
        }
    }
}

/// Between the top and bottom rows.
fn is_inside(row: u16, height: u16) -> bool {
    row > 0 && row + 1 < height
}

/// `[upper, middle, lower]`: the side slants out to a point at its middle row, between two rows when there is none.
fn pointed(row: u16, height: u16, [upper, middle, lower]: [Mark; 3]) -> Mark {
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
    use crate::{Canvas, Direction, GlyphSet, LineKind, Role};

    fn render(canvas: &Canvas, glyphs: GlyphSet) -> String {
        canvas.rows(glyphs).map(|runs| runs.into_iter().map(|run| run.text).collect::<String>()).collect::<Vec<_>>().join("\n")
    }

    /// The shape around a `Label` with a space on each side, as a flowchart node.
    fn node(shape: Shape, label_rows: u16) -> Canvas {
        let inset = shape.inset();
        let (width, height) = (5 + 2 + 2 * inset.side, label_rows + inset.top + inset.bottom);
        let mut canvas = Canvas::new(width, height).unwrap();
        canvas.shape(Rect::new(0, 0, width, height), shape, Role::Node);
        for row in 0..label_rows {
            canvas.text(Point::new(inset.side + 1, inset.top + row), "Label", Role::Label);
        }
        canvas
    }

    /// Each shape with a line joining the middle of each side, the left and right ones ending on the border.
    fn joined(shape: Shape) -> Canvas {
        let inset = shape.inset();
        let rect = Rect::new(3, 2, 5 + 2 + 2 * inset.side, 1 + inset.top + inset.bottom);
        let (middle_x, middle_y) = (rect.x + rect.width / 2, rect.y + inset.top);
        let (right, bottom) = (rect.x + rect.width - 1, rect.y + rect.height - 1);
        let mut canvas = Canvas::new(rect.width + 6, rect.height + 4).unwrap();
        canvas.shape(rect, shape, Role::Node);
        for (from, to) in [
            (Point::new(middle_x, 0), Point::new(middle_x, rect.y)),
            (Point::new(middle_x, bottom + 2), Point::new(middle_x, bottom)),
            (Point::new(0, middle_y), Point::new(rect.x, middle_y)),
            (Point::new(right + 3, middle_y), Point::new(right, middle_y)),
        ] {
            canvas.line(from, to, LineKind::Light, Role::Edge).unwrap();
        }
        canvas
    }

    fn every_shape(draw: impl Fn(Shape) -> Canvas, glyphs: GlyphSet) -> String {
        Shape::ALL.iter().map(|&shape| format!("{shape:?}\n{}", render(&draw(shape), glyphs))).collect::<Vec<_>>().join("\n\n")
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
        let mut canvas = Canvas::new(8, 3).unwrap();
        canvas.shape(Rect::new(3, 0, 5, 3), Shape::Hexagon, Role::Node);

        canvas.line(Point::new(0, 1), Point::new(3, 1), LineKind::Light, Role::Edge).unwrap();
        canvas.arrow(Point::new(2, 1), Direction::Right, Role::Edge);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "   ╱───╲\n──▸<   >\n   ╲───╱");
    }

    #[test]
    fn every_shape_keeps_a_glyph_on_each_border_cell() {
        for shape in Shape::ALL {
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
        let mut canvas = Canvas::new(2, 2).unwrap();

        canvas.shape(Rect::new(0, 0, 0, 2), Shape::Hexagon, Role::Node);
        canvas.shape(Rect::new(0, 0, 2, 0), Shape::DoubleCircle, Role::Node);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "  \n  ");
    }

    #[test]
    fn a_shape_leaving_the_canvas_is_clipped() {
        let mut canvas = Canvas::new(3, 2).unwrap();

        canvas.shape(Rect::new(1, 1, u16::MAX, u16::MAX), Shape::Trapezoid, Role::Node);

        assert_eq!(render(&canvas, GlyphSet::Ascii), "   \n /-");
    }
}
