//! Draws a small flowchart by hand and prints it with each glyph set: `cargo run --example demo`.

use vt_canvas::{Canvas, Corner, Direction, Error, Frame, GlyphSet, LineKind, Point, Rect, Role};

const NODE: Frame<'static> = Frame { kind: LineKind::Light, corner: Corner::Square, title: None, role: Role::Node };

fn main() -> Result<(), Error> {
    let canvas = flowchart()?;
    for glyphs in [GlyphSet::Unicode, GlyphSet::Ascii] {
        for runs in canvas.rows(glyphs) {
            println!("{}", runs.into_iter().map(|run| run.text).collect::<String>());
        }
        println!();
    }
    Ok(())
}

fn flowchart() -> Result<Canvas, Error> {
    let mut canvas = Canvas::new(41, 14)?;
    canvas.frame(Rect::new(0, 4, 41, 10), Frame { corner: Corner::Rounded, title: Some("cache"), role: Role::Group, ..NODE });
    node(&mut canvas, Rect::new(6, 0, 11, 3), "Request");
    node(&mut canvas, Rect::new(6, 6, 11, 3), "Cached?");
    node(&mut canvas, Rect::new(27, 6, 11, 3), "Fetch");

    edge(&mut canvas, &[Point::new(11, 2), Point::new(11, 5)], LineKind::Light)?;
    edge(&mut canvas, &[Point::new(16, 7), Point::new(26, 7)], LineKind::Light)?;
    edge(
        &mut canvas,
        &[Point::new(32, 8), Point::new(32, 11), Point::new(22, 11), Point::new(22, 1), Point::new(17, 1)],
        LineKind::Dashed,
    )?;

    canvas.text(Point::new(18, 6), "no", Role::Label);
    canvas.text(Point::new(24, 11), " retry ", Role::Label);
    Ok(canvas)
}

fn node(canvas: &mut Canvas, rect: Rect, label: &str) {
    canvas.frame(rect, NODE);
    let label_width = u16::try_from(vt_canvas::width(label)).unwrap_or(rect.width);
    canvas.text(Point::new(rect.x + rect.width.saturating_sub(label_width) / 2, rect.y + rect.height / 2), label, Role::Label);
}

/// Draws the path through `points`, leaving its source's border, with an arrow on its last point, next to its target.
fn edge(canvas: &mut Canvas, points: &[Point], kind: LineKind) -> Result<(), Error> {
    for pair in points.windows(2) {
        canvas.line(pair[0], pair[1], kind, Role::Edge)?;
    }
    if let [.., before, end] = points {
        let direction = if end.x > before.x {
            Direction::Right
        } else if end.x < before.x {
            Direction::Left
        } else if end.y > before.y {
            Direction::Down
        } else {
            Direction::Up
        };
        canvas.arrow(*end, direction, Role::Edge);
    }
    Ok(())
}
