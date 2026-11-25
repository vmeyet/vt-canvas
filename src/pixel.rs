use crate::block::Cover;
use crate::geometry::Direction;
use crate::glyph::GlyphSet;
use crate::role::Role;
use crate::styled::Styled;

/// A dot on the grid [`Dots`] cuts the cells into: `x` counts dots from the left of the canvas, `y` from its top.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pixel {
    /// Dots from the left.
    pub x: u32,
    /// Dots from the top.
    pub y: u32,
}

impl Pixel {
    /// The dot in column `x`, row `y` of the grid.
    pub const fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}

/// How many dots a cell is cut into, and the glyphs that light them.
///
/// A cell lit by two roles keeps the last one for all its dots, but for [`HalfBlock`](Self::HalfBlock), whose two dots each keep theirs.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dots {
    /// 2 by 4 round dots, `⠉ ⡠ ⣿`: the finest dots nearly every font holds; ASCII draws it as [`Quadrant`](Self::Quadrant).
    Braille,
    /// 2 by 4 square dots, Unicode 16; a set without them draws [`Sextant`](Self::Sextant), then [`Quadrant`](Self::Quadrant).
    Octant,
    /// 2 by 3 square dots, Unicode 13; a set without them draws [`Quadrant`](Self::Quadrant).
    Sextant,
    /// 2 by 2 square dots, `▘ ▚ ▟`; ASCII draws `'` for the upper two, `.` for the lower two and `:` for both, and `" , ;` for an odd [`texture`](crate::Role::texture).
    Quadrant,
    /// 1 by 2 dots, `▀ ▄ █`, the upper one drawn on the lower one's role: the one kind with two roles in a cell.
    HalfBlock,
}

impl Dots {
    #[cfg(test)]
    pub(crate) const ALL: &[Self] = &[Self::Braille, Self::Octant, Self::Sextant, Self::Quadrant, Self::HalfBlock];

    /// Dots across one cell.
    pub const fn across(self) -> u32 {
        match self {
            Self::HalfBlock => 1,
            Self::Braille | Self::Octant | Self::Sextant | Self::Quadrant => 2,
        }
    }

    /// Dots down one cell.
    pub const fn down(self) -> u32 {
        match self {
            Self::Braille | Self::Octant => 4,
            Self::Sextant => 3,
            Self::Quadrant | Self::HalfBlock => 2,
        }
    }

    /// The bit of `pixel` among the dots of its cell, counted row by row from the top left.
    pub(crate) const fn bit(self, pixel: Pixel) -> u8 {
        1 << ((pixel.y % self.down()) * self.across() + pixel.x % self.across())
    }

    /// The glyph for the dots `lit` in a cell, or the closest one `glyphs` holds.
    pub(crate) fn glyph(self, lit: u8, texture: u8, glyphs: GlyphSet) -> char {
        match (self, glyphs) {
            (_, _) if lit == 0 => ' ',
            (Self::Braille, GlyphSet::Ascii) => Self::Quadrant.glyph(regrid(lit, &[0, 0, 1, 1]), texture, glyphs),
            (Self::Braille, _) => braille(lit),
            (Self::Octant, GlyphSet::Unicode) => octant(lit),
            (Self::Octant, GlyphSet::Unicode13) => sextant(regrid(lit, &[0, 1, 1, 2])),
            (Self::Octant, _) => Self::Quadrant.glyph(regrid(lit, &[0, 0, 1, 1]), texture, glyphs),
            (Self::Sextant, GlyphSet::Unicode | GlyphSet::Unicode13) => sextant(lit),
            (Self::Sextant, _) => Self::Quadrant.glyph(regrid(lit, &[0, 1, 1]), texture, glyphs),
            (Self::Quadrant, GlyphSet::Ascii) => ascii(lit, texture),
            (Self::Quadrant, _) => QUADRANTS[usize::from(lit & 0b1111)],
            (Self::HalfBlock, _) => Self::Quadrant.glyph([0b0000, 0b0011, 0b1100, 0b1111][usize::from(lit & 0b11)], texture, glyphs),
        }
    }
}

/// The dots `lit` of a grid two dots across, each row moved to the row of a coarser grid `rows` names for it.
fn regrid(lit: u8, rows: &[u8]) -> u8 {
    let lit = |bit: &usize| lit >> bit & 1 == 1;
    (0..rows.len() * 2).filter(lit).fold(0, |regridded, bit| regridded | 1 << (rows[bit / 2] * 2 + (bit % 2) as u8))
}

