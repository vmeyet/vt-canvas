//! Arbitrary drawing operations on a canvas, then every row read back must be exactly the canvas's width.

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use unicode_width::UnicodeWidthChar;
use vt_canvas::{Canvas, Corner, Direction, Frame, GlyphSet, LineKind, Point, Rect, Role};

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
    Line { from: (u16, u16), to: (u16, u16), kind: u8, role: u8 },
    Path { points: Vec<(u16, u16)>, kind: u8, role: u8 },
    Frame { rect: (u16, u16, u16, u16), kind: u8, rounded: bool, title: Option<String>, role: u8 },
    Arrow { at: (u16, u16), direction: u8, role: u8 },
}

fuzz_target!(|drawing: Drawing| {
    let Ok(mut canvas) = Canvas::new(drawing.width % MAX_SIDE, drawing.height % MAX_SIDE) else { return };
    for operation in &drawing.operations {
        apply(&mut canvas, operation);
    }
    for glyphs in [GlyphSet::Unicode, GlyphSet::Ascii] {
        for row in canvas.rows(glyphs) {
            let text: String = row.into_iter().map(|run| run.text).collect();
            let columns: usize = text.chars().map(|char| char.width().filter(|&width| width > 0).unwrap_or(usize::MAX / 4)).sum();
            assert_eq!(columns, usize::from(drawing.width % MAX_SIDE), "row {text:?}");
        }
    }
});

fn apply(canvas: &mut Canvas, operation: &Operation) {
    match operation {
        Operation::Text { at, text, role } => canvas.text(point(*at), text, self::role(*role)),
        Operation::TextCut { at, text, max_width, role } => canvas.text_cut(point(*at), text, *max_width, self::role(*role)),
        Operation::Line { from, to, kind, role } => {
            let (from, to) = (point(*from), point(*to));
            let refused = canvas.line(from, to, line_kind(*kind), self::role(*role)).is_err();
            assert_eq!(refused, from.x != to.x && from.y != to.y);
        }
        Operation::Path { points, kind, role } => {
            let points: Vec<Point> = points.iter().copied().map(point).collect();
            let refused = canvas.path(&points, line_kind(*kind), self::role(*role)).is_err();
            assert_eq!(refused, points.windows(2).any(|pair| pair[0].x != pair[1].x && pair[0].y != pair[1].y));
        }
        Operation::Frame { rect, kind, rounded, title, role } => {
            let corner = if *rounded { Corner::Rounded } else { Corner::Square };
            let frame = Frame { kind: line_kind(*kind), corner, title: title.as_deref(), role: self::role(*role) };
            canvas.frame(Rect::new(coordinate(rect.0), coordinate(rect.1), coordinate(rect.2), coordinate(rect.3)), frame);
        }
        Operation::Arrow { at, direction, role } => canvas.arrow(point(*at), self::direction(*direction), self::role(*role)),
    }
}

fn point((x, y): (u16, u16)) -> Point {
    Point::new(coordinate(x), coordinate(y))
}

fn coordinate(value: u16) -> u16 {
    if value > u16::MAX - FAR { value } else { value % FAR }
}

fn role(byte: u8) -> Role {
    [Role::Plain, Role::Node, Role::Edge, Role::Label, Role::Group][usize::from(byte) % 5]
}

fn line_kind(byte: u8) -> LineKind {
    [LineKind::Light, LineKind::Heavy, LineKind::Double, LineKind::Dashed][usize::from(byte) % 4]
}

fn direction(byte: u8) -> Direction {
    [Direction::Up, Direction::Right, Direction::Down, Direction::Left][usize::from(byte) % 4]
}
