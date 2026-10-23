use boba_token::Token;

use crate::{LexerBuilder, LexerOutput, LexerResult, comment::CommentLexer};

/// Builds a divide token, or hands off to a comment lexer.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SlashLexer;

impl SlashLexer {
    /// The character that starts this lexer.
    pub const PRIMER: char = '/';
}

impl LexerBuilder for SlashLexer {
    type NextState = CommentLexer;

    fn finish(self) -> Option<LexerOutput> {
        Some(Token::Divide.into())
    }

    fn build(self, c: char) -> LexerResult<Self::NextState> {
        match c {
            // two slashes trigger a comment lexer
            '/' => LexerResult::Next(CommentLexer),
            // otherwise just bail and emit the divide token
            _ => LexerResult::Bail(Token::Divide.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Token, Trivia, test_utils::tokenize};

    #[test]
    fn build_on_slash_transitions_to_comment() {
        // a second slash hands off to the comment lexer
        let result = SlashLexer.build('/');
        assert_eq!(result, LexerResult::Next(CommentLexer));
    }

    #[test]
    fn build_on_other_bails_with_divide() {
        // any other character bails, leaving the slash as a divide token
        let result = SlashLexer.build('x');
        assert_eq!(result, LexerResult::Bail(Token::Divide.into()));
    }

    #[test]
    fn tokenizes_divide_operator() {
        // a lone slash that is not a comment is a divide operator
        let tokens = tokenize("/");
        assert_eq!(tokens, vec![Token::Divide.into()]);
    }

    #[test]
    fn tokenizes_divide_before_other_token() {
        // a slash followed by a non-slash bails, leaving the next char to lex
        let tokens = tokenize("/foo");
        assert_eq!(tokens, vec![Token::Divide.into(), Token::Identifier.into()]);
    }

    #[test]
    fn tokenizes_comment_from_double_slash() {
        // two slashes hand off to the comment lexer
        let tokens = tokenize("//");
        assert_eq!(tokens, vec![Trivia::Comment.into()]);
    }

    #[test]
    fn separated_slashes_are_two_divides() {
        // a slash only starts a comment when the next char is also a slash
        let tokens = tokenize("/ /");
        assert_eq!(
            tokens,
            vec![
                Token::Divide.into(),
                Trivia::Space.into(),
                Token::Divide.into(),
            ]
        );
    }
}
