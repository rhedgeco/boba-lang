use std::{iter::Peekable, mem::replace, ops::Range, str::Chars};

use crate::Token;

/// Tracks the build state of a token producer.
enum BuildState<T> {
    /// The character was consumed, and it produced a token.
    Consumed(Token),
    /// The character was ignored, and it produced a token.
    Bail(Token),
    /// The character was consumed, and is still pending.
    Pending(T),
}

impl<T> BuildState<T> {
    /// Maps a pending builder into a different one using `f`
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> BuildState<U> {
        match self {
            BuildState::Bail(token) => BuildState::Bail(token),
            BuildState::Consumed(token) => BuildState::Consumed(token),
            BuildState::Pending(item) => BuildState::Pending(f(item)),
        }
    }
}

/// The range of possible identifiers
enum Ident {
    Custom,
    Const,
    Pub,
}

impl Ident {
    // helper consts for tracking keyword characters
    pub const PUB_FIRST: char = Self::PUB_CHARS[0];
    pub const CONST_FIRST: char = Self::CONST_CHARS[0];
    pub const PUB_CHARS: &[char] = &['p', 'u', 'b'];
    pub const CONST_CHARS: &[char] = &['c', 'o', 'n', 's', 't'];
}

/// Builds an identifier token out of chars
struct IdentBuilder {
    length: usize,
    ident: Ident,
}

impl IdentBuilder {
    /// Constructs a new ident builder, if the starting character is valid
    pub fn new(c: char) -> Option<Self> {
        if !utils::is_valid_ident_start(c) {
            return None;
        }

        Some(Self {
            length: 1,
            ident: match c {
                Ident::PUB_FIRST => Ident::Pub,
                Ident::CONST_FIRST => Ident::Const,
                _ => Ident::Custom,
            },
        })
    }

    /// Injects a character, and attempts to build an ident token
    pub fn build(mut self, c: char) -> BuildState<Self> {
        // if the character is not a valid body character
        if !utils::is_valid_ident_body(c) {
            // then we can just bail with the finished token
            return BuildState::Bail(self.finish());
        }

        // if the token is still pending and the character is valid
        // match the character to build any prospective keywords
        self.ident = match self.ident {
            Ident::Pub => match Ident::PUB_CHARS.get(self.length) == Some(&c) {
                false => Ident::Custom,
                true => Ident::Pub,
            },
            Ident::Const => match Ident::CONST_CHARS.get(self.length) == Some(&c) {
                false => Ident::Custom,
                true => Ident::Const,
            },
            _ => Ident::Custom,
        };

        // increment the length of the token for the next iteration
        self.length += 1;

        BuildState::Pending(self)
    }

    /// Immediately resolve the identifier into a token
    pub fn finish(self) -> Token {
        // Resolves the tracked keyword, downgrading incomplete prefixes to identifier
        // A prefix like "pu" matches so far but is not the full "pub" keyword.
        match self.ident {
            Ident::Pub if self.length == Ident::PUB_CHARS.len() => Token::Public,
            Ident::Const if self.length == Ident::CONST_CHARS.len() => Token::Constant,
            _ => Token::Identifier,
        }
    }
}

/// Builds an inline string token out of chars
struct InlineStringBuilder {
    escaping: bool,
}

impl InlineStringBuilder {
    /// Returns a new empty inline string builder
    pub fn new() -> Self {
        Self { escaping: false }
    }

    /// Injects a character, and attempts to build an inline string token
    pub fn build(mut self, c: char) -> BuildState<Self> {
        match c {
            // newline tokens mean the inline string is unclosed
            // newlines cannot be escaped so this is unconditional
            // we bail here so the parser can detect the newline still
            '\r' | '\n' => return BuildState::Bail(Token::Invalid),

            // if not currently escaping, quotes close the string
            // the quotation char should be consumed in the process
            '"' if !self.escaping => return BuildState::Consumed(Token::StrLiteral),

            // escape characters start a new escape sequence if there isnt one already
            '\\' if !self.escaping => self.escaping = true,

            // any other character regardless of escape status is valid
            // in any case, the escape sequence should be reset to false
            _ => self.escaping = false,
        };

        BuildState::Pending(self)
    }

