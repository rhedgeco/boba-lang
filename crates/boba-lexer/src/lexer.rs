use std::{iter::Peekable, ops::Range, str::Chars};

use crate::{
    Token,
    token::{Bracket, Ident},
};

/// A lexer that turns source text into a stream of [`Token`]s.
/// Also tracks the byte span of the most recently produced token.
pub struct Lexer<'a> {
    chars: Peekable<Chars<'a>>,
    source: &'a str,
    start: usize,
    end: usize,
}

impl<'a> Lexer<'a> {
    /// Creates a new [`Lexer`] over the given source text.
    pub fn new(source: &'a str) -> Self {
        Self {
            chars: source.chars().peekable(),
            source,
            start: 0,
            end: 0,
        }
    }

    /// The byte offset where the current token starts.
    pub fn start(&self) -> usize {
        self.start
    }

    /// The byte offset where the current token ends.
    pub fn end(&self) -> usize {
        self.end
    }

    /// The byte span of the current token.
    pub fn span(&self) -> Range<usize> {
        self.start()..self.end()
    }

    /// The full source text being lexed.
    pub fn source(&self) -> &'a str {
        self.source
    }

    /// The slice of source text covered by the current token.
    pub fn token_source(&self) -> &'a str {
        &self.source()[self.span()]
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.parse_next_token()
    }
}

// helper functions for the iterator
impl<'a> Lexer<'a> {
    // peek at the next character without consuming it
    fn peek_char(&mut self) -> Option<char> {
        self.chars.peek().map(|c| *c)
    }

    // consume the next character and advance the end offset
    fn take_char(&mut self) -> Option<char> {
        let char = self.chars.next()?;
        self.end += char.len_utf8();
        Some(char)
    }

    fn parse_next_token(&mut self) -> Option<Token<'a>> {
        loop {
            // reset the token start location
            self.start = self.end();

            // parse token based on starting character
            return Some(match self.take_char()? {
                // skip whitespace tokens
                ' ' | '\r' | '\n' | '\t' => continue,

                // parse simple single char tokens
                '.' => Token::Point,
                ',' => Token::Comma,
                '=' => Token::Equality,
                ':' => Token::Colon,
                ';' => Token::SemiColon,
                '(' => Token::Open(Bracket::Round),
                ')' => Token::Close(Bracket::Round),
                '{' => Token::Open(Bracket::Curly),
                '}' => Token::Close(Bracket::Curly),

                // parse identifier if the token start is valid
                c if utils::is_ident_start(c) => self.parse_identifier(),

                // catch-all for invalid token characters
                c => Token::Invalid(c),
            });
        }
    }

    fn parse_identifier(&mut self) -> Token<'a> {
        loop {
            match self.peek_char() {
                Some(c) if utils::is_ident_end(c) => {
                    self.take_char(); // consume the valid character
                    continue; // and continue parsing the next one
                }
                _ => break,
            }
        }

        // build the identifier out of the parsed token source
        Token::Ident(Ident::build(self.token_source()))
    }
}

mod utils {
    // whether a character may start an identifier
    pub fn is_ident_start(c: char) -> bool {
        c == '_' || c.is_ascii_alphabetic()
    }

    // whether a character may continue an identifier
    pub fn is_ident_end(c: char) -> bool {
        c == '_' || c.is_ascii_alphanumeric()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexer_tokenizes_single_char_tokens() {
        // each symbol maps to its own token
        let tokens: Vec<_> = Lexer::new(".,=(){}").collect();
        assert_eq!(
            tokens,
            vec![
                Token::Point,
                Token::Comma,
                Token::Equality,
                Token::Open(Bracket::Round),
                Token::Close(Bracket::Round),
                Token::Open(Bracket::Curly),
                Token::Close(Bracket::Curly),
            ]
        );
    }

    #[test]
    fn lexer_tokenizes_custom_identifier() {
        // a run of valid identifier characters becomes one custom ident
        let tokens: Vec<_> = Lexer::new("foo_bar1").collect();
        assert_eq!(tokens, vec![Token::Ident(Ident::Custom("foo_bar1"))]);
    }

    #[test]
    fn lexer_tokenizes_keyword() {
        // a known keyword resolves to its dedicated variant
        let tokens: Vec<_> = Lexer::new("pub").collect();
        assert_eq!(tokens, vec![Token::Ident(Ident::Pub)]);
    }

    #[test]
    fn lexer_reports_invalid_characters() {
        // unrecognised characters surface as invalid tokens
        let tokens: Vec<_> = Lexer::new("$").collect();
        assert_eq!(tokens, vec![Token::Invalid('$')]);
    }

    #[test]
    fn lexer_tracks_token_span() {
        // the span covers the slice of source for the current token
        let mut lexer = Lexer::new("foo,bar");
        lexer.next();
        assert_eq!(lexer.span(), 0..3);
        assert_eq!(lexer.token_source(), "foo");
    }

    #[test]
    fn lexer_stops_identifier_at_boundary() {
        // an identifier ends at the first non-identifier character
        let tokens: Vec<_> = Lexer::new("foo.bar").collect();
        assert_eq!(
            tokens,
            vec![
                Token::Ident(Ident::Custom("foo")),
                Token::Point,
                Token::Ident(Ident::Custom("bar")),
            ]
        );
    }

    #[test]
    fn is_ident_start_accepts_letters_and_underscore() {
        // identifiers may start with a letter or an underscore
        assert!(utils::is_ident_start('a'));
        assert!(utils::is_ident_start('Z'));
        assert!(utils::is_ident_start('_'));
    }

    #[test]
    fn is_ident_start_rejects_digits_and_symbols() {
        // identifiers may not start with a digit or a symbol
        assert!(!utils::is_ident_start('1'));
        assert!(!utils::is_ident_start('$'));
    }

    #[test]
    fn is_ident_end_accepts_alphanumerics_and_underscore() {
        // identifiers may continue with a letter, digit, or underscore
        assert!(utils::is_ident_end('a'));
        assert!(utils::is_ident_end('9'));
        assert!(utils::is_ident_end('_'));
    }

    #[test]
    fn is_ident_end_rejects_symbols() {
        // identifiers may not continue with a symbol
        assert!(!utils::is_ident_end('$'));
        assert!(!utils::is_ident_end('.'));
    }
}
