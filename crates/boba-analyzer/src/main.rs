use std::{fs, path::PathBuf};

use boba_analyzer::lex;
use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Tokenizes a boba file
    Lex {
        /// Path to the boba file
        target: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Lex { target } => match fs::read_to_string(target) {
            Ok(content) => lex::print_tokens(&content),
            Err(err) => eprintln!("Failed to lex: {err}"),
        },
    }
}
