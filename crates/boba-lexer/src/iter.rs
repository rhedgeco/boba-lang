//! Iterator support for lexing, including peeking and output filtering.

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
}
