//! Evaluates every top-level `const` declaration's value to a concrete
//! [`crate::eval::Int`], resolving each against the others so consts can
//! reference earlier (or, since resolution order isn't textual, later)
//! consts — mirrors [`super::aliases`]'s lazy-resolve-and-memoize pattern
//! (including cycle detection: `const A = B; const B = A;` is rejected the
//! same way `type A = B; type B = A;` is) rather than requiring
//! declaration order to already be a topological sort.

use std::collections::HashMap;

use crate::ast::{ConstDeclaration, Expr, Program, Statement};
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

            if !is_int_shaped(&declaration.value) {
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
                    scope.insert(name, self.evaluate(referenced_id)?);
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
            Expr::Integer { .. } | Expr::String { .. } => {}
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

fn find_const_declaration<'a>(
    program: &'a Program,
    name: &str,
    symbols: &SymbolTable,
) -> Result<&'a ConstDeclaration, ResolveError> {
    program
        .statements
        .iter()
        .find_map(|statement| match statement {
            Statement::Const(decl) if decl.name == name => Some(decl),
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
fn is_int_shaped(expr: &Expr) -> bool {
    match expr {
        // Transparent: a splice's shape is whatever it wraps.
        Expr::Splice { inner, .. } => is_int_shaped(inner),
        _ => !matches!(expr, Expr::Member { .. } | Expr::Call { .. } | Expr::String { .. }),
    }
}
