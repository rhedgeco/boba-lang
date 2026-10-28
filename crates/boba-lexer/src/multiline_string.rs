//! Parsing for multiline strings introduced with `\\`.

use boba_token::Token;

use crate::{
    LexOutput, LexResult, LexState,
    iter::{PeekableChar, PeekableCharExt},
};

/// Finishes the multiline string as a multiline string token.
pub fn finish() -> LexOutput {
    Token::MultilineString.into()
}

/// Consumes the multiline string body until the end of the line.
///
/// A newline ends the string.
/// Any other character is part of the string body.
pub fn parse_next(mut input: impl PeekableChar) -> LexResult {
    loop {
        // try to peek the next char, returning a pending string if failed
        match input.peek_or(LexState::MultilineString)? {
            // if the character is a newline, return the finished string
            '\r' | '\n' => return finish().into(),
            // otherwise consume and try again
            _ => input.consume(),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finish_is_multiline_string() {
        assert_eq!(finish(), Token::MultilineString.into());
    }

    #[test]
    fn newline_ends_string() {
        for newline in ['\n', '\r'] {
            let mut input = [newline].into_iter().peekable();
            let result = parse_next(&mut input);
            assert_eq!(result, Ok(Token::MultilineString.into()));
            // the newline is left in place for the next token
            assert_eq!(input.next(), Some(newline));
        }
    }

    #[test]
    fn body_is_consumed_until_newline() {
        let mut input = "hello world\n".chars().peekable();
        let result = parse_next(&mut input);
        assert_eq!(result, Ok(Token::MultilineString.into()));
        assert_eq!(input.next(), Some('\n'));
    }

    #[test]
    fn empty_input_is_pending() {
        let mut input = "".chars().peekable();
        assert_eq!(parse_next(&mut input), Err(LexState::MultilineString));
    }
}
