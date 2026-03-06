//! Symbolic expression trees for algebraic terms.

use std::fmt;

use crate::algebra::operation::OperationId;

/// A symbolic expression node in an algebraic term tree.
///
/// A `Term` is either:
/// - A variable: `Var("x")`
/// - An operation applied to arguments: `App { op: OperationId, args: [x, y] }`
///
/// Nullary operations (constants) are represented as `App` with an empty args list:
/// `App { op: e_id, args: [] }`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Term {
    /// A variable symbol (e.g., `x`, `y`, `z`).
    Var(String),
    /// An operation applied to zero or more argument terms.
    App { op: OperationId, args: Vec<Term> },
}

impl Term {
    /// Creates a variable term.
    pub fn var(name: impl Into<String>) -> Self {
        Term::Var(name.into())
    }

    /// Creates an operation application term.
    pub fn app(op: OperationId, args: Vec<Term>) -> Self {
        Term::App { op, args }
    }

    /// Creates a nullary constant term (operation with no arguments).
    pub fn constant(op: OperationId) -> Self {
        Term::App {
            op,
            args: Vec::new(),
        }
    }

    /// Returns `true` if this term is a variable.
    pub fn is_var(&self) -> bool {
        matches!(self, Term::Var(_))
    }

    /// Returns the set of variable names appearing in this term.
    pub fn variables(&self) -> Vec<&str> {
        let mut vars = Vec::new();
        self.collect_variables(&mut vars);
        vars.sort();
        vars.dedup();
        vars
    }

    fn collect_variables<'a>(&'a self, vars: &mut Vec<&'a str>) {
        match self {
            Term::Var(name) => vars.push(name),
            Term::App { args, .. } => {
                for arg in args {
                    arg.collect_variables(vars);
                }
            }
        }
    }
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Term::Var(name) => write!(f, "{}", name),
            Term::App { op, args } if args.is_empty() => write!(f, "{}", op),
            Term::App { op, args } => {
                write!(f, "{}(", op)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::operation::OperationId;

    #[test]
    fn test_var() {
        let x = Term::var("x");
        assert!(x.is_var());
        assert_eq!(format!("{}", x), "x");
    }

    #[test]
    fn test_constant() {
        let e = Term::constant(OperationId(0));
        assert!(!e.is_var());
        assert_eq!(format!("{}", e), "#0");
    }

    #[test]
    fn test_app() {
        let x = Term::var("x");
        let y = Term::var("y");
        let mul = OperationId(0);
        let mul_xy = Term::app(mul, vec![x, y]);
        assert_eq!(format!("{}", mul_xy), "#0(x, y)");
    }

    #[test]
    fn test_nested() {
        let x = Term::var("x");
        let y = Term::var("y");
        let z = Term::var("z");
        let mul = OperationId(0);
        let inner = Term::app(mul, vec![x, y]);
        let outer = Term::app(mul, vec![inner, z]);
        assert_eq!(format!("{}", outer), "#0(#0(x, y), z)");
    }

    #[test]
    fn test_variables() {
        let x = Term::var("x");
        let y = Term::var("y");
        let mul = OperationId(0);
        let inv = OperationId(1);
        let add = OperationId(2);
        let e = OperationId(3);
        let term = Term::app(
            mul,
            vec![
                Term::app(inv, vec![x]),
                Term::app(add, vec![y, Term::constant(e)]),
            ],
        );
        assert_eq!(term.variables(), vec!["x", "y"]);
    }

    #[test]
    fn test_equality() {
        let mul = OperationId(0);
        let a = Term::app(mul, vec![Term::var("x"), Term::var("y")]);
        let b = Term::app(mul, vec![Term::var("x"), Term::var("y")]);
        let c = Term::app(mul, vec![Term::var("y"), Term::var("x")]);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
