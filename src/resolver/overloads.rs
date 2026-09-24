//! Declaration-time rules for macro overload sets. Macros sharing a name form
//! one overload set (see [`super::SymbolTable::insert`]); every member must
//! differ from every other in the number, types, or order of its parameters.
//! Parameter names and return types don't count: two declarations that differ
//! only there could never be told apart at a call site, which picks an
//! overload from its arguments' types alone
//! (`AliasResolver::resolve_macro_overload`). Checked once, up front, so a
//! clashing pair fails the program even if nothing ever calls it.
//!
//! "Same type" is [`ResolvedType`] equality: a plain alias is transparent, so
//! `r: Reg` and `r: bits<4>` clash, while `bits` (any width) and `bits<4>` are
//! different types. Generic parameters compare by where they first appear, so
//! `f<S>(a: S)` and `f<T>(b: T)` clash. Overloads that are distinct under
//! these rules can still both accept one particular call (`bits` and
//! `bits<4>` for a `bits<4>` argument, or a trailing default parameter); that
//! remains a call-site `AmbiguousMacroOverload`.

use std::collections::{HashMap, HashSet};

use crate::ast::MacroDeclaration;
use crate::types::GenericParameter;

use super::aliases::{AliasResolver, GenericBinding};
use super::structs::{describe_type, param_name};
use super::symbols::SymbolKind;
use super::types::{ResolvedGenericArg, ResolvedType};
use super::ResolveError;

impl AliasResolver<'_> {
    /// Rejects any overload whose parameter types repeat an earlier
    /// same-named declaration's, in program order (the later one is reported).
    pub fn check_macro_overloads(&mut self) -> Result<(), ResolveError> {
        let mut checked = HashSet::new();
        let names: Vec<String> = self
            .symbols
            .iter()
            .filter(|symbol| symbol.kind == SymbolKind::Macro)
            .map(|symbol| symbol.name.clone())
            .filter(|name| checked.insert(name.clone()))
            .collect();

        for name in names {
            let ids = self.lookup_symbols(&name);
            if ids.len() < 2 {
                continue;
            }

            let mut signatures: Vec<Vec<ResolvedType>> = Vec::with_capacity(ids.len());
            for id in ids {
                let declaration = self.find_macro_declaration_rc(id)?;
                let signature = self.macro_signature(&declaration)?;
                if signatures.contains(&signature) {
                    return Err(ResolveError::DuplicateMacroOverload {
                        params: signature.iter().map(|ty| describe_type(ty, self)).collect(),
                        name,
                        span: declaration.span,
                    });
                }
                signatures.push(signature);
            }
        }

        Ok(())
    }

    /// `declaration`'s parameter types, with its generic parameters renamed
    /// `#0`, `#1`, ... in order of first appearance in the parameter list, so
    /// neither their names nor their declaration order matters:
    /// `f<A, B>(x: A, y: B)` and `f<A, B>(x: B, y: A)` accept the same calls.
    fn macro_signature(&mut self, declaration: &MacroDeclaration) -> Result<Vec<ResolvedType>, ResolveError> {
        let scope = declaration
            .generic_params
            .iter()
            .map(|param| {
                let binding = match param {
                    GenericParameter::Type { name, .. } => {
                        GenericBinding::Type(ResolvedType::TypeParameter { name: name.clone() })
                    }
                    GenericParameter::Const { .. } => GenericBinding::Const(None),
                };
                (param_name(param).to_string(), binding)
            })
            .collect();

        let previous = std::mem::replace(&mut self.generic_scope, scope);
        let resolved = declaration
            .params
            .iter()
            .map(|param| self.resolve_type_expr(&param.ty))
            .collect::<Result<Vec<_>, _>>();
        self.generic_scope = previous;

        let generic_names: HashSet<&str> = declaration.generic_params.iter().map(param_name).collect();
        let mut positions = HashMap::new();
        Ok(resolved?
            .into_iter()
            .map(|ty| rename_generic_params(ty, &generic_names, &mut positions))
            .collect())
    }
}

