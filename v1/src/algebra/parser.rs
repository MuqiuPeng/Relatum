//! Text parser for terms and equations.
//!
//! Parses expressions like `mul(x, e)` and equations like `mul(x, e) = x`
//! against an [`OpRegistry`]'s declared operations.
//!
//! # Grammar
//!
//! ```text
//! equation  = term '=' term
//! term      = ident '(' term (',' term)* ')'   // operation application
//!           | ident                              // variable or nullary constant
//! ident     = [a-zA-Z_][a-zA-Z0-9_]*
//! ```
//!
//! An identifier is resolved as:
//! - A declared nullary operation → `Term::constant(id)`
//! - A declared n-ary operation with matching `(args…)` → `Term::app(id, args)`
//! - Otherwise → `Term::var(name)`
//!
//! # Example
//!
//! ```
//! use relatum::algebra::{builders, Equation, OpRegistry};
//! use relatum::algebra::parser::Parser;
//!
//! let mut reg = OpRegistry::new();
//! let _monoid = builders::monoid(&mut reg).unwrap();
//! let parser = Parser::new(&reg);
//!
//! let term = parser.parse_term("mul(x, e)").unwrap();
//! let eq = parser.parse_equation("identity", "mul(x, e) = x").unwrap();
//! ```

use super::equation::Equation;
use super::registry::OpRegistry;
use super::term::Term;

use std::fmt;

/// Parse error with position information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub position: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "parse error at position {}: {}",
            self.position, self.message
        )
    }
}

impl std::error::Error for ParseError {}

/// A parser that resolves identifiers against an [`OpRegistry`]'s operations.
pub struct Parser<'a> {
    registry: &'a OpRegistry,
}

impl<'a> Parser<'a> {
    pub fn new(registry: &'a OpRegistry) -> Self {
        Parser { registry }
    }

    /// Parses a term string like `mul(x, e)`.
    pub fn parse_term(&self, input: &str) -> Result<Term, ParseError> {
        let mut cursor = Cursor::new(input);
        let term = self.parse_term_inner(&mut cursor)?;
        cursor.skip_whitespace();
        if !cursor.is_eof() {
            return Err(ParseError {
                message: format!("unexpected character '{}'", cursor.peek().unwrap()),
                position: cursor.pos,
            });
        }
        Ok(term)
    }

    /// Parses an equation string like `mul(x, e) = x`.
    pub fn parse_equation(&self, name: &str, input: &str) -> Result<Equation, ParseError> {
        let mut cursor = Cursor::new(input);
        let lhs = self.parse_term_inner(&mut cursor)?;
        cursor.skip_whitespace();
        if cursor.consume_char('=').is_none() {
            return Err(ParseError {
                message: "expected '='".to_string(),
                position: cursor.pos,
            });
        }
        let rhs = self.parse_term_inner(&mut cursor)?;
        cursor.skip_whitespace();
        if !cursor.is_eof() {
            return Err(ParseError {
                message: format!("unexpected character '{}'", cursor.peek().unwrap()),
                position: cursor.pos,
            });
        }
        Ok(Equation::new(name, lhs, rhs))
    }

    fn parse_term_inner(&self, cursor: &mut Cursor) -> Result<Term, ParseError> {
        cursor.skip_whitespace();
        let pos = cursor.pos;
        let ident = cursor.read_ident().ok_or_else(|| ParseError {
            message: "expected identifier".to_string(),
            position: pos,
        })?;

        cursor.skip_whitespace();

        if cursor.consume_char('(').is_some() {
            let op_id = self
                .registry
                .find_operation_id(&ident)
                .ok_or_else(|| ParseError {
                    message: format!("unknown operation '{}'", ident),
                    position: pos,
                })?;

            let mut args = Vec::new();
            cursor.skip_whitespace();

            if cursor.consume_char(')').is_none() {
                args.push(self.parse_term_inner(cursor)?);
                loop {
                    cursor.skip_whitespace();
                    if cursor.consume_char(')').is_some() {
                        break;
                    }
                    if cursor.consume_char(',').is_none() {
                        return Err(ParseError {
                            message: "expected ',' or ')'".to_string(),
                            position: cursor.pos,
                        });
                    }
                    args.push(self.parse_term_inner(cursor)?);
                }
            }

            Ok(Term::app(op_id, args))
        } else {
            if let Some(op_id) = self.registry.find_operation_id(&ident) {
                let op = self.registry.get_operation(op_id).unwrap();
                if op.arity().accepts(0) {
                    return Ok(Term::constant(op_id));
                }
            }
            Ok(Term::var(ident))
        }
    }
}

// ── Cursor ──────────────────────────────────────────────────

