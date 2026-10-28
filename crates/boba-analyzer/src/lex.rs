use boba_lexer::{LexerOutput, Trivia, iter::Lexer};

pub fn print_tokens(source: &str) {
    let mut lexer = Lexer::from_source(source);
    while let Some(output) = lexer.next() {
        match output {
            // print lexical tokens
            LexerOutput::Token(token) => println!("{token:?}: {}", &source[lexer.range()]),
            // skip whitespace tokens
            LexerOutput::Trivia(Trivia::Newline | Trivia::Space | Trivia::Tab) => {}
            // and print comments directly
            LexerOutput::Trivia(Trivia::Comment) => println!("{}", &source[lexer.range()]),
            // show invalid tokens
            LexerOutput::Trivia(Trivia::Invalid) => {
                eprintln!("Invalid Token: '{}'", &source[lexer.range()])
            }
            LexerOutput::Trivia(Trivia::UnclosedStr) => {
                eprintln!("Unclosed String: {}", &source[lexer.range()])
            }
        }
    }
}
