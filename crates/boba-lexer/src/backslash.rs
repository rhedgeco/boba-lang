use crate::{LexerBuilder, LexerOutput, LexerResult, Trivia, multiline_string::MultilineStrLexer};

/// Builds a multiline string, or bails as invalid on a lone backslash.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BackSlashLexer;

impl BackSlashLexer {
    /// The character that starts this lexer.
    pub const PRIMER: char = '\\';
}

impl LexerBuilder for BackSlashLexer {
    type NextState = MultilineStrLexer;

    fn finish(self) -> Option<LexerOutput> {
        Some(Trivia::Invalid.into())
    }

    fn build(self, c: char) -> LexerResult<Self::NextState> {
        match c {
            // two backslashes trigger a multiline string
            '\\' => LexerResult::Next(MultilineStrLexer),
            // otherwise just bail and emit the invalid trivia
            _ => LexerResult::Bail(Trivia::Invalid.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Token, Trivia, test_utils::tokenize};

    #[test]
    fn build_on_backslash_transitions_to_multiline_string() {
        // a second backslash hands off to the multiline string lexer
        let result = BackSlashLexer.build('\\');
        assert_eq!(result, LexerResult::Next(MultilineStrLexer));
    }

    #[test]
    fn build_on_other_bails_with_invalid() {
        // any other character bails, leaving the lone backslash as invalid
        let result = BackSlashLexer.build('x');
        assert_eq!(result, LexerResult::Bail(Trivia::Invalid.into()));
    }

    #[test]
    fn reports_lone_backslash_as_invalid() {
        // a single backslash is not a multiline string and is invalid
        let tokens = tokenize(r"\");
        assert_eq!(tokens, vec![Trivia::Invalid.into()]);
    }

    #[test]
    fn reports_backslash_before_other_token_as_invalid() {
        // a backslash not followed by another backslash bails as invalid
        // the next character is left so it can still be lexed
        let tokens = tokenize(r"\foo");
        assert_eq!(
            tokens,
            vec![Trivia::Invalid.into(), Token::Identifier.into()]
        );
    }

    #[test]
    fn separated_backslashes_are_each_invalid() {
        // a backslash only starts a string when the next char is also a backslash
        let tokens = tokenize(r"\ \");
        assert_eq!(
            tokens,
            vec![
                Trivia::Invalid.into(),
                Trivia::Space.into(),
                Trivia::Invalid.into(),
            ]
        );
    }
}
