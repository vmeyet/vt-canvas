use crate::canvas::Role;
use crate::geometry::Direction;
use crate::line::{LineKind, Sides};
use crate::shape::{Mark, Marker};
use crate::tip::Tip;

/// The characters a drawing is read back in; text is written as given.
///
/// Each Unicode set holds the glyphs of the ones after it, so a drawing asks for the best glyph and gets the closest the set has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GlyphSet {
    /// Everything [`Unicode13`](Self::Unicode13) has, and the octants of Unicode 16 (U+1CD00…), which few fonts hold yet:
    /// for terminals that draw block glyphs themselves (Ghostty, kitty, WezTerm, foot).
    #[default]
    Unicode,
    /// Everything [`Unicode3`](Self::Unicode3) has, and Symbols for Legacy Computing (Unicode 13, U+1FB00…):
    /// sextants, wedges, one-eighth lines and the eighth blocks that fill from the top or the right.
    Unicode13,
    /// Box drawing (`─ ┼ ╭ ┿ ╬ ╱ ╲`), the small triangles `▴ ▸ ▾ ◂`, block elements (`▁ ▌ ▚ █`) and braille:
    /// what nearly every monospace font holds.
    Unicode3,
    /// `- | + = . : ' , ; " / \ ( ) ^ > v < # % * o @ x _`, for terminals and files that only take ASCII.
    Ascii,
}

/// What a filled cell is drawn with in ASCII, one character per series so they differ without colour.
const ASCII_FILLS: [char; 6] = ['#', '%', '*', 'o', '@', 'x'];

impl GlyphSet {
    /// Whether every glyph of `other` is in this set.
    pub(crate) const fn holds(self, other: Self) -> bool {
        self.rank() >= other.rank()
    }

    const fn rank(self) -> u8 {
        match self {
            Self::Ascii => 0,
            Self::Unicode3 => 1,
            Self::Unicode13 => 2,
            Self::Unicode => 3,
        }
    }

    const fn pick(self, unicode: char, ascii: char) -> char {
        if matches!(self, Self::Ascii) { ascii } else { unicode }
    }

    pub(crate) fn line(self, sides: Sides, rounded: bool) -> char {
        if self == Self::Ascii { ascii_line(sides, rounded) } else { unicode_line(sides, rounded) }
    }

    /// Brackets and points are ASCII in every set; the slants, the cross and the markers differ.
    ///
    /// The markers avoid `●` and `◇`, which are East Asian ambiguous.
    pub(crate) const fn mark(self, mark: Mark) -> char {
        match mark {
            Mark::Rising => self.pick('╱', '/'),
            Mark::Falling => self.pick('╲', '\\'),
            Mark::LeftArc => '(',
            Mark::RightArc => ')',
            Mark::LeftPoint => '<',
            Mark::RightPoint => '>',
            Mark::Cross => self.pick('╳', 'x'),
            Mark::Marker(Marker::Dot) => self.pick('⬤', '*'),
            Mark::Marker(Marker::RingedDot) => self.pick('◉', '@'),
            Mark::Marker(Marker::Diamond) => self.pick('⬥', '?'),
            Mark::Tip(tip, heading) => self.tip(tip, heading),
        }
    }

    /// A tip facing a node `heading` points at: a fork opens onto it, a hollow head points at it.
    const fn tip(self, tip: Tip, heading: Direction) -> char {
        match (tip, heading) {
            (Tip::Hollow, Direction::Up) => self.pick('▵', '^'),
            (Tip::Hollow, Direction::Right) => self.pick('▹', '>'),
            (Tip::Hollow, Direction::Down) => self.pick('▿', 'v'),
            (Tip::Hollow, Direction::Left) => self.pick('◃', '<'),
            (Tip::Diamond, _) => self.pick('⬦', 'o'),
            (Tip::FilledDiamond, _) => self.pick('⬥', '#'),
            (Tip::Ring, _) => 'o',
            (Tip::Bar, _) => self.pick('┼', '+'),
            (Tip::Fork, Direction::Right) => '<',
            (Tip::Fork, Direction::Left) => '>',
            (Tip::Fork, Direction::Down) => self.pick('Ʌ', '^'),
            (Tip::Fork, Direction::Up) => self.pick('V', 'v'),
        }
    }

