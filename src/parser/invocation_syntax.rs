//! Matches a real call site's token stream against a pool of candidate
//! `(macro name, SyntaxPattern)` pairs (parsed and validated in
//! `crate::facets::syntax`), producing the same [`Invocation`] shape
//! [`Parser::parse_invocation`]'s default `name arg, arg, ...` grammar
//! would. Everything downstream (`crate::resolver`) consumes `Invocation`
//! generically, so this is the only place custom syntax matters at all.
//!
//! A candidate pool mixes two different kinds of pattern (see
//! `facets::syntax::is_anchored`'s doc): patterns anchored to the current
//! statement's own leading identifier (`crate::parser::statements`'s
//! `macro_syntaxes[name]` lookup already filters to just those) and every
//! unanchored pattern in the program, regardless of which macro they're
//! for or what their own leading token is — an unanchored pattern's whole
//! point is that nothing about a call site's first token says which macro
//! it's for, so it has to be tried against *every* identifier-led
//! statement, not looked up by name.

use crate::facets::syntax::{PatternSegment, SyntaxPattern};

use super::*;

pub(super) enum SyntaxMatch {
    Matched(Invocation),

    /// No candidate matched — `best_error` is the deepest partial match
    /// found along the way (if any candidate got anywhere at all), for a
    /// caller that wants a real diagnostic instead of just falling back to
    /// default syntax silently.
    NoMatch { best_error: Option<ParseError> },
}

impl Parser {
    pub(super) fn parse_invocation_via_syntax_candidates(
        &mut self,
        candidates: &[(String, SyntaxPattern)],
    ) -> Result<SyntaxMatch, ParseError> {
        let start_pos = self.pos;
        let start_span = self.current().span;
        let mut matches: Vec<(Invocation, usize, usize)> = Vec::new();
        let mut best_error: Option<(usize, ParseError)> = None;

        for (name, pattern) in candidates {
            self.pos = start_pos;
            match self.parse_invocation_via_syntax(name, pattern) {
                Ok(invocation) => {
                    let end_pos = self.pos;
                    let specificity = literal_token_count(pattern);
                    if !matches.iter().any(|(known, _, _)| known == &invocation) {
                        matches.push((invocation, end_pos, specificity));
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

        // More than one distinct parse succeeded — before calling that an
        // unresolvable ambiguity, prefer whichever pattern(s) committed to the
        // most literal (non-capture) structure. A capture accepts *any*
        // expression, so a pattern with fewer literals around its captures can
        // always also parse whatever a more literal-heavy sibling would have
        // split up more specifically — e.g. `[$base$+$disp$]`'s `disp` capture
        // happily swallows `$index$*$scale$+$disp$`'s entire right-hand side
        // as one expression, since `rcx*4+0x10` is a perfectly ordinary,
        // well-typed arithmetic expression on its own. Preferring more literal
        // tokens only ever discards a "could also parse it more vaguely"
        // reading in favor of a "specifically recognized this shape" one — two
        // patterns with genuinely *equal* specificity (like `$a$ + $b$` vs.
        // `$b$ + $a$`) are never affected by this and still hit the ambiguity
        // error below exactly as before.
        if matches.len() > 1 {
            let max_specificity = matches.iter().map(|(_, _, specificity)| *specificity).max().unwrap();
            matches.retain(|(_, _, specificity)| *specificity == max_specificity);
        }

        match matches.len() {
            1 => {
                let (invocation, end_pos, _) = matches.pop().unwrap();
                self.pos = end_pos;
                Ok(SyntaxMatch::Matched(invocation))
            }
            0 => Ok(SyntaxMatch::NoMatch { best_error: best_error.map(|(_, error)| error) }),
            _ => {
                let mut names: Vec<&str> = matches.iter().map(|(invocation, _, _)| invocation.name.as_str()).collect();
                names.sort_unstable();
                names.dedup();

                Err(ParseError::new(
                    format!(
                        "ambiguous syntax for `{}`: multiple patterns match this invocation",
                        names.join("`/`"),
                    ),
                    start_span,
                ))
            }
        }
    }

    fn parse_invocation_via_syntax(
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

// Total literal (non-capture) token count across a pattern's segments — see
// `parse_invocation_via_syntax_candidates`'s own comment on why this is the
// tie-breaker between multiple successful parses of the same call site.
fn literal_token_count(pattern: &SyntaxPattern) -> usize {
    pattern
        .segments
        .iter()
        .map(|segment| match segment {
            PatternSegment::Literal(tokens) => tokens.len(),
            PatternSegment::Capture(_) => 0,
        })
        .sum()
}
