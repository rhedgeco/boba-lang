//! Parsing for line comments introduced with `//`.

use crate::{
    LexOutput, LexResult, LexState, Trivia,
    iter::{PeekableChar, PeekableCharExt},
};

/// Finishes the comment as a comment trivia token.
#[must_use]
pub fn finish() -> LexOutput {
    Trivia::Comment.into()
}

/// Consumes the comment body until the end of the line.
///
/// A newline ends the comment.
/// Any other character is consumed as part of the comment body.
///
/// # Errors
/// Returns the resumable [`LexState`] if the input is exhausted mid-token.
pub fn parse_next(mut input: impl PeekableChar) -> LexResult {
    loop {
        // try to peek the next char, returning a pending comment if failed
        match input.peek_or(LexState::Comment)? {
            // if the character is a newline, return the finished comment
            '\r' | '\n' => return finish().into(),
            // otherwise consume and try again
            _ => input.consume(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finish_is_comment_trivia() {
        assert_eq!(finish(), Trivia::Comment.into());
    }

    #[test]
    fn newline_ends_comment() {
        for newline in ['\n', '\r'] {
            let mut input = [newline].into_iter().peekable();
            let result = parse_next(&mut input);
            assert_eq!(result, Ok(Trivia::Comment.into()));
            // the newline is left in place for the next token
            assert_eq!(input.next(), Some(newline));
        }
    }

    #[test]
    fn body_is_consumed_until_newline() {
        let mut input = "abc\n".chars().peekable();
        let result = parse_next(&mut input);
        assert_eq!(result, Ok(Trivia::Comment.into()));
        // everything up to the newline was consumed
        assert_eq!(input.next(), Some('\n'));
    }

    #[test]
    fn empty_input_is_pending() {
        let mut input = "".chars().peekable();
        assert_eq!(parse_next(&mut input), Err(LexState::Comment));
    }
}