/// By the bits of the upper left, upper right, lower left and lower right dots.
const QUADRANTS: [char; 16] = [' ', '▘', '▝', '▀', '▖', '▌', '▞', '▛', '▗', '▚', '▐', '▜', '▄', '▙', '▟', '█'];

/// The sextants are in the order of their dots, without the two half blocks Unicode already had.
fn sextant(lit: u8) -> char {
    const LEFT_HALF: u8 = 0b01_0101;
    const RIGHT_HALF: u8 = 0b10_1010;
    match lit & 0b11_1111 {
        0 => ' ',
        LEFT_HALF => '▌',
        RIGHT_HALF => '▐',
        0b11_1111 => '█',
        lit => char::from_u32(0x1FB00 + u32::from(lit) - 1 - u32::from(lit > LEFT_HALF) - u32::from(lit > RIGHT_HALF)).unwrap_or(' '),
    }
}

/// The dot patterns Unicode had a glyph for before the octants, which start at U+1CD00 and skip them.
const OCTANTS_ELSEWHERE: [(u8, char); 26] = [
    (0x00, ' '),
    (0x01, '\u{1CEA8}'),
    (0x02, '\u{1CEAB}'),
    (0x03, '\u{1FB82}'),
    (0x05, '▘'),
    (0x0A, '▝'),
    (0x0F, '▀'),
    (0x14, '\u{1FBE6}'),
    (0x28, '\u{1FBE7}'),
    (0x3F, '\u{1FB85}'),
    (0x40, '\u{1CEA3}'),
    (0x50, '▖'),
    (0x55, '▌'),
    (0x5A, '▞'),
    (0x5F, '▛'),
    (0x80, '\u{1CEA0}'),
    (0xA0, '▗'),
    (0xA5, '▚'),
    (0xAA, '▐'),
    (0xAF, '▜'),
    (0xC0, '▂'),
    (0xF0, '▄'),
    (0xF5, '▙'),
    (0xFA, '▟'),
    (0xFC, '▆'),
    (0xFF, '█'),
];

fn octant(lit: u8) -> char {
    match OCTANTS_ELSEWHERE.binary_search_by_key(&lit, |(pattern, _)| *pattern) {
        Ok(index) => OCTANTS_ELSEWHERE[index].1,
        Err(skipped) => char::from_u32(0x1CD00 + u32::from(lit) - skipped as u32).unwrap_or(' '),
    }
}

/// Braille numbers its dots down each column, then the fourth row.
fn braille(lit: u8) -> char {
    const BITS: [u32; 8] = [0x01, 0x08, 0x02, 0x10, 0x04, 0x20, 0x40, 0x80];
    let pattern: u32 = BITS.iter().enumerate().filter(|(bit, _)| lit >> bit & 1 == 1).map(|(_, braille)| braille).sum();
    char::from_u32(0x2800 + pattern).unwrap_or(' ')
}

/// The upper dots, the lower dots and both; an odd texture has the second three, so two lines differ without colour.
const fn ascii(quadrants: u8, texture: u8) -> char {
    let [upper, lower, both] = if texture % 2 == 1 { ['"', ',', ';'] } else { ['\'', '.', ':'] };
    match (quadrants & 0b0011 != 0, quadrants & 0b1100 != 0) {
        (false, false) => ' ',
        (true, false) => upper,
        (false, true) => lower,
        (true, true) => both,
    }
}

/// The two dots of a [`Dots::HalfBlock`] cell, each with the role that lit it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Halves<R> {
    top: Option<R>,
    bottom: Option<R>,
}

impl<R> Default for Halves<R> {
    fn default() -> Self {
        Self { top: None, bottom: None }
    }
}

impl<R: Role> Halves<R> {
    pub(crate) fn lit(self, bit: u8, role: R) -> Self {
        if bit == 0b01 { Self { top: Some(role), ..self } } else { Self { bottom: Some(role), ..self } }
    }

