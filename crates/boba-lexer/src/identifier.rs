//! Parsing for identifiers and the character classes they allow.

use boba_token::Token;
use unicode_xid::UnicodeXID;

use crate::{
    LexOutput, LexResult, LexState,
    iter::{PeekableChar, PeekableCharExt},
};

/// Returns true if the character is a valid identifier start.
pub fn valid_start(c: char) -> bool {
    valid_ascii_start(c) || c.is_xid_start()
}

/// Returns true if the character is a valid identifier body.
pub fn valid_body(c: char) -> bool {
    valid_ascii_body(c) || c.is_xid_continue()
}

/// Returns true if the character is a valid ascii identifier start.
pub const fn valid_ascii_start(c: char) -> bool {
    matches!(c, '_' | 'a'..='z' | 'A'..='Z')
}

/// Returns true if the character is a valid ascii identifier body.
pub const fn valid_ascii_body(c: char) -> bool {
    matches!(c, '_' | 'a'..='z' | 'A'..='Z' | '0'..='9')
}

/// Finishes the identifier as an identifier token.
pub fn finish() -> LexOutput {
    Token::Identifier.into()
}

/// Consumes the identifier body until an invalid character is found.
///
/// Each valid body character is consumed.
/// The first invalid character ends the identifier and is left in place.
pub fn parse_next(mut input: impl PeekableChar) -> LexResult {
    loop {
        // if the next character is not valid, finish the identifier
        if !valid_body(input.peek_or(LexState::Identifier)?) {
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
    fn valid_ascii_start_characters() {
        for c in ['_', 'a', 'z', 'A', 'Z'] {
            assert!(valid_ascii_start(c), "{c:?} should be a valid ascii start");
        }
        // digits and symbols cannot start an identifier
        for c in ['0', '9', '-', ' ', '.'] {
            assert!(
                !valid_ascii_start(c),
                "{c:?} should not be a valid ascii start"
            );
        }
    }

    #[test]
    fn valid_ascii_body_characters() {
        for c in ['_', 'a', 'Z', '0', '9'] {
            assert!(
                valid_ascii_body(c),
                "{c:?} should be a valid ascii body char"
            );
        }
        // symbols and whitespace are not valid within an identifier
        for c in ['-', ' ', '.', '/'] {
            assert!(
                !valid_ascii_body(c),
                "{c:?} should not be a valid ascii body char"
            );
        }
    }

    #[test]
    fn valid_start_allows_ascii_and_underscore() {
        // ascii letters and underscore may start an identifier
        for c in ['_', 'a', 'Z'] {
            assert!(valid_start(c), "{c:?} should be a valid start");
        }
        // digits and symbols may not
        for c in ['0', '9', '-', ' ', '.'] {
            assert!(!valid_start(c), "{c:?} should not be a valid start");
        }
    }

    #[test]
    fn valid_body_allows_ascii_and_digits() {
        for c in ['_', 'a', 'Z', '0', '9'] {
            assert!(valid_body(c), "{c:?} should be a valid body char");
        }
        for c in ['-', ' ', '.', '/'] {
            assert!(!valid_body(c), "{c:?} should not be a valid body char");
        }
    }

    #[test]
    fn unicode_letters_are_valid() {
        // non-ascii letters follow XID and may start or continue an identifier
        for c in ['é', 'ñ', 'λ', '中', 'Ω'] {
            assert!(valid_start(c), "{c:?} should be a valid start");
            assert!(valid_body(c), "{c:?} should be a valid body char");
        }
    }

    #[test]
    fn digits_continue_but_do_not_start() {
        // ascii digits may appear in the body but not at the start
        for c in ['0', '5', '9'] {
            assert!(!valid_start(c), "{c:?} should not be a valid start");
            assert!(valid_body(c), "{c:?} should be a valid body char");
        }
    }

    #[test]
    fn emoji_and_symbols_are_not_valid() {
        // XID does not include emoji or symbols; supporting emoji would be additive
        for c in ['🦀', '😀', '❤', '§'] {
            assert!(!valid_start(c), "{c:?} should not be a valid start");
            assert!(!valid_body(c), "{c:?} should not be a valid body char");
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
    fn unicode_identifier_body_is_consumed() {
        // a full Unicode identifier is consumed up to the boundary
        let mut input = "café中 rest".chars().peekable();
        let result = parse_next(&mut input);
        assert_eq!(result, Ok(Token::Identifier.into()));
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
        assert_eq!(parse_next(&mut input), Err(LexState::Identifier));
    }
}
