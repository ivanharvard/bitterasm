use std::fmt;

use crate::ast::{
    Expr, ImportItems, ImportStatement, Invocation, 
    Label, ModulePath, Program, Statement,
};
use crate::lexer::{Span, Token, TokenKind};

// ================
// public entry point
// ================

pub fn parse(tokens: Vec<Token>) -> Result<Program, ParseError> {
    Parser::new(tokens).parse_program()
}

// ================
// errors
// ================

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl ParseError {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self { message: message.into(), span }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f, 
            "{} at byte {}..{}", 
            self.message, 
            self.span.start, 
            self.span.end
        )
    }
}

impl std::error::Error for ParseError {}

// ================
// parser
// ================ 

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    // ===============
    // program
    // ===============

    fn parse_program(mut self) -> Result<Program, ParseError> {
        let start = self.current().span.start;

        let mut statements = Vec::new();

        self.skip_newlines();

        while !self.at_eof() {
            statements.push(self.parse_statement()?);
            self.skip_newlines();
        }

        let end = self.current().span.end;

        Ok(Program {
            statements,
            span: Span::new(start, end),
        })
    }

    // ===============
    // statement
    // ===============

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        match &self.current().kind {
            TokenKind::From => {
                let import = self.parse_from_import()?;
                Ok(Statement::Import(import))
            }

            TokenKind::Identifier(_) => {
                if self.check_next(&TokenKind::Colon) {
                    let label = self.parse_label()?;
                    Ok(Statement::Label(label))
                } else {
                    let invocation = self.parse_invocation()?;
                    Ok(Statement::Invocation(invocation))
                }
            }

            other => Err(ParseError::new(
                format!("expected statement, found {other:?}"),
                self.current().span,
            )),
        }
    }

    // ===============
    // imports
    // ===============

    fn parse_from_import(&mut self) -> Result<ImportStatement, ParseError> {
        let start = self.current().span.start;

        self.expect_simple(TokenKind::From)?;

        let module = self.parse_module_path()?;

        self.expect_simple(TokenKind::Import)?;

        let items = if self.check(&TokenKind::Star) {
            self.advance();
            ImportItems::All
        } else {
            let mut names = Vec::new();

            names.push(self.expect_identifier()?);

            while self.check(&TokenKind::Comma) {
                self.advance();
                names.push(self.expect_identifier()?);
            }

            ImportItems::Names(names)
        };

        let end = self.statement_end()?;

        Ok(ImportStatement {
            module,
            items,
            span: Span::new(start, end),
        })
    }

    fn parse_module_path(&mut self) -> Result<ModulePath, ParseError> {
        let start = self.current().span.start;

        let mut relative_level = 0;

        // unresolved atm
        while self.check(&TokenKind::Dot) {
            relative_level += 1;
            self.advance();
        }

        let mut segments = Vec::new();

        segments.push(self.expect_identifier()?);

        while self.check(&TokenKind::Dot) {
            self.advance();
            segments.push(self.expect_identifier()?);
        }

        let end = self.previous().span.end;

        Ok(ModulePath {
            segments,
            relative_level,
            span: Span::new(start, end),
        })
    }

    // =============
    // labels
    // =============

    fn parse_label(&mut self) -> Result<Label, ParseError> {
        let start = self.current().span.start;

        let name = self.expect_identifier()?;

        self.expect_simple(TokenKind::Colon)?;

        let end = self.statement_end()?;

        Ok(Label {
            name,
            span: Span::new(start, end),
        })
    }

    // =============
    // invocations
    // =============

    fn parse_invocation(&mut self) -> Result<Invocation, ParseError> {
        let start = self.current().span.start;

        let name = self.expect_identifier()?;

        let mut operands = Vec::new();

        // if we haven't reached the end of the statement there is at least one operand
        if !self.at_statement_end() {
            operands.push(self.parse_expr()?);

            while self.check(&TokenKind::Comma) {
                self.advance();
                
                if self.at_statement_end() {
                    return Err(ParseError::new(
                        "expected operand after comma",
                        self.current().span,
                    ));
                }

                operands.push(self.parse_expr()?);
            }
        }

        // if theres something other than a newline or EOF here then we failed
        // to consume the whole invocation
        if !self.at_statement_end() {
            return Err(ParseError::new(
                format!(
                    "unexpected token in invocation: {:?}",
                    self.current().kind
                ),
                self.current().span,
            ));
        }

        let end = self.statement_end()?;

        Ok(Invocation {
            name,
            operands,
            span: Span::new(start, end),
        })
    }

    // =============
    // expressions
    // =============

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        let token = self.current().clone();

        match token.kind {
            TokenKind::Identifier(name) => {
                self.advance();
                Ok(Expr::Identifier {
                    name,
                    span: token.span,
                })
            }

            TokenKind::Integer(raw) => {
                self.advance();
                Ok(Expr::Integer {
                    raw,
                    span: token.span,
                })
            }

            other => Err(ParseError::new(
                format!("expected expression, found {other:?}"),
                token.span,
            )),
        }
    }

    // =============
    // statement end
    // =============

    fn at_statement_end(&self) -> bool {
        matches!(
            self.current().kind,
            TokenKind::Newline | TokenKind::Eof
        )
    }

    // consume a new line if present and return the ending byte offset
    // eof is also a valid statement terminator
    fn statement_end(&mut self) -> Result<usize, ParseError> {
        match self.current().kind {
            TokenKind::Newline => {
                let end = self.current().span.end;
                self.advance();
                Ok(end)
            }

            TokenKind::Eof => Ok(self.current().span.end),

            _ => Err(ParseError::new(
                "expected end of line",
                self.current().span,
            )),
        }
    }

    fn skip_newlines(&mut self) {
        while self.check(&TokenKind::Newline) {
            self.advance();
        }
    }

    // =============
    // expectations
    // =============

    fn expect_identifier(&mut self) -> Result<String, ParseError> {
        let token = self.current().clone();

        match token.kind {
            TokenKind::Identifier(name) => {
                self.advance();
                Ok(name)
            }

            other => Err(ParseError::new(
                format!("expected identifier, found {other:?}"),
                token.span,
            )),
        }
    }

    fn expect_simple(&mut self, expected: TokenKind) -> Result<(), ParseError> {
        if self.check(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::new(
                format!(
                    "expected {:?}, found {:?}",
                    expected,
                    self.current().kind
                ),
                self.current().span,
            ))
        }
    }

    // =============
    // token navigation
    // =============

    fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.pos - 1]
    }

    fn advance(&mut self) -> &Token {
        if !self.at_eof() {
            self.pos += 1;
        }
        self.previous()
    }

    fn at_eof(&self) -> bool {
        matches!(self.current().kind, TokenKind::Eof)
    }

    fn check(&self, kind: &TokenKind) -> bool {
        same_variant(&self.current().kind, kind)
    }

    fn check_next(&self, kind: &TokenKind) -> bool {
        self.tokens
            .get(self.pos + 1)
            .map(|token| same_variant(&token.kind, kind))
            .unwrap_or(false)
    }
}

