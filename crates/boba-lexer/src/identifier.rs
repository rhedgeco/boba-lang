//! Parsing for identifiers and the character classes they allow.

use boba_token::Token;

use crate::{
    LexerOutput, LexerResult, LexerState,
    iter::{PeekableChar, PeekableCharExt},
};

/// Returns whether a character may start an identifier.
pub const fn is_valid_start(c: char) -> bool {
    matches!(c, '_' | 'a'..='z' | 'A'..='Z')
}

/// Returns whether a character may appear within an identifier body.
pub const fn is_valid_body(c: char) -> bool {
    matches!(c, '_' | 'a'..='z' | 'A'..='Z' | '0'..='9')
}

/// Finishes the identifier as an identifier token.
pub fn finish() -> LexerOutput {
    Token::Identifier.into()
}

/// Consumes the identifier body until an invalid character is found.
///
/// Each valid body character is consumed.
/// The first invalid character ends the identifier and is left in place.
pub fn parse_next(mut input: impl PeekableChar) -> LexerResult {
    loop {
        // if the next character is not valid, finish the identifier
        if !is_valid_body(input.peek_or(LexerState::Identifier)?) {
            return finish().into();
        }

        // otherwise consume the character and continue
        input.consume();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_start_characters() {
        for c in ['_', 'a', 'z', 'A', 'Z'] {
            assert!(is_valid_start(c), "{c:?} should be a valid start");
        }
        // digits and symbols cannot start an identifier
        for c in ['0', '9', '-', ' ', '.'] {
            assert!(!is_valid_start(c), "{c:?} should not be a valid start");
        }
    }

    #[test]
    fn valid_body_characters() {
        for c in ['_', 'a', 'Z', '0', '9'] {
            assert!(is_valid_body(c), "{c:?} should be a valid body char");
        }
        // symbols and whitespace are not valid within an identifier
        for c in ['-', ' ', '.', '/'] {
            assert!(!is_valid_body(c), "{c:?} should not be a valid body char");
        }
    }

    #[test]
    fn finish_is_identifier() {
        assert_eq!(finish(), Token::Identifier.into());
    }

    #[test]
    fn body_is_consumed_until_invalid_char() {
        let mut input = "name1 rest".chars().peekable();
        let result = parse_next(&mut input);
        assert_eq!(result, Ok(Token::Identifier.into()));
        // the first invalid character is left in place
        assert_eq!(input.next(), Some(' '));
    }

    #[test]
    fn invalid_first_char_finishes_immediately() {
        let mut input = " rest".chars().peekable();
        let result = parse_next(&mut input);
        assert_eq!(result, Ok(Token::Identifier.into()));
        assert_eq!(input.next(), Some(' '));
    }

    #[test]
    fn empty_input_is_pending() {
        let mut input = "".chars().peekable();
        assert_eq!(parse_next(&mut input), Err(LexerState::Identifier));
    }
}
