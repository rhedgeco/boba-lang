use std::{iter::Peekable, mem::take, ops::Range, str::Chars};

// re-export token at crate root
pub use boba_token::Token;

use derive_more::From;

use crate::{
    backslash::BackSlashLexer,
    comment::CommentLexer,
    ident::IdentLexer,
    inline_string::InlineStringLexer,
    keyword::{Constant, KeywordInfo, KeywordLexer, Public, Variable},
    multiline_string::MultilineStrLexer,
    newline::NewlineLexer,
    slash::SlashLexer,
};

pub mod backslash;
pub mod comment;
pub mod ident;
pub mod inline_string;
pub mod keyword;
pub mod multiline_string;
pub mod newline;
pub mod slash;

/// Shared test helpers for the lexer and its sub-lexers.
#[cfg(test)]
pub(crate) mod test_utils;

/// Non lexical tokens produced by the lexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Trivia {
    /// An invalid token.
    Invalid,
    /// A string that is missing its closing quote.
    UnclosedStr,
    /// A line comment introduced with `//`.
    Comment,
    /// A line break from either `\n`, `\r\n`, or just `\r`.
    Newline,
    /// A single space character.
    Space,
    /// A single tab character.
    Tab,
}

/// Either a lexical or trivia token.
#[derive(Debug, From, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LexerOutput {
    /// A valid lexical token.
    Token(Token),
    /// A supporting trivia token.
    Trivia(Trivia),
}

/// The outcome of feeding one character to a lexer builder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LexerResult<T> {
    /// A token was produced, and the character was consumed.
    Consume(LexerOutput),
    /// A token was produced, and the character was ignored.
    Bail(LexerOutput),
    /// A token is still pending.
    Next(T),
}

impl<L> LexerResult<L> {
    /// Maps the pending lexer into another using its [`Into`] conversion.
    pub fn map_into<L2>(self) -> LexerResult<L2>
    where
        L: Into<L2>,
    {
        self.map(Into::into)
    }

    /// Maps the pending lexer with `f`, leaving any produced token untouched.
    pub fn map<L2>(self, f: impl FnOnce(L) -> L2) -> LexerResult<L2> {
        match self {
            LexerResult::Next(lexer) => LexerResult::Next(f(lexer)),
            LexerResult::Consume(out) => LexerResult::Consume(out),
            LexerResult::Bail(out) => LexerResult::Bail(out),
        }
    }
}

/// A state machine that builds a single token out of characters.
pub trait LexerBuilder {
    /// The state to transition into after consuming a character.
    type NextState: LexerBuilder;
    /// Immediately resolves any token still pending.
    fn finish(self) -> Option<LexerOutput>;
    /// Injects a character, and attempts to build a token.
    fn build(self, c: char) -> LexerResult<Self::NextState>;
}

/// The active sub-lexer, or empty when between tokens.
#[derive(Default, Debug, From, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum LexerState {
    #[default]
    Empty,
    Ident(IdentLexer),
    Public(KeywordLexer<Public>),
    Variable(KeywordLexer<Variable>),
    Constant(KeywordLexer<Constant>),
    Newline(NewlineLexer),
    InlineStr(InlineStringLexer),
    Slash(SlashLexer),
    Comment(CommentLexer),
    BackSlash(BackSlashLexer),
    MultilineStr(MultilineStrLexer),
}

impl LexerBuilder for LexerState {
    type NextState = Self;

    fn finish(self) -> Option<LexerOutput> {
        match self {
            Self::Empty => None,
            Self::Ident(lexer) => lexer.finish(),
            Self::Public(lexer) => lexer.finish(),
            Self::Variable(lexer) => lexer.finish(),
            Self::Constant(lexer) => lexer.finish(),
            Self::Newline(lexer) => lexer.finish(),
            Self::InlineStr(lexer) => lexer.finish(),
            Self::Slash(lexer) => lexer.finish(),
            Self::Comment(lexer) => lexer.finish(),
            Self::BackSlash(lexer) => lexer.finish(),
            Self::MultilineStr(lexer) => lexer.finish(),
        }
    }

