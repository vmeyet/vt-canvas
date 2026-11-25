//! Lines and what is drawn on them: paths, frames, shapes, tips and symbols, merged into junctions where they meet.

use super::{Canvas, Cell, Error};
use crate::frame::Corner;
use crate::geometry::{Border, Direction, Point, Rect};
use crate::line::{LineKind, Sides};
use crate::role::Role;
use crate::shape::Shape;
use crate::symbol::Symbol;
use crate::tip::Tip;

impl<R: Role> Canvas<R> {
    /// Draws a line along row `y` from column `from_x` to column `to_x`, both included; a cell it shares with another line becomes a junction.
    pub fn horizontal(&mut self, y: u16, from_x: u16, to_x: u16, kind: LineKind, role: R) {
        self.draw_straight(Point::new(from_x, y), Point::new(to_x, y), kind, role);
    }

    /// Draws a line down column `x` from row `from_y` to row `to_y`, both included; a cell it shares with another line becomes a junction.
    pub fn vertical(&mut self, x: u16, from_y: u16, to_y: u16, kind: LineKind, role: R) {
        self.draw_straight(Point::new(x, from_y), Point::new(x, to_y), kind, role);
    }

    /// Draws a line from each point to the next, so they turn a corner at every point between the ends.
    ///
    /// A pair of consecutive points sharing neither a row nor a column is refused before anything is drawn.
    pub fn path(&mut self, points: &[Point], kind: LineKind, role: R) -> Result<(), Error> {
        if let Some(&[from, to]) = points.windows(2).find(|pair| pair[0].x != pair[1].x && pair[0].y != pair[1].y) {
            return Err(Error::Diagonal { from, to });
        }
        for pair in points.windows(2) {
            self.draw_straight(pair[0], pair[1], kind, role);
        }
        Ok(())
    }

    /// Draws the border of `rect` in lines of `kind`.
    pub fn frame(&mut self, rect: Rect, kind: LineKind, corner: Corner, role: R) {
        if let Some(border) = rect.border() {
            self.draw_frame(border, kind, corner, role);
        }
    }

    /// Draws the outline of `shape` on the border of `rect`; content goes inside its [`inset`](Shape::inset).
    ///
    /// A line drawn onto a slant or a bracket of the outline leaves it whole, so an edge ending on a side touches it.
    pub fn shape(&mut self, rect: Rect, shape: Shape, role: R) {
        let Some(border) = rect.border() else { return };
        self.draw_frame(border, LineKind::Light, shape.corner(), role);
        for [from, to] in shape.inner_lines(border) {
            self.draw_straight(from, to, LineKind::Light, role);
        }
        for placed in shape.marks(border, self.height) {
            self.put(placed.at, Cell::Mark { mark: placed.mark, role });
        }
    }

    /// Draws `symbol` on one cell: a node too small for an outline.
    ///
    /// Like a shape's mark, a line drawn onto it later leaves it whole, so an edge starting on it keeps it.
    pub fn symbol(&mut self, at: Point, symbol: Symbol, role: R) {
        self.put(at, Cell::Symbol { symbol, role });
    }

    /// Draws `tip` on one cell, facing the node `direction` points at: an arrow head points that way.
    ///
    /// A bar merges with the lines through its cell into `┼`, in the kind of the line it crosses; any other tip is
    /// kept under a line, as a shape's mark is, so an edge drawn after it does not cut it.
    /// Like every drawing, a tip replaces a symbol, a mark or a tip already on its cell.
    pub fn tip(&mut self, at: Point, direction: Direction, tip: Tip, role: R) {
        if tip == Tip::Bar {
            let sides = Sides::every(self.line_along(at, direction).unwrap_or(LineKind::Light));
            match self.index(at).map(|index| self.cells[index]) {
                Some(Cell::Line { .. }) => self.join(at, sides, role),
                _ => self.put(at, Cell::Line { sides, rounded: false, role }),
            }
        } else {
            self.put(at, Cell::Tip { tip, direction, role });
        }
    }

    fn draw_frame(&mut self, border: Border, kind: LineKind, corner: Corner, role: R) {
        let corners = border.corners();
        for (index, from) in corners.into_iter().enumerate() {
            self.draw_straight(from, corners[(index + 1) % corners.len()], kind, role);
        }
        if border.width > 1 && border.height > 1 {
            for at in corners {
                self.set_corner(at, corner);
            }
        }
    }

