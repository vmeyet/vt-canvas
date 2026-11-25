/// A cell position: `x` counts columns from the left, `y` rows from the top.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point {
    /// Column, from 0 at the left.
    pub x: u16,
    /// Row, from 0 at the top.
    pub y: u16,
}

impl Point {
    /// The cell at column `x`, row `y`.
    pub const fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

/// A rectangle of cells, its top-left corner at `(x, y)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rect {
    /// Column of the left edge.
    pub x: u16,
    /// Row of the top edge.
    pub y: u16,
    /// Columns covered, edges included.
    pub width: u16,
    /// Rows covered, edges included.
    pub height: u16,
}

impl Rect {
    /// The rectangle whose top-left cell is `(x, y)`, `width` columns by `height` rows.
    pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self { x, y, width, height }
    }
}

/// One of the four sides of a cell, also where an arrow points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Towards row 0.
    Up,
    /// Towards higher columns.
    Right,
    /// Towards higher rows.
    Down,
    /// Towards column 0.
    Left,
}

impl Direction {
    pub(crate) const ALL: [Self; 4] = [Self::Up, Self::Right, Self::Down, Self::Left];

    pub(crate) const fn opposite(self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Right => Self::Left,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
        }
    }

    pub(crate) const fn is_horizontal(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }
}
