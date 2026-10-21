use boba_lexer::{
    Lexer,
    lexer::{ParseToken, Trivia},
};

pub fn print_tokens(source: &str) {
    let mut lexer = Lexer::new(source);
    while let Some(token) = lexer.next() {
        match token {
            // print lexical tokens
            ParseToken::Token(token) => println!("{token:?}: {}", lexer.content()),
            // show invalid tokens
            ParseToken::Trivia(Trivia::Invalid) => {
                eprintln!("Invalid Token: '{}'", lexer.content())
            }
            // skip whitespace tokens
            ParseToken::Trivia(Trivia::Newline | Trivia::Space | Trivia::Tab) => {}
            // and print comments directly
            ParseToken::Trivia(Trivia::Comment) => println!("{}", lexer.content()),
        }
    }
}