    /// Immediately resolve the string a an unclosed literal
    pub fn finish(self) -> Token {
        Token::Invalid
    }
}

enum TokenBuilder {
    Clear,
    Ident(IdentBuilder),
    InlineString(InlineStringBuilder),
}

impl From<IdentBuilder> for TokenBuilder {
    fn from(builder: IdentBuilder) -> Self {
        Self::Ident(builder)
    }
}

impl From<InlineStringBuilder> for TokenBuilder {
    fn from(builder: InlineStringBuilder) -> Self {
        Self::InlineString(builder)
    }
}

impl TokenBuilder {
    /// Injects a character, and attempts to build a token
    pub fn build(self, c: char) -> BuildState<Self> {
        match self {
            TokenBuilder::Ident(builder) => builder.build(c).map(Into::into),
            TokenBuilder::InlineString(builder) => builder.build(c).map(Into::into),
            TokenBuilder::Clear => match c {
                // skip whitespace tokens
                ' ' | '\r' | '\n' | '\t' => BuildState::Pending(TokenBuilder::Clear),

                // immediately consume and return simple char tokens
                '.' => BuildState::Consumed(Token::MemberProbe),
                ',' => BuildState::Consumed(Token::Delimiter),
                '=' => BuildState::Consumed(Token::Assign),
                ':' => BuildState::Consumed(Token::ScopeProbe),
                ';' => BuildState::Consumed(Token::CloseExpr),
                '(' => BuildState::Consumed(Token::OpenGroup),
                ')' => BuildState::Consumed(Token::CloseGroup),
                '{' => BuildState::Consumed(Token::OpenScope),
                '}' => BuildState::Consumed(Token::CloseScope),

                // start building an inline string if the character is a quote
                '"' => BuildState::Pending(Self::InlineString(InlineStringBuilder::new())),

                // lastly, try to build an identifier with any other character
                c => match IdentBuilder::new(c) {
                    // if it is valid, then return the pending ident builder
                    Some(builder) => BuildState::Pending(builder.into()),
                    // if it is invalid, then consume the invalid token
                    None => BuildState::Consumed(Token::Invalid),
                },
            },
        }
    }

    /// Immediately resolve the builder into any pending token
    ///
    /// An empty builder has nothing pending and yields no token
    pub fn finish(self) -> Option<Token> {
        match self {
            TokenBuilder::Clear => None,
            TokenBuilder::Ident(builder) => Some(builder.finish()),
            TokenBuilder::InlineString(builder) => Some(builder.finish()),
        }
    }
}

/// A lexer that turns source text into a stream of [`Token`]s.
/// Also tracks the byte span of the most recently produced token.
pub struct Lexer<'a> {
    builder: TokenBuilder,
    chars: Peekable<Chars<'a>>,
    source: &'a str,
    start: usize,
    end: usize,
}

impl<'a> Lexer<'a> {
    /// Creates a new [`Lexer`] over the given source text.
    pub fn new(source: &'a str) -> Self {
        Self {
            builder: TokenBuilder::Clear,
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

    /// The byte range of source covered by the current token.
    pub fn range(&self) -> Range<usize> {
        self.start()..self.end()
    }

    /// The source text the lexer is reading from.
    pub fn source(&self) -> &'a str {
        self.source
    }

    /// The slice of source text covered by the current token.
    pub fn token_source(&self) -> &'a str {
        &self.source()[self.range()]
    }

