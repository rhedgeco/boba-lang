//! Iterator support for lexing, including peeking and output filtering.

use std::{iter::Peekable, ops::Range, str::Chars};

use boba_token::Token;

use crate::{LexOutput, LexState, Trivia};

/// A character iterator that can look at the next character without consuming it.
///
/// This abstracts over the lexer's input so any one character lookahead source works.
/// The [`peek`](PeekableChar::peek) method returns the next character without advancing,
/// while [`next`](Iterator::next) consumes it as usual.
pub trait PeekableChar: Iterator<Item = char> {
    /// Returns the next character without consuming it.
    ///
    /// Returns [`None`] when the input is exhausted.
    fn peek(&mut self) -> Option<char>;
}

impl<I: PeekableChar> PeekableChar for &mut I {
    fn peek(&mut self) -> Option<char> {
        I::peek(self)
    }
}

impl<I> PeekableChar for std::iter::Peekable<I>
where
    I: Iterator<Item = char>,
{
    fn peek(&mut self) -> Option<char> {
        std::iter::Peekable::peek(self).copied()
    }
}

/// Convenience methods for consuming and peeking a [`PeekableChar`].
pub trait PeekableCharExt: PeekableChar {
    /// Consumes the next character, discarding it.
    fn consume(&mut self) {
        let _ = self.next();
    }

    /// Consumes the next character, returning `and`.
    ///
    /// `and` is returned, even if there was no char to consume.
    fn consume_and<T>(&mut self, and: T) -> T {
        self.consume();
        and
    }

    /// Consumes the next character, then calls `then` with this input.
    fn consume_then<'a: 'b, 'b, T>(&'a mut self, then: impl FnOnce(&'b mut Self) -> T) -> T {
        self.consume();
        then(self)
    }

    /// Peeks the next character, returning [`Ok`] if it exists.
    ///
    /// Returns [`Err`] containing `or` if there was no next character.
    fn peek_or(&mut self, or: LexState) -> Result<char, LexState> {
        self.peek().ok_or(or)
    }

    /// Takes the next character, returning [`Ok`] if it exists.
    ///
    /// Returns [`Err`] containing `or` if there was no next character.
    fn next_or(&mut self, or: LexState) -> Result<char, LexState> {
        let next_char = self.peek_or(or)?;
        let _ = self.next();
        Ok(next_char)
    }
}
impl<I: PeekableChar> PeekableCharExt for I {}

/// An iterator that yields only the trivia from a stream of lexer output.
///
/// Token output is skipped, so only [`Trivia`] values are produced.
/// Created with [`LexerIterExt::trivia`].
pub struct TriviaIter<I> {
    inner: I,
}

impl<I: Iterator<Item = LexOutput>> TriviaIter<I> {
    /// Creates a trivia iterator over the given lexer output.
    pub fn from_output_iter(iter: I) -> Self {
        Self { inner: iter }
    }
}

impl<I: Iterator<Item = LexOutput>> Iterator for TriviaIter<I> {
    type Item = Trivia;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            return match self.inner.next()? {
                LexOutput::Trivia(trivia) => Some(trivia),
                _ => continue,
            };
        }
    }
}

/// An iterator that yields only the tokens from a stream of lexer output.
///
/// Trivia output is skipped, so only [`Token`] values are produced.
/// Created with [`LexerIterExt::tokens`].
pub struct TokenIter<I> {
    inner: I,
}

impl<I: Iterator<Item = LexOutput>> TokenIter<I> {
    /// Creates a token iterator over the given lexer output.
    pub fn from_output_iter(iter: I) -> Self {
        Self { inner: iter }
    }
}

impl<I: Iterator<Item = LexOutput>> Iterator for TokenIter<I> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            return match self.inner.next()? {
                LexOutput::Token(token) => Some(token),
                _ => continue,
            };
        }
    }
}

