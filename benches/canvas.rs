//! How long a canvas at the cell limit takes to fill and read back: `cargo bench --bench canvas`.

use divan::{Bencher, black_box};
use vt_canvas::{Canvas, Dots, Error, GlyphSet, LineKind, MAX_CELLS, MAX_WIDTH, Pixel, Point, Role};

const HEIGHT: u16 = (MAX_CELLS / MAX_WIDTH as usize) as u16;

/// `cargo test --all-targets` runs this without `--bench`, and a canvas at the cell limit is slow unoptimised.
fn main() {
    if std::env::args().any(|arg| arg == "--bench") {
        divan::main();
    }
}

/// Every other row and column is a line, so half the cells are junctions: the costliest glyph lookup.
fn lattice() -> Result<Canvas, Error> {
    let mut canvas = Canvas::new(MAX_WIDTH, HEIGHT)?;
    for y in (0..HEIGHT).step_by(2) {
        canvas.line(Point::new(0, y), Point::new(MAX_WIDTH - 1, y), LineKind::Light, Role::Edge)?;
    }
    for x in (0..MAX_WIDTH).step_by(2) {
        canvas.line(Point::new(x, 0), Point::new(x, HEIGHT - 1), LineKind::Heavy, Role::Edge)?;
    }
    Ok(canvas)
}

#[divan::bench]
fn lattice_at_the_cell_limit_drawn() -> Result<Canvas, Error> {
    lattice()
}

#[divan::bench]
fn lattice_at_the_cell_limit_read(bencher: Bencher<'_, '_>) {
    bencher
        .with_inputs(lattice)
        .bench_refs(|canvas| canvas.as_ref().map(|canvas| canvas.rows(GlyphSet::Unicode).map(|runs| runs.len()).sum::<usize>()).ok());
}

#[divan::bench]
fn text_at_the_cell_limit_drawn() -> Result<Canvas, Error> {
    let row = "漢a".repeat(usize::from(MAX_WIDTH) / 3);
    let mut canvas = Canvas::new(MAX_WIDTH, HEIGHT)?;
    for y in 0..HEIGHT {
        canvas.text(Point::new(0, y), black_box(&row), Role::Label);
    }
    Ok(canvas)
}

/// A bar in every column, each a different height, then a line of dots across every row: every cell is drawn twice.
fn bars_under_dots() -> Result<Canvas, Error> {
    let mut canvas = Canvas::new(MAX_WIDTH, HEIGHT)?;
    let bottom = u32::from(HEIGHT) * 8;
    for x in 0..MAX_WIDTH {
        canvas.column(x, u32::from(x) * 7 % bottom..bottom, Role::Series(0));
    }
    let right = u32::from(MAX_WIDTH) * Dots::Octant.across() - 1;
    for y in 0..u32::from(HEIGHT) {
        let rise = y * Dots::Octant.down();
        canvas.segment(Pixel::new(0, rise), Pixel::new(right, rise + 3), Dots::Octant, Role::Series(1));
    }
    Ok(canvas)
}

#[divan::bench]
fn bars_under_dots_at_the_cell_limit_drawn() -> Result<Canvas, Error> {
    bars_under_dots()
}

#[divan::bench]
fn bars_under_dots_at_the_cell_limit_read(bencher: Bencher<'_, '_>) {
    bencher
        .with_inputs(bars_under_dots)
        .bench_refs(|canvas| canvas.as_ref().map(|canvas| canvas.rows(GlyphSet::Unicode).map(|runs| runs.len()).sum::<usize>()).ok());
}
