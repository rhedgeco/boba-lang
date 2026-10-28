//! Parsing for forward slashes, which may begin a comment.

use boba_token::Token;

use crate::{
    LexOutput, LexResult, LexState, comment,
    iter::{PeekableChar, PeekableCharExt},
};

/// Finishes a lone forward slash as a divide token.
#[must_use]
pub fn finish() -> LexOutput {
    Token::Divide.into()
}

/// Parses the character after an initial forward slash.
///
/// A following forward slash begins a comment.
/// Anything else finishes the forward slash as a divide token.
///
/// # Errors
/// Returns the resumable [`LexState`] if the input is exhausted mid-token.
pub fn parse_next(mut input: impl PeekableChar) -> LexResult {
    match input.peek_or(LexState::ForwardSlash)? {
        // if the next character is another forward slash
        // then we should start parsing the comment
        '/' => input.consume_then(comment::parse_next),
        // otherwise just finish the forward slash as is
        _ => finish().into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Trivia;

    #[test]
    fn finish_is_divide() {
        assert_eq!(finish(), Token::Divide.into());
    }

    #[test]
    fn double_forward_slash_begins_comment() {
        let mut input = "/rest".chars().peekable();
        let result = parse_next(&mut input);
        // the second slash is consumed and the comment consumes the rest
        assert_eq!(result, Err(LexState::Comment));
    }

    #[test]
    fn double_forward_slash_with_newline_completes_comment() {
        let mut input = "/text\n".chars().peekable();
        let result = parse_next(&mut input);
        assert_eq!(result, Ok(Trivia::Comment.into()));
    }

    #[test]
    fn other_char_finishes_as_divide() {
        let mut input = "x".chars().peekable();
        let result = parse_next(&mut input);
        assert_eq!(result, Ok(Token::Divide.into()));
        // the non matching character is left in place
        assert_eq!(input.next(), Some('x'));
    }

    #[test]
    fn empty_input_is_pending() {
        let mut input = "".chars().peekable();
        assert_eq!(parse_next(&mut input), Err(LexState::ForwardSlash));
    }
}
