use crate::geometry::Direction;
use crate::glyph::GlyphSet;

/// What an edge draws on one cell at its end, facing the node it goes into: an arrow, a cross, the ends of class and ER relations.
///
/// None of their glyphs is East Asian ambiguous: `○ ◆ ◇ △ ▷ ▶` are, so the set takes the small triangles, the medium
/// diamonds and the letter `o`.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tip {
    /// `▴ ▸ ▾ ◂` pointing at the node, `^ > v <` in ASCII: an arrow head.
    Arrow,
    /// `▵ ▹ ▿ ◃` pointing at the node, `^ > v <` in ASCII: a hollow triangle, as inheritance draws.
    Triangle,
    /// `⬦`, `o` in ASCII: as aggregation draws.
    Diamond,
    /// `⬥`, `#` in ASCII, so it is not read as the multiplicity `*`: as composition draws.
    FilledDiamond,
    /// `o`: a lollipop's ring, or a crow's foot's zero.
    Ring,
    /// A line across the edge, merged with it into `┼`, `+` in ASCII: a crow's foot's one.
    Bar,
    /// `< > Ʌ V` opening onto the node, `< > ^ v` in ASCII: a crow's foot's many.
    Fork,
    /// `╳`, `x` in ASCII: a crossed end, or where a line is struck out.
    Cross,
}

impl Tip {
    #[cfg(test)]
    pub(crate) const ALL: &[Self] =
        &[Self::Arrow, Self::Triangle, Self::Diamond, Self::FilledDiamond, Self::Ring, Self::Bar, Self::Fork, Self::Cross];

    /// The tip facing a node `direction` points at: a fork opens onto it, an arrow or a triangle points at it.
    ///
    /// A bar is drawn as a line across the edge, merged with it; alone on its cell it is `┼`, `+` in ASCII.
    pub(crate) const fn glyph(self, direction: Direction, glyphs: GlyphSet) -> char {
        match (self, direction) {
            (Self::Arrow, Direction::Up) => glyphs.pick('▴', '^'),
            (Self::Arrow, Direction::Right) => glyphs.pick('▸', '>'),
            (Self::Arrow, Direction::Down) => glyphs.pick('▾', 'v'),
            (Self::Arrow, Direction::Left) => glyphs.pick('◂', '<'),
            (Self::Triangle, Direction::Up) => glyphs.pick('▵', '^'),
            (Self::Triangle, Direction::Right) => glyphs.pick('▹', '>'),
            (Self::Triangle, Direction::Down) => glyphs.pick('▿', 'v'),
            (Self::Triangle, Direction::Left) => glyphs.pick('◃', '<'),
            (Self::Diamond, _) => glyphs.pick('⬦', 'o'),
            (Self::FilledDiamond, _) => glyphs.pick('⬥', '#'),
            (Self::Ring, _) => 'o',
            (Self::Bar, _) => glyphs.pick('┼', '+'),
            (Self::Fork, Direction::Right) => '<',
            (Self::Fork, Direction::Left) => '>',
            (Self::Fork, Direction::Down) => glyphs.pick('Ʌ', '^'),
            (Self::Fork, Direction::Up) => glyphs.pick('V', 'v'),
            (Self::Cross, _) => glyphs.pick('╳', 'x'),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fails to build when a tip is added, so that it is added to `Tip::ALL` too.
    #[test]
    fn every_tip_is_listed_once_in_order() {
        let place = |tip| match tip {
            Tip::Arrow => 0,
            Tip::Triangle => 1,
            Tip::Diamond => 2,
            Tip::FilledDiamond => 3,
            Tip::Ring => 4,
            Tip::Bar => 5,
            Tip::Fork => 6,
            Tip::Cross => 7,
        };

        assert_eq!(Tip::ALL.iter().map(|&tip| place(tip)).collect::<Vec<_>>(), (0..8).collect::<Vec<_>>());
    }
}
