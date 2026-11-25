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
    /// Every side in `kind`.
    pub(crate) const fn every(kind: LineKind) -> Self {
        Self([Some(kind); 4])
    }

    pub(crate) fn get(self, side: Direction) -> Option<LineKind> {
        self.0[side.index()]
    }

    pub(crate) fn with(self, side: Direction, kind: Option<LineKind>) -> Self {
        let mut kinds = self.0;
        kinds[side.index()] = kind;
        Self(kinds)
    }

    /// The sides `other` sets replace those of `self`: the last line drawn wins.
    pub(crate) fn merge(self, other: Self) -> Self {
        Self(std::array::from_fn(|side| other.0[side].or(self.0[side])))
    }

    pub(crate) fn map(self, f: impl Fn(Direction, LineKind) -> LineKind) -> Self {
        Self(std::array::from_fn(|side| self.0[side].map(|kind| f(Direction::ALL[side], kind))))
    }

    pub(crate) fn drawn(self) -> impl Iterator<Item = (Direction, LineKind)> {
        Direction::ALL.into_iter().filter_map(move |side| self.get(side).map(|kind| (side, kind)))
    }

    pub(crate) fn count(self) -> usize {
        self.drawn().count()
    }

    /// Whether a line enters and leaves through opposite sides.
    pub(crate) fn runs_through(self) -> bool {
        Direction::ALL.into_iter().any(|side| self.get(side).is_some() && self.get(side.opposite()).is_some())
    }

    /// Whether a side of `self` is one `other` lacks.
    pub(crate) fn adds_to(self, other: Self) -> bool {
        self.drawn().any(|(side, _)| other.get(side).is_none())
    }

    pub(crate) fn has_horizontal(self) -> bool {
        self.drawn().any(|(side, _)| side.is_horizontal())
    }

    pub(crate) fn has_vertical(self) -> bool {
        self.drawn().any(|(side, _)| !side.is_horizontal())
    }

    pub(crate) fn all(self, kind: LineKind) -> bool {
        self.drawn().all(|(_, drawn)| drawn == kind)
    }

    pub(crate) fn any(self, kind: LineKind) -> bool {
        self.drawn().any(|(_, drawn)| drawn == kind)
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
