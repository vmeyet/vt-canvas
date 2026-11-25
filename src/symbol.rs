use crate::glyph::GlyphSet;

/// A figure on one cell: a node too small for an outline.
///
/// None of their glyphs is East Asian ambiguous: `●` and `◆` are.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Symbol {
    /// `⬤`, `*` in ASCII.
    Dot,
    /// `◉`, `@` in ASCII: a dot inside a ring.
    RingedDot,
    /// `⬥`, `?` in ASCII.
    FilledDiamond,
}

impl Symbol {
    /// Every symbol, in the order the documentation lists them.
    pub const ALL: [Self; 3] = [Self::Dot, Self::RingedDot, Self::FilledDiamond];

    pub(crate) const fn glyph(self, glyphs: GlyphSet) -> char {
        match self {
            Self::Dot => glyphs.pick('⬤', '*'),
            Self::RingedDot => glyphs.pick('◉', '@'),
            Self::FilledDiamond => glyphs.pick('⬥', '?'),
        }
    }
}
