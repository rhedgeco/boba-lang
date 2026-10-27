//! Parsing for inline string literals delimited by double quotes.

use std::iter::Peekable;

use boba_token::Token;

use crate::{LexerOutput, LexerResult, LexerState, Trivia};

/// Finishes a string that was never closed as unclosed string trivia.
pub fn finish_unclosed() -> LexerOutput {
    Trivia::UnclosedStr.into()
}

/// Finishes a properly closed string as an inline string token.
pub fn finish() -> LexerOutput {
    Token::InlineString.into()
}

/// Parses the character following a back slash escape within a string.
///
/// A newline cannot be escaped and leaves the string unclosed.
/// Any other character is accepted and parsing returns to the string body.
pub fn parse_next_escape(input: &mut Peekable<impl Iterator<Item = char>>) -> LexerResult {
    match input.peek().ok_or(LexerState::InlineEscape)? {
        // a newline cannot be escaped, finish the string now
        '\r' | '\n' => finish_unclosed().into(),
        // otherwise any other character is valid here
        // and we go back to parsing the inline string
        _ => {
            let _ = input.next();
            parse_next(input)
        }
    }
}

/// Consumes the string body until it closes or the line ends.
///
/// A quote closes the string.
/// A back slash begins an escape.
/// A newline leaves the string unclosed.
/// Any other character is part of the string body.
pub fn parse_next(input: &mut Peekable<impl Iterator<Item = char>>) -> LexerResult {
    while let Some(next_char) = input.peek() {
        return match next_char {
            // a newline means the string parsing is over, finish the string now
            '\r' | '\n' => finish_unclosed().into(),

            // start an escape sequence if we find a backslash
            '\\' => {
                let _ = input.next();
                parse_next_escape(input)
            }

            // close the valid string if we find a quote
            '"' => {
                let _ = input.next();
                finish().into()
            }

            // any other character is valid for the string
            _ => {
                let _ = input.next();
                continue;
            }
        };
    }

    // if we run out of chars, then the string is still pending
    LexerState::InlineString.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finish_unclosed_is_trivia() {
        assert_eq!(finish_unclosed(), Trivia::UnclosedStr.into());
    }

    #[test]
    fn finish_is_inline_string() {
        assert_eq!(finish(), Token::InlineString.into());
    }

    #[test]
    fn quote_closes_string() {
        let mut input = "body\"rest".chars().peekable();
        let result = parse_next(&mut input);
        assert_eq!(result, Ok(Token::InlineString.into()));
        // the closing quote was consumed
        assert_eq!(input.next(), Some('r'));
    }

    #[test]
    fn newline_leaves_string_unclosed() {
        for newline in ['\n', '\r'] {
            let mut input = [newline].into_iter().peekable();
            let result = parse_next(&mut input);
            assert_eq!(result, Ok(Trivia::UnclosedStr.into()));
            // the newline is left in place for the next token
            assert_eq!(input.next(), Some(newline));
        }
    }

    #[test]
    fn escaped_char_is_accepted_and_string_continues() {
        // an escaped quote should not close the string
        let mut input = "a\\\"b\"".chars().peekable();
        let result = parse_next(&mut input);
        assert_eq!(result, Ok(Token::InlineString.into()));
    }

    #[test]
    fn empty_input_is_pending() {
        let mut input = "".chars().peekable();
        assert_eq!(parse_next(&mut input), Err(LexerState::InlineString));
    }

    #[test]
    fn escape_of_newline_is_unclosed() {
        for newline in ['\n', '\r'] {
            let mut input = [newline].into_iter().peekable();
            let result = parse_next_escape(&mut input);
            // a newline cannot be escaped, so the string is unclosed
            assert_eq!(result, Ok(Trivia::UnclosedStr.into()));
        }
    }

    #[test]
    fn escape_of_normal_char_returns_to_body() {
        // the escaped character is consumed, then the body closes on the quote
        let mut input = "n\"".chars().peekable();
        let result = parse_next_escape(&mut input);
        assert_eq!(result, Ok(Token::InlineString.into()));
    }

    #[test]
    fn escape_with_empty_input_is_pending() {
        let mut input = "".chars().peekable();
        assert_eq!(parse_next_escape(&mut input), Err(LexerState::InlineEscape));
    }
}