    /// `⋯`, not `…`, which is East Asian ambiguous.
    pub(crate) const fn ellipsis(self) -> char {
        self.pick('⋯', '~')
    }

    pub(crate) const fn arrow(self, direction: Direction) -> char {
        match direction {
            Direction::Up => self.pick('▴', '^'),
            Direction::Right => self.pick('▸', '>'),
            Direction::Down => self.pick('▾', 'v'),
            Direction::Left => self.pick('◂', '<'),
        }
    }

    /// A whole cell in `role`: `█`, or in ASCII a character of its own for each of the first series.
    pub(crate) const fn fill(self, role: Role) -> char {
        match (self, role) {
            (Self::Ascii, Role::Series(series)) => ASCII_FILLS[series as usize % ASCII_FILLS.len()],
            (Self::Ascii, _) => ASCII_FILLS[0],
            _ => '█',
        }
    }
}

fn unicode_line(sides: Sides, rounded: bool) -> char {
    if let Some(dash) = dash(sides) {
        return dash;
    }
    let sides = straighten(undashed(sides));
    if rounded && let Some(arc) = arc(sides) {
        return arc;
    }
    if sides.any(LineKind::Double) { SINGLE_DOUBLE[index(double_per_axis(sides))] } else { LIGHT_HEAVY[index(sides)] }
}

/// A rounded corner is `.` at the top and `'` at the bottom, so a rounded box differs from a square one.
fn ascii_line(sides: Sides, rounded: bool) -> char {
    if rounded && let Some(arc) = arc(undashed(sides)) {
        return if matches!(arc, '╭' | '╮') { '.' } else { '\'' };
    }
    match (sides.has_horizontal(), sides.has_vertical()) {
        (false, false) => ' ',
        (true, true) => '+',
        (true, false) if sides.all(LineKind::Dashed) => '.',
        (true, false) if sides.any(LineKind::Heavy) || sides.any(LineKind::Double) => '=',
        (true, false) => '-',
        (false, true) if sides.all(LineKind::Dashed) => ':',
        (false, true) => '|',
    }
}

fn undashed(sides: Sides) -> Sides {
    sides.map(|_, kind| if kind == LineKind::Dashed { LineKind::Light } else { kind })
}

/// Unicode has no dashed junction, so only a straight run of dashed sides stays dashed.
fn dash(sides: Sides) -> Option<char> {
    if !sides.all(LineKind::Dashed) {
        return None;
    }
    match (sides.has_horizontal(), sides.has_vertical()) {
        (true, false) => Some('╌'),
        (false, true) => Some('╎'),
        _ => None,
    }
}

/// A line end fills its cell instead of stopping half-way, as the half glyphs `╴ ╵` would.
fn straighten(sides: Sides) -> Sides {
    let mut present = sides.present();
    match (present.next(), present.next()) {
        (Some((side, kind)), None) => sides.with(side.opposite(), Some(kind)),
        _ => sides,
    }
}

fn arc(sides: Sides) -> Option<char> {
    if sides.count() != 2 || !sides.all(LineKind::Light) {
        return None;
    }
    let has = |side| sides.get(side).is_some();
    match (has(Direction::Up), has(Direction::Right), has(Direction::Down), has(Direction::Left)) {
        (false, true, true, false) => Some('╭'),
        (false, false, true, true) => Some('╮'),
        (true, false, false, true) => Some('╯'),
        (true, true, false, false) => Some('╰'),
        _ => None,
    }
}

/// Unicode mixes single and double only axis by axis, and never double with heavy.
fn double_per_axis(sides: Sides) -> Sides {
    let axis_has_double = |side: Direction| [side, side.opposite()].into_iter().any(|side| sides.get(side) == Some(LineKind::Double));
    sides.map(|side, _| if axis_has_double(side) { LineKind::Double } else { LineKind::Light })
}

