use boba_token::Token;

use crate::{LexerBuilder, LexerOutput, LexerResult};

/// Returns true if the character can start an identifier.
pub const fn is_valid_ident_start(c: char) -> bool {
    matches!(c, '_' | 'a'..='z' | 'A'..='Z')
}

/// Returns true if the character can continue an identifier.
pub const fn is_valid_ident_body(c: char) -> bool {
    matches!(c, '_' | 'a'..='z' | 'A'..='Z' | '0'..='9')
}

/// Builds an identifier token out of characters.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IdentLexer;

impl IdentLexer {
    /// Returns true if the character can start an identifier.
    pub const fn primer(c: char) -> bool {
        is_valid_ident_start(c)
    }
}

impl LexerBuilder for IdentLexer {
    type NextState = Self;

    fn finish(self) -> Option<LexerOutput> {
        Some(Token::Identifier.into())
    }

    fn build(self, c: char) -> LexerResult<Self::NextState> {
        if !is_valid_ident_body(c) {
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
    fn is_valid_ident_start_accepts_letters_and_underscore() {
        // a start character may be a letter or an underscore
        assert!(is_valid_ident_start('a'));
        assert!(is_valid_ident_start('Z'));
        assert!(is_valid_ident_start('_'));
    }

    #[test]
    fn is_valid_ident_start_rejects_digits_and_symbols() {
        // a start character may not be a digit or a symbol
        assert!(!is_valid_ident_start('1'));
        assert!(!is_valid_ident_start('$'));
    }

    #[test]
    fn is_valid_ident_body_accepts_alphanumerics_and_underscore() {
        // a body character may be a letter, a digit, or an underscore
        assert!(is_valid_ident_body('a'));
        assert!(is_valid_ident_body('Z'));
        assert!(is_valid_ident_body('_'));
        assert!(is_valid_ident_body('9'));
    }

    #[test]
    fn is_valid_ident_body_rejects_symbols() {
        // a body character may not be a symbol
        assert!(!is_valid_ident_body('$'));
        assert!(!is_valid_ident_body('.'));
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
