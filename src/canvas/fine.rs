//! Drawings finer than a cell: blocks by eighths, strokes, wedges and dots, and the backgrounds they leave.

use std::ops::Range;

use super::{Canvas, Cell};
use crate::block::{self, Stroke};
use crate::geometry::{Direction, Point};
use crate::pixel::{self, Dots, Halves, Pixel};
use crate::role::Role;
use crate::wedge::Wedge;

impl<R: Role> Canvas<R> {
    /// Fills column `x` between two heights, each counted in eighths of a row from the top of the canvas.
    ///
    /// A bar standing on the bottom of row 9 and 2.5 rows tall is `fill_column(x, 60..80, role)`.
    /// A stretch that touches neither end of its cell has no glyph, so it moves to the end it is nearer.
    pub fn fill_column(&mut self, x: u16, eighths: Range<u32>, role: R) {
        for placed in block::span(eighths, self.height, Direction::Up) {
            self.put(Point::new(x, placed.cell), Cell::Block { block: placed.block, role });
        }
    }

    /// Fills row `y` between two offsets, each counted in eighths of a column from the left of the canvas.
    pub fn fill_row(&mut self, y: u16, eighths: Range<u32>, role: R) {
        for placed in block::span(eighths, self.width, Direction::Left) {
            self.put(Point::new(placed.cell, y), Cell::Block { block: placed.block, role });
        }
    }

    /// Fills the part of the cell `at` that is on one side of a slanted edge; a wedge filling nothing draws nothing.
    pub fn wedge(&mut self, at: Point, wedge: Wedge, role: R) {
        if !wedge.fills_nothing() {
            self.put(at, Cell::Wedge { wedge, role });
        }
    }

    /// Draws a line one eighth of a row thick across column `x`, `eighth` eighths of a row below the top of the canvas.
    pub fn stroke(&mut self, x: u16, eighth: u32, role: R) {
        if let Ok(y) = u16::try_from(Stroke::row(eighth)) {
            self.put(Point::new(x, y), Cell::Stroke { stroke: Stroke::at(eighth), role });
        }
    }

    /// Lights the dot `at` of the grid `dots` cuts the cells into.
    ///
    /// A cell keeps one role for all its dots, the last drawn, but for [`Dots::HalfBlock`]; dots of another kind replace the cell's.
    pub fn dot(&mut self, at: Pixel, dots: Dots, role: R) {
        let (Ok(x), Ok(y)) = (u16::try_from(at.x / dots.across()), u16::try_from(at.y / dots.down())) else { return };
        let cell = Point::new(x, y);
        let Some(index) = self.index(cell) else { return };
        let bit = dots.bit(at);
        match (self.cells[index], dots) {
            (Cell::Dots { dots: under, lit, .. }, _) if under == dots => self.cells[index] = Cell::Dots { dots, lit: lit | bit, role },
            (Cell::Halves(halves), Dots::HalfBlock) => self.light(index, halves.lit(bit, role)),
            (_, Dots::HalfBlock) => self.put(cell, Cell::Halves(Halves::default().lit(bit, role))),
            _ => self.put(cell, Cell::Dots { dots, lit: bit, role }),
        }
    }

    /// Lights the dots of the straight line from `from` to `to`, both included.
    pub fn segment(&mut self, from: Pixel, to: Pixel, dots: Dots, role: R) {
        let size = Pixel::new(u32::from(self.width) * dots.across(), u32::from(self.height) * dots.down());
        for dot in pixel::between(from, to, size) {
            self.dot(dot, dots, role);
        }
    }

