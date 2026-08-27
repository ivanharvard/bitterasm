use crate::lexer::LexError;
use crate::loader::LoadError;
use crate::parser::ParseError;
use crate::resolver::ResolveError;
use super::{Diagnostic, SourceId, SourceMap};

pub fn parse_error(error: ParseError, source: SourceId) -> Diagnostic {
    Diagnostic::error(error.message).primary(source, error.span, "could not parse this")
}

pub fn lex_error(error: LexError, source: SourceId) -> Diagnostic {
    Diagnostic::error(error.message).primary(source, error.span, "invalid token starts here")
}

pub fn load_error(error: LoadError, sources: &mut SourceMap) -> Diagnostic {
    match error {
        LoadError::Lex { path, message, span } | LoadError::Parse { path, message, span } => {
            let mut diagnostic = Diagnostic::error(message);
            match std::fs::read_to_string(&path) {
                Ok(text) => diagnostic = diagnostic.primary(sources.add(&path, text), span, "while loading this module"),
                Err(_) => diagnostic.notes.push(format!("in {}", path.display())),
            }
            diagnostic
        }
        other => Diagnostic::error(other.to_string()),
    }
}

pub fn resolve_error(error: ResolveError, source: Option<SourceId>) -> Diagnostic {
    use ResolveError::*;
    let (message, span) = match error {
        UnknownType { name, span } => (format!("unknown type `{name}`"), span),
        DuplicateSymbol { name, span } => (format!("duplicate symbol `{name}`"), span),
        CyclicTypeAlias { cycle, span } => (format!("cyclic type alias: {}", cycle.join(" -> ")), span),
        CyclicConstant { cycle, span } => (format!("cyclic constant: {}", cycle.join(" -> ")), span),
        DivisionByZero { span } => ("division by zero".into(), span),
        ExpectedType { name, span } => (format!("expected `{name}` to name a type"), span),
        InvalidGenericArity { name, expected, actual, span } => (format!("`{name}` expects {expected} generic argument(s), but {actual} were supplied"), span),
        ExpectedConstant { name, span } => (format!("expected `{name}` to name a constant"), span),
        ExpectedConstantExpression { span } => ("expected a constant expression".into(), span),
        UnknownConstant { name, span } => (format!("unknown constant `{name}`"), span),
        UnknownField { type_name, field, span } => (format!("type `{type_name}` has no field `{field}`"), span),
        FacetNotApplicable { facet, span } => (format!("facet `{facet}` is not applicable here"), span),
        DuplicateFacet { facet, span } => (format!("duplicate facet `{facet}`"), span),
        InvalidArgumentCount { name, expected, actual, span } => (format!("`{name}` expects {expected} argument(s), but {actual} were supplied"), span),
        ExpectedStructCallee { name, span } => (format!("expected `{name}` to name a struct"), span),
        ExpectedIntValue { span } => ("expected an integer value".into(), span),
        ExpectedStructValue { span } => ("expected a struct value".into(), span),
        ExpectedValueExpression { span } => ("expected a value expression".into(), span),
        UnsupportedMacroStatement { kind, span } => (format!("unsupported `{kind}` statement in macro body"), span),
        UnsupportedSpliceValue { span } => ("only integer values can be spliced into declarations".into(), span),
        UnsupportedCallExpression { span } => ("unsupported call expression".into(), span),
        UnknownMacro { name, span } => (format!("unknown macro `{name}`"), span),
        ExpectedMacro { name, span } => (format!("expected `{name}` to name a macro"), span),
        NoMatchingMacroOverload { name, actual, span } => (format!("no overload of `{name}` accepts ({})", actual.join(", ")), span),
        AmbiguousMacroOverload { name, actual, span } => (format!("multiple overloads of `{name}` accept ({})", actual.join(", ")), span),
        MacroCallDepthExceeded { call_chain, max_depth, span } => (format!("macro call depth exceeded {max_depth}: {}", call_chain.join(" -> ")), span),
        MacroTailCallLimitExceeded { name, max_iterations, span } => (format!("tail call in `{name}` exceeded {max_iterations} iterations"), span),
        UnresolvedMacroGenericParam { name, macro_name, span } => (format!("macro `{macro_name}`'s generic param `{name}` couldn't be inferred from any argument"), span),
        AssertionFailed { message, span } => (message.unwrap_or_else(|| "assertion failed".into()), span),
        InvalidAssertMessage { span } => ("assertion message must be a string literal".into(), span),
        TypeMismatch { name, expected, actual, span } => (format!("type mismatch for `{name}`: expected `{expected}`, found `{actual}`"), span),
        InvariantViolated { type_name, invariant, span } => (format!("invariant `{invariant}` was violated for `{type_name}`"), span),
        CannotCoerce { type_name, span } => (format!("cannot implicitly convert to `{type_name}`"), span),
        AmbiguousConversion { source, target, span } => (format!("multiple conversions from `{source}` to `{target}` apply"), span),
        AmbiguousInvariantBinder { type_name, names, span } => (format!("invariant for `{type_name}` has ambiguous value names: {}", names.join(", ")), span),
        Internal { message, span } => (format!("internal compiler error: {message}"), span),
        ForLoopTooLarge { span } => ("@for range is too large".into(), span),
        ComputedNameNotAllowed { span } => ("computed name is not allowed here".into(), span),
        TopLevelForRequiresRange { span } => ("top-level @for requires a range".into(), span),
    };
    let diagnostic = Diagnostic::error(message);
    match source {
        Some(id) => diagnostic.primary(id, span, "error occurs here"),
        None => diagnostic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::Span;

    #[test]
    fn parse_errors_keep_their_source_span() {
        let diagnostic = parse_error(ParseError::new("expected expression", Span::new(4, 5)), SourceId(2));
        assert_eq!(diagnostic.message, "expected expression");
        assert_eq!(diagnostic.labels[0].source, SourceId(2));
        assert_eq!(diagnostic.labels[0].span, Span::new(4, 5));
    }

    #[test]
    fn resolver_errors_have_user_facing_messages() {
        let diagnostic = resolve_error(
            ResolveError::UnknownType { name: "Word".into(), span: Span::new(1, 5) },
            Some(SourceId(0)),
        );
        assert_eq!(diagnostic.message, "unknown type `Word`");
        assert_eq!(diagnostic.labels[0].span, Span::new(1, 5));
    }
}
