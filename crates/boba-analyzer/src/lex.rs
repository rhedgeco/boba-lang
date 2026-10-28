use boba_lexer::{LexOutput, Trivia, iter::Lexer};

pub fn print_tokens(source: &str) {
    let mut lexer = Lexer::from_source(source);
    while let Some(output) = lexer.next() {
        match output {
            // print lexical tokens
            LexOutput::Token(token) => println!("{token:?}: {}", &source[lexer.range()]),
            // skip whitespace tokens
            LexOutput::Trivia(Trivia::Newline | Trivia::Space | Trivia::Tab) => {}
            // and print comments directly
            LexOutput::Trivia(Trivia::Comment) => println!("{}", &source[lexer.range()]),
            // show invalid tokens
            LexOutput::Trivia(Trivia::Invalid) => {
                eprintln!("Invalid Token: '{}'", &source[lexer.range()]);
            }
            LexOutput::Trivia(Trivia::UnclosedStr) => {
                eprintln!("Unclosed String: {}", &source[lexer.range()]);
            }
        }
    }
}
