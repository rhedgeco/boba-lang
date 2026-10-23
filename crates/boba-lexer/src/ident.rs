use boba_token::Token;

use crate::{LexerBuilder, LexerOutput, LexerResult};

/// Builds an identifier token out of characters.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IdentLexer;

impl IdentLexer {
    /// Returns true if the character can start an identifier.
    pub const fn primer(c: char) -> bool {
        matches!(c, '_' | 'a'..='z' | 'A'..='Z')
    }
}

impl LexerBuilder for IdentLexer {
    type NextState = Self;

    fn finish(self) -> Option<LexerOutput> {
        Some(Token::Identifier.into())
    }

    fn build(self, c: char) -> LexerResult<Self::NextState> {
        if !matches!(c, '_' | 'a'..='z' | 'A'..='Z' | '0'..='9') {
            return LexerResult::Bail(Token::Identifier.into());
        }

        LexerResult::Next(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Token, Trivia, test_utils::tokenize};

    #[test]
    fn ident_primer_accepts_letters_and_underscore() {
        // identifiers may start with a letter or an underscore
        assert!(IdentLexer::primer('a'));
        assert!(IdentLexer::primer('Z'));
        assert!(IdentLexer::primer('_'));
    }

    #[test]
    fn ident_primer_rejects_digits_and_symbols() {
        // identifiers may not start with a digit or a symbol
        assert!(!IdentLexer::primer('1'));
        assert!(!IdentLexer::primer('$'));
    }

    #[test]
    fn build_on_identifier_char_stays_in_identifier() {
        // a valid body character keeps building the same identifier
        let result = IdentLexer.build('a');
        assert_eq!(result, LexerResult::Next(IdentLexer));
    }

    #[test]
    fn build_on_boundary_bails_with_identifier() {
        // the first non-identifier character bails, ending the identifier
        let result = IdentLexer.build('.');
        assert_eq!(result, LexerResult::Bail(Token::Identifier.into()));
    }

    #[test]
    fn tokenizes_custom_identifier() {
        // a run of valid identifier characters becomes one identifier
        let tokens = tokenize("foo_bar1");
        assert_eq!(tokens, vec![Token::Identifier.into()]);
    }

    #[test]
    fn continues_identifier_with_alphanumerics_and_underscore() {
        // identifiers may continue with letters, digits, or underscores
        let tokens = tokenize("a9_b");
        assert_eq!(tokens, vec![Token::Identifier.into()]);
    }

    #[test]
    fn tokenizes_underscore_prefixed_identifier() {
        // an identifier may start with an underscore
        let tokens = tokenize("_private");
        assert_eq!(tokens, vec![Token::Identifier.into()]);
    }

    #[test]
    fn tokenizes_single_character_identifier() {
        // a single valid start character is a whole identifier
        let tokens = tokenize("x");
        assert_eq!(tokens, vec![Token::Identifier.into()]);
    }

    #[test]
    fn tokenizes_words_as_identifiers() {
        // words are all plain identifiers, separated by whitespace trivia
        let tokens = tokenize("pub const");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier.into(),
                Trivia::Space.into(),
                Token::Identifier.into(),
            ]
        );
    }

    #[test]
    fn emits_trailing_identifier() {
        // an identifier that ends at the source boundary is still emitted
        let tokens = tokenize("foo");
        assert_eq!(tokens, vec![Token::Identifier.into()]);
    }

    #[test]
    fn stops_identifier_at_boundary() {
        // an identifier ends at the first non-identifier character
        let tokens = tokenize("foo.bar");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier.into(),
                Token::AccessMember.into(),
                Token::Identifier.into(),
            ]
        );
    }
}
