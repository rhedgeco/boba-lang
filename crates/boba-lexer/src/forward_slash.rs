//! Parsing for forward slashes, which may begin a comment.

use std::iter::Peekable;

use boba_token::Token;

use crate::{LexerOutput, LexerResult, LexerState, comment};

/// Finishes a lone forward slash as a divide token.
pub fn finish() -> LexerOutput {
    Token::Divide.into()
}

/// Parses the character after an initial forward slash.
///
/// A following forward slash begins a comment.
/// Anything else finishes the forward slash as a divide token.
pub fn parse_next(input: &mut Peekable<impl Iterator<Item = char>>) -> LexerResult {
    let next_char = *input.peek().ok_or(LexerState::ForwardSlash)?;

    // if the next character is another forward slash
    // then we should start parsing the comment
    if next_char == '/' {
        let _ = input.next();
        return comment::parse_next(input);
    }

    // otherwise just finish the forward slash as is
    finish().into()
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
        assert_eq!(result, Err(LexerState::Comment));
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
        assert_eq!(parse_next(&mut input), Err(LexerState::ForwardSlash));
    }
}
