use boba_lexer::{Lexer, LexerOutput, Trivia};

pub fn print_tokens(source: &str) {
    let mut lexer = Lexer::new(source);
    while let Some(token) = lexer.next() {
        match token {
            // print lexical tokens
            LexerOutput::Token(token) => println!("{token:?}: {}", lexer.token_source()),
            // skip whitespace tokens
            LexerOutput::Trivia(Trivia::Newline | Trivia::Space | Trivia::Tab) => {}
            // and print comments directly
            LexerOutput::Trivia(Trivia::Comment) => println!("{}", lexer.token_source()),
            // show invalid tokens
            LexerOutput::Trivia(Trivia::Invalid) => {
                eprintln!("Invalid Token: '{}'", lexer.token_source())
            }
            LexerOutput::Trivia(Trivia::UnclosedStr) => {
                eprintln!("Unclosed String: '{}'", lexer.token_source())
            }
        }
    }
}
