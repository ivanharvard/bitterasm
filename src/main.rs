use std::env;
use std::path::Path;

use bitterasm::ast::Statement;
use bitterasm::{eval, loader, resolver};

fn main() {
    let path = env::args()
        .nth(1)
        .expect("usage: bitter <file>");

    let program = match loader::load_program(Path::new(&path)) {
        Ok(program) => program,

        Err(error) => {
            eprintln!("load error: {error}");
            std::process::exit(1);
        }
    };

    if let Err(error) = resolver::validate_facets(&program) {
        eprintln!("resolver error: {error:?}");
        std::process::exit(1);
    }

    let symbols = match resolver::collect_symbols(&program) {
        Ok(symbols) => symbols,

        Err(error) => {
            eprintln!("resolver error: {error:?}");
            std::process::exit(1);
        }
    };

    let consts = match resolver::ConstEvaluator::new(&program, &symbols).evaluate_all() {
        Ok(consts) => consts,

        Err(error) => {
            eprintln!("resolver error: {error:?}");
            std::process::exit(1);
        }
    };

    let consts_by_name: std::collections::HashMap<String, eval::Int> = consts
        .iter()
        .map(|(id, value)| (symbols.get(*id).name.clone(), value.clone()))
        .collect();

    let mut alias_resolver =
        resolver::AliasResolver::new(&program, &symbols, &consts_by_name);

    let structs = match alias_resolver.resolve_all_structs() {
        Ok(structs) => structs,

        Err(error) => {
            eprintln!("resolver error: {error:?}");
            std::process::exit(1);
        }
    };

    let aliases = match alias_resolver.resolve_all() {
        Ok(aliases) => aliases,

        Err(error) => {
            eprintln!("resolver error: {error:?}");
            std::process::exit(1);
        }
    };

    let mut instantiations = std::collections::HashMap::new();

    for (id, ty) in &aliases {
        if let resolver::ResolvedType::Struct { symbol, args } = ty {
            let fields = match alias_resolver.instantiate_struct_fields(*symbol, args) {
                Ok(fields) => fields,

                Err(error) => {
                    eprintln!("resolver error: {error:?}");
                    std::process::exit(1);
                }
            };

            instantiations.insert(*id, fields);
        }
    }

    // Expand every top-level invocation (`mov r1, 7`, or a macro calling
    // another macro) in program order, against an empty scope — nothing at
    // the top level is a bound parameter. `generated` declarations (a
    // `pub const`, a nested `struct`/`type`/`macro`, a label) have nowhere
    // to be spliced back into the program yet, so they're just collected
    // and printed alongside `emitted`, same as everything else here.
    let mut emitted = Vec::new();
    let mut generated = Vec::new();

    for statement in &program.statements {
        if let Statement::Invocation(invocation) = statement {
            match alias_resolver.expand_invocation(invocation, &std::collections::HashMap::new()) {
                Ok(expansion) => {
                    emitted.extend(expansion.emitted);
                    generated.extend(expansion.generated);
                }

                Err(error) => {
                    eprintln!("resolver error: {error:?}");
                    std::process::exit(1);
                }
            }
        }
    }

    println!("{program:#?}");
    println!("resolved structs: {structs:#?}");
    println!("resolved aliases: {aliases:#?}");
    println!("instantiated fields: {instantiations:#?}");
    println!("evaluated consts: {consts:#?}");
    println!("emitted values: {emitted:#?}");
    println!("generated declarations: {generated:#?}");
}