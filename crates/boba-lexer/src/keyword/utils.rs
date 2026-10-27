//! Helpers for describing keywords as validated identifier strings.

use std::num::NonZeroUsize;

use crate::{
    LexerOutput, LexerResult, LexerState,
    identifier::{self, is_valid_body, is_valid_start},
    iter::PeekableChar,
};

/// A keyword whose text is validated as a legal identifier.
///
/// Every character is a valid identifier character, which is always ASCII.
/// Because the text is guaranteed ASCII, each byte is exactly one character,
/// so a byte index and a character index are always the same here.
///
/// This struct is capable of being built at compile time for any &'static str.
pub struct KeywordStr<'a> {
    length: NonZeroUsize,
    value: &'a str,
}

impl<'a> KeywordStr<'a> {
    /// Returns the length of the keyword.
    ///
    /// Because the text is ASCII, this is both the byte length and the character length.
    pub const fn len(&self) -> NonZeroUsize {
        self.length
    }

    /// Returns the character at the given index, if one exists.
    ///
    /// The index counts ASCII characters, which are each a single byte.
    pub const fn get(&self, index: usize) -> Option<char> {
        let bytes = self.value.as_bytes();
        if index >= bytes.len() {
            return None;
        }

        // each byte is a single ASCII character, so this is always the real character
        Some(bytes[index] as char)
    }

    /// Builds a keyword, returning `None` if the text is not a valid identifier.
    ///
    /// The first character must be a valid identifier start.
    /// Every remaining character must be a valid identifier body character.
    /// Every byte must also be ASCII,
    /// so that each byte is guaranteed to be exactly one character.
    pub const fn build(value: &'a str) -> Option<Self> {
        // ensure the keyword is not empty
        let bytes = value.as_bytes();
        let Some(length) = NonZeroUsize::new(bytes.len()) else {
            return None;
        };

        // enforce ascii so this byte is exactly one character
        let first_byte = bytes[0];
        if !first_byte.is_ascii() {
            return None;
        }

        // ensure the character is valid as an identifier start
        if !is_valid_start(first_byte as char) {
            return None;
        }

        // check each consecutive byte to see if it is a valid ascii body char
        let mut index = 1;
        while index < bytes.len() {
            // enforce ascii so this byte is exactly one character
            let byte = bytes[index];
            if !byte.is_ascii() {
                return None;
            }

            // ensure the character is valid within an identifier body
            if !is_valid_body(byte as char) {
                return None;
            }

            index += 1;
        }

        // if all pass, then we have a valid keyword
        Some(Self { length, value })
    }
}

/// Continues matching a keyword from the given matched length.
///
/// The length is the number of characters already matched,
/// which is never zero since the first character is matched before this is called.
/// Each character that still matches the keyword is consumed.
/// A diverging character finishes the run as an identifier instead.
/// A completed keyword followed by a boundary finishes with `finish`.
/// If the input runs out, the keyword is still pending via `pending`.
pub fn parse_next_keyword(
    keyword: &KeywordStr,
    mut length: NonZeroUsize,
    pending: impl Fn(NonZeroUsize) -> LexerState,
    finish: impl Fn(NonZeroUsize) -> LexerOutput,
    mut input: impl PeekableChar,
) -> LexerResult {
    loop {
        return match (input.peek(), keyword.get(length.get())) {
            // the input ran out, so park as pending until more input or a final finish
            // we cannot commit to the keyword here since later chars may extend it
            (None, _) => pending(length).into(),
            // the next char diverges mid keyword, so this is an identifier
            (Some(char), Some(expected)) if char != expected => identifier::parse_next(input),
            // the keyword is complete but a body char follows, so this is a longer identifier
            (Some(char), None) if is_valid_body(char) => identifier::parse_next(input),
            // the keyword is complete and a boundary follows, so finish the keyword
            (Some(_), None) => finish(length).into(),
            // the next char continues the keyword, so consume it and move on
            (Some(_), Some(_)) => {
                let _ = input.next();
                length = length.saturating_add(1);
                continue;
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_accepts_valid_keyword() {
        let keyword = KeywordStr::build("const").unwrap();
        assert_eq!(keyword.len(), NonZeroUsize::new(5).unwrap());
        assert_eq!(keyword.get(0), Some('c'));
        assert_eq!(keyword.get(4), Some('t'));
        // indexing past the end yields nothing
        assert_eq!(keyword.get(5), None);
    }

    #[test]
    fn build_rejects_empty() {
        assert!(KeywordStr::build("").is_none());
    }

    #[test]
    fn build_rejects_bad_start() {
        // a digit cannot start an identifier
        assert!(KeywordStr::build("1abc").is_none());
    }

    #[test]
    fn build_rejects_bad_body() {
        // a dash is not a valid identifier body character
        assert!(KeywordStr::build("a-b").is_none());
    }

    #[test]
    fn build_rejects_non_ascii() {
        // non ascii characters must be rejected to keep byte and char indices aligned
        assert!(KeywordStr::build("é").is_none());
        assert!(KeywordStr::build("añ").is_none());
    }

    // a keyword used to exercise parse_next_keyword independently of the boba keywords
    const WORD: KeywordStr<'static> = KeywordStr::build("word").unwrap();
    const WORD_TOKEN: boba_token::Token = boba_token::Token::Public;

    // runs parse_next_keyword against WORD, starting at the given matched length
    // the pending state and completion token are stand ins for a real caller
    fn parse_word(length: NonZeroUsize, rest: &str) -> LexerResult {
        parse_next_keyword(
            &WORD,
            length,
            |_| LexerState::Identifier,
            |_| WORD_TOKEN.into(),
            rest.chars().peekable(),
        )
    }

    #[test]
    fn parse_next_keyword_completes_on_boundary() {
        // the first letter is already consumed, so start at length 1
        // the rest matches and a boundary follows, completing the keyword
        let result = parse_word(NonZeroUsize::MIN, "ord ");
        assert_eq!(result, Ok(WORD_TOKEN.into()));
    }

    #[test]
    fn parse_next_keyword_pending_at_eof() {
        // the rest matches but the input ends, so it stays pending
        let result = parse_word(NonZeroUsize::MIN, "ord");
        assert_eq!(result, Err(LexerState::Identifier));
    }

    #[test]
    fn parse_next_keyword_diverges_to_identifier() {
        // "work" diverges from "word" at the final character
        let result = parse_word(NonZeroUsize::MIN, "ork ");
        assert_eq!(result, Ok(boba_token::Token::Identifier.into()));
    }

    #[test]
    fn parse_next_keyword_longer_run_is_identifier() {
        // "words" matches the keyword then continues, making it a longer identifier
        let result = parse_word(NonZeroUsize::MIN, "ords ");
        assert_eq!(result, Ok(boba_token::Token::Identifier.into()));
    }

    #[test]
    fn parse_next_keyword_leaves_boundary_in_place() {
        // the boundary character that completes the keyword is not consumed
        let mut input = "ord.".chars().peekable();
        let _ = parse_next_keyword(
            &WORD,
            NonZeroUsize::MIN,
            |_| LexerState::Identifier,
            |_| WORD_TOKEN.into(),
            &mut input,
        );
        assert_eq!(input.next(), Some('.'));
    }
}
