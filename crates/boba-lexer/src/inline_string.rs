use boba_token::Token;

use crate::{LexerBuilder, LexerOutput, LexerResult, Trivia};

/// Builds an inline string token out of characters.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InlineStringLexer {
    escaping: bool,
}

impl InlineStringLexer {
    /// The character that starts this lexer.
    pub const PRIMER: char = '"';

    /// Returns a new inline string lexer that is not escaping.
    pub const fn new() -> Self {
        Self { escaping: false }
    }
}

impl LexerBuilder for InlineStringLexer {
    type NextState = Self;

    fn finish(self) -> Option<LexerOutput> {
        Some(Trivia::UnclosedStr.into())
    }

    fn build(mut self, c: char) -> LexerResult<Self::NextState> {
        match c {
            // newline tokens mean the inline string is unclosed
            // newlines cannot be escaped so this is unconditional
            '\r' | '\n' => return LexerResult::Bail(Trivia::UnclosedStr.into()),

            // escape characters start a new escape sequence if there isnt one already
            '\\' if !self.escaping => self.escaping = true,

            // if not currently escaping, quotes close the string
            // the quotation char should be consumed in the process
            '"' if !self.escaping => {
                return LexerResult::Consume(Token::InlineStrLiteral.into());
            }

            // any other character regardless of escape status is valid
            // in any case, the escape sequence should be reset to false
            _ => self.escaping = false,
        };

        LexerResult::Next(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Token, Trivia, test_utils::tokenize};

    #[test]
    fn build_on_backslash_enters_escaping_state() {
        // a backslash begins an escape sequence for the next character
        let result = InlineStringLexer::new().build('\\');
        assert_eq!(
            result,
            LexerResult::Next(InlineStringLexer { escaping: true })
        );
    }

    #[test]
    fn build_on_escaped_char_leaves_escaping_state() {
        // the escaped character is consumed, so the next one is unescaped again
        let result = InlineStringLexer { escaping: true }.build('"');
        assert_eq!(
            result,
            LexerResult::Next(InlineStringLexer { escaping: false })
        );
    }

    #[test]
    fn build_on_unescaped_quote_consumes_and_closes() {
        // an unescaped quote consumes the character and closes the string
        let result = InlineStringLexer::new().build('"');
        assert_eq!(result, LexerResult::Consume(Token::InlineStrLiteral.into()));
    }

    #[test]
    fn build_on_newline_bails_as_unclosed() {
        // a newline cannot be escaped, so it always bails the string as unclosed
        let result = InlineStringLexer { escaping: true }.build('\n');
        assert_eq!(result, LexerResult::Bail(Trivia::UnclosedStr.into()));
    }

    #[test]
    fn tokenizes_closed_string_literal() {
        // a terminated string literal is a closed string token
        let tokens = tokenize(r#""hi""#);
        assert_eq!(tokens, vec![Token::InlineStrLiteral.into()]);
    }

    #[test]
    fn tokenizes_escaped_quote_in_string() {
        // an escaped quote does not close the string
        let tokens = tokenize(r#""a\"b""#);
        assert_eq!(tokens, vec![Token::InlineStrLiteral.into()]);
    }

    #[test]
    fn tokenizes_unclosed_string_literal_at_end() {
        // a string with no closing quote before the end is unclosed
        let tokens = tokenize(r#""hi"#);
        assert_eq!(tokens, vec![Trivia::UnclosedStr.into()]);
    }

    #[test]
    fn tokenizes_unclosed_string_literal_at_newline() {
        // a newline terminates an inline string as unclosed
        // the newline is left so later tokens can still see it
        let tokens = tokenize(concat!(r#""hi"#, "\n"));
        assert_eq!(
            tokens,
            vec![Trivia::UnclosedStr.into(), Trivia::Newline.into()]
        );
    }

    #[test]
    fn tokenizes_unclosed_string_literal_at_carriage_return() {
        // a carriage return also terminates an inline string as unclosed
        // the carriage return is left so later tokens can still see it
        let tokens = tokenize(concat!(r#""hi"#, "\r"));
        assert_eq!(
            tokens,
            vec![Trivia::UnclosedStr.into(), Trivia::Newline.into()]
        );
    }

    #[test]
    fn tokenizes_empty_string_literal() {
        // an immediately closed string is still a closed string token
        let tokens = tokenize(r#""""#);
        assert_eq!(tokens, vec![Token::InlineStrLiteral.into()]);
    }

    #[test]
    fn escaped_backslash_does_not_escape_the_closing_quote() {
        // the second backslash is escaped, so the quote after it still closes
        let tokens = tokenize(r#""a\\""#);
        assert_eq!(tokens, vec![Token::InlineStrLiteral.into()]);
    }

    #[test]
    fn escape_sequence_resets_after_one_character() {
        // an escaped character consumes the escape, so a later quote closes
        let tokens = tokenize(r#""\n""#);
        assert_eq!(tokens, vec![Token::InlineStrLiteral.into()]);
    }

    #[test]
    fn closed_string_is_followed_by_the_next_token() {
        // the closing quote is consumed, so a trailing token lexes on its own
        let tokens = tokenize(r#""hi","#);
        assert_eq!(
            tokens,
            vec![Token::InlineStrLiteral.into(), Token::Delimiter.into()]
        );
    }
}
