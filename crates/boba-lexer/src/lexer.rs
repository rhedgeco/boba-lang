use std::{iter::Peekable, mem::replace, ops::Range, str::Chars};

use derive_more::From;

use crate::Token;

/// Either a lexical or trivia token produced by a [`Lexer`].
#[derive(Debug, From, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ParseToken {
    /// A valid lexical token.
    Token(Token),
    /// A supporting trivia token.
    Trivia(Trivia),
}

/// Non lexical tokens produced by the lexer
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Trivia {
    /// Source that does not form a valid token.
    Invalid,
    /// A line comment introduced with `//`.
    Comment,
    /// A line break, from either `\n`, `\r\n`, or just '\r'.
    Newline,
    /// A single space character.
    Space,
    /// A single tab character.
    Tab,
}

/// Tracks the build state of a token producer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum BuildState<T> {
    /// The character was consumed, and it produced a token.
    Consumed(ParseToken),
    /// The character was ignored, and it produced a token.
    Bail(ParseToken),
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

/// Builds a newline trivia token out of characters
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct NewlineBuilder;

impl NewlineBuilder {
    /// Immediately resolve the newline trivia
    pub fn finish(self) -> ParseToken {
        Trivia::Newline.into()
    }

    /// Injects a character, and attempts to build a newline trivia
    pub fn build(self, c: char) -> BuildState<Self> {
        match c {
            // consume a trailing \n token
            '\n' => BuildState::Consumed(self.finish()),
            // but anything else is ignored
            _ => BuildState::Bail(self.finish()),
        }
    }
}

/// Builds a divide token or comment trivia out of characters
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ForwardSlashBuilder;

impl ForwardSlashBuilder {
    /// Immediately resolve the divide token
    pub fn finish(self) -> ParseToken {
        Token::Divide.into()
    }

    /// Injects a character, and attempts to build a divide token or comment trivia
    pub fn build(self, c: char) -> BuildState<CommentBuilder> {
        match c {
            // if we get a slash, then start processing the comment
            '/' => BuildState::Pending(CommentBuilder),
            // if we get anything else, then the slash is a divide
            _ => BuildState::Bail(self.finish()),
        }
    }
}

/// Builds a multiline string out of characters, starting from a backslash
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct BackSlashBuilder;

impl BackSlashBuilder {
    /// Immediately resolve the backslash as an invalid token
    pub fn finish(self) -> ParseToken {
        Trivia::Invalid.into()
    }

    /// Injects a character, and attempts to start a multiline string
    pub fn build(self, c: char) -> BuildState<MultiLineStrBuilder> {
        match c {
            // if we get another backslash, then we can officially start the string
            '\\' => BuildState::Pending(MultiLineStrBuilder),
            // otherwise we bail on any other token
            _ => BuildState::Bail(self.finish()),
        }
    }
}

/// Builds a multiline string token out of characters
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct MultiLineStrBuilder;

impl MultiLineStrBuilder {
    /// Immediately resolve the multiline string token
    pub fn finish(self) -> ParseToken {
        Token::MultiLineStrLiteral.into()
    }

    /// Injects a character, and attempts to build a multiline string token
    pub fn build(self, c: char) -> BuildState<MultiLineStrBuilder> {
        match c {
            // if we get a newline character, then this part of the multiline string is complete
            '\r' | '\n' => BuildState::Bail(self.finish()),
            // otherwise literally any other character is acceptable
            _ => BuildState::Pending(self),
        }
    }
}

/// Builds comment trivia out of characters
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct CommentBuilder;

impl CommentBuilder {
    /// Immediately resolve the comment trivia
    pub fn finish(self) -> ParseToken {
        Trivia::Comment.into()
    }

    /// Injects a character, and attempts to build a comment trivia
    pub fn build(self, c: char) -> BuildState<Self> {
        match c {
            // if we get a newline character, then the comment is complete
            '\r' | '\n' => BuildState::Bail(self.finish()),
            // otherwise literally any other character is acceptable
            _ => BuildState::Pending(self),
        }
    }
}

/// The range of possible identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Ident {
    Custom,
    Const,
    Pub,
    Let,
}

