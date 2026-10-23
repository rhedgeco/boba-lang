use crate::{LexerBuilder, LexerOutput, LexerResult, Trivia};

/// Builds a newline trivia token out of characters.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NewlineLexer;

impl NewlineLexer {
    /// The character that starts this lexer.
    pub const PRIMER: char = '\r';
}

impl LexerBuilder for NewlineLexer {
    type NextState = Self;

    fn finish(self) -> Option<LexerOutput> {
        Some(Trivia::Newline.into())
    }

    fn build(self, c: char) -> LexerResult<Self::NextState> {
        match c {
            // capture the CRLF when a '\n' comes after the '\r'
            '\n' => LexerResult::Consume(Trivia::Newline.into()),
            // otherwise any other character should bail
            _ => LexerResult::Bail(Trivia::Newline.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Token, Trivia, test_utils::tokenize};

    #[test]
    fn build_on_line_feed_consumes_as_crlf() {
        // a line feed after the carriage return is consumed into one newline
        let result = NewlineLexer.build('\n');
        assert_eq!(result, LexerResult::Consume(Trivia::Newline.into()));
    }

    #[test]
    fn build_on_other_bails_with_newline() {
        // any other character bails, leaving the carriage return as a newline
        let result = NewlineLexer.build('x');
        assert_eq!(result, LexerResult::Bail(Trivia::Newline.into()));
    }

    #[test]
    fn tokenizes_newline_from_line_feed() {
        // a bare line feed is a single newline trivia
        let tokens = tokenize("\n");
        assert_eq!(tokens, vec![Trivia::Newline.into()]);
    }

    #[test]
    fn tokenizes_newline_from_carriage_return() {
        // a carriage return followed by a line feed is one newline trivia
        let tokens = tokenize("\r\n");
        assert_eq!(tokens, vec![Trivia::Newline.into()]);
    }

    #[test]
    fn tokenizes_lone_carriage_return_as_newline() {
        // a carriage return alone still resolves to a newline trivia
        // the following character is bailed so it can be lexed next
        let tokens = tokenize("\rfoo");
        assert_eq!(
            tokens,
            vec![Trivia::Newline.into(), Token::Identifier.into()]
        );
    }

    #[test]
    fn carriage_return_before_symbol_bails_to_next_token() {
        // a non-newline after the carriage return bails, leaving it to lex
        let tokens = tokenize("\r,");
        assert_eq!(
            tokens,
            vec![Trivia::Newline.into(), Token::Delimiter.into()]
        );
    }

    #[test]
    fn consecutive_carriage_returns_are_separate_newlines() {
        // each carriage return without a following line feed is its own newline
        let tokens = tokenize("\r\r");
        assert_eq!(tokens, vec![Trivia::Newline.into(), Trivia::Newline.into()]);
    }
}
