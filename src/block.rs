use std::ops::Range;

use crate::canvas::{Role, Styled};
use crate::geometry::Direction;
use crate::glyph::GlyphSet;

const EIGHTHS: u8 = 8;
const HALF: u8 = EIGHTHS / 2;

const FROM_DOWN: [char; 7] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇'];
const FROM_LEFT: [char; 7] = ['▏', '▎', '▍', '▌', '▋', '▊', '▉'];
const FROM_UP: [char; 7] = ['▔', '\u{1FB82}', '\u{1FB83}', '▀', '\u{1FB84}', '\u{1FB85}', '\u{1FB86}'];
const FROM_RIGHT: [char; 7] = ['▕', '\u{1FB87}', '\u{1FB88}', '▐', '\u{1FB89}', '\u{1FB8A}', '\u{1FB8B}'];

/// A cell filled from one of its sides, by 1 to 8 eighths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Block {
    from: Direction,
    eighths: u8,
}

impl Block {
    /// The block between two offsets of a cell, `low < high <= 8`, counted from its side `low_side`.
    ///
    /// A stretch touching neither side has no glyph: it keeps its size and moves to the side it is nearer.
    const fn between(low: u8, high: u8, low_side: Direction) -> Self {
        let nearer_low = low == 0 || (high < EIGHTHS && low + high < EIGHTHS);
        Self { from: if nearer_low { low_side } else { low_side.opposite() }, eighths: high - low }
    }

    pub(crate) const fn fills_cell(self) -> bool {
        self.eighths >= EIGHTHS
    }

    pub(crate) const fn fills_half(self) -> bool {
        self.eighths >= HALF
    }

    pub(crate) fn styled(self, role: Role, ground: Option<Role>, glyphs: GlyphSet) -> Styled {
        if glyphs != GlyphSet::Ascii {
            return Styled { char: self.unicode(glyphs), role, background: ground };
        }
        match ground {
            // ASCII has one character for the cell: it shows whichever of the two fills more of it.
            Some(ground) if !self.fills_half() => Styled { char: glyphs.fill(ground), role: ground, background: Some(role) },
            _ => Styled { char: self.ascii(role), role, background: ground },
        }
    }

    /// Blocks from the top and the right are Legacy Computing glyphs but for one eighth and one half;
    /// without them the block takes the nearest of one eighth, one half and the whole cell.
    fn unicode(self, glyphs: GlyphSet) -> char {
        let (table, in_unicode3) = match self.from {
            Direction::Down => (FROM_DOWN, true),
            Direction::Left => (FROM_LEFT, true),
            Direction::Up => (FROM_UP, false),
            Direction::Right => (FROM_RIGHT, false),
        };
        let eighths = if in_unicode3 || glyphs.holds(GlyphSet::Unicode13) { self.eighths } else { coarse(self.eighths) };
        if eighths >= EIGHTHS { glyphs.fill(Role::Plain) } else { table[usize::from(eighths.max(1)) - 1] }
    }

    /// Under half a cell, a mark on the side the block starts from, so a small value still shows.
    fn ascii(self, role: Role) -> char {
        match self.from {
            _ if self.fills_half() => GlyphSet::Ascii.fill(role),
            Direction::Down => '_',
            Direction::Up => '"',
            Direction::Left | Direction::Right => '|',
        }
    }
}

const fn coarse(eighths: u8) -> u8 {
    match eighths {
        0..=2 => 1,
        3..=5 => HALF,
        _ => EIGHTHS,
    }
}

/// The block of each cell a stretch covers, the cells counted along the stretch.
///
/// `eighths` counts eighths of a cell from the first of `cells` cells, whose side `low_side` faces the start; what is past the last cell is cut.
pub(crate) fn span(eighths: Range<u32>, cells: usize, low_side: Direction) -> impl Iterator<Item = (usize, Block)> {
    let per_cell = u32::from(EIGHTHS);
    let end = eighths.end.min(u32::try_from(cells).unwrap_or(u32::MAX).saturating_mul(per_cell));
    let start = eighths.start.min(end);
    (start / per_cell..end.div_ceil(per_cell)).filter(move |_| start < end).map(move |cell| {
        let low = start.saturating_sub(cell * per_cell);
        let high = (end - cell * per_cell).min(per_cell);
        (cell as usize, Block::between(low as u8, high as u8, low_side))
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    const SETS: [GlyphSet; 4] = [GlyphSet::Unicode, GlyphSet::Unicode13, GlyphSet::Unicode3, GlyphSet::Ascii];

    pub(crate) fn every_glyph() -> Vec<char> {
        [FROM_DOWN, FROM_LEFT, FROM_UP, FROM_RIGHT].concat()
    }

    fn blocks(cells: usize, eighths: Range<u32>) -> Vec<(usize, Block)> {
        span(eighths, cells, Direction::Up).collect()
    }

    #[test]
    fn blocks_fill_from_each_side_by_eighths_in_each_glyph_set() {
        let table: Vec<String> = SETS
            .into_iter()
            .flat_map(|glyphs| Direction::ALL.map(move |from| (glyphs, from)))
            .map(|(glyphs, from)| {
                let row: String = (1..=EIGHTHS).map(|eighths| Block { from, eighths }.styled(Role::Series(1), None, glyphs).char).collect();
                format!("{glyphs:?} from {from:?}: {row}")
            })
            .collect();

        insta::assert_snapshot!(table.join("\n"));
    }

    #[test]
    fn a_stretch_fills_whole_cells_and_a_block_at_each_end() {
        let full = Block { from: Direction::Up, eighths: 8 };

        assert_eq!(
            blocks(4, 5..27),
            [(0, Block { from: Direction::Down, eighths: 3 }), (1, full), (2, full), (3, Block { from: Direction::Up, eighths: 3 })]
        );
    }

    #[test]
    fn a_stretch_past_the_last_cell_is_cut() {
        assert_eq!(blocks(2, 12..u32::MAX), [(1, Block { from: Direction::Down, eighths: 4 })]);
    }

    #[test]
    fn a_stretch_starting_past_the_last_cell_fills_nothing() {
        assert_eq!(blocks(2, 16..40), []);
    }

    #[test]
    fn an_empty_stretch_fills_nothing() {
        assert_eq!(blocks(2, 5..5), []);
    }

    #[test]
    fn a_stretch_inside_a_cell_moves_to_the_side_it_is_nearer() {
        assert_eq!(blocks(1, 1..4), [(0, Block { from: Direction::Up, eighths: 3 })]);
        assert_eq!(blocks(1, 4..7), [(0, Block { from: Direction::Down, eighths: 3 })]);
    }

    #[test]
    fn ascii_shows_whichever_of_a_block_and_its_background_fills_more() {
        let small = Block { from: Direction::Down, eighths: 3 };

        let styled = small.styled(Role::Series(0), Some(Role::Series(1)), GlyphSet::Ascii);

        assert_eq!(styled, Styled { char: '%', role: Role::Series(1), background: Some(Role::Series(0)) });
    }
}
