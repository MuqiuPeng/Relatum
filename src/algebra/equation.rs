//! Algebraic equations (relations between terms).
//!
//! Named `Equation` rather than `Relation` to avoid confusion with the
//! graph-level `crate::relation::Relation` (directed edge between nodes).

use std::fmt;

use super::term::Term;

/// An equation asserting that two terms are equal.
///
/// This is the algebraic notion of "relation" — a structural axiom like:
/// ```text
/// mul(mul(x, y), z) = mul(x, mul(y, z))
/// ```
///
/// Equations are purely declarative: they describe structure,
/// they do not perform rewriting or inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Equation {
    name: String,
    lhs: Term,
    rhs: Term,
}

impl Equation {
    pub fn new(name: impl Into<String>, lhs: Term, rhs: Term) -> Self {
        Equation {
            name: name.into(),
            lhs,
            rhs,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn lhs(&self) -> &Term {
        &self.lhs
    }

    pub fn rhs(&self) -> &Term {
        &self.rhs
    }
}

impl fmt::Display for Equation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {} = {}", self.name, self.lhs, self.rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equation_creation() {
        let x = Term::var("x");
        let e = Term::constant("e");
        let lhs = Term::app("mul", vec![x.clone(), e]);
        let rhs = x;
        let eq = Equation::new("right_identity", lhs, rhs);

        assert_eq!(eq.name(), "right_identity");
        assert_eq!(format!("{}", eq), "right_identity: mul(x, e) = x");
    }

    #[test]
    fn test_equation_equality() {
        let mk = |name: &str| {
            Equation::new(
                name,
                Term::app("mul", vec![Term::var("x"), Term::constant("e")]),
                Term::var("x"),
            )
        };
        // Same name and content -> equal
        assert_eq!(mk("id"), mk("id"));
    }
}