// =============
// token comparison
// =============

fn same_variant(a: &TokenKind, b: &TokenKind) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    #[test]
    fn parses_empty_program() {
        let program = parse(lex("").unwrap()).unwrap();

        assert!(program.statements.is_empty());
    }

    #[test]
    fn parses_import_all() {
        let program =
            parse(lex("from tinycpu.native import *\n").unwrap()).unwrap();

        assert_eq!(program.statements.len(), 1);

        let Statement::Import(import) = &program.statements[0] else {
            panic!("expected import");
        };

        assert_eq!(
            import.module.segments,
            vec!["tinycpu".to_string(), "native".to_string()]
        );

        assert_eq!(import.module.relative_level, 0);
        assert_eq!(import.items, ImportItems::All);
    }

    #[test]
    fn parses_named_imports() {
        let program =
            parse(lex("from tinycpu.native import mov, add\n").unwrap())
                .unwrap();

        let Statement::Import(import) = &program.statements[0] else {
            panic!("expected import");
        };

        assert_eq!(
            import.items,
            ImportItems::Names(vec![
                "mov".to_string(),
                "add".to_string(),
            ])
        );
    }

    #[test]
    fn parses_relative_import() {
        let program =
            parse(lex("from .foobar import qux\n").unwrap()).unwrap();

        let Statement::Import(import) = &program.statements[0] else {
            panic!("expected import");
        };

        assert_eq!(import.module.relative_level, 1);
        assert_eq!(import.module.segments, vec!["foobar".to_string()]);
    }

    #[test]
    fn parses_label() {
        let program = parse(lex("start:\n").unwrap()).unwrap();

        let Statement::Label(label) = &program.statements[0] else {
            panic!("expected label");
        };

        assert_eq!(label.name, "start");
    }

    #[test]
    fn parses_no_operand_invocation() {
        let program = parse(lex("nop\n").unwrap()).unwrap();

        let Statement::Invocation(invocation) = &program.statements[0] else {
            panic!("expected invocation");
        };

        assert_eq!(invocation.name, "nop");
        assert!(invocation.operands.is_empty());
    }

    #[test]
    fn parses_invocation() {
        let program = parse(lex("mov r1, 7\n").unwrap()).unwrap();

        let Statement::Invocation(invocation) = &program.statements[0] else {
            panic!("expected invocation");
        };

        assert_eq!(invocation.name, "mov");
        assert_eq!(invocation.operands.len(), 2);

        assert!(matches!(
            &invocation.operands[0],
            Expr::Identifier { name, .. } if name == "r1"
        ));

        assert!(matches!(
            &invocation.operands[1],
            Expr::Integer { raw, .. } if raw == "7"
        ));
    }

    #[test]
    fn parses_whole_example() {
        let source = r#"from tinycpu.native import *

start:
    mov r1, 7
    add r1, r2
    nop
"#;

        let program = parse(lex(source).unwrap()).unwrap();

        assert_eq!(program.statements.len(), 5);

        assert!(matches!(
            program.statements[0],
            Statement::Import(_)
        ));

        assert!(matches!(
            program.statements[1],
            Statement::Label(_)
        ));

        assert!(matches!(
            program.statements[2],
            Statement::Invocation(_)
        ));

        assert!(matches!(
            program.statements[3],
            Statement::Invocation(_)
        ));

        assert!(matches!(
            program.statements[4],
            Statement::Invocation(_)
        ));
    }

    #[test]
    fn accepts_no_final_newline() {
        let program = parse(lex("nop").unwrap()).unwrap();

        assert_eq!(program.statements.len(), 1);
    }

    #[test]
    fn rejects_trailing_comma() {
        let error = parse(lex("mov r1,\n").unwrap()).unwrap_err();

        assert_eq!(error.message, "expected operand after comma");
    }
}