    fn build(self, c: char) -> LexerResult<Self::NextState> {
        // create a macro for consuming simple tokens
        macro_rules! consume {
            ($kind:ident::$variant:ident) => {
                LexerResult::Consume($kind::$variant.into())
            };
        }

        match self {
            // try to build any lexer state that is currently active
            LexerState::Ident(lexer) => lexer.build(c).map_into(),
            LexerState::Public(lexer) => lexer.build(c).map_into(),
            LexerState::Variable(lexer) => lexer.build(c).map_into(),
            LexerState::Constant(lexer) => lexer.build(c).map_into(),
            LexerState::Newline(lexer) => lexer.build(c).map_into(),
            LexerState::InlineStr(lexer) => lexer.build(c).map_into(),
            LexerState::Slash(lexer) => lexer.build(c).map_into(),
            LexerState::Comment(lexer) => lexer.build(c).map_into(),
            LexerState::BackSlash(lexer) => lexer.build(c).map_into(),
            LexerState::MultilineStr(lexer) => lexer.build(c).map_into(),

            // otherwise match the character and progress the lexer
            LexerState::Empty => match c {
                // some simple characters can be processed immediately
                ' ' => consume!(Trivia::Space),
                '.' => consume!(Token::AccessMember),
                ',' => consume!(Token::Delimiter),
                '=' => consume!(Token::Assign),
                '+' => consume!(Token::Add),
                '-' => consume!(Token::Subtract),
                '*' => consume!(Token::Multiply),
                '%' => consume!(Token::Modulo),
                ':' => consume!(Token::AccessScope),
                ';' => consume!(Token::CloseExpr),
                '(' => consume!(Token::OpenGroup),
                ')' => consume!(Token::CloseGroup),
                '{' => consume!(Token::OpenScope),
                '}' => consume!(Token::CloseScope),
                '\t' => consume!(Trivia::Tab),
                '\n' => consume!(Trivia::Newline),

                // other lexers come with PRIMER helpers to match the char that triggers it
                NewlineLexer::PRIMER => LexerResult::Next(NewlineLexer.into()),
                InlineStringLexer::PRIMER => LexerResult::Next(InlineStringLexer::new().into()),
                SlashLexer::PRIMER => LexerResult::Next(SlashLexer.into()),
                BackSlashLexer::PRIMER => LexerResult::Next(BackSlashLexer.into()),
                Public::PRIMER => LexerResult::Next(KeywordLexer::<Public>::primed().into()),
                Variable::PRIMER => LexerResult::Next(KeywordLexer::<Variable>::primed().into()),
                Constant::PRIMER => LexerResult::Next(KeywordLexer::<Constant>::primed().into()),

                // and some lexers have to be primed with a function
                c if IdentLexer::primer(c) => LexerResult::Next(IdentLexer.into()),

                // always fallback and produce invalid trivia
                _ => LexerResult::Consume(Trivia::Invalid.into()),
            },
        }
    }
}

/// Turns source text into a stream of [`LexerOutput`]s.
///
/// Tracks the byte span of the most recently produced token.
pub struct Lexer<'a> {
    state: LexerState,
    chars: Peekable<Chars<'a>>,
    source: &'a str,
    start: usize,
    end: usize,
}

impl<'a> Lexer<'a> {
    /// Creates a new lexer over the given source text.
    pub fn new(source: &'a str) -> Self {
        Self {
            state: LexerState::Empty,
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

    /// The full source text the lexer is reading from.
    pub fn source(&self) -> &'a str {
        self.source
    }

    /// The slice of source text covered by the current token.
    pub fn token_source(&self) -> &'a str {
        &self.source()[self.range()]
    }