/// Extension methods for iterators of lexer output.
///
/// These adapt a raw [`LexOutput`] stream into one that yields only tokens or only trivia.
pub trait LexerIterExt: Iterator<Item = LexOutput> {
    /// Adapts this iterator to yield only its trivia, skipping tokens.
    fn trivia(self) -> TriviaIter<Self>
    where
        Self: Sized,
    {
        TriviaIter::from_output_iter(self)
    }

    /// Adapts this iterator to yield only its tokens, skipping trivia.
    fn tokens(self) -> TokenIter<Self>
    where
        Self: Sized,
    {
        TokenIter::from_output_iter(self)
    }
}
impl<I: Iterator<Item = LexOutput>> LexerIterExt for I {}

/// A character iterator that tracks the byte range of the current token.
pub struct RangeCounter<I: Iterator<Item = char>> {
    inner: Peekable<I>,
    start: usize,
    end: usize,
}

impl<I: Iterator<Item = char>> RangeCounter<I> {
    /// Creates a range counter over the given character iterator.
    pub fn from_char_iter(iter: I) -> Self {
        Self {
            inner: iter.peekable(),
            start: 0,
            end: 0,
        }
    }

    /// Moves the start of the tracked range up to the current end.
    ///
    /// This should be called after a token is produced so that the next
    /// [`range`](RangeCounter::range) begins where the previous token ended.
    pub fn reset_range(&mut self) {
        self.start = self.end;
    }

    /// Returns the byte range consumed since the last [`reset_range`](RangeCounter::reset_range).
    pub fn range(&self) -> Range<usize> {
        self.start..self.end
    }
}

impl<I: Iterator<Item = char>> PeekableChar for RangeCounter<I> {
    fn peek(&mut self) -> Option<char> {
        self.inner.peek().copied()
    }
}

impl<I: Iterator<Item = char>> Iterator for RangeCounter<I> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        let next_char = self.inner.next()?;
        self.end += next_char.len_utf8();
        Some(next_char)
    }
}

/// Extension trait for wrapping a character iterator in a [`RangeCounter`].
pub trait RangeCounterExt: Iterator<Item = char> {
    /// Wraps this iterator in a [`RangeCounter`] that tracks consumed byte ranges.
    ///
    /// The tracked range starts empty at byte offset `0`.
    fn range_counter(self) -> RangeCounter<Self>
    where
        Self: Sized,
    {
        RangeCounter::from_char_iter(self)
    }
}
impl<I: Iterator<Item = char>> RangeCounterExt for I {}

/// An iterator that lexes a stream of characters into [`LexOutput`].
///
/// Each call yields the next token or trivia from the input.
/// A token still in progress when the input ends is finished and produced.
/// Created with [`LexerExt::lexer`] or [`Lexer::from_source`].
pub struct Lexer<I: Iterator<Item = char>> {
    inner: ExtendableLexer<I>,
}

impl<'a> Lexer<Chars<'a>> {
    /// Creates a lexer over the characters of `source`.
    pub fn from_source(source: &'a str) -> Self {
        Self::from_char_iter(source.chars())
    }
}

impl<I: Iterator<Item = char>> Lexer<I> {
    /// Creates a lexer over the given character iterator.
    pub fn from_char_iter(iter: I) -> Self {
        Self {
            inner: ExtendableLexer::from_char_iter(iter),
        }
    }

    /// Returns the byte range of the most recently produced output.
    pub fn range(&self) -> Range<usize> {
        self.inner.range()
    }
}

impl<I: Iterator<Item = char>> Iterator for Lexer<I> {
    type Item = LexOutput;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next() {
            Some(output) => Some(output),
            None => self.inner.finish(),
        }
    }
}

/// A lexer that can continue over input supplied in separate pieces.
///
/// Unlike [`Lexer`], it does not finish a token in progress when the input ends.
/// Instead, lexing can be continued across a boundary with [`extend`](ExtendableLexer::extend),
/// so a token split between two pieces of input is produced as a single token.
/// Any token still in progress once no more input will follow is produced by
/// [`finish`](ExtendableLexer::finish).
/// Created with [`LexerExt::extendable_lexer`] or [`ExtendableLexer::from_source`].
pub struct ExtendableLexer<I: Iterator<Item = char>> {
    inner: RangeCounter<I>,
    state: Option<LexState>,
}

