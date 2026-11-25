//! Draws bars under a stroke, a band of wedges and a wave in each kind of dots, with each glyph set: `cargo run --example pixels`.
//!
//! Its roles are numbers, one per series, so each series has an ASCII texture of its own.

use vt_canvas::{Canvas, Dots, Error, Fill, GlyphSet, Pixel, Point, Wedge};

const ALL_DOTS: [Dots; 5] = [Dots::Braille, Dots::Octant, Dots::Sextant, Dots::Quadrant, Dots::HalfBlock];
const WAVE_WIDTH: u16 = 12;

fn main() -> Result<(), Error> {
    let canvas = drawing()?;
    for glyphs in [GlyphSet::Unicode, GlyphSet::Unicode13, GlyphSet::Unicode3, GlyphSet::Ascii] {
        println!("{glyphs:?}");
        for runs in canvas.rows(glyphs) {
            println!("{}", runs.into_iter().map(|run| run.text).collect::<String>());
        }
        println!();
    }
    Ok(())
}

fn drawing() -> Result<Canvas<u8>, Error> {
    let mut canvas = Canvas::new(16 + WAVE_WIDTH * 5, 4)?;
    bars_under_a_stroke(&mut canvas);
    band(&mut canvas, 9);
    for (dots, left) in ALL_DOTS.into_iter().zip((16..).step_by(usize::from(WAVE_WIDTH))) {
        wave(&mut canvas, dots, left);
    }
    Ok(canvas)
}

fn bars_under_a_stroke(canvas: &mut Canvas<u8>) {
    for (x, eighths) in [(0, 9), (1, 9), (3, 21), (4, 21), (6, 30), (7, 30)] {
        canvas.column(x, 32 - eighths..32, 0);
    }
    for x in 0..8 {
        canvas.stroke(x, 26 - u32::from(x) * 3, 1);
    }
}

/// A band two rows thick falling one row over three cells, then level.
fn band(canvas: &mut Canvas<u8>, left: u16) {
    for (step, x) in (left..left + 3).enumerate() {
        let (high, low) = (3 - step as u8, 2 - step as u8);
        canvas.wedge(Point::new(x, 0), Wedge { left: high, right: low, fill: Fill::Below }, 2);
        canvas.column(x, 8..16, 2);
        canvas.wedge(Point::new(x, 2), Wedge { left: high, right: low, fill: Fill::Above }, 2);
    }
    for x in left + 3..left + 6 {
        canvas.column(x, 8..24, 2);
    }
}

fn wave(canvas: &mut Canvas<u8>, dots: Dots, left: u16) {
    let (width, height) = (u32::from(WAVE_WIDTH - 2) * dots.across(), 4 * dots.down() - 1);
    let dot = |step: u32| {
        let turn = f64::from(step) / f64::from(width) * std::f64::consts::TAU;
        Pixel::new(u32::from(left) * dots.across() + step, ((1.0 - turn.sin()) / 2.0 * f64::from(height)).round() as u32)
    };
    for step in 0..width - 1 {
        canvas.segment(dot(step), dot(step + 1), dots, 3);
    }
}
