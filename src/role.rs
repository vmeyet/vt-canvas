/// What a cell is drawn for, as the caller names it: the canvas keeps one per cell and hands it back, the caller maps it to a colour.
///
/// Any small enum of the caller's does, its [`Default`] being the role of empty cells.
pub trait Role: Copy + Eq + Default {
    /// Which ASCII texture fills and dots in this role take, so that roles differ without colour: 0 unless told otherwise.
    ///
    /// A filled cell cycles through `# % * o @ x`; dots are `' . :` for an even texture and `" , ;` for an odd one.
    fn texture(self) -> u8 {
        0
    }
}

/// A role can be a plain number, its texture the number itself.
impl Role for u8 {
    fn texture(self) -> u8 {
        self
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::Role;

    /// The roles the tests draw in, as a diagram or a chart names them.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub(crate) enum Part {
        #[default]
        Plain,
        Node,
        Edge,
        Label,
        /// A chart's series, its ASCII texture its number.
        Series(u8),
    }

    impl Role for Part {
        fn texture(self) -> u8 {
            match self {
                Self::Series(series) => series,
                Self::Plain | Self::Node | Self::Edge | Self::Label => 0,
            }
        }
    }
}
