//! Parsing for line comments introduced with `//`.

use std::iter::Peekable;

use crate::{LexerOutput, LexerResult, LexerState, Trivia};

/// Finishes the comment as a comment trivia token.
pub fn finish() -> LexerOutput {
    Trivia::Comment.into()
}

/// Consumes the comment body until the end of the line.
///
/// A newline ends the comment.
/// Any other character is consumed as part of the comment body.
pub fn parse_next(input: &mut Peekable<impl Iterator<Item = char>>) -> LexerResult {
    while let Some(next_char) = input.peek() {
        return match next_char {
            // if the character is a newline, finish the comment
            '\r' | '\n' => finish().into(),
            // otherwise keep consuming until the end of line
            _ => {
                let _ = input.next();
                continue;
            }
        };
    }

    // if we run out of chars, then the comment is still pending
    LexerState::Comment.into()
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
        assert_eq!(parse_next(&mut input), Err(LexerState::Comment));
    }
}
