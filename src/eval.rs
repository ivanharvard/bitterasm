//! Compile-time evaluation of `Int` expressions. This is what `@return`
//! folds before splicing (`@return 1+1` produces `2`), what `invariant`
//! will eventually check, and what generic const arguments (`bits<4+4>`)
//! will eventually be compared by once this is wired into those call
//! sites — this module only implements the evaluator itself.
//!
//! There is no distinct boolean type here (matching the language: `bool`
//! is `bits<1>` in `std`, not a builtin) — comparison and logical operators
//! just produce `0` or `1`.

use std::collections::HashMap;

use num_bigint::BigInt;

use crate::ast::{BinaryOp, Expr, NamePart, UnaryOp};
use crate::token::Span;

pub type Int = BigInt;

#[derive(Debug, Clone, PartialEq)]
pub enum EvalError {
    UnknownConstant {
        name: String,
        span: Span,
    },

    NotConstant {
        span: Span,
    },

    DivisionByZero {
        span: Span,
    },
}

/// Evaluates `expr` to a concrete `Int`, resolving identifiers against
/// `scope` (already-evaluated consts, bound generic const params, or
/// eventually bound macro/invocation arguments — this function doesn't
/// care where a caller's bindings came from).
pub fn eval(expr: &Expr, scope: &HashMap<String, Int>) -> Result<Int, EvalError> {
    eval_in(expr, scope)
}

/// Where [`eval_in`] looks names up.
pub trait IntScope {
    fn get_int(&self, name: &str) -> Option<Int>;
}

impl IntScope for HashMap<String, Int> {
    fn get_int(&self, name: &str) -> Option<Int> {
        self.get(name).cloned()
    }
}

/// [`eval`] against any [`IntScope`], so a caller with its names in more
/// than one map needn't merge them into a new one first.
pub fn eval_in<S: IntScope + ?Sized>(expr: &Expr, scope: &S) -> Result<Int, EvalError> {
    match expr {
        Expr::Integer { raw, span } => parse_int_literal(raw, *span),

        Expr::Identifier { name, span } => scope.get_int(name).ok_or_else(|| {
            EvalError::UnknownConstant {
                name: name.clone(),
                span: *span,
            }
        }),

        Expr::SplicedIdentifier { name, span } => {
            let mut resolved = String::new();

            for part in name {
                match part {
                    NamePart::Literal(text) => resolved.push_str(text),
                    NamePart::Splice(expr) => resolved.push_str(&eval_in(expr, scope)?.to_string()),
                }
            }

            scope.get_int(&resolved).ok_or(EvalError::UnknownConstant { name: resolved, span: *span })
        }

        Expr::String { span, .. }
        | Expr::Member { span, .. }
        | Expr::Call { span, .. }
        | Expr::EnumVariant { span, .. }
        | Expr::Construct { span, .. }
        | Expr::AnonStruct { span, .. }
        | Expr::As { span, .. }
        | Expr::Range { span, .. }
        | Expr::In { span, .. }
        | Expr::Fold { span, .. }
        | Expr::Extend { span, .. }
        | Expr::HasField { span, .. } => Err(EvalError::NotConstant { span: *span }),

        Expr::Unary { op, operand, .. } => Ok(apply_unary(*op, eval_in(operand, scope)?)),

        Expr::Binary { left, op, right, span } => eval_binary(*op, left, right, scope, *span),

        // A splice is only meaningful where the surrounding code is
        // otherwise literal/unevaluated (not the case here — `eval` always
        // evaluates), so it's a transparent no-op: `` `1 + 1` `` folds the
        // same as `1 + 1`.
        Expr::Splice { inner, .. } => eval_in(inner, scope),
    }
}

fn eval_binary<S: IntScope + ?Sized>(
    op: BinaryOp,
    left: &Expr,
    right: &Expr,
    scope: &S,
    span: Span,
) -> Result<Int, EvalError> {
    // Short-circuit before evaluating the right side, same as any language
    // with these operators — lets `x != 0 && 10 / x > 1`-shaped guards work
    // instead of always evaluating both sides.
    match op {
        BinaryOp::And => {
            let l = eval_in(left, scope)?;
            if l == zero() {
                return Ok(zero());
            }
            let r = eval_in(right, scope)?;
            return Ok(bool_int(r != zero()));
        }

        BinaryOp::Or => {
            let l = eval_in(left, scope)?;
            if l != zero() {
                return Ok(bool_int(true));
            }
            let r = eval_in(right, scope)?;
            return Ok(bool_int(r != zero()));
        }

        _ => {}
    }

    let l = eval_in(left, scope)?;
    let r = eval_in(right, scope)?;
    apply_binary(op, l, r, span)
}

/// `op` applied to an already-evaluated operand.
pub fn apply_unary(op: UnaryOp, value: Int) -> Int {
    match op {
        UnaryOp::Negate => -value,
        UnaryOp::Not => bool_int(value == zero()),
        // Arbitrary-precision two's-complement bitwise not: `!x == -x - 1`.
        UnaryOp::BitNot => -(value + Int::from(1)),
    }
}