impl<'a> ExtendableLexer<Chars<'a>> {
    /// Creates a lexer over the characters of `source`.
    pub fn from_source(source: &'a str) -> Self {
        Self::from_char_iter(source.chars())
    }
}

impl<I: Iterator<Item = char>> ExtendableLexer<I> {
    /// Creates a lexer over the given character iterator.
    pub fn from_char_iter(iter: I) -> Self {
        Self {
            inner: iter.range_counter(),
            state: None,
        }
    }

    /// Returns the byte range of the most recently produced output.
    pub fn range(&self) -> Range<usize> {
        self.inner.range()
    }

    /// Continues lexing over the characters of `source`.
    ///
    /// A token left in progress when the previous input ended is resumed,
    /// so a token split across the boundary is produced as a single token.
    pub fn extend<'a>(&self, source: &'a str) -> ExtendableLexer<Chars<'a>> {
        self.extend_with(source.chars())
    }

    /// Continues lexing over the given character iterator.
    ///
    /// A token left in progress when the previous input ended is resumed,
    /// so a token split across the boundary is produced as a single token.
    pub fn extend_with<I2>(&self, iter: I2) -> ExtendableLexer<I2>
    where
        I2: Iterator<Item = char>,
    {
        ExtendableLexer {
            inner: iter.range_counter(),
            state: self.state,
        }
    }

    /// Finishes a token that is still in progress at the end of input.
    ///
    /// Returns [`None`] when there is no token in progress.
    pub fn finish(&mut self) -> Option<LexOutput> {
        Some(self.state.take()?.finish())
    }
}

impl<I: Iterator<Item = char>> Iterator for ExtendableLexer<I> {
    type Item = LexOutput;

    fn next(&mut self) -> Option<Self::Item> {
        // try to resume from an old state
        let result = match self.state.take() {
            Some(state) => state.try_parse(&mut self.inner),
            None => {
                self.inner.reset_range();
                crate::try_parse(&mut self.inner)?
            }
        };

        // extract the resulting token or state
        match result {
            Ok(output) => Some(output),
            Err(state) => {
                self.state = Some(state);
                None
            }
        }
    }
}

/// Extension methods for lexing a character iterator.
pub trait LexerExt: Iterator<Item = char> {
    /// Wraps this iterator in a [`Lexer`] that yields its tokens and trivia.
    fn lexer(self) -> Lexer<Self>
    where
        Self: Sized,
    {
        Lexer::from_char_iter(self)
    }

    /// Wraps this iterator in an [`ExtendableLexer`] that can continue over
    /// input supplied in separate pieces.
    fn extendable_lexer(self) -> ExtendableLexer<Self>
    where
        Self: Sized,
    {
        ExtendableLexer::from_char_iter(self)
    }
}
impl<I: Iterator<Item = char>> LexerExt for I {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peekable_char_peek_does_not_consume() {
        let mut input = "ab".chars().peekable();
        // peeking returns the next character without advancing
        assert_eq!(PeekableChar::peek(&mut input), Some('a'));
        assert_eq!(PeekableChar::peek(&mut input), Some('a'));
        // next consumes it, so the following peek sees the second character
        assert_eq!(input.next(), Some('a'));
        assert_eq!(PeekableChar::peek(&mut input), Some('b'));
    }

    #[test]
    fn peekable_char_peek_at_end_is_none() {
        let mut input = "".chars().peekable();
        assert_eq!(PeekableChar::peek(&mut input), None);
    }

    #[test]
    fn peekable_char_forwards_through_mut_ref() {
        // a mutable reference forwards peek to the underlying iterator
        let mut input = "x".chars().peekable();
        let mut by_ref = &mut input;
        assert_eq!(PeekableChar::peek(&mut by_ref), Some('x'));
    }

