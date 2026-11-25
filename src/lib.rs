#![doc = include_str!("../README.md")]

mod block;
mod canvas;
mod frame;
mod geometry;
mod glyph;
mod line;
mod pixel;
mod role;
mod shape;
mod symbol;
mod text;
mod tip;
mod wedge;

pub use canvas::{Canvas, Error, MAX_CELLS, MAX_WIDTH, Run};
pub use frame::Corner;
pub use geometry::{Direction, Point, Rect};
pub use glyph::GlyphSet;
pub use line::LineKind;
pub use pixel::{Dots, Pixel};
pub use role::Role;
pub use shape::{Inset, Shape};
pub use symbol::Symbol;
pub use text::text_width;
pub use tip::Tip;
pub use wedge::{Fill, Wedge};
