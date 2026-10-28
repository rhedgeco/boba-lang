//! Parsing for newlines, including the `\r\n` sequence.

use crate::{
    LexOutput, LexResult, LexState, Trivia,
    iter::{PeekableChar, PeekableCharExt},
};

/// Finishes the newline as a newline trivia token.
pub fn finish() -> LexOutput {
    Trivia::Newline.into()
}

/// Parses the character after an initial `\r`.
///
/// A following line feed is consumed so that `\r\n` is treated as a single newline.
/// Any other character finishes the newline as is.
pub fn parse_next(mut input: impl PeekableChar) -> LexResult {
    // if the character is a line feed, consume it
    // this covers the \r\n single token newline scenario
    if input.peek_or(LexState::Newline)? == '\n' {
        input.consume();
    }

    // then complete the newline
    finish().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finish_is_newline_trivia() {
        assert_eq!(finish(), Trivia::Newline.into());
    }

    #[test]
    fn carriage_return_line_feed_is_single_newline() {
        let mut input = "\nrest".chars().peekable();
        let result = parse_next(&mut input);
        assert_eq!(result, Ok(Trivia::Newline.into()));
        // the line feed was consumed as part of the single newline
        assert_eq!(input.next(), Some('r'));
    }

    #[test]
    fn lone_carriage_return_finishes_as_newline() {
        let mut input = "x".chars().peekable();
        let result = parse_next(&mut input);
        assert_eq!(result, Ok(Trivia::Newline.into()));
        // a non line feed character is left in place
        assert_eq!(input.next(), Some('x'));
    }

    #[test]
    fn empty_input_is_pending() {
        let mut input = "".chars().peekable();
        assert_eq!(parse_next(&mut input), Err(LexState::Newline));
    }
}
