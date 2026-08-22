//! Matches a real call site's token stream against one macro's
//! [`SyntaxPattern`] (parsed and validated in `crate::facets::syntax`),
//! producing the same [`Invocation`] shape [`Parser::parse_invocation`]'s
//! default `name arg, arg, ...` grammar would. Everything downstream
//! (`crate::resolver`) consumes `Invocation` generically, so this is the
//! only place custom syntax matters at all.

use crate::facets::syntax::{PatternSegment, SyntaxPattern};

use super::*;

impl Parser {
    pub(super) fn parse_invocation_via_syntax(
        &mut self,
        name: &str,
        pattern: &SyntaxPattern,
    ) -> Result<Invocation, ParseError> {
        let start = self.current().span.start;

        let mut captured: HashMap<String, Expr> = HashMap::new();

        for (index, segment) in pattern.segments.iter().enumerate() {
            match segment {
                PatternSegment::Literal(tokens) => {
                    for expected in tokens {
                        if &self.current().kind != expected {
                            return Err(ParseError::new(
                                format!(
                                    "expected {:?} while matching `{name}`'s syntax pattern, found {:?}",
                                    expected,
                                    self.current().kind
                                ),
                                self.current().span,
                            ));
                        }

                        self.advance();
                    }
                }

                PatternSegment::Capture(param_name) => {
                    // No-empty-gap validation at registration time guarantees
                    // the segment right after a capture, if any, is always a
                    // Literal — never another Capture — so this is the only
                    // stop boundary a capture ever needs.
                    let stop = match pattern.segments.get(index + 1) {
                        Some(PatternSegment::Literal(tokens)) => tokens.first(),
                        _ => None,
                    };

                    let expr = self.parse_expr_bp_until(0, stop)?;
                    captured.insert(param_name.clone(), expr);
                }
            }
        }

        if !self.at_statement_end() {
            return Err(ParseError::new(
                format!(
                    "unexpected token after matching `{name}`'s syntax pattern: {:?}",
                    self.current().kind
                ),
                self.current().span,
            ));
        }

        let end = self.statement_end()?;

        let operands = pattern
            .param_order
            .iter()
            .map(|param_name| {
                captured.remove(param_name).unwrap_or_else(|| {
                    unreachable!(
                        "syntax pattern validation guarantees every param is captured exactly once"
                    )
                })
            })
            .collect();

        Ok(Invocation {
            name: name.to_string(),
            operands,
            span: Span::new(start, end),
        })
    }
}
