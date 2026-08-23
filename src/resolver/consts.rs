//! Evaluates every top-level `const` declaration's value to a concrete
//! [`crate::eval::Int`], resolving each against the others so consts can
//! reference earlier (or, since resolution order isn't textual, later)
//! consts — mirrors [`super::aliases`]'s lazy-resolve-and-memoize pattern
//! (including cycle detection: `const A = B; const B = A;` is rejected the
//! same way `type A = B; type B = A;` is) rather than requiring
//! declaration order to already be a topological sort.

use std::collections::{HashMap, HashSet};

use crate::ast::{literal_name, ConstDeclaration, Expr, Program, Statement};
use crate::eval::{self, EvalError, Int};

use super::symbols::{SymbolId, SymbolKind, SymbolTable};
use super::ResolveError;

#[derive(Debug, Clone)]
enum ConstState {
    Unvisited,
    Visiting,
    Resolved(Int),
}

pub struct ConstEvaluator<'a> {
    program: &'a Program,
    symbols: &'a SymbolTable,

    states: HashMap<SymbolId, ConstState>,
    stack: Vec<SymbolId>,
}

impl<'a> ConstEvaluator<'a> {
    pub fn new(program: &'a Program, symbols: &'a SymbolTable) -> Self {
        let mut states = HashMap::new();

        for symbol in symbols.iter() {
            if symbol.kind == SymbolKind::Const {
                states.insert(symbol.id, ConstState::Unvisited);
            }
        }

        Self {
            program,
            symbols,
            states,
            stack: Vec::new(),
        }
    }

    pub fn evaluate_all(&mut self) -> Result<HashMap<SymbolId, Int>, ResolveError> {
        let const_ids: Vec<_> = self
            .symbols
            .iter()
            .filter(|symbol| symbol.kind == SymbolKind::Const)
            .map(|symbol| symbol.id)
            .collect();

        let mut resolved = HashMap::new();

        for id in const_ids {
            let name = self.symbols.get(id).name.clone();

            let declaration = find_const_declaration(self.program, &name, self.symbols)?;

            if !is_int_shaped(self.program, &declaration.value) {
                // Not every const is meant to hold an Int (e.g. a struct
                // value like `const r0: Reg64 = Reg64(0)`) — skip it here
                // rather than failing the whole program. Such a const
                // being referenced *inside* an arithmetic expression
                // elsewhere is still a real error, caught below when that
                // reference is evaluated.
                continue;
            }

            let value = self.evaluate(id)?;
            resolved.insert(id, value);
        }

        Ok(resolved)
    }

    pub fn evaluate(&mut self, id: SymbolId) -> Result<Int, ResolveError> {
        match self.states.get(&id) {
            Some(ConstState::Resolved(value)) => return Ok(value.clone()),
            Some(ConstState::Visiting) => return Err(self.make_cycle_error(id)),
            Some(ConstState::Unvisited) => {}
            None => {
                let symbol = self.symbols.get(id);

                return Err(ResolveError::UnknownConstant {
                    name: symbol.name.clone(),
                    span: symbol.span,
                });
            }
        }

        self.states.insert(id, ConstState::Visiting);
        self.stack.push(id);

        let result = self.evaluate_declaration(id);

        self.stack.pop();

        match result {
            Ok(value) => {
                self.states.insert(id, ConstState::Resolved(value.clone()));
                Ok(value)
            }

            Err(err) => {
                self.states.insert(id, ConstState::Unvisited);
                Err(err)
            }
        }
    }

    fn evaluate_declaration(&mut self, id: SymbolId) -> Result<Int, ResolveError> {
        let symbol = self.symbols.get(id);
        let name = symbol.name.clone();

        let declaration = find_const_declaration(self.program, &name, self.symbols)?;
        let value = declaration.value.clone();

        // Other consts referenced inside this one need to be evaluated
        // (and memoized) on demand, in whatever order they're first used —
        // not assumed to already be in scope from textual declaration
        // order.
        let mut scope = HashMap::new();

        for name in referenced_identifiers(&value) {
            if let Some(referenced_id) = self.symbols.lookup(&name) {
                if self.symbols.get(referenced_id).kind == SymbolKind::Const {
                    let referenced = find_const_declaration(self.program, &name, self.symbols)?;

                    // A struct-valued const referenced here (`const foo =
                    // some_reg + 1`) isn't this evaluator's problem to
                    // reject — `eval::eval` below will, once `name` turns
                    // out missing from `scope`, with a clearer
                    // `UnknownConstant` than a confusing type error would
                    // be. Skipping it here only prevents a *legitimately*
                    // int-shaped const from crashing while it chases
                    // through a struct-valued one that happens to share a
                    // reference chain with it.
                    if is_int_shaped(self.program, &referenced.value) {
                        scope.insert(name, self.evaluate(referenced_id)?);
                    }
                }
            }
        }

        eval::eval(&value, &scope).map_err(|error| into_resolve_error(error, &symbol.name))
    }

    fn make_cycle_error(&self, repeated: SymbolId) -> ResolveError {
        let start = self
            .stack
            .iter()
            .position(|id| *id == repeated)
            .unwrap_or(0);

        let mut cycle: Vec<String> = self.stack[start..]
            .iter()
            .map(|id| self.symbols.get(*id).name.clone())
            .collect();

        cycle.push(self.symbols.get(repeated).name.clone());

        ResolveError::CyclicConstant {
            cycle,
            span: self.symbols.get(repeated).span,
        }
    }
}

