/// The corners [`Canvas::frame`](crate::Canvas::frame) draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Corner {
    /// `┌ ┐ └ ┘`
    #[default]
    Square,
    /// `╭ ╮ ╰ ╯`, square when the frame is not light or dashed, as Unicode has no other arc.
    Rounded,
}
