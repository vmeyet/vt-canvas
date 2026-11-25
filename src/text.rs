use unicode_width::UnicodeWidthChar;

/// Columns `text` takes once drawn: control and zero-width characters are left out, wide characters count two.
pub fn text_width(text: &str) -> usize {
    letters(text).map(|letter| usize::from(letter.width)).sum()
}

/// A character of text and the columns it takes, 1 or 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Letter {
    pub(crate) char: char,
    pub(crate) width: u8,
}

/// The characters of `text` a cell can hold; anything else would break the column count of its row.
pub(crate) fn letters(text: &str) -> impl Iterator<Item = Letter> {
    text.chars().filter_map(|char| match char.width() {
        Some(1) => Some(Letter { char, width: 1 }),
        Some(2) => Some(Letter { char, width: 2 }),
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
    fn letters_drop_what_a_cell_cannot_hold() {
        let chars: String = letters("e\u{301}\u{7}x").map(|letter| letter.char).collect();

        assert_eq!(chars, "ex");
    }
}