// Every identifier an expression tree mentions, so the const being
// evaluated can pre-populate its scope without needing a full symbol
// resolver pass over expressions (which doesn't exist yet — this is
// intentionally a simple textual walk, not a scoped name resolver).
fn referenced_identifiers(expr: &crate::ast::Expr) -> Vec<String> {
    use crate::ast::Expr;

    let mut names = Vec::new();
    let mut stack = vec![expr];

    while let Some(expr) = stack.pop() {
        match expr {
            Expr::Identifier { name, .. } => names.push(name.clone()),
            Expr::Integer { .. } | Expr::String { .. } | Expr::Here { .. } => {}
            Expr::Member { object, .. } => stack.push(object),
            Expr::Call { callee, arguments, .. } => {
                stack.push(callee);
                stack.extend(arguments.iter().map(|arg| &arg.value));
            }
            Expr::Unary { operand, .. } => stack.push(operand),
            Expr::Binary { left, right, .. } => {
                stack.push(left);
                stack.push(right);
            }
            Expr::Splice { inner, .. } => stack.push(inner),
        }
    }

    names
}

fn into_resolve_error(error: EvalError, const_name: &str) -> ResolveError {
    match error {
        EvalError::UnknownConstant { name, span } => ResolveError::UnknownConstant { name, span },
        EvalError::NotConstant { span } => ResolveError::ExpectedConstant {
            name: const_name.to_string(),
            span,
        },
        EvalError::DivisionByZero { span } => ResolveError::DivisionByZero { span },
    }
}

pub(super) fn find_const_declaration<'a>(
    program: &'a Program,
    name: &str,
    symbols: &SymbolTable,
) -> Result<&'a ConstDeclaration, ResolveError> {
    program
        .statements
        .iter()
        .find_map(|statement| match statement {
            // A top-level const's name is always fully literal — see
            // `ast::literal_name`'s doc — so a spliced name never matches
            // here rather than being an error in its own right.
            Statement::Const(decl) if literal_name(&decl.name).as_deref() == Some(name) => Some(decl),
            _ => None,
        })
        .ok_or_else(|| {
            let span = symbols
                .lookup(name)
                .map(|id| symbols.get(id).span)
                .unwrap_or_else(|| program.span);

            ResolveError::Internal {
                message: format!(
                    "symbol table contains const `{name}` but no matching AST declaration exists",
                ),
                span,
            }
        })
}

// Only expressions that are unambiguously meant to produce a value — a
// literal, a reference to another const, or an operation on one of those —
// are worth attempting to evaluate. `Member`/`Call`/`String` at the top
// level mean this const holds something else entirely (a struct value, a
// field access, plain text), not an `Int` that failed to fold.
//
// A bare identifier needs one more step than that syntactic check can give
// it: `const zero = x0` *looks* int-shaped (it's just a name), but is only
// really int-shaped if `x0` itself is — so this chases a reference chain
// through the program's own const declarations, the same way `evaluate`
// would, rather than assuming a name is always fine to attempt.
fn is_int_shaped(program: &Program, expr: &Expr) -> bool {
    is_int_shaped_inner(program, expr, &mut HashSet::new())
}

fn is_int_shaped_inner(program: &Program, expr: &Expr, visiting: &mut HashSet<String>) -> bool {
    match expr {
        // Transparent: a splice's shape is whatever it wraps.
        Expr::Splice { inner, .. } => is_int_shaped_inner(program, inner, visiting),

        Expr::Identifier { name, .. } => {
            // A cycle here (`const a = b; const b = a;`) isn't this
            // function's job to reject — `evaluate`'s own cycle detection
            // will, if anything ever actually needs one of them as an Int.
            // Treating it as not-int-shaped just means neither gets folded
            // speculatively while chasing the other's shape.
            if !visiting.insert(name.clone()) {
                return false;
            }

            let shaped = program
                .statements
                .iter()
                .find_map(|statement| match statement {
                    Statement::Const(decl) if literal_name(&decl.name).as_deref() == Some(name.as_str()) => {
                        Some(decl)
                    }
                    _ => None,
                })
                .is_some_and(|decl| is_int_shaped_inner(program, &decl.value, visiting));

            visiting.remove(name);
            shaped
        }

        _ => !matches!(expr, Expr::Member { .. } | Expr::Call { .. } | Expr::String { .. }),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::ast::Program;
    use crate::lexer;
    use crate::parser;
    use crate::resolver::collect_symbols;

    use super::ConstEvaluator;

    fn parse_fixture(name: &str) -> Program {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/emit")
            .join(name);

        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read fixture {}: {error}", path.display()));

        let tokens = lexer::lex(&source).expect("fixture should lex");
        parser::parse(tokens).expect("fixture should parse")
    }

    #[test]
    fn struct_valued_const_referencing_another_is_skipped_not_rejected() {
        let program = parse_fixture("named_const_reference.basm");
        let symbols = collect_symbols(&program).unwrap();

        // `zero = r0` is a bare identifier referencing a struct-valued
        // const — `is_int_shaped` used to take that at face value and let
        // `ConstEvaluator` try (and fail) to fold it as an Int. Neither
        // `r0` nor `zero` is Int-shaped, so both should just be absent
        // from the result, not blow up the whole evaluation.
        let resolved = ConstEvaluator::new(&program, &symbols).evaluate_all().unwrap();

        assert!(resolved.is_empty());
    }
}
