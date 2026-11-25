//! Arbitrary drawing operations on a canvas, then every row read back in every glyph set must be exactly the canvas's width.

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use unicode_width::UnicodeWidthChar;
use vt_canvas::{Canvas, Corner, Direction, Dots, Fill, GlyphSet, LineKind, Pixel, Point, Rect, Shape, Tip, Wedge};

/// Wide enough to reach every clipping branch, small enough to keep each run fast.
const MAX_SIDE: u16 = 64;
/// Coordinates past it are folded back under it, but for the last ones before `u16::MAX`, where sums overflow:
/// a line to 60000 walks 60000 cells, which slows every run without reaching anything new.
const FAR: u16 = 1024;

#[derive(Debug, Arbitrary)]
struct Drawing {
    width: u16,
    height: u16,
    operations: Vec<Operation>,
}

#[derive(Debug, Arbitrary)]
enum Operation {
    Text { at: (u16, u16), text: String, role: u8 },
    TextCut { at: (u16, u16), text: String, max_width: u16, role: u8 },
    Horizontal { y: u16, from_x: u16, to_x: u16, kind: u8, role: u8 },
    Vertical { x: u16, from_y: u16, to_y: u16, kind: u8, role: u8 },
    Path { points: Vec<(u16, u16)>, kind: u8, role: u8 },
    Frame { rect: (u16, u16, u16, u16), kind: u8, rounded: bool, role: u8 },
    Tip { at: (u16, u16), direction: u8, tip: u8, role: u8 },
    Shape { rect: (u16, u16, u16, u16), shape: u8, role: u8 },
    Column { x: u16, eighths: (u32, u32), role: u8 },
    Row { y: u16, eighths: (u32, u32), role: u8 },
    Stroke { x: u16, eighth: u32, role: u8 },
    Wedge { at: (u16, u16), left: u8, right: u8, above: bool, role: u8 },
    Segment { from: (u32, u32), to: (u32, u32), dots: u8, role: u8 },
}

fuzz_target!(|drawing: Drawing| {
    let Ok(mut canvas) = Canvas::new(drawing.width % MAX_SIDE, drawing.height % MAX_SIDE) else { return };
    for operation in &drawing.operations {
        apply(&mut canvas, operation);
    }
    for glyphs in [GlyphSet::Unicode, GlyphSet::Unicode13, GlyphSet::Unicode3, GlyphSet::Ascii] {
        for row in canvas.rows(glyphs) {
            let text: String = row.into_iter().map(|run| run.text).collect();
            let columns: usize = text.chars().map(|char| char.width().filter(|&width| width > 0).unwrap_or(usize::MAX / 4)).sum();
            assert_eq!(columns, usize::from(drawing.width % MAX_SIDE), "row {text:?}");
        }
    }
});

fn apply(canvas: &mut Canvas<u8>, operation: &Operation) {
    match operation {
        Operation::Text { at, text, role } => canvas.text(point(*at), text, *role),
        Operation::TextCut { at, text, max_width, role } => canvas.text_cut(point(*at), text, *max_width, *role),
        Operation::Horizontal { y, from_x, to_x, kind, role } => {
            canvas.horizontal(coordinate(*y), coordinate(*from_x), coordinate(*to_x), line_kind(*kind), *role);
        }
        Operation::Vertical { x, from_y, to_y, kind, role } => {
            canvas.vertical(coordinate(*x), coordinate(*from_y), coordinate(*to_y), line_kind(*kind), *role);
        }
        Operation::Path { points, kind, role } => {
            let points: Vec<Point> = points.iter().copied().map(point).collect();
            let refused = canvas.path(&points, line_kind(*kind), *role).is_err();
            assert_eq!(refused, points.windows(2).any(|pair| pair[0].x != pair[1].x && pair[0].y != pair[1].y));
        }
        Operation::Frame { rect, kind, rounded, role } => {
            let corner = if *rounded { Corner::Rounded } else { Corner::Square };
            canvas.frame(self::rect(*rect), line_kind(*kind), corner, *role);
        }
        Operation::Tip { at, direction, tip, role } => {
            canvas.tip(point(*at), self::direction(*direction), tip_kind(*tip), *role);
        }
        Operation::Shape { rect, shape, role } => {
            canvas.shape(self::rect(*rect), Shape::ALL[usize::from(*shape) % Shape::ALL.len()], *role)
        }
        Operation::Column { x, eighths, role } => canvas.column(coordinate(*x), fine(eighths.0)..fine(eighths.1), *role),
        Operation::Row { y, eighths, role } => canvas.row(coordinate(*y), fine(eighths.0)..fine(eighths.1), *role),
        Operation::Stroke { x, eighth, role } => canvas.stroke(coordinate(*x), fine(*eighth), *role),
        Operation::Wedge { at, left, right, above, role } => {
            let fill = if *above { Fill::Above } else { Fill::Below };
            canvas.wedge(point(*at), Wedge { left: *left, right: *right, fill }, *role);
        }
        Operation::Segment { from, to, dots, role } => {
            let (from, to) = (Pixel::new(fine(from.0), fine(from.1)), Pixel::new(fine(to.0), fine(to.1)));
            canvas.segment(from, to, self::dots(*dots), *role);
        }
    }
}

fn point((x, y): (u16, u16)) -> Point {
    Point::new(coordinate(x), coordinate(y))
}

fn rect((x, y, width, height): (u16, u16, u16, u16)) -> Rect {
    Rect::new(coordinate(x), coordinate(y), coordinate(width), coordinate(height))
}

fn coordinate(value: u16) -> u16 {
    if value > u16::MAX - FAR { value } else { value % FAR }
}

/// Dots and eighths past `FAR` cells are folded back too, but for the last ones before `u32::MAX`.
fn fine(value: u32) -> u32 {
    if value > u32::MAX - u32::from(FAR) { value } else { value % (u32::from(FAR) * 8) }
}

fn tip_kind(byte: u8) -> Tip {
    [Tip::Arrow, Tip::Triangle, Tip::Diamond, Tip::FilledDiamond, Tip::Ring, Tip::Bar, Tip::Fork, Tip::Cross][usize::from(byte) % 8]
}

fn dots(byte: u8) -> Dots {
    [Dots::Braille, Dots::Octant, Dots::Sextant, Dots::Quadrant, Dots::HalfBlock][usize::from(byte) % 5]
}

fn line_kind(byte: u8) -> LineKind {
    [LineKind::Light, LineKind::Heavy, LineKind::Double, LineKind::Dashed][usize::from(byte) % 4]
}

fn direction(byte: u8) -> Direction {
    [Direction::Up, Direction::Right, Direction::Down, Direction::Left][usize::from(byte) % 4]
}
