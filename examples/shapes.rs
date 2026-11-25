//! Draws every shape, named inside, with each glyph set: `cargo run --example shapes`.

use vt_canvas::{Canvas, Error, GlyphSet, Point, Rect, Role, Shape};

/// The roles this drawing needs; a real caller maps each one to a colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Part {
    #[default]
    Plain,
    Node,
    Label,
}

impl Role for Part {}

const COLUMNS: u16 = 3;
/// Room for the widest name and its outline, with a gap.
const CELL_WIDTH: u16 = 24;
/// Room for the tallest outline, the cylinder's four rows, with a gap.
const CELL_HEIGHT: u16 = 5;

fn main() -> Result<(), Error> {
    let canvas = every_shape()?;
    for glyphs in [GlyphSet::Unicode, GlyphSet::Ascii] {
        for runs in canvas.rows(glyphs) {
            println!("{}", runs.into_iter().map(|run| run.text).collect::<String>().trim_end());
        }
        println!();
    }
    Ok(())
}

fn every_shape() -> Result<Canvas<Part>, Error> {
    let rows = u16::try_from(Shape::ALL.len()).unwrap_or(u16::MAX).div_ceil(COLUMNS);
    let mut canvas = Canvas::new(COLUMNS * CELL_WIDTH, rows * CELL_HEIGHT)?;
    for (index, &shape) in (0..).zip(Shape::ALL) {
        node(&mut canvas, Point::new(index % COLUMNS * CELL_WIDTH, index / COLUMNS * CELL_HEIGHT), shape);
    }
    Ok(canvas)
}

/// The shape around its name, with a space on each side of it.
fn node(canvas: &mut Canvas<Part>, at: Point, shape: Shape) {
    let name = format!("{shape:?}");
    let inset = shape.inset();
    let name_width = u16::try_from(vt_canvas::text_width(&name)).unwrap_or(CELL_WIDTH);
    let rect = Rect::new(at.x, at.y, name_width + 2 + 2 * inset.sides, 1 + inset.top + inset.bottom);
    canvas.shape(rect, shape, Part::Node);
    canvas.text(Point::new(at.x + inset.sides + 1, at.y + inset.top), &name, Part::Label);
}
