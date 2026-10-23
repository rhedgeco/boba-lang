use crate::{LexerBuilder, LexerOutput, LexerResult, Trivia};

/// Builds comment trivia out of characters.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommentLexer;

impl LexerBuilder for CommentLexer {
    type NextState = Self;

    fn finish(self) -> Option<LexerOutput> {
        Some(Trivia::Comment.into())
    }

    fn build(self, c: char) -> LexerResult<Self::NextState> {
        match c {
            // if we get a newline character, then the comment is complete
            '\r' | '\n' => LexerResult::Bail(Trivia::Comment.into()),
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
    fn build_on_body_char_stays_in_comment() {
        // any non-newline character keeps extending the comment
        let result = CommentLexer.build('x');
        assert_eq!(result, LexerResult::Next(CommentLexer));
    }

    #[test]
    fn build_on_newline_bails_with_comment() {
        // a newline bails, leaving the comment complete and the newline to lex
        let result = CommentLexer.build('\n');
        assert_eq!(result, LexerResult::Bail(Trivia::Comment.into()));
    }

    #[test]
    fn tokenizes_comment() {
        // a double slash starts a comment that runs to the end of source
        let tokens = tokenize("// a comment");
        assert_eq!(tokens, vec![Trivia::Comment.into()]);
    }

    #[test]
    fn stops_comment_at_newline() {
        // a comment ends at a newline, which is left for the next token
        let tokens = tokenize("// comment\nfoo");
        assert_eq!(
            tokens,
            vec![
                Trivia::Comment.into(),
                Trivia::Newline.into(),
                Token::Identifier.into(),
            ]
        );
    }

    #[test]
    fn stops_comment_at_carriage_return() {
        // a lone carriage return also ends a comment, left for the next token
        let tokens = tokenize("// comment\rfoo");
        assert_eq!(
            tokens,
            vec![
                Trivia::Comment.into(),
                Trivia::Newline.into(),
                Token::Identifier.into(),
            ]
        );
    }

    #[test]
    fn stops_comment_at_crlf() {
        // a comment ends at the carriage return of a CRLF, which lexes as one newline
        let tokens = tokenize("// comment\r\nfoo");
        assert_eq!(
            tokens,
            vec![
                Trivia::Comment.into(),
                Trivia::Newline.into(),
                Token::Identifier.into(),
            ]
        );
    }

    #[test]
    fn tokenizes_empty_comment() {
        // a comment with no body is still comment trivia
        let tokens = tokenize("//");
        assert_eq!(tokens, vec![Trivia::Comment.into()]);
    }
}