/// `op` applied to two already-evaluated operands. `&&` and `||` only see
/// whether each is nonzero; a caller that wants them to short-circuit
/// checks `l` before evaluating `r`, as [`eval`] does.
pub fn apply_binary(op: BinaryOp, l: Int, r: Int, span: Span) -> Result<Int, EvalError> {
    Ok(match op {
        BinaryOp::Add => l + r,
        BinaryOp::Subtract => l - r,
        BinaryOp::Multiply => l * r,

        BinaryOp::Divide => {
            if r == zero() {
                return Err(EvalError::DivisionByZero { span });
            }
            l / r
        }

        BinaryOp::Remainder => {
            if r == zero() {
                return Err(EvalError::DivisionByZero { span });
            }
            l % r
        }

        BinaryOp::ShiftLeft => l << shift_amount(&r),
        BinaryOp::ShiftRight => l >> shift_amount(&r),

        BinaryOp::BitAnd => l & r,
        BinaryOp::BitXor => l ^ r,
        BinaryOp::BitOr => l | r,

        BinaryOp::Equal => bool_int(l == r),
        BinaryOp::NotEqual => bool_int(l != r),
        BinaryOp::Less => bool_int(l < r),
        BinaryOp::LessEqual => bool_int(l <= r),
        BinaryOp::Greater => bool_int(l > r),
        BinaryOp::GreaterEqual => bool_int(l >= r),

        BinaryOp::And => bool_int(l != zero() && r != zero()),
        BinaryOp::Or => bool_int(l != zero() || r != zero()),
    })
}

fn zero() -> Int {
    Int::from(0)
}

fn bool_int(value: bool) -> Int {
    Int::from(if value { 1 } else { 0 })
}

// A negative or absurdly large shift amount isn't meaningful; clamping to
// usize::MAX just makes it "shift everything out" rather than panicking,
// since this evaluator has no other place to report a soft warning.
fn shift_amount(amount: &Int) -> usize {
    use num_bigint::Sign;

    match amount.sign() {
        Sign::Minus => 0,
        _ => usize::try_from(amount).unwrap_or(usize::MAX),
    }
}

/// An integer literal's value: decimal, or `0x`/`0b`/`0o`, with any `_`
/// separators. `NotConstant` if it isn't one.
pub fn parse_int_literal(raw: &str, span: Span) -> Result<Int, EvalError> {
    // Most literals have no `_`, and need no copy to drop them.
    let cleaned: std::borrow::Cow<str> = if raw.contains('_') {
        raw.chars().filter(|ch| *ch != '_').collect::<String>().into()
    } else {
        raw.into()
    };

    let (digits, radix) = match cleaned.get(0..2) {
        Some("0x") | Some("0X") => (&cleaned[2..], 16),
        Some("0b") | Some("0B") => (&cleaned[2..], 2),
        Some("0o") | Some("0O") => (&cleaned[2..], 8),
        _ => (&cleaned[..], 10),
    };

    // Nearly every literal fits in 64 bits, which is far cheaper to parse
    // than the general case.
    if let Ok(small) = u64::from_str_radix(digits, radix) {
        return Ok(Int::from(small));
    }

    Int::parse_bytes(digits.as_bytes(), radix).ok_or(EvalError::NotConstant { span })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer;
    use crate::parser;

    fn eval_source(source: &str) -> Result<Int, EvalError> {
        let mut full = source.to_string();
        full.push('\n');

        let tokens = lexer::lex(&full).expect("source should lex");
        let program = parser::parse(tokens).expect("source should parse");

        eval(&expr_from(&program), &HashMap::new())
    }

    fn expr_from(program: &crate::ast::Program) -> Expr {
        let crate::ast::Statement::Const(decl) = &program.statements[0] else {
            panic!("expected a const declaration");
        };

        decl.value.clone()
    }

    #[test]
    fn evaluates_arithmetic_with_precedence() {
        assert_eq!(eval_source("const x = 1 + 2 * 3").unwrap(), Int::from(7));
    }

    #[test]
    fn splice_evaluates_transparently() {
        assert_eq!(
            eval_source("const x = `1 + 2 * 3`").unwrap(),
            eval_source("const x = 1 + 2 * 3").unwrap(),
        );
    }

    #[test]
    fn evaluates_number_bases() {
        assert_eq!(eval_source("const x = 0xff").unwrap(), Int::from(255));
        assert_eq!(eval_source("const x = 0b1010").unwrap(), Int::from(10));
        assert_eq!(eval_source("const x = 0o17").unwrap(), Int::from(15));
    }

    #[test]
    fn evaluates_number_separators() {
        assert_eq!(eval_source("const x = 1_000_000").unwrap(), Int::from(1_000_000));
    }

    #[test]
    fn evaluates_shifts_and_bitwise_ops() {
        assert_eq!(eval_source("const x = 1 << 8").unwrap(), Int::from(256));
        assert_eq!(eval_source("const x = 0xff & 0x0f").unwrap(), Int::from(0x0f));
    }

    #[test]
    fn evaluates_comparisons_and_logic() {
        assert_eq!(eval_source("const x = 3 < 5 && 5 < 10").unwrap(), Int::from(1));
        assert_eq!(eval_source("const x = 3 > 5 || 2 == 2").unwrap(), Int::from(1));
        assert_eq!(eval_source("const x = 3 > 5 && 2 == 2").unwrap(), Int::from(0));
    }

    #[test]
    fn short_circuits_and_avoids_division_by_zero() {
        // If `&&` didn't short-circuit, this would divide by zero and error.
        assert_eq!(eval_source("const x = 0 == 1 && 10 / 0 == 1").unwrap(), Int::from(0));
    }

    #[test]
    fn rejects_division_by_zero() {
        assert!(matches!(
            eval_source("const x = 10 / 0"),
            Err(EvalError::DivisionByZero { .. })
        ));
    }

    #[test]
    fn resolves_identifiers_via_scope() {
        let tokens = lexer::lex("const y = width\n").unwrap();
        let program = parser::parse(tokens).unwrap();
        let expr = expr_from(&program);

        let mut scope = HashMap::new();
        scope.insert("width".to_string(), Int::from(8));

        assert_eq!(eval(&expr, &scope).unwrap(), Int::from(8));
    }

    #[test]
    fn reports_unknown_identifiers() {
        assert!(matches!(
            eval_source("const x = mystery"),
            Err(EvalError::UnknownConstant { .. })
        ));
    }
}
