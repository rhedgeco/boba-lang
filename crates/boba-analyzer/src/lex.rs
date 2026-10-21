use boba_lexer::{Lexer, Token};

pub fn print_tokens(source: &str) {
    let mut lexer = Lexer::new(source);
    while let Some(token) = lexer.next() {
        match token {
            Token::Invalid => eprintln!("Invalid Token: '{}'", lexer.token_source()),
            token => println!("{token:?}: {}", lexer.token_source()),
        }
    }
}
