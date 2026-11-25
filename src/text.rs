use unicode_width::UnicodeWidthChar;

/// Columns `text` takes once drawn: control and zero-width characters are left out, wide characters count two.
pub fn text_width(text: &str) -> usize {
    glyphs(text).map(|glyph| glyph.width).sum()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Glyph {
    pub(crate) char: char,
    pub(crate) width: usize,
}

/// The characters of `text` a cell can hold; anything else would break the column count of its row.
pub(crate) fn glyphs(text: &str) -> impl Iterator<Item = Glyph> {
    text.chars().filter_map(|char| match char.width() {
        Some(width @ 1..=2) => Some(Glyph { char, width }),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_width_counts_wide_characters_twice() {
        assert_eq!(text_width("a漢b"), 4);
    }

    #[test]
    fn text_width_leaves_out_control_and_zero_width_characters() {
        assert_eq!(text_width("a\u{1b}[31m\tb\u{301}\u{200b}\n"), 6);
    }

    #[test]
    fn glyphs_drop_what_a_cell_cannot_hold() {
        let chars: String = glyphs("e\u{301}\u{7}x").map(|glyph| glyph.char).collect();

        assert_eq!(chars, "ex");
    }
}
