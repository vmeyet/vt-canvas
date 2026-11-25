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
    #[cfg(test)]
    pub(crate) const ALL: &[Self] = &[Self::Dot, Self::RingedDot, Self::FilledDiamond];

    pub(crate) const fn glyph(self, glyphs: GlyphSet) -> char {
        match self {
            Self::Dot => glyphs.pick('⬤', '*'),
            Self::RingedDot => glyphs.pick('◉', '@'),
            Self::FilledDiamond => glyphs.pick('⬥', '?'),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fails to build when a symbol is added, so that it is added to `Symbol::ALL` too.
    #[test]
    fn every_symbol_is_listed_once_in_order() {
        let place = |symbol| match symbol {
            Symbol::Dot => 0,
            Symbol::RingedDot => 1,
            Symbol::FilledDiamond => 2,
        };

        assert_eq!(Symbol::ALL.iter().map(|&symbol| place(symbol)).collect::<Vec<_>>(), (0..3).collect::<Vec<_>>());
    }
}
