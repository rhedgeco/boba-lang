//! Parsing for keywords, which share their character class with identifiers.
//!
//! A keyword is matched character by character against its expected text.
//! A character that diverges before the keyword completes falls back to an identifier.
//! A completed keyword is still an identifier if a valid body character follows it.

pub mod utils;

use std::iter::Peekable;
use std::num::NonZeroUsize;

use boba_token::Token;

use crate::{
    LexerOutput, LexerResult, LexerState, identifier,
    keyword::utils::{KeywordStr, parse_next_keyword},
};

/// The `pub` keyword.
pub const PUBLIC: KeywordStr<'static> = KeywordStr::build("pub").unwrap();
/// The `var` keyword.
pub const VARIABLE: KeywordStr<'static> = KeywordStr::build("var").unwrap();
/// The `const` keyword.
pub const CONSTANT: KeywordStr<'static> = KeywordStr::build("const").unwrap();

/// Finishes a pending `pub` keyword.
///
/// A full match finishes as a public token.
/// A partial match finishes as an identifier instead.
pub fn finish_public(length: NonZeroUsize) -> LexerOutput {
    if length != PUBLIC.len() {
        return identifier::finish();
    }

    Token::Public.into()
}

/// Finishes a pending `var` keyword.
///
/// A full match finishes as a variable token.
/// A partial match finishes as an identifier instead.
pub fn finish_variable(length: NonZeroUsize) -> LexerOutput {
    if length != VARIABLE.len() {
        return identifier::finish();
    }

    Token::Variable.into()
}

/// Finishes a pending `const` keyword.
///
/// A full match finishes as a constant token.
/// A partial match finishes as an identifier instead.
pub fn finish_constant(length: NonZeroUsize) -> LexerOutput {
    if length != CONSTANT.len() {
        return identifier::finish();
    }

    Token::Constant.into()
}

/// Continues trying to match a `pub` keyword from the given matched length.
pub fn parse_next_public(
    length: NonZeroUsize,
    input: &mut Peekable<impl Iterator<Item = char>>,
) -> LexerResult {
    parse_next_keyword(
        &PUBLIC,
        length,
        LexerState::KeywordPublic,
        finish_public,
        input,
    )
}

/// Continues trying to match a `var` keyword from the given matched length.
pub fn parse_next_variable(
    length: NonZeroUsize,
    input: &mut Peekable<impl Iterator<Item = char>>,
) -> LexerResult {
    parse_next_keyword(
        &VARIABLE,
        length,
        LexerState::KeywordVariable,
        finish_variable,
        input,
    )
}

/// Continues trying to match a `const` keyword from the given matched length.
pub fn parse_next_constant(
    length: NonZeroUsize,
    input: &mut Peekable<impl Iterator<Item = char>>,
) -> LexerResult {
    parse_next_keyword(
        &CONSTANT,
        length,
        LexerState::KeywordConstant,
        finish_constant,
        input,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finish_full_match_is_keyword_token() {
        assert_eq!(finish_public(PUBLIC.len()), Token::Public.into());
        assert_eq!(finish_variable(VARIABLE.len()), Token::Variable.into());
        assert_eq!(finish_constant(CONSTANT.len()), Token::Constant.into());
    }

    #[test]
    fn finish_partial_match_is_identifier() {
        // a length short of the full keyword means it was only a prefix
        assert_eq!(
            finish_public(NonZeroUsize::new(1).unwrap()),
            Token::Identifier.into()
        );
        assert_eq!(
            finish_variable(NonZeroUsize::new(2).unwrap()),
            Token::Identifier.into()
        );
        assert_eq!(
            finish_constant(NonZeroUsize::new(3).unwrap()),
            Token::Identifier.into()
        );
    }

    #[test]
    fn parse_next_full_keyword_is_pending_at_eof() {
        // dispatch starts at length 1 with the first letter already consumed
        // the rest of the keyword matches but the input ends, so it is pending
        let mut input = "ub".chars().peekable();
        let result = parse_next_public(NonZeroUsize::MIN, &mut input);
        assert_eq!(
            result,
            Err(LexerState::KeywordPublic(NonZeroUsize::new(3).unwrap()))
        );
    }

    #[test]
    fn parse_next_completed_keyword_then_boundary() {
        // "pub " matches the whole keyword, then the space is a boundary
        // a boundary after a complete keyword finishes it as the keyword token
        let mut input = "ub ".chars().peekable();
        let result = parse_next_public(NonZeroUsize::MIN, &mut input);
        assert_eq!(result, Ok(Token::Public.into()));
        // the boundary character is left in place for the next token
        assert_eq!(input.next(), Some(' '));
    }

    #[test]
    fn parse_next_divergence_becomes_identifier() {
        // "pur..." diverges from "pub" at the third character
        let mut input = "ur ".chars().peekable();
        let result = parse_next_public(NonZeroUsize::MIN, &mut input);
        assert_eq!(result, Ok(Token::Identifier.into()));
        // the diverging run is consumed as an identifier up to the space
        assert_eq!(input.next(), Some(' '));
    }

    #[test]
    fn parse_next_longer_than_keyword_is_identifier() {
        // "public" starts like the keyword but continues, making it an identifier
        let mut input = "ublic ".chars().peekable();
        let result = parse_next_public(NonZeroUsize::MIN, &mut input);
        assert_eq!(result, Ok(Token::Identifier.into()));
    }

    #[test]
    fn parse_next_empty_input_is_pending() {
        let mut input = "".chars().peekable();
        assert_eq!(
            parse_next_variable(NonZeroUsize::MIN, &mut input),
            Err(LexerState::KeywordVariable(NonZeroUsize::MIN))
        );
    }
}
