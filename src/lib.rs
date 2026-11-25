#![doc = include_str!("../README.md")]

mod block;
mod canvas;
mod geometry;
mod glyph;
mod line;
mod shape;
mod text;
mod tip;
mod wedge;

pub use canvas::{Canvas, Corner, Error, Frame, MAX_CELLS, MAX_WIDTH, Role, Run};
pub use geometry::{Direction, Point, Rect};
pub use glyph::GlyphSet;
pub use line::LineKind;
pub use shape::{Inset, Marker, Shape};
pub use text::width;
pub use tip::Tip;
pub use wedge::{Fill, Wedge};
