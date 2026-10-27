//! Parsing for back slashes, which may begin a multiline string.

use crate::{LexerOutput, LexerResult, LexerState, Trivia, iter::PeekableChar, multiline_string};

/// Finishes a lone back slash as an invalid token.
pub fn finish() -> LexerOutput {
    Trivia::Invalid.into()
}

/// Parses the character after an initial back slash.
///
/// A following back slash begins a multiline string.
/// Anything else finishes the back slash as an invalid token.
pub fn parse_next(mut input: impl PeekableChar) -> LexerResult {
    let next_char = input.peek().ok_or(LexerState::BackSlash)?;

    // if the next character is another back slash
    // then we should start parsing the multiline string
    if next_char == '\\' {
        let _ = input.next();
        return multiline_string::parse_next(input);
    }

    // otherwise just finish the back slash as is
    finish().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finish_is_invalid() {
        assert_eq!(finish(), Trivia::Invalid.into());
    }

    #[test]
    fn second_back_slash_begins_multiline_string() {
        // the leading back slash here is the second one, the first was already consumed
        // with no newline the multiline string is still pending
        let mut input = "\\rest".chars().peekable();
        let result = parse_next(&mut input);
        assert_eq!(result, Err(LexerState::MultilineString));
    }

    #[test]
    fn multiline_string_completes_on_newline() {
        // the second back slash begins the string, which finishes at the newline
        let mut input = "\\body\n".chars().peekable();
        let result = parse_next(&mut input);
        assert_eq!(result, Ok(crate::Token::MultilineString.into()));
    }

    #[test]
    fn other_char_finishes_as_invalid() {
        let mut input = "x".chars().peekable();
        let result = parse_next(&mut input);
        assert_eq!(result, Ok(Trivia::Invalid.into()));
        // the non matching character is left in place
        assert_eq!(input.next(), Some('x'));
    }

    #[test]
    fn empty_input_is_pending() {
        let mut input = "".chars().peekable();
        let result = parse_next(&mut input);
        assert_eq!(result, Err(LexerState::BackSlash));
    }
}