/// Renames every generic parameter `ty` mentions (a `TypeParameter`, or an
/// unbound const generic's `ConstParam`) to `#n`, numbering new ones as
/// they're first reached. Anything not in `generic_names` is left alone.
fn rename_generic_params(
    ty: ResolvedType,
    generic_names: &HashSet<&str>,
    positions: &mut HashMap<String, String>,
) -> ResolvedType {
    let position_of = |name: String, positions: &mut HashMap<String, String>| {
        if !generic_names.contains(name.as_str()) {
            return name;
        }
        let next = format!("#{}", positions.len());
        positions.entry(name).or_insert(next).clone()
    };

    match ty {
        ResolvedType::TypeParameter { name } => ResolvedType::TypeParameter { name: position_of(name, positions) },
        ResolvedType::Struct { symbol, args } => ResolvedType::Struct {
            symbol,
            args: rename_args(args, generic_names, positions),
        },
        ResolvedType::Enum { symbol, args } => ResolvedType::Enum {
            symbol,
            args: rename_args(args, generic_names, positions),
        },
        ResolvedType::Alias { symbol, binder, invariants, underlying } => ResolvedType::Alias {
            symbol,
            binder,
            invariants,
            underlying: Box::new(rename_generic_params(*underlying, generic_names, positions)),
        },
        ResolvedType::MacroType { params, ret } => ResolvedType::MacroType {
            params: params
                .into_iter()
                .map(|param| rename_generic_params(param, generic_names, positions))
                .collect(),
            ret: ret.map(|ret| Box::new(rename_generic_params(*ret, generic_names, positions))),
        },
        other => other,
    }
}

fn rename_args(
    args: Vec<ResolvedGenericArg>,
    generic_names: &HashSet<&str>,
    positions: &mut HashMap<String, String>,
) -> Vec<ResolvedGenericArg> {
    args.into_iter()
        .map(|arg| match arg {
            ResolvedGenericArg::ConstParam(name) => {
                match rename_generic_params(ResolvedType::TypeParameter { name }, generic_names, positions) {
                    ResolvedType::TypeParameter { name } => ResolvedGenericArg::ConstParam(name),
                    _ => unreachable!("renaming a type parameter yields a type parameter"),
                }
            }
            ResolvedGenericArg::Type(inner) => {
                ResolvedGenericArg::Type(Box::new(rename_generic_params(*inner, generic_names, positions)))
            }
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{lexer, parser};

    use super::super::{collect_symbols, AliasResolver, ResolveError};

    fn check(source: &str) -> Result<(), ResolveError> {
        let program = parser::parse(lexer::lex(source).unwrap()).unwrap();
        let symbols = collect_symbols(&program, &vec![0; program.statements.len()]).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);
        resolver.check_macro_overloads()
    }

    fn assert_duplicate(source: &str, expected_params: &[&str]) {
        match check(source) {
            Err(ResolveError::DuplicateMacroOverload { name, params, .. }) => {
                assert_eq!(name, "f");
                assert_eq!(params, expected_params);
            }
            other => panic!("expected a duplicate overload of `f`, got {other:?}"),
        }
    }

    #[test]
    fn overloads_differing_in_count_types_or_order_are_allowed() {
        let source = "struct Reg { pub id: int }\n\
                      macro f(a: int) { @emit a\n }\n\
                      macro f(a: int, b: int) { @emit a\n }\n\
                      macro f(a: Reg) { @emit 1\n }\n\
                      macro f(a: int, b: Reg) { @emit a\n }\n\
                      macro f(a: Reg, b: int) { @emit b\n }\n";
        assert_eq!(check(source), Ok(()));
    }

    #[test]
    fn only_parameter_names_differing_is_a_duplicate() {
        assert_duplicate(
            "macro f(a: int, b: int) { @emit a\n }\n\
             macro f(x: int, y: int) { @emit y\n }\n",
            &["int", "int"],
        );
    }

    #[test]
    fn only_the_return_type_differing_is_a_duplicate() {
        assert_duplicate(
            "struct Reg { pub id: int }\n\
             macro f(a: int) -> int { @return a\n }\n\
             macro f(a: int) -> Reg { @return Reg(id = a)\n }\n",
            &["int"],
        );
    }

    #[test]
    fn a_plain_alias_is_the_same_type_as_its_target() {
        assert_duplicate(
            "struct Reg { pub id: int }\n\
             type R = Reg\n\
             macro f(a: Reg) { @emit 1\n }\n\
             macro f(a: R) { @emit 2\n }\n",
            &["Reg"],
        );
    }

    #[test]
    fn generic_parameters_compare_by_first_appearance_not_name() {
        assert_duplicate(
            "macro f<S>(a: S) { @emit 1\n }\n\
             macro f<T>(b: T) { @emit 2\n }\n",
            &["#0"],
        );
        assert_duplicate(
            "macro f<A, B>(x: A, y: B) { @emit 1\n }\n\
             macro f<A, B>(x: B, y: A) { @emit 2\n }\n",
            &["#0", "#1"],
        );
    }

    #[test]
    fn repeated_versus_distinct_generic_parameters_differ() {
        let source = "macro f<A>(x: A, y: A) { @emit 1\n }\n\
                      macro f<A, B>(x: A, y: B) { @emit 2\n }\n";
        assert_eq!(check(source), Ok(()));
    }

    #[test]
    fn a_generic_and_a_concrete_overload_differ() {
        let source = "macro f<S>(a: S) { @emit 1\n }\n\
                      macro f(a: int) { @emit 2\n }\n";
        assert_eq!(check(source), Ok(()));
    }
}
