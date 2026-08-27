//! Matches a real call site's token stream against one macro's
//! [`SyntaxPattern`] (parsed and validated in `crate::facets::syntax`),
//! producing the same [`Invocation`] shape [`Parser::parse_invocation`]'s
//! default `name arg, arg, ...` grammar would. Everything downstream
//! (`crate::resolver`) consumes `Invocation` generically, so this is the
//! only place custom syntax matters at all.

use crate::facets::syntax::{PatternSegment, SyntaxPattern};

use super::*;

impl Parser {
    pub(super) fn parse_invocation_via_syntax_overloads(
        &mut self,
        name: &str,
        patterns: &[SyntaxPattern],
    ) -> Result<Invocation, ParseError> {
        let start_pos = self.pos;
        let start_span = self.current().span;
        let mut matches: Vec<(Invocation, usize)> = Vec::new();
        let mut best_error: Option<(usize, ParseError)> = None;

        for pattern in patterns {
            self.pos = start_pos;
            match self.parse_invocation_via_syntax(name, pattern) {
                Ok(invocation) => {
                    let end_pos = self.pos;
                    if !matches.iter().any(|(known, _)| known == &invocation) {
                        matches.push((invocation, end_pos));
                    }
                }
                Err(error) => {
                    let error_pos = self.pos;
                    if best_error.as_ref().is_none_or(|(known_pos, _)| error_pos > *known_pos) {
                        best_error = Some((error_pos, error));
                    }
                }
            }
        }

        self.pos = start_pos;
        match matches.len() {
            1 => {
                let (invocation, end_pos) = matches.pop().unwrap();
                self.pos = end_pos;
                Ok(invocation)
            }
            0 => Err(best_error
                .map(|(_, error)| error)
                .unwrap_or_else(|| ParseError::new(
                    format!("no syntax pattern registered for `{name}`"),
                    start_span,
                ))),
            _ => Err(ParseError::new(
                format!("ambiguous syntax for `{name}`: multiple patterns match this invocation"),
                start_span,
            )),
        }
    }

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
