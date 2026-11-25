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
    /// Every tip, in the order the documentation lists them.
    pub const ALL: [Self; 8] =
        [Self::Arrow, Self::Triangle, Self::Diamond, Self::FilledDiamond, Self::Ring, Self::Bar, Self::Fork, Self::Cross];
}
