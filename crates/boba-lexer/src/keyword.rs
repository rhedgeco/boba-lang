use boba_token::Token;

use crate::{
    LexOutput, LexResult, LexState, identifier,
    iter::{PeekableChar, PeekableCharExt},
};

// useful keyword compile time constants
pub const PUB_FIRST: char = PUB.next_char().unwrap();
pub const VAR_FIRST: char = VAR.next_char().unwrap();
pub const CONST_FIRST: char = CONST.next_char().unwrap();
pub const PUB: PendingKeyword = PendingKeyword::build("pub", Token::Public).unwrap();
pub const VAR: PendingKeyword = PendingKeyword::build("var", Token::Variable).unwrap();
pub const CONST: PendingKeyword = PendingKeyword::build("const", Token::Constant).unwrap();

/// A keyword that is partway through being matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PendingKeyword {
    pending: &'static str,
    token: Token,
}

impl PendingKeyword {
    /// Returns the length of the pending keyword characters
    pub const fn len(&self) -> usize {
        self.pending.len()
    }

    /// Returns true if there are no pending characters left to match.
    pub const fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Returns the first pending character in the keyword if it exists.
    pub const fn next_char(&self) -> Option<char> {
        match self.pending.as_bytes() {
            [b, ..] => Some(*b as char),
            [] => None,
        }
    }

    /// Finishes the keyword and produces a token.
    ///
    /// Produces the keyword's token if there are no pending characters.
    /// Otherwise, an identifier token is produced.
    pub const fn finish(self) -> LexOutput {
        if self.is_empty() {
            return LexOutput::Token(self.token);
        }

        LexOutput::Token(Token::Identifier)
    }

    /// Skips the next pending character in the parse sequence.
    pub fn skip_next(mut self) -> Self {
        self.pending = self.pending.get(1..).unwrap_or("");
        self
    }

    /// Continues parsing the keyword using the `input` characters.
    pub fn parse_next(mut self, mut input: impl PeekableChar) -> LexResult
    where
        Self: Into<LexState>,
    {
        loop {
            // if the next character is valid, keep parsing the keyword
            let next_char = input.peek_or(self.into())?;
            if Some(next_char) == self.next_char() {
                self = self.skip_next();
                input.consume();
                continue;
            }

            // if the next character is not a valid identifier body, finish the keyword
            if !identifier::valid_body(next_char) {
                return self.finish().into();
            }

            // otherwise, consume the character and finish the identifier
            return input.consume_then(identifier::parse_next);
        }
    }