    // a mix of tokens and trivia used to exercise the filtering iterators
    fn sample() -> Vec<LexOutput> {
        vec![
            Token::Public.into(),
            Trivia::Space.into(),
            Token::Identifier.into(),
            Trivia::Newline.into(),
        ]
    }

    #[test]
    fn tokens_yields_only_tokens() {
        let tokens: Vec<Token> = sample().into_iter().tokens().collect();
        assert_eq!(tokens, vec![Token::Public, Token::Identifier]);
    }

    #[test]
    fn trivia_yields_only_trivia() {
        let trivia: Vec<Trivia> = sample().into_iter().trivia().collect();
        assert_eq!(trivia, vec![Trivia::Space, Trivia::Newline]);
    }

    #[test]
    fn tokens_on_all_trivia_is_empty() {
        let output = vec![Trivia::Space.into(), Trivia::Tab.into()];
        let tokens: Vec<Token> = output.into_iter().tokens().collect();
        assert!(tokens.is_empty());
    }

    #[test]
    fn trivia_on_all_tokens_is_empty() {
        let output = vec![Token::Add.into(), Token::Subtract.into()];
        let trivia: Vec<Trivia> = output.into_iter().trivia().collect();
        assert!(trivia.is_empty());
    }

    #[test]
    fn empty_stream_yields_nothing() {
        let empty: Vec<LexOutput> = Vec::new();
        assert_eq!(empty.clone().into_iter().tokens().count(), 0);
        assert_eq!(empty.into_iter().trivia().count(), 0);
    }

    #[test]
    fn range_counter_starts_with_an_empty_range() {
        let counter = "abc".chars().range_counter();
        assert_eq!(counter.range(), 0..0);
    }

    #[test]
    fn range_counter_next_advances_the_end_by_byte_length() {
        let mut counter = "abc".chars().range_counter();
        assert_eq!(counter.next(), Some('a'));
        assert_eq!(counter.range(), 0..1);
        assert_eq!(counter.next(), Some('b'));
        assert_eq!(counter.range(), 0..2);
    }

    #[test]
    fn range_counter_peek_does_not_advance_the_range() {
        let mut counter = "abc".chars().range_counter();
        assert_eq!(counter.peek(), Some('a'));
        // peeking must not change the tracked range
        assert_eq!(counter.range(), 0..0);
        // and the peeked char is still produced by next
        assert_eq!(counter.next(), Some('a'));
        assert_eq!(counter.range(), 0..1);
    }

    #[test]
    fn range_counter_multibyte_chars_advance_by_their_utf8_length() {
        // 'é' is two bytes, '🦀' is four bytes in UTF-8
        let mut counter = "é🦀".chars().range_counter();
        assert_eq!(counter.next(), Some('é'));
        assert_eq!(counter.range(), 0..2);
        assert_eq!(counter.next(), Some('🦀'));
        assert_eq!(counter.range(), 0..6);
    }

    #[test]
    fn range_counter_reset_range_moves_start_to_current_end() {
        let mut counter = "ab".chars().range_counter();
        counter.next();
        counter.reset_range();
        // the start now tracks from the second character onward
        assert_eq!(counter.range(), 1..1);
        counter.next();
        assert_eq!(counter.range(), 1..2);
    }

    #[test]
    fn range_counter_range_slices_the_original_source_per_token() {
        // tokens separated by spaces are recovered by resetting between them
        let source = "ab cd";
        let mut counter = source.chars().range_counter();

        // consume "ab"
        counter.next();
        counter.next();
        assert_eq!(&source[counter.range()], "ab");
        counter.reset_range();

        // consume the space
        counter.next();
        assert_eq!(&source[counter.range()], " ");
        counter.reset_range();

        // consume "cd"
        counter.next();
        counter.next();
        assert_eq!(&source[counter.range()], "cd");
    }

    #[test]
    fn range_counter_range_is_empty_when_no_chars_are_consumed_after_reset() {
        let mut counter = "ab".chars().range_counter();
        counter.next();
        counter.reset_range();
        // with nothing consumed since the reset, the range is empty
        assert_eq!(counter.range(), 1..1);
        assert!(counter.range().is_empty());
    }