    /// `from` and `to` share a row or a column; the cells past the canvas are never walked.
    fn draw_straight(&mut self, from: Point, to: Point, kind: LineKind, role: R) {
        let horizontal = from.y == to.y;
        let (low, high, before, cells) = if horizontal {
            (from.x.min(to.x), from.x.max(to.x), Direction::Left, self.width)
        } else {
            (from.y.min(to.y), from.y.max(to.y), Direction::Up, self.height)
        };
        for step in low..=high.min(cells.saturating_sub(1)) {
            let at = if horizontal { Point::new(step, from.y) } else { Point::new(from.x, step) };
            self.join(at, sides_at(step, low, high, kind, before), role);
        }
    }

    /// Merges `sides` into the lines of the cell; a line with both ends on one cell leaves it through no side, so it draws nothing.
    ///
    /// A junction keeps the role of the border it is on (two sides or more) when the new line only ends on it and adds
    /// a side the border lacks, so a node's border stays whole where an edge joins it; otherwise it takes the new line's
    /// role, so a frame redrawn in another role is wholly in that role.
    fn join(&mut self, at: Point, sides: Sides, role: R) {
        let Some(index) = self.index(at).filter(|_| sides.count() > 0) else { return };
        match self.cells[index] {
            Cell::Line { sides: below, rounded, role: below_role } => {
                let joins_border = below.count() > 1 && !sides.runs_through() && sides.adds_to(below);
                let role = if joins_border { below_role } else { role };
                self.cells[index] = Cell::Line { sides: below.merge(sides), rounded, role };
            }
            Cell::Mark { .. } | Cell::Tip { .. } | Cell::Symbol { .. } => {}
            _ => self.put(at, Cell::Line { sides, rounded: false, role }),
        }
    }

    /// The kind of the line through the cell along `direction`'s axis.
    fn line_along(&self, at: Point, direction: Direction) -> Option<LineKind> {
        match self.index(at).map(|index| self.cells[index]) {
            Some(Cell::Line { sides, .. }) => sides.get(direction).or(sides.get(direction.opposite())),
            _ => None,
        }
    }

    /// The last frame drawn on a corner sets whether it is rounded.
    fn set_corner(&mut self, at: Point, corner: Corner) {
        if let Some(index) = self.index(at)
            && let Cell::Line { sides, role, .. } = self.cells[index]
        {
            self.cells[index] = Cell::Line { sides, rounded: corner == Corner::Rounded, role };
        }
    }
}

