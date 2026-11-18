//! UTF-16 offsets, as UIKit's text views count them, and Rune's byte offsets in the same text.

/// The byte offset in `text` of UTF-16 offset `utf16` (clamped to its end).
pub(crate) fn byte_of(text: &str, utf16: usize) -> usize {
    let mut units = 0;
    for (i, c) in text.char_indices() {
        if units >= utf16 { return i; }
        units += c.len_utf16();
    }
    text.len()
}

/// The UTF-16 offset in `text` of byte offset `byte`.
pub(crate) fn utf16_of(text: &str, byte: usize) -> usize {
    let byte = byte.min(text.len());
    text.char_indices().take_while(|(i, _)| *i < byte).map(|(_, c)| c.len_utf16()).sum()
}

#[cfg(test)]
mod tests {
    use super::{byte_of, utf16_of};

    #[test]
    fn utf16_and_byte_offsets_round_trip_past_wide_characters() {
        let text = "a😀é b";
        assert_eq!(byte_of(text, 1), 1);
        assert_eq!(byte_of(text, 3), 5, "the emoji is two UTF-16 units and four bytes");
        assert_eq!(byte_of(text, 4), 7);
        assert_eq!(byte_of(text, 99), text.len());
        for (i, _) in text.char_indices() { assert_eq!(byte_of(text, utf16_of(text, i)), i); }
    }
}
