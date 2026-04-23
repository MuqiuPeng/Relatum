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
    /// Optional high-level category for grouping related axioms
    /// (e.g. `"identity"`, `"additive_group"`).
    /// Empty string means uncategorized.
    category: String,
}

impl Equation {
    pub fn new(name: impl Into<String>, lhs: Term, rhs: Term) -> Self {
        Equation {
            name: name.into(),
            lhs,
            rhs,
            category: String::new(),
        }
    }

    /// Sets the category for this equation, returning `self` for chaining.
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = category.into();
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn category(&self) -> &str {
        &self.category
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
    use crate::algebra::operation::OperationId;

    #[test]
    fn test_equation_creation() {
        let x = Term::var("x");
        let mul = OperationId(0);
        let e = OperationId(1);
        let lhs = Term::app(mul, vec![x.clone(), Term::constant(e)]);
        let rhs = x;
        let eq = Equation::new("right_identity", lhs, rhs);

        assert_eq!(eq.name(), "right_identity");
        assert_eq!(format!("{}", eq), "right_identity: #0(x, #1) = x");
    }

    #[test]
    fn test_equation_equality() {
        let mul = OperationId(0);
        let e = OperationId(1);
        let mk = |name: &str| {
            Equation::new(
                name,
                Term::app(mul, vec![Term::var("x"), Term::constant(e)]),
                Term::var("x"),
            )
        };
        assert_eq!(mk("id"), mk("id"));
    }
}
