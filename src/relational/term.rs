//! Symbolic expression terms.
//!
//! A [`Term`] is either a variable (pattern placeholder) or a function application
//! with a named symbol and zero or more arguments.

use std::collections::HashSet;
use std::fmt;

/// A symbolic expression: variable or function application.
///
/// - `Var("x")` — a pattern variable, used in rules.
/// - `App { symbol: "f", args: [a, b] }` — function application `f(a, b)`.
/// - `App { symbol: "a", args: [] }` — a ground constant `a`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Term {
    Var(String),
    App { symbol: String, args: Vec<Term> },
}

impl Term {
    pub fn var(name: impl Into<String>) -> Self {
        Term::Var(name.into())
    }

    pub fn app(symbol: impl Into<String>, args: Vec<Term>) -> Self {
        Term::App {
            symbol: symbol.into(),
            args,
        }
    }

    /// A nullary application — a ground constant.
    pub fn constant(symbol: impl Into<String>) -> Self {
        Self::app(symbol, vec![])
    }

    /// True if the term contains no variables.
    pub fn is_ground(&self) -> bool {
        match self {
            Term::Var(_) => false,
            Term::App { args, .. } => args.iter().all(|a| a.is_ground()),
        }
    }

    /// Nesting depth (1 for atoms/vars, 1 + max child depth for applications).
    pub fn depth(&self) -> usize {
        match self {
            Term::Var(_) => 1,
            Term::App { args, .. } => 1 + args.iter().map(|a| a.depth()).max().unwrap_or(0),
        }
    }

    /// Inserts this term and all of its subterms into `set`.
    pub fn collect_subterms(&self, set: &mut HashSet<Term>) {
        set.insert(self.clone());
        if let Term::App { args, .. } = self {
            for arg in args {
                arg.collect_subterms(set);
            }
        }
    }
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Term::Var(name) => write!(f, "{}", name),
            Term::App { symbol, args } if args.is_empty() => write!(f, "{}", symbol),
            Term::App { symbol, args } => {
                write!(f, "{}(", symbol)?;
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
    fn test_display() {
        assert_eq!(Term::var("x").to_string(), "x");
        assert_eq!(Term::constant("a").to_string(), "a");
        assert_eq!(
            Term::app("f", vec![Term::constant("a"), Term::var("x")]).to_string(),
            "f(a, x)"
        );
        assert_eq!(
            Term::app(
                "g",
                vec![Term::app("f", vec![Term::constant("a")])]
            )
            .to_string(),
            "g(f(a))"
        );
    }

    #[test]
    fn test_is_ground() {
        assert!(Term::constant("a").is_ground());
        assert!(!Term::var("x").is_ground());
        assert!(Term::app("f", vec![Term::constant("a")]).is_ground());
        assert!(!Term::app("f", vec![Term::var("x")]).is_ground());
    }

    #[test]
    fn test_depth() {
        assert_eq!(Term::var("x").depth(), 1);
        assert_eq!(Term::constant("a").depth(), 1);
        assert_eq!(Term::app("f", vec![Term::constant("a")]).depth(), 2);
        assert_eq!(
            Term::app("g", vec![Term::app("f", vec![Term::constant("a")])]).depth(),
            3
        );
    }

    #[test]
    fn test_collect_subterms() {
        let t = Term::app("f", vec![Term::constant("a"), Term::constant("b")]);
        let mut set = HashSet::new();
        t.collect_subterms(&mut set);
        assert_eq!(set.len(), 3); // f(a,b), a, b
        assert!(set.contains(&Term::constant("a")));
        assert!(set.contains(&Term::constant("b")));
        assert!(set.contains(&t));
    }

    #[test]
    fn test_equality() {
        let a = Term::app("f", vec![Term::constant("a")]);
        let b = Term::app("f", vec![Term::constant("a")]);
        let c = Term::app("f", vec![Term::constant("b")]);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