    /// Builds a keyword, returning `None` if the text is not valid.
    ///
    /// # Requirements
    /// - Every byte must be a valid ascii character.
    /// - The first byte must be a valid ascii identifier start character.
    /// - Every remaining byte must be a valid ascii identifier body character.
    pub const fn build(keyword: &'static str, token: Token) -> Option<Self> {
        // ensure the keyword is not empty
        let bytes = keyword.as_bytes();
        if bytes.is_empty() {
            return None;
        }

        // get the first byte and ensure its ascii
        let first_byte = bytes[0];
        if !first_byte.is_ascii() {
            return None;
        }

        // ensure the byte is a valid ascii identifier start character
        if !identifier::valid_ascii_start(first_byte as char) {
            return None;
        }

        // each consecutive byte should also be a valid ascii identifier body character
        let mut index = 1;
        while index < bytes.len() {
            // enforce ascii so this byte is exactly one character
            let next_byte = bytes[index];
            if !next_byte.is_ascii() {
                return None;
            }

            // ensure the byte is a valid identifier body character
            if !identifier::valid_ascii_body(next_byte as char) {
                return None;
            }

            index += 1;
        }

        // if all pass, then we have a valid keyword
        Some(Self {
            pending: keyword,
            token,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // compile-time proof that every shipped keyword is ASCII-only
    const _: () = assert!(PUB.pending.is_ascii());
    const _: () = assert!(VAR.pending.is_ascii());
    const _: () = assert!(CONST.pending.is_ascii());

    #[test]
    fn build_rejects_non_ascii_keywords() {
        // a sweep of non-ascii inputs in various positions must all be rejected
        let non_ascii = [
            "á",       // non-ascii start
            "aé",      // non-ascii in the body
            "abç",     // non-ascii at the end
            "_ü_",     // non-ascii in the middle
            "é",       // single non-ascii char
            "a\u{80}", // first non-ascii code point
        ];
        for keyword in non_ascii {
            assert_eq!(
                PendingKeyword::build(keyword, Token::Public),
                None,
                "{keyword:?} is not ascii and must be rejected"
            );
        }
    }

    #[test]
    fn build_only_accepts_ascii_keywords() {
        // anything build accepts must have ASCII-only pending characters
        for keyword in ["pub", "var", "const", "_x9", "a", "_"] {
            let built =
                PendingKeyword::build(keyword, Token::Public).expect("keyword should be valid");
            assert!(
                built.pending.is_ascii(),
                "{keyword:?} produced non-ascii pending characters"
            );
        }
    }

    #[test]
    fn pending_stays_ascii_while_skipping() {
        // the ASCII invariant must hold through every skip_next step
        let mut pending = CONST;
        while !pending.is_empty() {
            assert!(pending.pending.is_ascii());
            pending = pending.skip_next();
        }
        assert!(pending.pending.is_ascii());
    }

    #[test]
    fn build_rejects_invalid_keywords() {
        // empty text has no start character
        assert_eq!(PendingKeyword::build("", Token::Public), None);
        // a keyword may not start with a digit
        assert_eq!(PendingKeyword::build("1ab", Token::Public), None);
        // body characters must be valid identifier characters
        assert_eq!(PendingKeyword::build("a-b", Token::Public), None);
        // non ascii bytes are rejected
        assert_eq!(PendingKeyword::build("á", Token::Public), None);
    }

    #[test]
    fn build_accepts_valid_keywords() {
        assert!(PendingKeyword::build("pub", Token::Public).is_some());
        assert!(PendingKeyword::build("_x9", Token::Public).is_some());
    }

    #[test]
    fn len_and_is_empty_track_pending_characters() {
        assert_eq!(PUB.len(), 3);
        assert!(!PUB.is_empty());

        // skipping every character leaves nothing pending
        let matched = PUB.skip_next().skip_next().skip_next();
        assert_eq!(matched.len(), 0);
        assert!(matched.is_empty());
    }

    #[test]
    fn next_char_returns_the_leading_pending_character() {
        assert_eq!(PUB.next_char(), Some('p'));
        assert_eq!(PUB.skip_next().next_char(), Some('u'));
        // an exhausted keyword has no next character
        assert_eq!(PUB.skip_next().skip_next().skip_next().next_char(), None);
    }

    #[test]
    fn skip_next_past_the_end_stays_empty() {
        let empty = PUB.skip_next().skip_next().skip_next().skip_next();
        assert!(empty.is_empty());
        assert_eq!(empty.next_char(), None);
    }

    #[test]
    fn finish_produces_keyword_only_when_fully_matched() {
        // fully matched: the keyword token is produced
        let matched = PUB.skip_next().skip_next().skip_next();
        assert_eq!(matched.finish(), LexOutput::Token(Token::Public));
        // still pending: an identifier is produced instead
        assert_eq!(PUB.finish(), LexOutput::Token(Token::Identifier));
    }

    #[test]
    fn parse_next_completes_a_matching_keyword() {
        // the first character is already consumed by the dispatcher, so skip it
        let mut input = "ub ".chars().peekable();
        let result = PUB.skip_next().parse_next(&mut input);
        assert_eq!(result, Ok(Token::Public.into()));
        // the boundary character is left in place
        assert_eq!(input.next(), Some(' '));
    }

    #[test]
    fn parse_next_becomes_an_identifier_when_input_diverges() {
        // "public" starts like the keyword but continues into an identifier
        let mut input = "ublic ".chars().peekable();
        let result = PUB.skip_next().parse_next(&mut input);
        assert_eq!(result, Ok(Token::Identifier.into()));
        assert_eq!(input.next(), Some(' '));
    }

    #[test]
    fn parse_next_diverges_before_the_keyword_completes() {
        // "pa..." diverges on the second character and is an identifier
        let mut input = "at ".chars().peekable();
        let result = PUB.skip_next().parse_next(&mut input);
        assert_eq!(result, Ok(Token::Identifier.into()));
        assert_eq!(input.next(), Some(' '));
    }

    #[test]
    fn parse_next_parks_as_pending_at_end_of_input() {
        // running out of input before the keyword completes yields resumable state
        let mut input = "u".chars().peekable();
        let pending = PUB.skip_next();
        let result = pending.parse_next(&mut input);
        assert_eq!(result, Err(LexState::Keyword(pending.skip_next())));
    }

    #[test]
    fn parse_next_finishes_keyword_on_a_non_identifier_boundary() {
        // a boundary that is not an identifier character finishes the keyword
        let mut input = "ub.".chars().peekable();
        let result = PUB.skip_next().parse_next(&mut input);
        assert_eq!(result, Ok(Token::Public.into()));
        assert_eq!(input.next(), Some('.'));
    }

    #[test]
    fn parse_next_on_invalid_body_still_finishes_the_keyword() {
        // a trailing invalid character is left for the next parse step
        let mut input = "ub§".chars().peekable();
        let result = PUB.skip_next().parse_next(&mut input);
        assert_eq!(result, Ok(Token::Public.into()));
        assert_eq!(input.next(), Some('§'));
    }
}