/// Position in the tables below: one base-3 digit per side, in the order up, right, down, left.
fn index(sides: Sides) -> usize {
    Direction::ALL.into_iter().fold(0, |index, side| {
        let weight = match sides.get(side) {
            None => 0,
            Some(LineKind::Light | LineKind::Dashed) => 1,
            Some(LineKind::Heavy | LineKind::Double) => 2,
        };
        index * 3 + weight
    })
}

/// Indexed by `index`: 0 none, 1 light, 2 heavy.
const LIGHT_HEAVY: [char; 81] = [
    ' ', '╴', '╸', '╷', '┐', '┑', '╻', '┒', '┓', //
    '╶', '─', '╾', '┌', '┬', '┭', '┎', '┰', '┱', //
    '╺', '╼', '━', '┍', '┮', '┯', '┏', '┲', '┳', //
    '╵', '┘', '┙', '│', '┤', '┥', '╽', '┧', '┪', //
    '└', '┴', '┵', '├', '┼', '┽', '┟', '╁', '╅', //
    '┕', '┶', '┷', '┝', '┾', '┿', '┢', '╆', '╈', //
    '╹', '┚', '┛', '╿', '┦', '┩', '┃', '┨', '┫', //
    '┖', '┸', '┹', '┞', '╀', '╃', '┠', '╂', '╉', //
    '┗', '┺', '┻', '┡', '╄', '╇', '┣', '╊', '╋', //
];