impl Ident {
    // helper consts for tracking keyword characters
    pub const PUB_FIRST: char = Self::PUB_CHARS[0];
    pub const LET_FIRST: char = Self::LET_CHARS[0];
    pub const CONST_FIRST: char = Self::CONST_CHARS[0];
    pub const PUB_CHARS: &[char] = &['p', 'u', 'b'];
    pub const LET_CHARS: &[char] = &['l', 'e', 't'];
    pub const CONST_CHARS: &[char] = &['c', 'o', 'n', 's', 't'];
}

/// Builds an identifier token out of chars
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
                Ident::LET_FIRST => Ident::Let,
                Ident::CONST_FIRST => Ident::Const,
                _ => Ident::Custom,
            },
        })
    }

    /// Returns true if the character is valid for the expected keyword
    pub fn keyword_char(&self, c: char) -> bool {
        match self.ident {
            Ident::Pub => Ident::PUB_CHARS.get(self.length) == Some(&c),
            Ident::Let => Ident::LET_CHARS.get(self.length) == Some(&c),
            Ident::Const => Ident::CONST_CHARS.get(self.length) == Some(&c),
            _ => false,
        }
    }

    /// Immediately resolve the identifier into a token
    pub fn finish(self) -> ParseToken {
        // Resolves the tracked keyword, downgrading incomplete prefixes to identifier
        // A prefix like "pu" matches so far but is not the full "pub" keyword.
        match self.ident {
            Ident::Pub if self.length == Ident::PUB_CHARS.len() => Token::Public.into(),
            Ident::Let if self.length == Ident::LET_CHARS.len() => Token::Variable.into(),
            Ident::Const if self.length == Ident::CONST_CHARS.len() => Token::Constant.into(),
            _ => Token::Identifier.into(),
        }
    }

    /// Injects a character, and attempts to build an ident token
    pub fn build(mut self, c: char) -> BuildState<Self> {
        // if the character is not a valid body character
        if !utils::is_valid_ident_body(c) {
            // then we can just bail with the finished token
            return BuildState::Bail(self.finish());
        }

        // if the character is not valid for the expected keyword
        if !self.keyword_char(c) {
            // then set the identifier to custom
            self.ident = Ident::Custom;
        }

        // increment the length of the token for the next iteration
        self.length += 1;

        BuildState::Pending(self)
    }
}

/// Builds an inline string token out of chars
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct InlineStrBuilder {
    escaping: bool,
}

impl InlineStrBuilder {
    /// Returns a new empty inline string builder
    pub fn new() -> Self {
        Self { escaping: false }
    }

    /// Immediately resolve the string a an unclosed literal
    pub fn finish(self) -> ParseToken {
        Trivia::Invalid.into()
    }

    /// Injects a character, and attempts to build an inline string token
    pub fn build(mut self, c: char) -> BuildState<Self> {
        match c {
            // newline tokens mean the inline string is unclosed
            // newlines cannot be escaped so this is unconditional
            // we bail here so the parser can detect the newline still
            '\r' | '\n' => return BuildState::Bail(Trivia::Invalid.into()),

            // escape characters start a new escape sequence if there isnt one already
            '\\' if !self.escaping => self.escaping = true,

            // if not currently escaping, quotes close the string
            // the quotation char should be consumed in the process
            '"' if !self.escaping => {
                return BuildState::Consumed(Token::InlineStrLiteral.into());
            }

            // any other character regardless of escape status is valid
            // in any case, the escape sequence should be reset to false
            _ => self.escaping = false,
        };

        BuildState::Pending(self)
    }
}

#[derive(Debug, From, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum TokenBuilder {
    Clear,
    Ident(IdentBuilder),
    InlineStr(InlineStrBuilder),
    MultiLineStr(MultiLineStrBuilder),
    ForwardSlash(ForwardSlashBuilder),
    BackSlash(BackSlashBuilder),
    Comment(CommentBuilder),
    Newline(NewlineBuilder),
}

