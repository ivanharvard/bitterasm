mod ast;
mod lexer;
mod token;
mod parser;
mod types;
mod resolver;

use std::{env, fs};

fn main() {
    let path = env::args()
        .nth(1)
        .expect("usage: bitter <file>");

    let source = fs::read_to_string(&path)
        .expect("failed to read source file");

    let tokens = match lexer::lex(&source) {
        Ok(tokens) => tokens,

        Err(error) => {
            eprintln!("lexer error: {error}");
            std::process::exit(1);
        }
    };

    let program = match parser::parse(tokens) {
        Ok(program) => program,

        Err(error) => {
            eprintln!("parser error: {error}");
            std::process::exit(1);
        }
    };

    let symbols = match resolver::collect_symbols(&program) {
        Ok(symbols) => symbols,

        Err(error) => {
            eprintln!("resolver error: {error:?}");
            std::process::exit(1);
        }
    };

    let mut alias_resolver =
        resolver::AliasResolver::new(&program, &symbols);

    let aliases = match alias_resolver.resolve_all() {
        Ok(aliases) => aliases,

        Err(error) => {
            eprintln!("resolver error: {error:?}");
            std::process::exit(1);
        }
    };

    println!("{program:#?}");
    println!("resolved aliases: {aliases:#?}");

}