struct Cursor<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(input: &'a str) -> Self {
        Cursor { input, pos: 0 }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_ascii_whitespace() {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
    }

    fn consume_char(&mut self, expected: char) -> Option<char> {
        if self.peek() == Some(expected) {
            self.pos += expected.len_utf8();
            Some(expected)
        } else {
            None
        }
    }

    fn read_ident(&mut self) -> Option<String> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
        if self.pos > start {
            Some(self.input[start..self.pos].to_string())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::builders;

    #[test]
    fn test_parse_variable() {
        let mut reg = OpRegistry::new();
        builders::monoid(&mut reg).unwrap();
        let parser = Parser::new(&reg);
        assert_eq!(parser.parse_term("x").unwrap(), Term::var("x"));
    }

    #[test]
    fn test_parse_nullary_constant() {
        let mut reg = OpRegistry::new();
        builders::monoid(&mut reg).unwrap();
        let parser = Parser::new(&reg);
        let e_id = reg.find_operation_id("e").unwrap();
        assert_eq!(parser.parse_term("e").unwrap(), Term::constant(e_id));
    }

    #[test]
    fn test_parse_binary_application() {
        let mut reg = OpRegistry::new();
        builders::monoid(&mut reg).unwrap();
        let parser = Parser::new(&reg);
        let mul = reg.find_operation_id("mul").unwrap();
        let e_id = reg.find_operation_id("e").unwrap();
        let expected = Term::app(mul, vec![Term::var("x"), Term::constant(e_id)]);
        assert_eq!(parser.parse_term("mul(x, e)").unwrap(), expected);
    }

    #[test]
    fn test_parse_nested() {
        let mut reg = OpRegistry::new();
        builders::monoid(&mut reg).unwrap();
        let parser = Parser::new(&reg);
        let mul = reg.find_operation_id("mul").unwrap();
        let expected = Term::app(
            mul,
            vec![
                Term::app(mul, vec![Term::var("a"), Term::var("b")]),
                Term::var("c"),
            ],
        );
        assert_eq!(parser.parse_term("mul(mul(a, b), c)").unwrap(), expected);
    }

    #[test]
    fn test_parse_equation() {
        let mut reg = OpRegistry::new();
        builders::monoid(&mut reg).unwrap();
        let parser = Parser::new(&reg);
        let mul = reg.find_operation_id("mul").unwrap();
        let e_id = reg.find_operation_id("e").unwrap();

        let eq = parser.parse_equation("right_id", "mul(x, e) = x").unwrap();
        assert_eq!(eq.name(), "right_id");
        assert_eq!(
            *eq.lhs(),
            Term::app(mul, vec![Term::var("x"), Term::constant(e_id)])
        );
        assert_eq!(*eq.rhs(), Term::var("x"));
    }

    #[test]
    fn test_parse_error_unknown_op() {
        let mut reg = OpRegistry::new();
        builders::monoid(&mut reg).unwrap();
        let parser = Parser::new(&reg);
        let err = parser.parse_term("unknown(x)").unwrap_err();
        assert!(err.message.contains("unknown operation"));
    }

    #[test]
    fn test_parse_error_missing_equals() {
        let mut reg = OpRegistry::new();
        builders::monoid(&mut reg).unwrap();
        let parser = Parser::new(&reg);
        let err = parser.parse_equation("bad", "mul(x, e) x").unwrap_err();
        assert!(err.message.contains("unexpected character") || err.message.contains("expected"));
    }

    #[test]
    fn test_parse_whitespace_tolerance() {
        let mut reg = OpRegistry::new();
        builders::monoid(&mut reg).unwrap();
        let parser = Parser::new(&reg);
        let mul = reg.find_operation_id("mul").unwrap();
        let expected = Term::app(mul, vec![Term::var("x"), Term::var("y")]);
        assert_eq!(parser.parse_term("  mul( x ,  y )  ").unwrap(), expected);
    }

    #[test]
    fn test_parse_group_inverse() {
        let mut reg = OpRegistry::new();
        builders::group(&mut reg).unwrap();
        let parser = Parser::new(&reg);
        let inv = reg.find_operation_id("inv").unwrap();
        let mul = reg.find_operation_id("mul").unwrap();
        let expected = Term::app(
            mul,
            vec![Term::app(inv, vec![Term::var("x")]), Term::var("x")],
        );
        assert_eq!(parser.parse_term("mul(inv(x), x)").unwrap(), expected);
    }

    #[test]
    fn test_parse_explicit_nullary_parens() {
        let mut reg = OpRegistry::new();
        builders::monoid(&mut reg).unwrap();
        let parser = Parser::new(&reg);
        let e_id = reg.find_operation_id("e").unwrap();
        assert_eq!(parser.parse_term("e()").unwrap(), Term::constant(e_id));
    }
}
