/// What an edge draws on one cell where it meets a node, facing the node: the ends of class and ER relations.
///
/// None of their glyphs is East Asian ambiguous: `○ ◆ ◇ △ ▷` are, so the set takes the small triangles, the medium
/// diamonds and the letter `o`.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tip {
    /// `▹ ◃ ▵ ▿` pointing at the node, `> < ^ v` in ASCII: a hollow head, as inheritance draws.
    Hollow,
    /// `⬦`, `o` in ASCII: as aggregation draws.
    Diamond,
    /// `⬥`, `*` in ASCII: as composition draws.
    FilledDiamond,
    /// `o`: a lollipop's ring, or a crow's foot's zero.
    Ring,
    /// A line across the edge, merged with it into `┼`, `+` in ASCII: a crow's foot's one.
    Bar,
    /// `< > Ʌ V` opening onto the node, `< > ^ v` in ASCII: a crow's foot's many.
    Fork,
}

impl Tip {
    /// Every tip, in the order the documentation lists them.
    pub const ALL: [Self; 6] = [Self::Hollow, Self::Diamond, Self::FilledDiamond, Self::Ring, Self::Bar, Self::Fork];
}