    /// Continues lexing into a new source buffer,
    /// picking up where the current lexer left off.
    ///
    /// This means that any mid-parse of strings or idents will still be completed.
    pub fn extend<'b>(self, source: &'b str) -> Lexer<'b> {
        // the builder contains all the parsing state
        // so to extend the lexer, we just need to pass along the builder
        Lexer {
            builder: self.builder,
            ..Lexer::new(source)
        }
    }

    /// Consumes the lexer and resolves any token still pending.
    ///
    /// Does not parse any more characters from the source text.
    pub fn finish(self) -> Option<Token> {
        self.builder.finish()
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // if there is no next char, return None
            let next_char = *self.chars.peek()?;

            // otherwise extract the builder
            let builder = replace(&mut self.builder, TokenBuilder::Clear);

            // if the builder is clear, then we are at the start of a new token
            if matches!(builder, TokenBuilder::Clear) {
                // reset the starting span location for the token
                self.start = self.end;
            }

            // then try to build the next token
            self.builder = match builder.build(next_char) {
                // if the builder bailed, we can just return the token as is
                BuildState::Bail(token) => return Some(token),
                BuildState::Pending(builder) => {
                    // if the builder is still pending, then we
                    let _ = self.chars.next(); // consume the char
                    self.end += next_char.len_utf8(); // update the token end
                    builder // and put the builder back into place
                }
                BuildState::Consumed(token) => {
                    // if the builder consumed the token, then we
                    let _ = self.chars.next(); // consume the char
                    self.end += next_char.len_utf8(); // update the token end
                    return Some(token); // and return the built token
                }
            };
        }
    }
}

pub mod utils {
    /// returns true if the character is a valid identifier start character
    pub fn is_valid_ident_start(c: char) -> bool {
        c == '_' || c.is_ascii_alphabetic()
    }

