use crate::block::{Block, Cover};
use crate::canvas::{Role, Styled};
use crate::geometry::Direction;
use crate::glyph::GlyphSet;

const THIRDS: u8 = 3;

/// The glyph filled below an edge, by the thirds it meets the left side at, then the right side.
///
/// Legacy Computing has no edge from a bottom corner to the opposite top corner; the half block on the filled side stands in.
const BELOW: [[char; 4]; 4] = [
    [' ', '\u{1FB48}', '\u{1FB4A}', '▐'],
    ['\u{1FB3D}', '\u{1FB2D}', '\u{1FB46}', '\u{1FB44}'],
    ['\u{1FB3F}', '\u{1FB51}', '\u{1FB39}', '\u{1FB42}'],
    ['▌', '\u{1FB4F}', '\u{1FB4D}', '█'],
];
const ABOVE: [[char; 4]; 4] = [
    ['█', '\u{1FB5E}', '\u{1FB60}', '▌'],
    ['\u{1FB53}', '\u{1FB0E}', '\u{1FB5C}', '\u{1FB5A}'],
    ['\u{1FB55}', '\u{1FB67}', '\u{1FB02}', '\u{1FB58}'],
    ['▐', '\u{1FB65}', '\u{1FB63}', ' '],
];

/// Which side of a [`Wedge`]'s edge is filled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fill {
    /// From the edge down to the bottom of the cell.
    Below,
    /// From the edge up to the top of the cell.
    Above,
}

/// The part of a cell on one side of a straight edge running from its left side to its right side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Wedge {
    /// Where the edge meets the left side, in thirds of the cell above its bottom: 0 to 3, more counts as 3.
    pub left: u8,
    /// Where the edge meets the right side, counted the same way.
    pub right: u8,
    /// The side of the edge that is filled.
    pub fill: Fill,
}

impl Wedge {
    /// Sixths of the cell under the edge.
    const fn below(self) -> u8 {
        self.left() + self.right()
    }

    const fn left(self) -> u8 {
        if self.left > THIRDS { THIRDS } else { self.left }
    }

    const fn right(self) -> u8 {
        if self.right > THIRDS { THIRDS } else { self.right }
    }

    const fn filled(self) -> u8 {
        match self.fill {
            Fill::Below => self.below(),
            Fill::Above => 2 * THIRDS - self.below(),
        }
    }

    pub(crate) const fn fills_nothing(self) -> bool {
        self.filled() == 0
    }

    pub(crate) const fn cover(self) -> Cover {
        let from = match self.fill {
            Fill::Below => Direction::Down,
            Fill::Above => Direction::Up,
        };
        Cover { from, parts: self.filled() * (Cover::WHOLE / (2 * THIRDS)) }
    }

    pub(crate) fn styled(self, role: Role, ground: Option<Role>, glyphs: GlyphSet) -> Styled {
        let char = if glyphs.holds(GlyphSet::Unicode13) {
            self.unicode13()
        } else if glyphs == GlyphSet::Ascii {
            self.ascii(role)
        } else {
            self.level().unicode(glyphs)
        };
        Styled { char, role, background: ground }
    }

    const fn unicode13(self) -> char {
        let table = match self.fill {
            Fill::Below => BELOW,
            Fill::Above => ABOVE,
        };
        table[self.left() as usize][self.right() as usize]
    }

    fn ascii(self, role: Role) -> char {
        match self.left().cmp(&self.right()) {
            std::cmp::Ordering::Less => '/',
            std::cmp::Ordering::Greater => '\\',
            std::cmp::Ordering::Equal => self.level().ascii(role),
        }
    }

    /// The block filling as much of the cell from the same side, for the sets without wedges.
    const fn level(self) -> Block {
        let eighths = (self.filled() * 4 + 1) / THIRDS;
        match self.fill {
            Fill::Below => Block::new(Direction::Down, eighths),
            Fill::Above => Block::new(Direction::Up, eighths),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::canvas::Role as Part;

    pub(crate) fn every_glyph() -> Vec<char> {
        [BELOW, ABOVE].concat().concat()
    }

    fn table(glyphs: GlyphSet) -> String {
        let rows: Vec<String> = [Fill::Below, Fill::Above]
            .into_iter()
            .flat_map(|fill| (0..=THIRDS).map(move |left| (fill, left)))
            .map(|(fill, left)| {
                let row: String =
                    (0..=THIRDS).map(|right| Wedge { left, right, fill }.styled(Part::Series(0), None, glyphs).char).collect();
                format!("{fill:?} from {left}: {row}|")
            })
            .collect();
        format!("{glyphs:?}\n{}", rows.join("\n"))
    }

    #[test]
    fn wedges_cover_every_pair_of_thirds_in_each_glyph_set() {
        let tables = [GlyphSet::Unicode13, GlyphSet::Unicode3, GlyphSet::Ascii].map(table);

        insta::assert_snapshot!(tables.join("\n"));
    }

    #[test]
    fn a_wedge_and_the_one_filled_on_the_other_side_fill_the_cell() {
        for (left, right) in (0..=THIRDS).flat_map(|left| (0..=THIRDS).map(move |right| (left, right))) {
            let [below, above] = [Fill::Below, Fill::Above].map(|fill| Wedge { left, right, fill });

            assert_eq!(below.filled() + above.filled(), 2 * THIRDS);
            assert_eq!(below.fills_nothing(), above.cover().parts == Cover::WHOLE);
        }
    }

    #[test]
    fn thirds_past_the_top_of_the_cell_count_as_the_top() {
        let wedge = Wedge { left: 200, right: 9, fill: Fill::Below };

        assert_eq!(wedge.cover().parts, Cover::WHOLE);
        assert_eq!(wedge.styled(Part::Plain, None, GlyphSet::Unicode).char, '█');
    }
}
