//! Relations between terms.
//!
//! A [`Relation`] is a named connection between an ordered sequence of terms.
//! It carries no built-in semantics — `equiv(a, b)` and `lt(x, y)` are equally
//! valid relations.

use super::term::Term;
use std::fmt;

/// A named relation over terms: `name(t₁, t₂, …, tₙ)`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Relation {
    name: String,
    terms: Vec<Term>,
}

impl Relation {
    pub fn new(name: impl Into<String>, terms: Vec<Term>) -> Self {
        Relation {
            name: name.into(),
            terms,
        }
    }

    /// Convenience constructor for binary relations.
    pub fn binary(name: impl Into<String>, a: Term, b: Term) -> Self {
        Self::new(name, vec![a, b])
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn terms(&self) -> &[Term] {
        &self.terms
    }

    pub fn arity(&self) -> usize {
        self.terms.len()
    }

    pub fn is_ground(&self) -> bool {
        self.terms.iter().all(|t| t.is_ground())
    }
}

impl fmt::Display for Relation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(", self.name)?;
        for (i, t) in self.terms.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", t)?;
        }
        write!(f, ")")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let r = Relation::binary("equiv", Term::constant("a"), Term::constant("b"));
        assert_eq!(r.to_string(), "equiv(a, b)");
    }

    #[test]
    fn test_ternary() {
        let r = Relation::new(
            "mul",
            vec![
                Term::constant("a"),
                Term::constant("b"),
                Term::constant("c"),
            ],
        );
        assert_eq!(r.to_string(), "mul(a, b, c)");
        assert_eq!(r.arity(), 3);
    }

    #[test]
    fn test_is_ground() {
        assert!(Relation::binary("r", Term::constant("a"), Term::constant("b")).is_ground());
        assert!(!Relation::binary("r", Term::var("x"), Term::constant("b")).is_ground());
    }

    #[test]
    fn test_equality() {
        let a = Relation::binary("equiv", Term::constant("a"), Term::constant("b"));
        let b = Relation::binary("equiv", Term::constant("a"), Term::constant("b"));
        let c = Relation::binary("equiv", Term::constant("b"), Term::constant("a"));
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
