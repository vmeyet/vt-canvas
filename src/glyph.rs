use crate::geometry::Direction;
use crate::line::{LineKind, Sides};
use crate::shape::{Mark, Marker};
use crate::tip::Tip;

/// The characters lines, frames and arrows are drawn with; text is written as given.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GlyphSet {
    /// Box drawing (`─ ┼ ╭ ┿ ╬ ╱ ╲`) and the small triangles `▴ ▸ ▾ ◂`, which are never East Asian ambiguous.
    #[default]
    Unicode,
    /// `- | + = . : ' / \ ( ) ^ > v <`, for terminals and files that only take ASCII.
    Ascii,
}

impl GlyphSet {
    pub(crate) fn line(self, sides: Sides, rounded: bool) -> char {
        match self {
            Self::Unicode => unicode_line(sides, rounded),
            Self::Ascii => ascii_line(sides, rounded),
        }
    }

    /// Brackets and points are ASCII in both sets; the slants, the cross and the markers differ.
    ///
    /// The markers avoid `●` and `◇`, which are East Asian ambiguous.
    pub(crate) fn mark(self, mark: Mark) -> char {
        match (self, mark) {
            (Self::Unicode, Mark::Rising) => '╱',
            (Self::Unicode, Mark::Falling) => '╲',
            (Self::Ascii, Mark::Rising) => '/',
            (Self::Ascii, Mark::Falling) => '\\',
            (_, Mark::LeftArc) => '(',
            (_, Mark::RightArc) => ')',
            (_, Mark::LeftPoint) => '<',
            (_, Mark::RightPoint) => '>',
            (Self::Unicode, Mark::Cross) => '╳',
            (Self::Ascii, Mark::Cross) => 'x',
            (Self::Unicode, Mark::Marker(Marker::Dot)) => '⬤',
            (Self::Unicode, Mark::Marker(Marker::RingedDot)) => '◉',
            (Self::Unicode, Mark::Marker(Marker::Diamond)) => '⬥',
            (Self::Ascii, Mark::Marker(Marker::Dot)) => '*',
            (Self::Ascii, Mark::Marker(Marker::RingedDot)) => '@',
            (Self::Ascii, Mark::Marker(Marker::Diamond)) => '?',
            (_, Mark::Tip(tip, heading)) => self.tip(tip, heading),
        }
    }

    /// A tip facing a node `heading` points at: a fork opens onto it, a hollow head points at it.
    fn tip(self, tip: Tip, heading: Direction) -> char {
        match (self, tip, heading) {
            (Self::Unicode, Tip::Hollow, Direction::Up) => '▵',
            (Self::Unicode, Tip::Hollow, Direction::Right) => '▹',
            (Self::Unicode, Tip::Hollow, Direction::Down) => '▿',
            (Self::Unicode, Tip::Hollow, Direction::Left) => '◃',
            (Self::Ascii, Tip::Hollow, direction) => self.arrow(direction),
            (Self::Unicode, Tip::Diamond, _) => '⬦',
            (Self::Unicode, Tip::FilledDiamond, _) => '⬥',
            (Self::Ascii, Tip::Diamond, _) | (_, Tip::Ring, _) => 'o',
            (Self::Ascii, Tip::FilledDiamond, _) => '#',
            (Self::Unicode, Tip::Bar, _) => '┼',
            (Self::Ascii, Tip::Bar, _) => '+',
            (_, Tip::Fork, Direction::Right) => '<',
            (_, Tip::Fork, Direction::Left) => '>',
            (Self::Unicode, Tip::Fork, Direction::Down) => 'Ʌ',
            (Self::Ascii, Tip::Fork, Direction::Down) => '^',
            (Self::Unicode, Tip::Fork, Direction::Up) => 'V',
            (Self::Ascii, Tip::Fork, Direction::Up) => 'v',
        }
    }

    /// `⋯`, not `…`, which is East Asian ambiguous.
    pub(crate) fn ellipsis(self) -> char {
        match self {
            Self::Unicode => '⋯',
            Self::Ascii => '~',
        }
    }

    pub(crate) fn arrow(self, direction: Direction) -> char {
        match (self, direction) {
            (Self::Unicode, Direction::Up) => '▴',
            (Self::Unicode, Direction::Right) => '▸',
            (Self::Unicode, Direction::Down) => '▾',
            (Self::Unicode, Direction::Left) => '◂',
            (Self::Ascii, Direction::Up) => '^',
            (Self::Ascii, Direction::Right) => '>',
            (Self::Ascii, Direction::Down) => 'v',
            (Self::Ascii, Direction::Left) => '<',
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
        lines.chain(arrows).chain(marks).chain([GlyphSet::Unicode.ellipsis()]).filter(|glyph| *glyph != ' ').collect()
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

    /// Box drawing is East Asian ambiguous in Unicode yet drawn one column wide by terminals; it has no replacement.
    #[test]
    fn no_unicode_glyph_outside_box_drawing_is_east_asian_ambiguous() {
        let box_drawing = '\u{2500}'..='\u{257F}';
        let ambiguous: Vec<char> =
            every_unicode_glyph().into_iter().filter(|glyph| !box_drawing.contains(glyph) && glyph.width_cjk() != Some(1)).collect();

        assert_eq!(ambiguous, Vec::<char>::new());
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
