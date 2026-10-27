//! Parsing for multiline strings introduced with `\\`.

use std::iter::Peekable;

use boba_token::Token;

use crate::{LexerOutput, LexerResult, LexerState};

/// Finishes the multiline string as a multiline string token.
pub fn finish() -> LexerOutput {
    Token::MultilineString.into()
}

/// Consumes the multiline string body until the end of the line.
///
/// A newline ends the string.
/// Any other character is part of the string body.
pub fn parse_next(input: &mut Peekable<impl Iterator<Item = char>>) -> LexerResult {
    while let Some(next_char) = input.peek() {
        return match next_char {
            // a newline means the string parsing is over, finish the string now
            '\r' | '\n' => finish().into(),

            // any other character is valid for the string
            _ => {
                let _ = input.next();
                continue;
            }
        };
    }

    // if we run out of chars, then the string is still pending
    LexerState::MultilineString.into()
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
        assert_eq!(parse_next(&mut input), Err(LexerState::MultilineString));
    }
}