/// Indexed by `index`: 0 none, 1 single, 2 double; blank where an axis mixes single and double, which Unicode lacks.
const SINGLE_DOUBLE: [char; 81] = [
    ' ', '╴', ' ', '╷', '┐', '╕', ' ', '╖', '╗', //
    '╶', '─', ' ', '┌', '┬', ' ', '╓', '╥', ' ', //
    ' ', ' ', '═', '╒', ' ', '╤', '╔', ' ', '╦', //
    '╵', '┘', '╛', '│', '┤', '╡', ' ', ' ', ' ', //
    '└', '┴', ' ', '├', '┼', ' ', ' ', ' ', ' ', //
    '╘', ' ', '╧', '╞', ' ', '╪', ' ', ' ', ' ', //
    ' ', '╜', '╝', ' ', ' ', ' ', '║', '╢', '╣', //
    '╙', '╨', ' ', ' ', ' ', ' ', '╟', '╫', ' ', //
    '╚', ' ', '╩', ' ', ' ', ' ', '╠', ' ', '╬', //
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Role as Part;
    use unicode_width::UnicodeWidthChar;

    const KINDS: [Option<LineKind>; 5] =
        [None, Some(LineKind::Light), Some(LineKind::Heavy), Some(LineKind::Double), Some(LineKind::Dashed)];

    fn label(kind: Option<LineKind>) -> char {
        match kind {
            None => '.',
            Some(LineKind::Light) => 'l',
            Some(LineKind::Heavy) => 'h',
            Some(LineKind::Double) => 'd',
            Some(LineKind::Dashed) => '-',
        }
    }

    fn sides(up: Option<LineKind>, right: Option<LineKind>, down: Option<LineKind>, left: Option<LineKind>) -> Sides {
        Sides::default().with(Direction::Up, up).with(Direction::Right, right).with(Direction::Down, down).with(Direction::Left, left)
    }

    /// One row per (up, down) pair, one column per (left, right) pair.
    fn junction_table(set: GlyphSet) -> String {
        let pairs = || KINDS.into_iter().flat_map(|first| KINDS.into_iter().map(move |second| (first, second)));
        let header: String = pairs().map(|(left, right)| format!(" {}{}", label(left), label(right))).collect();
        let rows: Vec<String> = pairs()
            .map(|(up, down)| {
                let glyphs: String = pairs().map(|(left, right)| format!("  {}", set.line(sides(up, right, down, left), false))).collect();
                format!("{}{}{glyphs}", label(up), label(down))
            })
            .collect();
        format!("ud\\lr{header}\n{}", rows.join("\n"))
    }

    fn every_unicode_glyph() -> Vec<char> {
        let all_sides = KINDS.into_iter().flat_map(|up| {
            KINDS.into_iter().flat_map(move |right| {
                KINDS.into_iter().flat_map(move |down| KINDS.into_iter().map(move |left| sides(up, right, down, left)))
            })
        });
        let lines = all_sides.flat_map(|sides| [false, true].map(|rounded| GlyphSet::Unicode.line(sides, rounded)));
        let arrows = Direction::ALL.map(|direction| GlyphSet::Unicode.arrow(direction));
        let marks = [Mark::Rising, Mark::Falling, Mark::LeftArc, Mark::RightArc, Mark::LeftPoint, Mark::RightPoint, Mark::Cross]
            .into_iter()
            .chain(Marker::ALL.map(Mark::Marker))
            .chain(Tip::ALL.into_iter().flat_map(|tip| Direction::ALL.map(|heading| Mark::Tip(tip, heading))))
            .map(|mark| GlyphSet::Unicode.mark(mark));
        let fills = [GlyphSet::Unicode.fill(Part::Plain)];
        lines
            .chain(arrows)
            .chain(marks)
            .chain(fills)
            .chain([GlyphSet::Unicode.ellipsis()])
            .chain(crate::block::tests::every_glyph())
            .chain(crate::wedge::tests::every_glyph())
            .chain(crate::pixel::tests::every_glyph())
            .filter(|glyph| *glyph != ' ')
            .collect()
    }

    #[test]
    fn unicode_junctions_cover_every_combination_of_sides_and_kinds() {
        insta::assert_snapshot!(junction_table(GlyphSet::Unicode));
    }

    #[test]
    fn ascii_junctions_cover_every_combination_of_sides_and_kinds() {
        insta::assert_snapshot!(junction_table(GlyphSet::Ascii));
    }

    #[test]
    fn every_unicode_glyph_is_one_column_wide() {
        let wide: Vec<char> = every_unicode_glyph().into_iter().filter(|glyph| glyph.width() != Some(1)).collect();

        assert_eq!(wide, Vec::<char>::new());
    }

    /// Box drawing and block elements are East Asian ambiguous in Unicode yet drawn one column wide by terminals; they have no replacement.
    #[test]
    fn no_unicode_glyph_outside_box_drawing_and_block_elements_is_east_asian_ambiguous() {
        let exempt = '\u{2500}'..='\u{259F}';
        let ambiguous: Vec<char> =
            every_unicode_glyph().into_iter().filter(|glyph| !exempt.contains(glyph) && glyph.width_cjk() != Some(1)).collect();

        assert_eq!(ambiguous, Vec::<char>::new());
    }

    #[test]
    fn a_unicode_set_holds_the_glyphs_of_the_sets_after_it() {
        let sets = [GlyphSet::Unicode, GlyphSet::Unicode13, GlyphSet::Unicode3, GlyphSet::Ascii];

        let held: Vec<usize> = sets.map(|set| sets.into_iter().filter(|other| set.holds(*other)).count()).to_vec();

        assert_eq!(held, [4, 3, 2, 1]);
    }

    #[test]
    fn ascii_fills_each_series_with_a_character_of_its_own() {
        let fills: String = [Part::Node, Part::Series(0), Part::Series(1), Part::Series(5), Part::Series(6)]
            .map(|role| GlyphSet::Ascii.fill(role))
            .into_iter()
            .collect();

        assert_eq!(fills, "##%x#");
    }

    #[test]
    fn a_line_end_fills_its_cell() {
        assert_eq!(GlyphSet::Unicode.line(sides(None, None, None, Some(LineKind::Heavy)), false), '━');
    }

    #[test]
    fn a_rounded_corner_only_rounds_two_light_sides() {
        let light = sides(None, Some(LineKind::Light), Some(LineKind::Light), None);
        let heavy = sides(None, Some(LineKind::Heavy), Some(LineKind::Heavy), None);
        let tee = sides(None, Some(LineKind::Light), Some(LineKind::Light), Some(LineKind::Light));

        assert_eq!([light, heavy, tee].map(|sides| GlyphSet::Unicode.line(sides, true)), ['╭', '┏', '┬']);
    }
}
