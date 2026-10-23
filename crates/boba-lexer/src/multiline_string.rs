use boba_token::Token;

use crate::{LexerBuilder, LexerOutput, LexerResult};

/// Builds a multiline string token out of characters.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MultilineStrLexer;

impl LexerBuilder for MultilineStrLexer {
    type NextState = Self;

    fn finish(self) -> Option<LexerOutput> {
        Some(Token::MultiLineStrLiteral.into())
    }

    fn build(self, c: char) -> LexerResult<Self::NextState> {
        match c {
            // if we get a newline character, then the string is complete
            '\r' | '\n' => LexerResult::Bail(Token::MultiLineStrLiteral.into()),
            // otherwise literally any other character is acceptable
            _ => LexerResult::Next(self),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Token, Trivia, test_utils::tokenize};

    #[test]
    fn build_on_body_char_stays_in_multiline_string() {
        // any non-newline character keeps extending the multiline string
        let result = MultilineStrLexer.build('x');
        assert_eq!(result, LexerResult::Next(MultilineStrLexer));
    }

    #[test]
    fn build_on_newline_bails_with_multiline_string() {
        // a newline bails, completing the string and leaving the newline to lex
        let result = MultilineStrLexer.build('\n');
        assert_eq!(result, LexerResult::Bail(Token::MultiLineStrLiteral.into()));
    }

    #[test]
    fn tokenizes_multiline_string_at_end() {
        // a double backslash starts a multiline string that runs to the end
        let tokens = tokenize(r"\\hello");
        assert_eq!(tokens, vec![Token::MultiLineStrLiteral.into()]);
    }

    #[test]
    fn tokenizes_multiline_string_at_newline() {
        // a newline ends the multiline string, left for the next token
        let tokens = tokenize(concat!(r"\\hello", "\n"));
        assert_eq!(
            tokens,
            vec![Token::MultiLineStrLiteral.into(), Trivia::Newline.into()]
        );
    }

    #[test]
    fn tokenizes_multiline_string_at_carriage_return() {
        // a carriage return also ends the multiline string, left for the next token
        let tokens = tokenize(concat!(r"\\hello", "\r"));
        assert_eq!(
            tokens,
            vec![Token::MultiLineStrLiteral.into(), Trivia::Newline.into()]
        );
    }

    #[test]
    fn tokenizes_empty_multiline_string() {
        // a double backslash with no body is still a multiline string token
        let tokens = tokenize(r"\\");
        assert_eq!(tokens, vec![Token::MultiLineStrLiteral.into()]);
    }
}