    /// One half lit is a block half a cell tall, so it may stay behind what is drawn over it.
    pub(crate) fn cover(self) -> Option<Cover> {
        let half = Cover::WHOLE / 2;
        match (self.top, self.bottom) {
            (None, None) => None,
            (Some(_), None) => Some(Cover { from: Direction::Up, parts: half }),
            (None, Some(_)) => Some(Cover { from: Direction::Down, parts: half }),
            (Some(_), Some(_)) => Some(Cover { from: Direction::Down, parts: Cover::WHOLE }),
        }
    }

    /// The role that stays behind a drawing over the cell: the lower half's, when both are lit.
    pub(crate) fn lowest(self) -> Option<R> {
        if self.bottom.is_some() { self.bottom } else { self.top }
    }

    pub(crate) fn fills_cell(self) -> bool {
        self.top.is_some() && self.bottom.is_some()
    }

    /// Two roles in a cell are the upper half drawn on the lower one's role; ASCII fills the cell as the upper one.
    pub(crate) fn styled(self, ground: Option<R>, glyphs: GlyphSet) -> Styled<R> {
        let ascii = glyphs == GlyphSet::Ascii;
        match (self.top, self.bottom) {
            (Some(top), Some(bottom)) if top == bottom => Styled { char: glyphs.fill(top.texture()), role: top, background: None },
            (Some(top), Some(bottom)) => {
                Styled { char: if ascii { glyphs.fill(top.texture()) } else { '▀' }, role: top, background: Some(bottom) }
            }
            (Some(top), None) => Styled { char: if ascii { '\'' } else { '▀' }, role: top, background: ground },
            (None, Some(bottom)) => Styled { char: if ascii { '.' } else { '▄' }, role: bottom, background: ground },
            (None, None) => Styled { char: ' ', role: R::default(), background: ground },
        }
    }
}