    /// returns true if the character is a valid identifier body character
    pub fn is_valid_ident_body(c: char) -> bool {
        c == '_' || c.is_ascii_alphanumeric()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // collects every token then flushes the final pending one, as a caller
    // would once it knows there is no more source to extend the lexer with
    fn tokenize(source: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(source);
        let mut tokens: Vec<_> = lexer.by_ref().collect();
        tokens.extend(lexer.finish());
        tokens
    }

    #[test]
    fn lexer_tokenizes_single_char_tokens() {
        // each symbol maps to its own token
        let tokens = tokenize(".,=:;(){}");
        assert_eq!(
            tokens,
            vec![
                Token::MemberProbe,
                Token::Delimiter,
                Token::Assign,
                Token::ScopeProbe,
                Token::CloseExpr,
                Token::OpenGroup,
                Token::CloseGroup,
                Token::OpenScope,
                Token::CloseScope,
            ]
        );
    }

    #[test]
    fn lexer_tokenizes_custom_identifier() {
        // a run of valid identifier characters becomes one identifier
        let tokens = tokenize("foo_bar1");
        assert_eq!(tokens, vec![Token::Identifier]);
    }

    #[test]
    fn lexer_tokenizes_keywords() {
        // a known keyword resolves to its dedicated variant
        let tokens = tokenize("pub const");
        assert_eq!(tokens, vec![Token::Public, Token::Constant]);
    }

    #[test]
    fn lexer_treats_keyword_prefix_as_custom() {
        // a prefix of a keyword is still just an identifier
        let tokens = tokenize("pu");
        assert_eq!(tokens, vec![Token::Identifier]);
    }

    #[test]
    fn lexer_treats_keyword_superstring_as_custom() {
        // an identifier that merely starts with a keyword stays an identifier
        // this also guards against indexing past the end of the keyword
        let tokens = tokenize("public");
        assert_eq!(tokens, vec![Token::Identifier]);
    }

    #[test]
    fn lexer_tokenizes_closed_string_literal() {
        // a terminated string literal is a closed string token
        let tokens = tokenize("\"hi\"");
        assert_eq!(tokens, vec![Token::StrLiteral]);
    }

    #[test]
    fn lexer_tokenizes_escaped_quote_in_string() {
        // an escaped quote does not close the string
        let tokens = tokenize("\"a\\\"b\"");
        assert_eq!(tokens, vec![Token::StrLiteral]);
    }

    #[test]
    fn lexer_tokenizes_unclosed_string_literal_at_end() {
        // a string with no closing quote before the end is invalid
        let tokens = tokenize("\"hi");
        assert_eq!(tokens, vec![Token::Invalid]);
    }

    #[test]
    fn lexer_tokenizes_unclosed_string_literal_at_newline() {
        // a newline terminates an inline string as invalid
        // the newline is left so later tokens can still see it
        let tokens = tokenize("\"hi\n");
        assert_eq!(tokens, vec![Token::Invalid]);
    }

    #[test]
    fn lexer_reports_invalid_characters() {
        // unrecognised characters surface as invalid tokens
        let tokens = tokenize("$");
        assert_eq!(tokens, vec![Token::Invalid]);
    }

    #[test]
    fn lexer_emits_trailing_identifier() {
        // an identifier that ends at the source boundary is still emitted
        let tokens = tokenize("foo");
        assert_eq!(tokens, vec![Token::Identifier]);
    }

    #[test]
    fn lexer_leaves_pending_token_for_flush() {
        // next stops at the end of source without finishing the pending token
        // so it can still be extended, and flush resolves it afterwards
        let mut lexer = Lexer::new("foo");
        assert_eq!(lexer.next(), None);
        assert_eq!(lexer.finish(), Some(Token::Identifier));
    }

    #[test]
    fn lexer_flush_on_clear_yields_nothing() {
        // with no token in progress there is nothing to flush
        let mut lexer = Lexer::new("foo,");
        let _ = lexer.by_ref().count();
        assert_eq!(lexer.finish(), None);
    }

    #[test]
    fn lexer_extends_token_across_buffers() {
        // a token split across two buffers resolves once both are lexed
        // the keyword only completes after the second buffer is consumed
        let mut lexer = Lexer::new("pu");
        assert_eq!(lexer.next(), None);
        let mut lexer = lexer.extend("b");
        assert_eq!(lexer.next(), None);
        assert_eq!(lexer.finish(), Some(Token::Public));
    }

    #[test]
    fn lexer_skips_whitespace() {
        // whitespace separates tokens but produces none of its own
        let tokens = tokenize("  foo \t bar\n");
        assert_eq!(tokens, vec![Token::Identifier, Token::Identifier]);
    }

    #[test]
    fn lexer_tracks_token_range() {
        // the range covers the slice of source for the current token
        let mut lexer = Lexer::new("foo,bar");
        lexer.next();
        assert_eq!(lexer.range(), 0..3);
        assert_eq!(lexer.token_source(), "foo");
    }

    #[test]
    fn lexer_advances_range_per_token() {
        // leading whitespace is excluded and the range moves with each token
        let mut lexer = Lexer::new("foo  bar");
        lexer.next();
        assert_eq!(lexer.token_source(), "foo");
        lexer.next();
        assert_eq!(lexer.range(), 5..8);
        assert_eq!(lexer.token_source(), "bar");
    }

    #[test]
    fn lexer_stops_identifier_at_boundary() {
        // an identifier ends at the first non-identifier character
        let tokens = tokenize("foo.bar");
        assert_eq!(
            tokens,
            vec![Token::Identifier, Token::MemberProbe, Token::Identifier]
        );
    }

    #[test]
    fn is_ident_start_accepts_letters_and_underscore() {
        // identifiers may start with a letter or an underscore
        assert!(utils::is_valid_ident_start('a'));
        assert!(utils::is_valid_ident_start('Z'));
        assert!(utils::is_valid_ident_start('_'));
    }

    #[test]
    fn is_ident_start_rejects_digits_and_symbols() {
        // identifiers may not start with a digit or a symbol
        assert!(!utils::is_valid_ident_start('1'));
        assert!(!utils::is_valid_ident_start('$'));
    }

    #[test]
    fn is_ident_end_accepts_alphanumerics_and_underscore() {
        // identifiers may continue with a letter, digit, or underscore
        assert!(utils::is_valid_ident_body('a'));
        assert!(utils::is_valid_ident_body('9'));
        assert!(utils::is_valid_ident_body('_'));
    }

    #[test]
    fn is_ident_end_rejects_symbols() {
        // identifiers may not continue with a symbol
        assert!(!utils::is_valid_ident_body('$'));
        assert!(!utils::is_valid_ident_body('.'));
    }
}