/// The sides a line from `low` to `high` leaves the cell at `at` through; `before` is the side facing `low`.
fn sides_at(at: u16, low: u16, high: u16, kind: LineKind, before: Direction) -> Sides {
    Sides::default().with(before, (at > low).then_some(kind)).with(before.opposite(), (at < high).then_some(kind))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::tests::{canvas, line, render, styles};
    use crate::glyph::GlyphSet;
    use crate::role::tests::Part;

    #[test]
    fn a_line_leaving_the_canvas_is_clipped() {
        let mut canvas = canvas(4, 2);

        line(&mut canvas, (1, 0), (u16::MAX, 0), LineKind::Heavy);
        line(&mut canvas, (2, 0), (2, 9), LineKind::Light);

        assert_eq!(render(&canvas, GlyphSet::Unicode), " ━┯━\n  │ ");
    }

    #[test]
    fn text_replaces_a_line() {
        let mut canvas = canvas(5, 1);
        line(&mut canvas, (0, 0), (4, 0), LineKind::Light);

        canvas.text(Point::new(1, 0), "ab", Part::Label);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "─ab──");
    }

    #[test]
    fn a_line_replaces_text() {
        let mut canvas = canvas(4, 1);
        canvas.text(Point::new(0, 0), "ab", Part::Label);

        line(&mut canvas, (1, 0), (3, 0), LineKind::Double);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "a═══");
    }

    #[test]
    fn an_arrow_stays_whole_under_a_line_drawn_after_it() {
        let mut canvas = canvas(4, 1);
        canvas.tip(Point::new(3, 0), Direction::Right, Tip::Arrow, Part::Edge);

        line(&mut canvas, (0, 0), (3, 0), LineKind::Light);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "───▸");
    }

    #[test]
    fn a_cross_ends_a_line_and_stays_whole_under_another() {
        let mut canvas = canvas(4, 1);
        canvas.tip(Point::new(3, 0), Direction::Right, Tip::Cross, Part::Edge);

        line(&mut canvas, (0, 0), (3, 0), LineKind::Light);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "───╳");
        assert_eq!(render(&canvas, GlyphSet::Ascii), "---x");
    }

    /// Each tip in each direction, one row per direction: `Tip::ALL` across, with a bar on a line of its own.
    fn tips_facing_each_way() -> Canvas<Part> {
        let mut canvas = canvas(8, 4);
        for (y, direction) in (0..).zip(Direction::ALL) {
            for (x, &tip) in (0..).zip(Tip::ALL) {
                canvas.tip(Point::new(x, y), direction, tip, Part::Edge);
            }
        }
        canvas
    }

    #[test]
    fn arrows_triangles_and_forks_face_the_node_their_edge_goes_into() {
        let canvas = tips_facing_each_way();

        assert_eq!(render(&canvas, GlyphSet::Unicode), "▴▵⬦⬥o┼V╳\n▸▹⬦⬥o┼<╳\n▾▿⬦⬥o┼Ʌ╳\n◂◃⬦⬥o┼>╳");
    }

    #[test]
    fn tips_in_ascii_keep_their_direction() {
        let canvas = tips_facing_each_way();

        assert_eq!(render(&canvas, GlyphSet::Ascii), "^^o#o+vx\n>>o#o+<x\nvvo#o+^x\n<<o#o+>x");
    }

    #[test]
    fn a_tip_on_a_line_stays_whole_but_a_bar_merges_with_it() {
        let mut canvas = canvas(4, 1);
        line(&mut canvas, (0, 0), (3, 0), LineKind::Light);

        canvas.tip(Point::new(1, 0), Direction::Left, Tip::Bar, Part::Edge);
        canvas.tip(Point::new(2, 0), Direction::Left, Tip::Triangle, Part::Edge);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "─┼◃─");
    }

    #[test]
    fn a_bar_crosses_the_end_of_a_line() {
        let mut canvas = canvas(3, 1);
        line(&mut canvas, (0, 0), (1, 0), LineKind::Light);

        canvas.tip(Point::new(1, 0), Direction::Right, Tip::Bar, Part::Edge);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "─┼ ");
    }

    #[test]
    fn a_bar_crosses_a_line_in_its_kind() {
        let mut canvas = canvas(3, 2);
        line(&mut canvas, (0, 0), (2, 0), LineKind::Heavy);
        line(&mut canvas, (0, 1), (2, 1), LineKind::Double);

        canvas.tip(Point::new(1, 0), Direction::Right, Tip::Bar, Part::Edge);
        canvas.tip(Point::new(1, 1), Direction::Left, Tip::Bar, Part::Edge);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "━╋━\n═╬═");
    }

    #[test]
    fn a_tip_drawn_on_a_symbol_or_a_mark_replaces_it() {
        let mut canvas = canvas(5, 3);
        canvas.symbol(Point::new(0, 1), Symbol::Dot, Part::Node);
        canvas.shape(Rect::new(1, 0, 4, 3), Shape::Stadium, Part::Node);

        canvas.tip(Point::new(0, 1), Direction::Right, Tip::Arrow, Part::Edge);
        canvas.tip(Point::new(1, 1), Direction::Right, Tip::Arrow, Part::Edge);

        assert_eq!(render(&canvas, GlyphSet::Unicode), " ╭──╮\n▸▸  )\n ╰──╯");
    }

    #[test]
    fn a_line_ending_on_a_frame_leaves_the_junction_in_the_frame_role() {
        let mut canvas = canvas(5, 4);
        canvas.frame(Rect::new(0, 1, 5, 3), LineKind::Light, Corner::Square, Part::Node);

        line(&mut canvas, (2, 0), (2, 1), LineKind::Light);

        assert_eq!(styles(&canvas, GlyphSet::Unicode)[1], "[┌─┴─┐ Node on None]");
    }

    #[test]
    fn a_line_crossing_a_frame_takes_the_junction_in_its_role() {
        let mut canvas = canvas(5, 4);
        canvas.frame(Rect::new(0, 1, 5, 3), LineKind::Light, Corner::Square, Part::Node);

        line(&mut canvas, (2, 0), (2, 3), LineKind::Light);

        assert_eq!(styles(&canvas, GlyphSet::Unicode)[1], "[┌─ Node on None][┼ Edge on None][─┐ Node on None]");
    }

    #[test]
    fn a_symbol_fills_its_cell_and_stays_whole_under_a_line_leaving_it() {
        let mut canvas = canvas(3, 3);
        canvas.symbol(Point::new(0, 0), Symbol::Dot, Part::Node);
        canvas.symbol(Point::new(2, 0), Symbol::RingedDot, Part::Node);
        canvas.symbol(Point::new(1, 2), Symbol::FilledDiamond, Part::Node);

        line(&mut canvas, (0, 0), (0, 2), LineKind::Light);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "⬤ ◉\n│  \n│⬥ ");
        assert_eq!(render(&canvas, GlyphSet::Ascii), "* @\n|  \n|? ");
    }

    #[test]
    fn a_line_with_both_ends_on_one_cell_draws_nothing() {
        let mut canvas = canvas(1, 1);
        canvas.text(Point::new(0, 0), "a", Part::Label);

        line(&mut canvas, (0, 0), (0, 0), LineKind::Light);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "a");
    }

    #[test]
    fn a_path_turns_a_corner_at_each_inner_point() {
        let mut canvas = canvas(4, 3);
        let points = [Point::new(0, 0), Point::new(3, 0), Point::new(3, 2), Point::new(1, 2)];

        canvas.path(&points, LineKind::Light, Part::Edge).unwrap();

        assert_eq!(render(&canvas, GlyphSet::Unicode), "───┐\n   │\n ──┘");
    }

    #[test]
    fn a_path_with_a_diagonal_step_draws_nothing() {
        let mut canvas = canvas(3, 3);
        let points = [Point::new(0, 0), Point::new(2, 0), Point::new(0, 2)];

        let result = canvas.path(&points, LineKind::Light, Part::Edge);

        assert_eq!(result, Err(Error::Diagonal { from: points[1], to: points[2] }));
        assert_eq!(render(&canvas, GlyphSet::Unicode), "   \n   \n   ");
    }

    #[test]
    fn crossing_lines_merge_into_a_junction() {
        let mut canvas = canvas(3, 3);

        line(&mut canvas, (0, 1), (2, 1), LineKind::Light);
        line(&mut canvas, (1, 2), (1, 0), LineKind::Heavy);

        assert_eq!(render(&canvas, GlyphSet::Unicode), " ┃ \n─╂─\n ┃ ");
    }

    #[test]
    fn lines_meeting_at_their_ends_turn_a_corner() {
        let mut canvas = canvas(3, 3);

        line(&mut canvas, (0, 0), (2, 0), LineKind::Dashed);
        line(&mut canvas, (2, 0), (2, 2), LineKind::Dashed);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "╌╌┐\n  ╎\n  ╎");
    }

    #[test]
    fn a_line_ending_on_a_frame_joins_it() {
        let mut canvas = canvas(5, 5);
        canvas.frame(Rect::new(0, 2, 5, 3), LineKind::Light, Corner::Square, Part::Node);

        line(&mut canvas, (2, 0), (2, 2), LineKind::Light);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "  │  \n  │  \n┌─┴─┐\n│   │\n└───┘");
    }

    #[test]
    fn the_last_frame_on_a_corner_sets_whether_it_is_rounded() {
        let mut canvas = canvas(6, 2);
        canvas.frame(Rect::new(0, 0, 3, 2), LineKind::Light, Corner::Rounded, Part::Node);
        canvas.frame(Rect::new(3, 0, 3, 2), LineKind::Light, Corner::Square, Part::Node);

        canvas.frame(Rect::new(0, 0, 3, 2), LineKind::Light, Corner::Square, Part::Node);
        canvas.frame(Rect::new(3, 0, 3, 2), LineKind::Light, Corner::Rounded, Part::Node);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "┌─┐╭─╮\n└─┘╰─╯");
    }

    #[test]
    fn a_frame_on_an_empty_rectangle_draws_nothing() {
        let mut canvas = canvas(2, 2);

        canvas.frame(Rect::new(0, 0, 0, 2), LineKind::Light, Corner::Square, Part::Node);
        canvas.frame(Rect::new(0, 0, 2, 0), LineKind::Light, Corner::Square, Part::Node);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "  \n  ");
    }

    #[test]
    fn frames_draw_each_line_kind_and_corner() {
        let mut canvas = canvas(17, 3);
        let kinds = [LineKind::Light, LineKind::Heavy, LineKind::Double, LineKind::Dashed];

        for (x, kind) in (0..).step_by(4).zip(kinds) {
            canvas.frame(Rect::new(x, 0, 4, 3), kind, Corner::Rounded, Part::Node);
        }
        canvas.frame(Rect::new(16, 0, 1, 3), LineKind::Light, Corner::Square, Part::Node);

        insta::assert_snapshot!(render(&canvas, GlyphSet::Unicode));
    }

    #[test]
    fn the_ascii_set_draws_lines_junctions_and_arrows() {
        let mut canvas = canvas(9, 3);
        canvas.frame(Rect::new(0, 0, 4, 3), LineKind::Light, Corner::Rounded, Part::Node);
        line(&mut canvas, (3, 1), (8, 1), LineKind::Dashed);
        line(&mut canvas, (6, 0), (6, 2), LineKind::Heavy);

        canvas.tip(Point::new(8, 1), Direction::Right, Tip::Arrow, Part::Edge);

        assert_eq!(render(&canvas, GlyphSet::Ascii), ".--.  |  \n|  +..+.>\n'--'  |  ");
    }

    #[test]
    fn an_edge_ending_on_a_corner_leaves_it_in_the_frame_role() {
        let mut canvas = canvas(4, 3);
        canvas.frame(Rect::new(1, 0, 3, 3), LineKind::Light, Corner::Rounded, Part::Node);

        line(&mut canvas, (0, 0), (1, 0), LineKind::Light);

        assert_eq!(styles(&canvas, GlyphSet::Unicode)[0], "[─ Edge on None][┬─╮ Node on None]");
    }

    #[test]
    fn a_frame_redrawn_in_another_role_is_wholly_in_that_role() {
        let mut canvas = canvas(5, 4);
        canvas.frame(Rect::new(0, 1, 5, 3), LineKind::Light, Corner::Square, Part::Node);
        line(&mut canvas, (2, 0), (2, 1), LineKind::Light);
        line(&mut canvas, (0, 0), (0, 1), LineKind::Light);

        canvas.frame(Rect::new(0, 1, 5, 3), LineKind::Light, Corner::Square, Part::Series(0));

        let styles = styles(&canvas, GlyphSet::Unicode);
        assert_eq!([&styles[1], &styles[3]], ["[├─┴─┐ Series(0) on None]", "[└───┘ Series(0) on None]"]);
    }

    #[test]
    fn a_shape_redrawn_in_another_role_is_wholly_in_that_role() {
        let mut canvas = canvas(6, 3);
        canvas.shape(Rect::new(0, 0, 6, 3), Shape::Cylinder, Part::Node);

        canvas.shape(Rect::new(0, 0, 6, 3), Shape::Cylinder, Part::Series(0));

        assert_eq!(
            styles(&canvas, GlyphSet::Unicode),
            ["[╭────╮ Series(0) on None]", "[├────┤ Series(0) on None]", "[╰────╯ Series(0) on None]"]
        );
    }

    #[test]
    fn a_frame_one_cell_wide_sets_no_corner() {
        let mut canvas = canvas(3, 3);
        canvas.frame(Rect::new(0, 0, 3, 3), LineKind::Light, Corner::Square, Part::Node);

        canvas.frame(Rect::new(0, 0, 1, 1), LineKind::Light, Corner::Rounded, Part::Node);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "┌─┐\n│ │\n└─┘");
    }

    #[test]
    fn a_bar_replaces_a_tip_on_its_cell() {
        let mut canvas = canvas(1, 1);
        canvas.tip(Point::new(0, 0), Direction::Right, Tip::Arrow, Part::Edge);

        canvas.tip(Point::new(0, 0), Direction::Right, Tip::Bar, Part::Edge);

        assert_eq!(render(&canvas, GlyphSet::Unicode), "┼");
    }
}