impl TokenBuilder {
    /// Injects a character, and attempts to build a token
    pub fn build(self, c: char) -> BuildState<Self> {
        match self {
            Self::Ident(builder) => builder.build(c).map(Into::into),
            Self::InlineStr(builder) => builder.build(c).map(Into::into),
            Self::MultiLineStr(builder) => builder.build(c).map(Into::into),
            Self::ForwardSlash(builder) => builder.build(c).map(Into::into),
            Self::BackSlash(builder) => builder.build(c).map(Into::into),
            Self::Comment(builder) => builder.build(c).map(Into::into),
            Self::Newline(builder) => builder.build(c).map(Into::into),
            Self::Clear => match c {
                // immediately consume and return simple char tokens
                ' ' => BuildState::Consumed(Trivia::Space.into()),
                '.' => BuildState::Consumed(Token::AccessMember.into()),
                ',' => BuildState::Consumed(Token::Delimiter.into()),
                '=' => BuildState::Consumed(Token::Assign.into()),
                ':' => BuildState::Consumed(Token::AccessScope.into()),
                ';' => BuildState::Consumed(Token::CloseExpr.into()),
                '(' => BuildState::Consumed(Token::OpenGroup.into()),
                ')' => BuildState::Consumed(Token::CloseGroup.into()),
                '{' => BuildState::Consumed(Token::OpenScope.into()),
                '}' => BuildState::Consumed(Token::CloseScope.into()),
                '\t' => BuildState::Consumed(Trivia::Tab.into()),
                '\n' => BuildState::Consumed(Trivia::Newline.into()),

                // specific starting characters trigger specific builders
                '/' => BuildState::Pending(ForwardSlashBuilder.into()),
                '"' => BuildState::Pending(InlineStrBuilder::new().into()),
                '\\' => BuildState::Pending(BackSlashBuilder.into()),
                '\r' => BuildState::Pending(NewlineBuilder.into()),

                // lastly, try to build an identifier with any other character
                c => match IdentBuilder::new(c) {
                    // if it is valid, then return the pending ident builder
                    Some(builder) => BuildState::Pending(builder.into()),
                    // if it is invalid, then consume the invalid token
                    None => BuildState::Consumed(Trivia::Invalid.into()),
                },
            },
        }
    }

    /// Immediately resolve the builder into any pending token
    ///
    /// An empty builder has nothing pending and yields no token
    pub fn finish(self) -> Option<ParseToken> {
        match self {
            Self::Clear => None,
            Self::Ident(builder) => Some(builder.finish()),
            Self::InlineStr(builder) => Some(builder.finish()),
            Self::MultiLineStr(builder) => Some(builder.finish()),
            Self::ForwardSlash(builder) => Some(builder.finish()),
            Self::BackSlash(builder) => Some(builder.finish()),
            Self::Comment(builder) => Some(builder.finish()),
            Self::Newline(builder) => Some(builder.finish()),
        }
    }
}

/// A lexer that turns source text into a stream of [`Token`]s.
/// Also tracks the byte span of the most recently produced token.
pub struct Lexer<'source> {
    builder: TokenBuilder,
    chars: Peekable<Chars<'source>>,
    source: &'source str,
    start: usize,
    end: usize,
}

impl<'source> Lexer<'source> {
    /// Creates a new [`Lexer`] over the given source text.
    pub fn new(source: &'source str) -> Self {
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

    /// The slice of source text covered by the current token.
    pub fn content(&self) -> &'source str {
        &self.source[self.range()]
    }