/// The dots of the straight line from `from` to `to`, both included, left out where the longer way of the line leaves a grid of `size` dots.
///
/// One dot per step along the longer way, so the walk is never longer than the grid.
pub(crate) fn between(from: Pixel, to: Pixel, size: Pixel) -> impl Iterator<Item = Pixel> {
    let (dx, dy) = (i128::from(to.x) - i128::from(from.x), i128::from(to.y) - i128::from(from.y));
    let steps = dx.abs().max(dy.abs());
    let along = move |start: u32, delta: i128, step: i128| {
        i128::from(start) + if steps == 0 { 0 } else { (2 * delta * step + steps).div_euclid(2 * steps) }
    };
    let (start, delta, limit) = if dx.abs() >= dy.abs() { (from.x, dx, size.x) } else { (from.y, dy, size.y) };
    let inside = if delta >= 0 {
        0..=(i128::from(limit) - 1 - i128::from(start)).min(steps)
    } else {
        (i128::from(start) - i128::from(limit) + 1).max(0)..=steps
    };
    inside.filter_map(move |step| {
        Some(Pixel::new(u32::try_from(along(from.x, dx, step)).ok()?, u32::try_from(along(from.y, dy, step)).ok()?))
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use unicode_width::UnicodeWidthChar;

    use super::*;
    use crate::canvas::Canvas;
    use crate::role::tests::Part;

    const SETS: [GlyphSet; 4] = [GlyphSet::Unicode, GlyphSet::Unicode13, GlyphSet::Unicode3, GlyphSet::Ascii];

    pub(crate) fn every_glyph() -> Vec<char> {
        Dots::ALL.iter().flat_map(|dots| (0..=u8::MAX).map(move |lit| dots.glyph(lit, 0, GlyphSet::Unicode))).collect()
    }

    /// A ring and a line through it, so every slope and every dot of a cell is used.
    fn drawing(dots: Dots) -> Canvas<Part> {
        let mut canvas = Canvas::<Part>::new(16, 5).unwrap();
        let (width, height) = (16.0 * f64::from(dots.across()) - 1.0, 5.0 * f64::from(dots.down()) - 1.0);
        let on_ring = |step: u32| {
            let angle = f64::from(step) * std::f64::consts::TAU / 48.0;
            Pixel::new(((1.0 + angle.cos()) * width / 2.0).round() as u32, ((1.0 + angle.sin()) * height / 2.0).round() as u32)
        };
        for step in 0..48 {
            canvas.segment(on_ring(step), on_ring(step + 1), dots, Part::Series(0));
        }
        canvas.segment(Pixel::new(0, height as u32), Pixel::new(width as u32, 0), dots, Part::Series(1));
        canvas
    }

    fn drawn(dots: Dots) -> String {
        let canvas = drawing(dots);
        let in_set = |glyphs| {
            let rows: Vec<String> = canvas.lines(glyphs).map(|line| line + "|").collect();
            format!("{glyphs:?}\n{}", rows.join("\n"))
        };
        SETS.map(in_set).join("\n")
    }

    #[test]
    fn braille_draws_in_each_glyph_set() {
        insta::assert_snapshot!(drawn(Dots::Braille));
    }

    #[test]
    fn octants_draw_in_each_glyph_set() {
        insta::assert_snapshot!(drawn(Dots::Octant));
    }

    #[test]
    fn sextants_draw_in_each_glyph_set() {
        insta::assert_snapshot!(drawn(Dots::Sextant));
    }

    #[test]
    fn quadrants_draw_in_each_glyph_set() {
        insta::assert_snapshot!(drawn(Dots::Quadrant));
    }

    #[test]
    fn half_blocks_draw_in_each_glyph_set() {
        insta::assert_snapshot!(drawn(Dots::HalfBlock));
    }

    #[test]
    fn every_pattern_of_dots_has_a_glyph_of_its_own() {
        for (dots, patterns) in [(Dots::Braille, 256), (Dots::Octant, 256), (Dots::Sextant, 64), (Dots::Quadrant, 16)] {
            let glyphs: std::collections::BTreeSet<char> = (0..patterns).map(|lit| dots.glyph(lit as u8, 0, GlyphSet::Unicode)).collect();

            assert_eq!(glyphs.len(), patterns, "{dots:?}");
        }
    }

    #[test]
    fn octants_come_from_their_block_but_for_the_patterns_unicode_already_had() {
        let in_block = (0..=u8::MAX).map(octant).filter(|glyph| ('\u{1CD00}'..='\u{1CDE5}').contains(glyph)).count();

        assert_eq!(in_block, 230);
        assert_eq!([0x04, 0x06, 0xFE].map(octant), ['\u{1CD00}', '\u{1CD01}', '\u{1CDE5}']);
    }

    #[test]
    fn every_glyph_is_one_column_wide() {
        let wide: Vec<char> = every_glyph().into_iter().filter(|glyph| glyph.width() != Some(1)).collect();

        assert_eq!(wide, Vec::<char>::new());
    }

    #[test]
    fn a_coarser_grid_lights_the_dot_nearest_each_dot() {
        let top_left_and_bottom_right = 0b1000_0001;

        let drawn = [GlyphSet::Unicode13, GlyphSet::Unicode3, GlyphSet::Ascii]
            .map(|glyphs| Dots::Octant.glyph(top_left_and_bottom_right, 0, glyphs));

        assert_eq!(drawn, ['\u{1FB1F}', '▚', ':']);
    }

    #[test]
    fn ascii_draws_every_other_texture_with_dots_of_its_own() {
        let drawn = |texture| [0b0001, 0b1000, 0b1001].map(|dots| Dots::Quadrant.glyph(dots, texture, GlyphSet::Ascii));

        assert_eq!([drawn(0), drawn(1), drawn(2)], [['\'', '.', ':'], ['"', ',', ';'], ['\'', '.', ':']]);
    }

    #[test]
    fn a_line_has_one_dot_for_each_step_of_its_longer_way() {
        let lit: Vec<(u32, u32)> = between(Pixel::new(1, 0), Pixel::new(7, 2), Pixel::new(100, 100)).map(|dot| (dot.x, dot.y)).collect();

        assert_eq!(lit, [(1, 0), (2, 0), (3, 1), (4, 1), (5, 1), (6, 2), (7, 2)]);
    }

    #[test]
    fn a_line_between_far_dots_only_walks_the_grid() {
        let far = Pixel::new(u32::MAX, u32::MAX);

        assert_eq!(between(Pixel::new(0, 0), far, Pixel::new(8, 4)).count(), 8);
        assert_eq!(between(far, Pixel::new(0, 0), Pixel::new(8, 4)).count(), 8);
        assert_eq!(between(far, far, Pixel::new(8, 4)).count(), 0);
    }

    #[test]
    fn a_line_between_a_dot_and_itself_is_that_dot() {
        let dot = Pixel::new(3, 2);

        assert_eq!(between(dot, dot, Pixel::new(8, 4)).collect::<Vec<_>>(), [dot]);
    }
}