    #[test]
    fn range_counter_exhausted_iterator_keeps_the_final_range() {
        let mut counter = "ab".chars().range_counter();
        counter.next();
        counter.next();
        // next past the end yields None and leaves the range unchanged
        assert_eq!(counter.next(), None);
        assert_eq!(counter.range(), 0..2);
    }

    #[test]
    fn lexer_yields_tokens_and_trivia_in_order() {
        let output: Vec<LexOutput> = "pub x".chars().lexer().collect();
        assert_eq!(
            output,
            vec![
                Token::Public.into(),
                Trivia::Space.into(),
                Token::Identifier.into(),
            ]
        );
    }

    #[test]
    fn lexer_finishes_a_trailing_token_at_end_of_input() {
        // a keyword that ends the input is still produced as its token
        let output: Vec<LexOutput> = "const".chars().lexer().collect();
        assert_eq!(output, vec![Token::Constant.into()]);
    }

    #[test]
    fn lexer_from_source_lexes_the_whole_string() {
        let output: Vec<LexOutput> = Lexer::from_source("a.b").collect();
        assert_eq!(
            output,
            vec![
                Token::Identifier.into(),
                Token::AccessMember.into(),
                Token::Identifier.into(),
            ]
        );
    }

    #[test]
    fn lexer_range_slices_the_source_of_the_last_output() {
        let source = "pub x";
        let mut lexer = source.chars().lexer();
        lexer.next();
        assert_eq!(&source[lexer.range()], "pub");
        lexer.next();
        assert_eq!(&source[lexer.range()], " ");
        lexer.next();
        assert_eq!(&source[lexer.range()], "x");
    }

    #[test]
    fn extendable_lexer_does_not_finish_a_trailing_token() {
        // unlike Lexer, a token still in progress at the end is not produced
        let output: Vec<LexOutput> = "const".chars().extendable_lexer().collect();
        assert!(output.is_empty());
    }

    #[test]
    fn extendable_lexer_finish_produces_the_trailing_token() {
        let mut lexer = "const".chars().extendable_lexer();
        // nothing is produced while the token is still in progress
        assert_eq!(lexer.next(), None);
        // finishing yields the token that was in progress
        assert_eq!(lexer.finish(), Some(Token::Constant.into()));
        // and there is nothing left to finish
        assert_eq!(lexer.finish(), None);
    }

    #[test]
    fn extendable_lexer_resumes_a_token_split_across_the_boundary() {
        // "con" + "st" is a single keyword once the pieces are joined
        let mut first = "con".chars().extendable_lexer();
        // the keyword is still in progress, so nothing is produced yet
        assert_eq!(first.next(), None);

        // continuing over the rest produces the keyword as one token
        let output: Vec<LexOutput> = first.extend("st ").collect();
        assert_eq!(output, vec![Token::Constant.into(), Trivia::Space.into()]);
    }

    #[test]
    fn extendable_lexer_resumed_token_is_not_repeated() {
        // once a resumed token is produced it must not be produced again
        let mut first = "pu".chars().extendable_lexer();
        assert_eq!(first.next(), None);

        // the trailing space gives the identifier a boundary to complete on
        let mut rest = first.extend("b x ");
        assert_eq!(rest.next(), Some(Token::Public.into()));
        assert_eq!(rest.next(), Some(Trivia::Space.into()));
        assert_eq!(rest.next(), Some(Token::Identifier.into()));
        assert_eq!(rest.next(), Some(Trivia::Space.into()));
        assert_eq!(rest.next(), None);
    }

    #[test]
    fn extendable_lexer_range_tracks_the_current_piece() {
        let source = "a.b";
        let mut lexer = ExtendableLexer::from_source(source);
        lexer.next();
        assert_eq!(&source[lexer.range()], "a");
        lexer.next();
        assert_eq!(&source[lexer.range()], ".");
    }
}
