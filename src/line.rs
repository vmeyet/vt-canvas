use crate::geometry::Direction;

/// How a line is drawn.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineKind {
    /// `─ │`
    Light,
    /// `━ ┃`
    Heavy,
    /// `═ ║`
    Double,
    /// `╌ ╎`, light where it meets another line.
    Dashed,
}

impl LineKind {
    #[cfg(test)]
    pub(crate) const ALL: &[Self] = &[Self::Light, Self::Heavy, Self::Double, Self::Dashed];
}

/// Which sides of a cell a line leaves through, and in which kind.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Sides([Option<LineKind>; 4]);

impl Sides {
    pub(crate) fn get(self, side: Direction) -> Option<LineKind> {
        self.0[side as usize]
    }

    pub(crate) fn with(self, side: Direction, kind: Option<LineKind>) -> Self {
        let mut kinds = self.0;
        kinds[side as usize] = kind;
        Self(kinds)
    }

    /// The sides `other` sets replace those of `self`: the last line drawn wins.
    pub(crate) fn merge(self, other: Self) -> Self {
        Self(std::array::from_fn(|side| other.0[side].or(self.0[side])))
    }

    pub(crate) fn map(self, f: impl Fn(Direction, LineKind) -> LineKind) -> Self {
        Self(std::array::from_fn(|side| self.0[side].map(|kind| f(Direction::ALL[side], kind))))
    }

    pub(crate) fn present(self) -> impl Iterator<Item = (Direction, LineKind)> {
        Direction::ALL.into_iter().filter_map(move |side| self.get(side).map(|kind| (side, kind)))
    }

    pub(crate) fn count(self) -> usize {
        self.present().count()
    }

    pub(crate) fn has_horizontal(self) -> bool {
        self.present().any(|(side, _)| side.is_horizontal())
    }

    pub(crate) fn has_vertical(self) -> bool {
        self.present().any(|(side, _)| !side.is_horizontal())
    }

    pub(crate) fn all(self, kind: LineKind) -> bool {
        self.present().all(|(_, present)| present == kind)
    }

    pub(crate) fn any(self, kind: LineKind) -> bool {
        self.present().any(|(_, present)| present == kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merging_keeps_the_sides_only_the_first_has() {
        let first = Sides::default().with(Direction::Up, Some(LineKind::Light));
        let second = Sides::default().with(Direction::Left, Some(LineKind::Heavy));

        let merged = first.merge(second);

        assert_eq!(merged.get(Direction::Up), Some(LineKind::Light));
        assert_eq!(merged.get(Direction::Left), Some(LineKind::Heavy));
    }

    #[test]
    fn merging_gives_a_side_drawn_twice_the_last_kind() {
        let first = Sides::default().with(Direction::Right, Some(LineKind::Light));
        let second = Sides::default().with(Direction::Right, Some(LineKind::Double));

        assert_eq!(first.merge(second).get(Direction::Right), Some(LineKind::Double));
    }
}
