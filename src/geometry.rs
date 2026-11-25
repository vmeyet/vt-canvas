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

    /// Its border, `None` when it covers no cell; a border past the last column or row stops there.
    pub(crate) const fn border(self) -> Option<Border> {
        if self.width == 0 || self.height == 0 {
            return None;
        }
        let (width, height) = (self.width, self.height);
        Some(Border {
            left: self.x,
            top: self.y,
            right: self.x.saturating_add(width - 1),
            bottom: self.y.saturating_add(height - 1),
            width,
            height,
        })
    }
}

/// The columns and rows of a rectangle's border, with its size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Border {
    pub(crate) left: u16,
    pub(crate) top: u16,
    pub(crate) right: u16,
    pub(crate) bottom: u16,
    pub(crate) width: u16,
    pub(crate) height: u16,
}

impl Border {
    /// Clockwise from the top left.
    pub(crate) const fn corners(self) -> [Point; 4] {
        [
            Point::new(self.left, self.top),
            Point::new(self.right, self.top),
            Point::new(self.right, self.bottom),
            Point::new(self.left, self.bottom),
        ]
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
    /// In the order of [`index`](Self::index).
    pub(crate) const ALL: [Self; 4] = [Self::Up, Self::Right, Self::Down, Self::Left];

    /// Its place in [`ALL`](Self::ALL), clockwise from the top.
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Up => 0,
            Self::Right => 1,
            Self::Down => 2,
            Self::Left => 3,
        }
    }

    /// The direction it faces away from.
    pub const fn opposite(self) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_direction_is_at_its_index() {
        assert_eq!(Direction::ALL.map(Direction::index), [0, 1, 2, 3]);
    }
}
