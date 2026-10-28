use boba_lexer::{LexerOutput, Trivia, range::CounterExt};

pub fn print_tokens(source: &str) {
    let mut chars = source.chars().counter();
    while let Some(result) = boba_lexer::try_parse(&mut chars) {
        let output = match result {
            Err(state) => state.finish(),
            Ok(output) => output,
        };

        match output {
            // print lexical tokens
            LexerOutput::Token(token) => println!("{token:?}: {}", &source[chars.range()]),
            // skip whitespace tokens
            LexerOutput::Trivia(Trivia::Newline | Trivia::Space | Trivia::Tab) => {}
            // and print comments directly
            LexerOutput::Trivia(Trivia::Comment) => println!("{}", &source[chars.range()]),
            // show invalid tokens
            LexerOutput::Trivia(Trivia::Invalid) => {
                eprintln!("Invalid Token: '{}'", &source[chars.range()])
            }
            LexerOutput::Trivia(Trivia::UnclosedStr) => {
                eprintln!("Unclosed String: {}", &source[chars.range()])
            }
        }

        chars.reset_range();
    }
}