    /// Continues lexing into a new source buffer,
    /// picking up where the current lexer left off.
    ///
    /// This means that any mid-parse of strings or idents will still be completed.
    pub fn extend<'a>(self, source: &'a str) -> Lexer<'a> {
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
    pub fn finish(self) -> Option<ParseToken> {
        self.builder.finish()
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = ParseToken;

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
    fn tokenize(source: &str) -> Vec<ParseToken> {
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
                Token::AccessMember.into(),
                Token::Delimiter.into(),
                Token::Assign.into(),
                Token::AccessScope.into(),
                Token::CloseExpr.into(),
                Token::OpenGroup.into(),
                Token::CloseGroup.into(),
                Token::OpenScope.into(),
                Token::CloseScope.into(),
            ]
        );
    }

    #[test]
    fn lexer_tokenizes_custom_identifier() {
        // a run of valid identifier characters becomes one identifier
        let tokens = tokenize("foo_bar1");
        assert_eq!(tokens, vec![Token::Identifier.into()]);
    }

    #[test]
    fn lexer_tokenizes_keywords() {
        // a known keyword resolves to its dedicated variant
        let tokens = tokenize("pub const");
        assert_eq!(
            tokens,
            vec![
                Token::Public.into(),
                Trivia::Space.into(),
                Token::Constant.into()
            ]
        );
    }

    #[test]
    fn lexer_treats_keyword_prefix_as_custom() {
        // a prefix of a keyword is still just an identifier
        let tokens = tokenize("pu");
        assert_eq!(tokens, vec![Token::Identifier.into()]);
    }

    #[test]
    fn lexer_treats_keyword_superstring_as_custom() {
        // an identifier that merely starts with a keyword stays an identifier
        // this also guards against indexing past the end of the keyword
        let tokens = tokenize("public");
        assert_eq!(tokens, vec![Token::Identifier.into()]);
    }

    #[test]
    fn lexer_tokenizes_closed_string_literal() {
        // a terminated string literal is a closed string token
        let tokens = tokenize("\"hi\"");
        assert_eq!(tokens, vec![Token::InlineStrLiteral.into()]);
    }

    #[test]
    fn lexer_tokenizes_escaped_quote_in_string() {
        // an escaped quote does not close the string
        let tokens = tokenize("\"a\\\"b\"");
        assert_eq!(tokens, vec![Token::InlineStrLiteral.into()]);
    }

    #[test]
    fn lexer_tokenizes_unclosed_string_literal_at_end() {
        // a string with no closing quote before the end is invalid
        let tokens = tokenize("\"hi");
        assert_eq!(tokens, vec![Trivia::Invalid.into()]);
    }

    #[test]
    fn lexer_tokenizes_unclosed_string_literal_at_newline() {
        // a newline terminates an inline string as invalid
        // the newline is left so later tokens can still see it
        let tokens = tokenize("\"hi\n");
        assert_eq!(tokens, vec![Trivia::Invalid.into(), Trivia::Newline.into()]);
    }

    #[test]
    fn lexer_tokenizes_multiline_string_at_end() {
        // a double backslash starts a multiline string that runs to the end
        let tokens = tokenize("\\\\hello");
        assert_eq!(tokens, vec![Token::MultiLineStrLiteral.into()]);
    }

    #[test]
    fn lexer_tokenizes_multiline_string_at_newline() {
        // a newline ends the multiline string, left for the next token
        let tokens = tokenize("\\\\hello\n");
        assert_eq!(
            tokens,
            vec![Token::MultiLineStrLiteral.into(), Trivia::Newline.into()]
        );
    }

    #[test]
    fn lexer_reports_lone_backslash_as_invalid() {
        // a single backslash is not a multiline string and is invalid
        let tokens = tokenize("\\");
        assert_eq!(tokens, vec![Trivia::Invalid.into()]);
    }

    #[test]
    fn lexer_reports_backslash_before_other_token_as_invalid() {
        // a backslash not followed by another backslash bails as invalid
        // the next character is left so it can still be lexed
        let tokens = tokenize("\\foo");
        assert_eq!(
            tokens,
            vec![Trivia::Invalid.into(), Token::Identifier.into()]
        );
    }

    #[test]
    fn lexer_reports_invalid_characters() {
        // unrecognised characters surface as invalid tokens
        let tokens = tokenize("$");
        assert_eq!(tokens, vec![Trivia::Invalid.into()]);
    }

    #[test]
    fn lexer_emits_trailing_identifier() {
        // an identifier that ends at the source boundary is still emitted
        let tokens = tokenize("foo");
        assert_eq!(tokens, vec![Token::Identifier.into()]);
    }

    #[test]
    fn lexer_leaves_pending_token_for_flush() {
        // next stops at the end of source without finishing the pending token
        // so it can still be extended, and flush resolves it afterwards
        let mut lexer = Lexer::new("foo");
        assert_eq!(lexer.next(), None);
        assert_eq!(lexer.finish(), Some(Token::Identifier.into()));
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
        assert_eq!(lexer.finish(), Some(Token::Public.into()));
    }

    #[test]
    fn lexer_emits_whitespace_as_trivia() {
        // each whitespace character surfaces as its own trivia token
        let tokens = tokenize("  foo \t bar\n");
        assert_eq!(
            tokens,
            vec![
                Trivia::Space.into(),
                Trivia::Space.into(),
                Token::Identifier.into(),
                Trivia::Space.into(),
                Trivia::Tab.into(),
                Trivia::Space.into(),
                Token::Identifier.into(),
                Trivia::Newline.into(),
            ]
        );
    }

    #[test]
    fn lexer_tokenizes_newline_from_line_feed() {
        // a bare line feed is a single newline trivia
        let tokens = tokenize("\n");
        assert_eq!(tokens, vec![Trivia::Newline.into()]);
    }

    #[test]
    fn lexer_tokenizes_newline_from_carriage_return() {
        // a carriage return followed by a line feed is one newline trivia
        let tokens = tokenize("\r\n");
        assert_eq!(tokens, vec![Trivia::Newline.into()]);
    }

    #[test]
    fn lexer_tokenizes_lone_carriage_return_as_newline() {
        // a carriage return alone still resolves to a newline trivia
        // the following character is bailed so it can be lexed next
        let tokens = tokenize("\rfoo");
        assert_eq!(
            tokens,
            vec![Trivia::Newline.into(), Token::Identifier.into()]
        );
    }

    #[test]
    fn lexer_tokenizes_divide_operator() {
        // a lone slash that is not a comment is a divide operator
        let tokens = tokenize("/");
        assert_eq!(tokens, vec![Token::Divide.into()]);
    }

    #[test]
    fn lexer_tokenizes_divide_before_other_token() {
        // a slash followed by a non-slash bails, leaving the next char to lex
        let tokens = tokenize("/foo");
        assert_eq!(tokens, vec![Token::Divide.into(), Token::Identifier.into()]);
    }

    #[test]
    fn lexer_tokenizes_comment() {
        // a double slash starts a comment that runs to the end of source
        let tokens = tokenize("// a comment");
        assert_eq!(tokens, vec![Trivia::Comment.into()]);
    }

    #[test]
    fn lexer_stops_comment_at_newline() {
        // a comment ends at a newline, which is left for the next token
        let tokens = tokenize("// comment\nfoo");
        assert_eq!(
            tokens,
            vec![
                Trivia::Comment.into(),
                Trivia::Newline.into(),
                Token::Identifier.into()
            ]
        );
    }

    #[test]
    fn lexer_stops_comment_at_carriage_return() {
        // a lone carriage return also ends a comment, left for the next token
        let tokens = tokenize("// comment\rfoo");
        assert_eq!(
            tokens,
            vec![
                Trivia::Comment.into(),
                Trivia::Newline.into(),
                Token::Identifier.into()
            ]
        );
    }

    #[test]
    fn lexer_tracks_token_range() {
        // the range covers the slice of source for the current token
        let mut lexer = Lexer::new("foo,bar");
        lexer.next();
        assert_eq!(lexer.range(), 0..3);
        assert_eq!(lexer.content(), "foo");
    }

    #[test]
    fn lexer_advances_range_per_token() {
        // the range moves to cover each token, including whitespace trivia
        let mut lexer = Lexer::new("foo  bar");
        lexer.next();
        assert_eq!(lexer.content(), "foo");
        // step past the two spaces separating the identifiers
        lexer.next();
        assert_eq!(lexer.range(), 3..4);
        assert_eq!(lexer.content(), " ");
        lexer.next();
        lexer.next();
        assert_eq!(lexer.range(), 5..8);
        assert_eq!(lexer.content(), "bar");
    }

    #[test]
    fn lexer_stops_identifier_at_boundary() {
        // an identifier ends at the first non-identifier character
        let tokens = tokenize("foo.bar");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier.into(),
                Token::AccessMember.into(),
                Token::Identifier.into()
            ]
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
