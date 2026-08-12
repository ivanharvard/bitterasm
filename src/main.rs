mod lexer;

use std::{env, fs};

fn main() {
    let path = env::args()
        .nth(1)
        .expect("usage: bitter <file>");

    let source = fs::read_to_string(&path)
        .expect("failed to read source file");

    match lexer::lex(&source) {
        Ok(tokens) => {
            for token in tokens {
                println!("{token:?}");
            }
        }

        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    }
}