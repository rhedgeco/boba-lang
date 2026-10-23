use crate::{Lexer, LexerOutput};

/// Collects every token then flushes the final pending one
pub fn tokenize(source: &str) -> Vec<LexerOutput> {
    let mut lexer = Lexer::new(source);
    let mut tokens: Vec<_> = lexer.by_ref().collect();
    tokens.extend(lexer.finish());
    tokens
}
