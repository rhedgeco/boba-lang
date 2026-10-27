use boba_lexer::{LexerOutput, Trivia};

pub fn print_tokens(source: &str) {
    let mut chars = source.chars().peekable();
    while let Some(result) = boba_lexer::try_parse(&mut chars) {
        let output = match result {
            Err(state) => state.finish(),
            Ok(output) => output,
        };

        match output {
            // print lexical tokens
            LexerOutput::Token(token) => println!("{token:?}"),
            // skip whitespace tokens
            LexerOutput::Trivia(Trivia::Newline | Trivia::Space | Trivia::Tab) => {}
            // and print comments directly
            LexerOutput::Trivia(Trivia::Comment) => println!("// comment"),
            // show invalid tokens
            LexerOutput::Trivia(Trivia::Invalid) => {
                eprintln!("Invalid Token")
            }
            LexerOutput::Trivia(Trivia::UnclosedStr) => {
                eprintln!("Unclosed String")
            }
        }
    }
}
