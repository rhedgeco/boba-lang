//! Parsing for newlines, including the `\r\n` sequence.

use crate::{LexerOutput, LexerResult, LexerState, Trivia, iter::PeekableChar};

/// Finishes the newline as a newline trivia token.
pub fn finish() -> LexerOutput {
    Trivia::Newline.into()
}

/// Parses the character after an initial `\r`.
///
/// A following line feed is consumed so that `\r\n` is treated as a single newline.
/// Any other character finishes the newline as is.
pub fn parse_next(mut input: impl PeekableChar) -> LexerResult {
    let next_char = input.peek().ok_or(LexerState::Newline)?;

    // if the character is a line feed, consume it
    // this covers the \r\n single token newline scenario
    if next_char == '\n' {
        let _ = input.next();
    }

    // otherwise just finish the newline as is
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
        assert_eq!(parse_next(&mut input), Err(LexerState::Newline));
    }
}
