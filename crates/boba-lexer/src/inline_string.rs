//! Parsing for inline string literals delimited by double quotes.

use boba_token::Token;

use crate::{
    LexOutput, LexResult, LexState, Trivia,
    iter::{PeekableChar, PeekableCharExt},
};

/// Finishes a string that was never closed as unclosed string trivia.
#[must_use]
pub fn finish_unclosed() -> LexOutput {
    Trivia::UnclosedStr.into()
}

/// Finishes a properly closed string as an inline string token.
#[must_use]
pub fn finish() -> LexOutput {
    Token::InlineString.into()
}

/// Reports whether the next character can be escaped, without consuming it.
///
/// A newline cannot be escaped, so this returns `false`.
/// Any other character can be escaped, so this returns `true`.
///
/// # Errors
/// Returns the resumable [`LexState`] if the input has no next character.
pub fn can_escape(mut input: impl PeekableChar) -> Result<bool, LexState> {
    // try to get the next character, returning the pending escape state if there is none
    Ok(match input.peek_or(LexState::InlineEscape)? {
        // return false on newline. newlines are invalid escape chars.
        '\r' | '\n' => false,
        // any other character can be escaped
        _ => true,
    })
}

/// Parses the character following a back slash escape within a string.
///
/// A newline cannot be escaped and leaves the string unclosed.
/// Any other character is accepted and parsing returns to the string body.
///
/// # Errors
/// Returns the resumable [`LexState`] if the input is exhausted mid-token.
pub fn parse_next_escape(mut input: impl PeekableChar) -> LexResult {
    // if the next char can be escaped, consume and keep parsing the string
    if can_escape(&mut input)? {
        return input.consume_then(parse_next);
    }

    // otherwise, return an unclosed string
    finish_unclosed().into()
}

/// Consumes the string body until it closes or the line ends.
///
/// A quote closes the string.
/// A back slash begins an escape.
/// A newline leaves the string unclosed.
/// Any other character is part of the string body.
///
/// # Errors
/// Returns the resumable [`LexState`] if the input is exhausted mid-token.
pub fn parse_next(mut input: impl PeekableChar) -> LexResult {
    loop {
        match input.peek_or(LexState::InlineString)? {
            // a newline means the string parsing is over, finish and return the unclosed string now
            '\r' | '\n' => return finish_unclosed().into(),
            // close the valid string if we find a quote
            '"' => return input.consume_and(finish()).into(),
            // try to escape the next character if we find a backslash
            // if we cannot escape it, immediately return the unclosed string
            '\\' if !input.consume_then(can_escape)? => return finish_unclosed().into(),
            // any other character is valid for the string. consume it and continue
            _ => input.consume(),
        }
    }
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
    fn escaped_back_slash_is_consumed_as_the_escaped_char() {
        // the second back slash is the escaped character, so the quote still closes the string
        let mut input = "\\\\\"rest".chars().peekable();
        let result = parse_next(&mut input);
        assert_eq!(result, Ok(Token::InlineString.into()));
        // only the two back slashes and the closing quote were consumed
        assert_eq!(input.next(), Some('r'));
    }

    #[test]
    fn escaped_newline_in_body_is_unclosed() {
        // a newline following a back slash cannot be escaped, even mid body
        for newline in ['\n', '\r'] {
            let mut input = ['a', '\\', newline].into_iter().peekable();
            let result = parse_next(&mut input);
            assert_eq!(result, Ok(Trivia::UnclosedStr.into()));
            // the unescapable newline is left in place for the next token
            assert_eq!(input.next(), Some(newline));
        }
    }

    #[test]
    fn empty_input_is_pending() {
        let mut input = "".chars().peekable();
        assert_eq!(parse_next(&mut input), Err(LexState::InlineString));
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
        assert_eq!(parse_next_escape(&mut input), Err(LexState::InlineEscape));
    }

    #[test]
    fn long_body_does_not_overflow_the_stack() {
        // the body is consumed in a loop, so a huge string stays in constant stack
        // the input is generated lazily to avoid allocating the whole body
        let mut input = std::iter::repeat_n('a', 1_000_000)
            .chain(std::iter::once('"'))
            .peekable();
        assert_eq!(parse_next(&mut input), Ok(Token::InlineString.into()));
    }

    #[test]
    fn long_escape_run_does_not_overflow_the_stack() {
        // every escape is handled in place, so an all escape body also stays in constant stack
        // each unit is a back slash followed by its escaped character
        let mut input = std::iter::repeat_n(['\\', 'a'], 1_000_000)
            .flatten()
            .chain(std::iter::once('"'))
            .peekable();
        assert_eq!(parse_next(&mut input), Ok(Token::InlineString.into()));
    }
}
