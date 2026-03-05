//! Symbolic expression trees for algebraic terms.

use std::fmt;

/// A symbolic expression node in an algebraic term tree.
///
/// A `Term` is either:
/// - A variable: `Var("x")`
/// - An operation applied to arguments: `App { op: "mul", args: [x, y] }`
///
/// Nullary operations (constants) are represented as `App` with an empty args list:
/// `App { op: "e", args: [] }`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Term {
    /// A variable symbol (e.g., `x`, `y`, `z`).
    Var(String),
    /// An operation applied to zero or more argument terms.
    App { op: String, args: Vec<Term> },
}

impl Term {
    /// Creates a variable term.
    pub fn var(name: impl Into<String>) -> Self {
        Term::Var(name.into())
    }

    /// Creates an operation application term.
    pub fn app(op: impl Into<String>, args: Vec<Term>) -> Self {
        Term::App {
            op: op.into(),
            args,
        }
    }

    /// Creates a nullary constant term (operation with no arguments).
    pub fn constant(name: impl Into<String>) -> Self {
        Term::App {
            op: name.into(),
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

    #[test]
    fn test_var() {
        let x = Term::var("x");
        assert!(x.is_var());
        assert_eq!(format!("{}", x), "x");
    }

    #[test]
    fn test_constant() {
        let e = Term::constant("e");
        assert!(!e.is_var());
        assert_eq!(format!("{}", e), "e");
    }

    #[test]
    fn test_app() {
        let x = Term::var("x");
        let y = Term::var("y");
        let mul_xy = Term::app("mul", vec![x, y]);
        assert_eq!(format!("{}", mul_xy), "mul(x, y)");
    }

    #[test]
    fn test_nested() {
        let x = Term::var("x");
        let y = Term::var("y");
        let z = Term::var("z");
        let inner = Term::app("mul", vec![x, y]);
        let outer = Term::app("mul", vec![inner, z]);
        assert_eq!(format!("{}", outer), "mul(mul(x, y), z)");
    }

    #[test]
    fn test_variables() {
        let x = Term::var("x");
        let y = Term::var("y");
        let e = Term::constant("e");
        let term = Term::app("mul", vec![
            Term::app("inv", vec![x]),
            Term::app("add", vec![y, e]),
        ]);
        assert_eq!(term.variables(), vec!["x", "y"]);
    }

    #[test]
    fn test_equality() {
        let a = Term::app("mul", vec![Term::var("x"), Term::var("y")]);
        let b = Term::app("mul", vec![Term::var("x"), Term::var("y")]);
        let c = Term::app("mul", vec![Term::var("y"), Term::var("x")]);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
