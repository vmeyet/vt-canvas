/// A cell as it is written: its character, the role it is drawn in and the role behind it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Styled<R> {
    pub(crate) char: char,
    pub(crate) role: R,
    pub(crate) background: Option<R>,
}
