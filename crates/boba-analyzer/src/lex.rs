use boba_lexer::{Lexer, Token};

pub fn print_tokens(source: &str) {
    for token in Lexer::new(source) {
        match token {
            Token::Invalid(c) => eprintln!("Invalid Token: '{c}'"),
            token => println!("{token}"),
        }
    }
}