    /// A cell of half blocks keeps its background until both halves are lit.
    fn light(&mut self, index: usize, halves: Halves<R>) {
        if halves.fills_cell() {
            self.grounds[index] = None;
        }
        self.cells[index] = Cell::Halves(halves);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::tests::{canvas, line, render, styles};
    use crate::glyph::GlyphSet;
    use crate::line::LineKind;
    use crate::role::tests::Part;
    use crate::wedge::Fill;

    #[test]
    fn a_column_stands_on_the_bottom_of_a_row_in_eighths() {
        let mut canvas = canvas(3, 3);

        canvas.fill_column(0, 4..24, Part::Series(0));
        canvas.fill_column(1, 21..24, Part::Series(0));
        canvas.fill_column(2, 0..3, Part::Series(0));

        assert_eq!(render(&canvas, GlyphSet::Unicode), "▄ 🮃\n█  \n█▃ ");
        assert_eq!(render(&canvas, GlyphSet::Ascii), "# \"\n#  \n#_ ");
    }

    #[test]
    fn a_row_starts_at_the_left_in_eighths() {
        let mut canvas = canvas(3, 2);

        canvas.fill_row(0, 0..19, Part::Series(0));
        canvas.fill_row(1, 21..80, Part::Series(1));

        assert_eq!(render(&canvas, GlyphSet::Unicode), "██▍\n  🮈");
        assert_eq!(render(&canvas, GlyphSet::Ascii), "##|\n  |");
    }

    #[test]
    fn a_drawing_over_a_block_at_least_half_a_cell_keeps_it_as_background() {
        let mut canvas = canvas(3, 1);
        canvas.fill_column(0, 0..8, Part::Series(0));
        canvas.fill_column(1, 4..8, Part::Series(0));
        canvas.fill_column(2, 5..8, Part::Series(0));

        canvas.text(Point::new(0, 0), "abc", Part::Label);

        assert_eq!(styles(&canvas, GlyphSet::Unicode), ["[ab Label on Some(Series(0))][c Label on None]"]);
    }

    #[test]
    fn a_wide_character_is_drawn_on_its_left_cells_background() {
        let mut canvas = canvas(4, 1);
        canvas.fill_column(1, 0..8, Part::Series(0));
        canvas.fill_column(2, 0..8, Part::Series(1));

        canvas.text(Point::new(0, 0), "漢字", Part::Label);

        assert_eq!(styles(&canvas, GlyphSet::Unicode), ["[漢 Label on None][字 Label on Some(Series(1))]"]);
    }

    #[test]
    fn a_line_over_a_block_keeps_it_as_background() {
        let mut canvas = canvas(3, 1);
        canvas.fill_column(1, 0..8, Part::Series(2));

        line(&mut canvas, (0, 0), (2, 0), LineKind::Light);

        assert_eq!(styles(&canvas, GlyphSet::Unicode), ["[─ Edge on None][─ Edge on Some(Series(2))][─ Edge on None]"]);
    }

    #[test]
    fn a_stroke_over_a_bar_is_drawn_on_the_bar() {
        let mut canvas = canvas(3, 2);
        canvas.fill_column(1, 3..16, Part::Series(0));

        for x in 0..3 {
            canvas.stroke(x, 5, Part::Series(1));
        }

        assert_eq!(
            styles(&canvas, GlyphSet::Unicode),
            [
                "[🭺 Series(1) on None][🭺 Series(1) on Some(Series(0))][🭺 Series(1) on None]",
                "[  Plain on None][█ Series(0) on None][  Plain on None]"
            ]
        );
        assert_eq!(render(&canvas, GlyphSet::Unicode3), "───\n █ ");
        assert_eq!(render(&canvas, GlyphSet::Ascii), "---\n # ");
    }

    #[test]
    fn a_stroke_below_the_canvas_draws_nothing() {
        let mut canvas = canvas(1, 1);

        canvas.stroke(0, 8, Part::Series(0));
        canvas.stroke(0, u32::MAX, Part::Series(0));

        assert_eq!(render(&canvas, GlyphSet::Unicode), " ");
    }

    #[test]
    fn a_wedge_over_a_wedge_at_least_half_a_cell_keeps_it_as_background() {
        let mut canvas = canvas(2, 1);
        let rising = |fill| Wedge { left: 2, right: 1, fill };
        canvas.wedge(Point::new(0, 0), rising(Fill::Below), Part::Series(0));
        canvas.wedge(Point::new(1, 0), Wedge { left: 3, right: 2, fill: Fill::Below }, Part::Series(0));

        canvas.wedge(Point::new(0, 0), rising(Fill::Above), Part::Series(1));
        canvas.wedge(Point::new(1, 0), rising(Fill::Above), Part::Series(1));

        assert_eq!(styles(&canvas, GlyphSet::Unicode), ["[🭜 Series(1) on Some(Series(0))][🭜 Series(1) on None]"]);
        assert_eq!(render(&canvas, GlyphSet::Ascii), "//");
    }

    #[test]
    fn a_wedge_filling_nothing_leaves_the_cell_as_it_was() {
        let mut canvas = canvas(1, 1);
        canvas.text(Point::new(0, 0), "a", Part::Label);

        canvas.wedge(Point::new(0, 0), Wedge { left: 3, right: 3, fill: Fill::Below }, Part::Series(0));

        assert_eq!(render(&canvas, GlyphSet::Unicode), "a");
    }

    #[test]
    fn dots_of_one_kind_share_a_cell_in_the_role_drawn_last() {
        let mut canvas = canvas(2, 1);

        canvas.dot(Pixel::new(0, 0), Dots::Braille, Part::Series(0));
        canvas.dot(Pixel::new(1, 3), Dots::Braille, Part::Series(1));
        canvas.dot(Pixel::new(2, 1), Dots::Braille, Part::Series(0));

        assert_eq!(styles(&canvas, GlyphSet::Unicode), ["[⢁ Series(1) on None][⠂ Series(0) on None]"]);
    }

    #[test]
    fn dots_of_another_kind_replace_the_cell() {
        let mut canvas = canvas(1, 1);
        canvas.dot(Pixel::new(0, 0), Dots::Braille, Part::Series(0));

        canvas.dot(Pixel::new(1, 1), Dots::Quadrant, Part::Series(1));

        assert_eq!(styles(&canvas, GlyphSet::Unicode), ["[▗ Series(1) on None]"]);
    }

    #[test]
    fn the_two_halves_of_a_cell_each_keep_their_role() {
        let mut canvas = canvas(3, 1);

        canvas.dot(Pixel::new(0, 0), Dots::HalfBlock, Part::Series(0));
        canvas.dot(Pixel::new(0, 1), Dots::HalfBlock, Part::Series(1));
        canvas.dot(Pixel::new(1, 0), Dots::HalfBlock, Part::Series(2));
        canvas.dot(Pixel::new(1, 1), Dots::HalfBlock, Part::Series(2));
        canvas.dot(Pixel::new(2, 1), Dots::HalfBlock, Part::Series(3));

        assert_eq!(styles(&canvas, GlyphSet::Unicode), ["[▀ Series(0) on Some(Series(1))][█ Series(2) on None][▄ Series(3) on None]"]);
        assert_eq!(render(&canvas, GlyphSet::Ascii), "#*.");
    }

    #[test]
    fn dots_over_a_bar_are_drawn_on_the_bar() {
        let mut canvas = canvas(2, 1);
        canvas.fill_row(0, 0..16, Part::Series(0));

        canvas.segment(Pixel::new(0, 1), Pixel::new(3, 1), Dots::Braille, Part::Series(1));
        canvas.dot(Pixel::new(0, 0), Dots::HalfBlock, Part::Series(2));
        canvas.dot(Pixel::new(0, 0), Dots::HalfBlock, Part::Series(2));

        assert_eq!(styles(&canvas, GlyphSet::Unicode), ["[▀ Series(2) on Some(Series(0))][⠒ Series(1) on Some(Series(0))]"]);
    }

    #[test]
    fn a_segment_leaving_the_canvas_is_clipped() {
        let mut canvas = canvas(2, 1);

        canvas.segment(Pixel::new(0, 0), Pixel::new(u32::MAX, 1), Dots::Quadrant, Part::Series(0));
        canvas.dot(Pixel::new(4, 0), Dots::Quadrant, Part::Series(0));
        canvas.dot(Pixel::new(0, 2), Dots::Quadrant, Part::Series(0));

        assert_eq!(render(&canvas, GlyphSet::Unicode), "▀▀");
    }

    #[test]
    fn a_block_over_a_block_keeps_the_first_as_background() {
        let mut canvas = canvas(1, 2);
        canvas.fill_column(0, 0..16, Part::Series(1));

        canvas.fill_column(0, 11..16, Part::Series(0));

        assert_eq!(styles(&canvas, GlyphSet::Unicode), ["[█ Series(1) on None]", "[▅ Series(0) on Some(Series(1))]"]);
    }

    #[test]
    fn a_small_block_stays_behind_a_block_that_leaves_it_most_of_the_rest() {
        let mut canvas = canvas(2, 1);
        canvas.fill_column(0, 0..2, Part::Series(1));
        canvas.fill_column(1, 0..1, Part::Series(1));

        canvas.fill_column(0, 3..8, Part::Series(0));
        canvas.fill_column(1, 3..8, Part::Series(0));

        assert_eq!(styles(&canvas, GlyphSet::Unicode), ["[▅ Series(0) on Some(Series(1))][▅ Series(0) on None]"]);
    }

    #[test]
    fn a_background_stays_under_every_drawing_that_follows() {
        let mut canvas = canvas(1, 1);
        canvas.fill_column(0, 0..8, Part::Series(1));
        canvas.fill_column(0, 6..8, Part::Series(0));

        canvas.text(Point::new(0, 0), "a", Part::Label);

        assert_eq!(styles(&canvas, GlyphSet::Unicode), ["[a Label on Some(Series(1))]"]);
    }

    #[test]
    fn a_whole_block_leaves_no_background() {
        let mut canvas = canvas(1, 1);
        canvas.fill_column(0, 0..8, Part::Series(1));

        canvas.fill_column(0, 0..8, Part::Series(0));

        assert_eq!(styles(&canvas, GlyphSet::Unicode), ["[█ Series(0) on None]"]);
    }

    #[test]
    fn cells_with_different_backgrounds_are_different_runs() {
        let mut canvas = canvas(4, 1);
        canvas.fill_row(0, 8..16, Part::Series(0));
        canvas.fill_row(0, 16..24, Part::Series(1));

        canvas.text(Point::new(0, 0), "abcd", Part::Label);

        assert_eq!(
            styles(&canvas, GlyphSet::Unicode),
            ["[a Label on None][b Label on Some(Series(0))][c Label on Some(Series(1))][d Label on None]"]
        );
    }
}