    /// Continues lexing into a new source buffer, keeping the current state.
    ///
    /// Any token mid-parse across the boundary is still completed.
    pub fn extend<'b>(self, source: &'b str) -> Lexer<'b> {
        Lexer {
            state: self.state,
            ..Lexer::new(source)
        }
    }

    /// Consumes the lexer and resolves any token still pending.
    ///
    /// Does not parse any more characters from the source text.
    pub fn finish(self) -> Option<LexerOutput> {
        self.state.finish()
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = LexerOutput;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // get the next char in the chain
            let next_char = *self.chars.peek()?;

            // if the lexer is in an empty state
            // we should reset the token start index
            if matches!(self.state, LexerState::Empty) {
                self.start = self.end;
            }

            // then try to build the next output token
            let state = take(&mut self.state);
            return Some(match state.build(next_char) {
                // if the lexer state bailed
                // just return the output as is
                LexerResult::Bail(output) => output,

                // if the lexer state consumed the token
                LexerResult::Consume(output) => {
                    let _ = self.chars.next(); // consume the character
                    self.end += 1; // advance the token end index
                    output // and return the output
                }

                // if the lexer state is incomplete
                LexerResult::Next(state) => {
                    let _ = self.chars.next(); // consume the character
                    self.state = state; // inject the new state
                    self.end += 1; // advance the token end index
                    continue; // and continue to the next char
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::tokenize;

    #[test]
    fn empty_primes_newline_lexer_on_carriage_return() {
        // a carriage return from empty primes the newline lexer
        let result = LexerState::Empty.build(NewlineLexer::PRIMER);
        assert_eq!(result, LexerResult::Next(NewlineLexer.into()));
    }

    #[test]
    fn empty_primes_inline_string_lexer_on_quote() {
        // a quote from empty primes a fresh inline string lexer
        let result = LexerState::Empty.build(InlineStringLexer::PRIMER);
        assert_eq!(result, LexerResult::Next(InlineStringLexer::new().into()));
    }

    #[test]
    fn empty_primes_slash_lexer_on_slash() {
        // a slash from empty primes the slash lexer
        let result = LexerState::Empty.build(SlashLexer::PRIMER);
        assert_eq!(result, LexerResult::Next(SlashLexer.into()));
    }

    #[test]
    fn empty_primes_backslash_lexer_on_backslash() {
        // a backslash from empty primes the backslash lexer
        let result = LexerState::Empty.build(BackSlashLexer::PRIMER);
        assert_eq!(result, LexerResult::Next(BackSlashLexer.into()));
    }

    #[test]
    fn empty_primes_ident_lexer_on_ident_start() {
        // an identifier start character from empty primes the ident lexer
        let result = LexerState::Empty.build('a');
        assert_eq!(result, LexerResult::Next(IdentLexer.into()));
    }

    #[test]
    fn empty_consumes_simple_token() {
        // a single character token is consumed immediately from empty
        let result = LexerState::Empty.build(',');
        assert_eq!(result, LexerResult::Consume(Token::Delimiter.into()));
    }

    #[test]
    fn empty_consumes_unrecognised_char_as_invalid() {
        // an unrecognised character is consumed as invalid trivia
        let result = LexerState::Empty.build('$');
        assert_eq!(result, LexerResult::Consume(Trivia::Invalid.into()));
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
    fn lexer_tokenizes_arithmetic_operators() {
        // each arithmetic symbol maps to its own operator token
        let tokens = tokenize("+-*%");
        assert_eq!(
            tokens,
            vec![
                Token::Add.into(),
                Token::Subtract.into(),
                Token::Multiply.into(),
                Token::Modulo.into(),
            ]
        );
    }

    #[test]
    fn lexer_reports_invalid_characters() {
        // unrecognised characters surface as invalid trivia
        let tokens = tokenize("$");
        assert_eq!(tokens, vec![Trivia::Invalid.into()]);
    }

    #[test]
    fn lexer_leaves_pending_token_for_flush() {
        // next stops at the end of source without finishing the pending token
        // so it can still be extended, and flush resolves it afterwards
        let mut lexer = Lexer::new("foo");
        assert_eq!(lexer.by_ref().count(), 0);
        assert_eq!(lexer.finish(), Some(Token::Identifier.into()));
    }

    #[test]
    fn lexer_flush_on_empty_yields_nothing() {
        // with no token in progress there is nothing to flush
        let mut lexer = Lexer::new("foo,");
        let _ = lexer.by_ref().count();
        assert_eq!(lexer.finish(), None);
    }

    #[test]
    fn lexer_extends_token_across_buffers() {
        // a token split across two buffers resolves once both are lexed
        // the identifier only completes after the second buffer is consumed
        let mut lexer = Lexer::new("fo");
        assert_eq!(lexer.by_ref().count(), 0);
        let mut lexer = lexer.extend("o");
        assert_eq!(lexer.by_ref().count(), 0);
        assert_eq!(lexer.finish(), Some(Token::Identifier.into()));
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
    fn lexer_tracks_token_range() {
        // the range covers the slice of source for the current token
        let mut lexer = Lexer::new("foo,bar");
        lexer.next();
        assert_eq!(lexer.range(), 0..3);
        assert_eq!(lexer.token_source(), "foo");
    }

    #[test]
    fn lexer_advances_range_per_token() {
        // the range moves to cover each token, including whitespace trivia
        let mut lexer = Lexer::new("foo  bar");
        lexer.next();
        assert_eq!(lexer.token_source(), "foo");
        // step past the two spaces separating the identifiers
        lexer.next();
        assert_eq!(lexer.range(), 3..4);
        assert_eq!(lexer.token_source(), " ");
        lexer.next();
        lexer.next();
        assert_eq!(lexer.range(), 5..8);
        assert_eq!(lexer.token_source(), "bar");
    }
}
