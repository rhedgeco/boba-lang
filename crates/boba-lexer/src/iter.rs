//! Iterator support for lexing, including peeking and output filtering.

use std::{iter::Peekable, ops::Range};

use boba_token::Token;

use crate::{LexerOutput, LexerState, Trivia};

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
    fn peek_or(&mut self, or: LexerState) -> Result<char, LexerState> {
        self.peek().ok_or(or)
    }

    /// Takes the next character, returning [`Ok`] if it exists.
    ///
    /// Returns [`Err`] containing `or` if there was no next character.
    fn next_or(&mut self, or: LexerState) -> Result<char, LexerState> {
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
    lexer_iter: I,
}

impl<I: Iterator<Item = LexerOutput>> Iterator for TriviaIter<I> {
    type Item = Trivia;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            return match self.lexer_iter.next()? {
                LexerOutput::Trivia(trivia) => Some(trivia),
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
    lexer_iter: I,
}

impl<I: Iterator<Item = LexerOutput>> Iterator for TokenIter<I> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            return match self.lexer_iter.next()? {
                LexerOutput::Token(token) => Some(token),
                _ => continue,
            };
        }
    }
}

/// Extension methods for iterators of lexer output.
///
/// These adapt a raw [`LexerOutput`] stream into one that yields only tokens or only trivia.
pub trait LexerIterExt: Iterator<Item = LexerOutput> {
    /// Adapts this iterator to yield only its trivia, skipping tokens.
    fn trivia(self) -> TriviaIter<Self>
    where
        Self: Sized,
    {
        TriviaIter { lexer_iter: self }
    }

    /// Adapts this iterator to yield only its tokens, skipping trivia.
    fn tokens(self) -> TokenIter<Self>
    where
        Self: Sized,
    {
        TokenIter { lexer_iter: self }
    }
}
impl<I: Iterator<Item = LexerOutput>> LexerIterExt for I {}

/// A character iterator that tracks the byte range of the current token.
pub struct RangeCounter<I: Iterator<Item = char>> {
    inner: Peekable<I>,
    start: usize,
    end: usize,
}

impl<I: Iterator<Item = char>> RangeCounter<I> {
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

/// Extension trait for wrapping a character iterator in a [`Counter`].
pub trait RangeCounterExt: Iterator<Item = char> {
    /// Wraps this iterator in a [`RangeCounter`] that tracks consumed byte ranges.
    ///
    /// The tracked range starts empty at byte offset `0`.
    fn range_counter(self) -> RangeCounter<Self>
    where
        Self: Sized,
    {
        RangeCounter {
            inner: self.peekable(),
            start: 0,
            end: 0,
        }
    }
}
impl<I: Iterator<Item = char>> RangeCounterExt for I {}

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
    fn sample() -> Vec<LexerOutput> {
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
        let empty: Vec<LexerOutput> = Vec::new();
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
}
