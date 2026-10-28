//! Byte range tracking for lexer input.

use std::{iter::Peekable, ops::Range};

use crate::iter::PeekableChar;

/// A character iterator that tracks the byte range of the current token.
pub struct Counter<I: Iterator<Item = char>> {
    inner: Peekable<I>,
    start: usize,
    end: usize,
}

impl<I: Iterator<Item = char>> Counter<I> {
    /// Moves the start of the tracked range up to the current end.
    ///
    /// This should be called after a token is produced so that the next
    /// [`range`](Counter::range) begins where the previous token ended.
    pub fn reset_range(&mut self) {
        self.start = self.end;
    }

    /// Returns the byte range consumed since the last [`reset_range`](Counter::reset_range).
    pub fn range(&self) -> Range<usize> {
        self.start..self.end
    }
}

impl<I: Iterator<Item = char>> PeekableChar for Counter<I> {
    fn peek(&mut self) -> Option<char> {
        self.inner.peek().copied()
    }
}

impl<I: Iterator<Item = char>> Iterator for Counter<I> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        let next_char = self.inner.next()?;
        self.end += next_char.len_utf8();
        Some(next_char)
    }
}

/// Extension trait for wrapping a character iterator in a [`Counter`].
pub trait CounterExt: Iterator<Item = char> {
    /// Wraps this iterator in a [`Counter`] that tracks consumed byte ranges.
    ///
    /// The tracked range starts empty at byte offset `0`.
    fn counter(self) -> Counter<Self>
    where
        Self: Sized,
    {
        Counter {
            inner: self.peekable(),
            start: 0,
            end: 0,
        }
    }
}
impl<I: Iterator<Item = char>> CounterExt for I {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iter::PeekableChar;

    #[test]
    fn counter_starts_with_an_empty_range() {
        let counter = "abc".chars().counter();
        assert_eq!(counter.range(), 0..0);
    }

    #[test]
    fn next_advances_the_end_by_byte_length() {
        let mut counter = "abc".chars().counter();
        assert_eq!(counter.next(), Some('a'));
        assert_eq!(counter.range(), 0..1);
        assert_eq!(counter.next(), Some('b'));
        assert_eq!(counter.range(), 0..2);
    }

    #[test]
    fn peek_does_not_advance_the_range() {
        let mut counter = "abc".chars().counter();
        assert_eq!(counter.peek(), Some('a'));
        // peeking must not change the tracked range
        assert_eq!(counter.range(), 0..0);
        // and the peeked char is still produced by next
        assert_eq!(counter.next(), Some('a'));
        assert_eq!(counter.range(), 0..1);
    }

    #[test]
    fn multibyte_chars_advance_by_their_utf8_length() {
        // 'é' is two bytes, '🦀' is four bytes in UTF-8
        let mut counter = "é🦀".chars().counter();
        assert_eq!(counter.next(), Some('é'));
        assert_eq!(counter.range(), 0..2);
        assert_eq!(counter.next(), Some('🦀'));
        assert_eq!(counter.range(), 0..6);
    }

    #[test]
    fn reset_range_moves_start_to_current_end() {
        let mut counter = "ab".chars().counter();
        counter.next();
        counter.reset_range();
        // the start now tracks from the second character onward
        assert_eq!(counter.range(), 1..1);
        counter.next();
        assert_eq!(counter.range(), 1..2);
    }

    #[test]
    fn range_slices_the_original_source_per_token() {
        // tokens separated by spaces are recovered by resetting between them
        let source = "ab cd";
        let mut counter = source.chars().counter();

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
    fn range_is_empty_when_no_chars_are_consumed_after_reset() {
        let mut counter = "ab".chars().counter();
        counter.next();
        counter.reset_range();
        // with nothing consumed since the reset, the range is empty
        assert_eq!(counter.range(), 1..1);
        assert!(counter.range().is_empty());
    }

    #[test]
    fn exhausted_iterator_keeps_the_final_range() {
        let mut counter = "ab".chars().counter();
        counter.next();
        counter.next();
        // next past the end yields None and leaves the range unchanged
        assert_eq!(counter.next(), None);
        assert_eq!(counter.range(), 0..2);
    }
}
