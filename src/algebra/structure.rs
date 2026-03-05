//! Algebraic structures defined by operations and equations.

use std::fmt;

use super::equation::Equation;
use super::operation::Operation;

/// An algebraic structure described by its signature (operations) and axioms (equations).
///
/// A `Structure` is a purely declarative description — it says *what* a structure is,
/// not how to compute within it.
///
/// # Example
///
/// ```
/// use relatum::algebra::{Operation, Term, Equation, Structure};
///
/// let group = Structure::new("Group")
///     .with_operation(Operation::binary("mul"))
///     .with_operation(Operation::unary("inv"))
///     .with_operation(Operation::nullary("e"))
///     .with_equation(Equation::new(
///         "right_identity",
///         Term::app("mul", vec![Term::var("x"), Term::constant("e")]),
///         Term::var("x"),
///     ));
/// assert_eq!(group.operations().len(), 3);
/// ```
#[derive(Debug, Clone)]
pub struct Structure {
    name: String,
    operations: Vec<Operation>,
    equations: Vec<Equation>,
}

impl Structure {
    pub fn new(name: impl Into<String>) -> Self {
        Structure {
            name: name.into(),
            operations: Vec::new(),
            equations: Vec::new(),
        }
    }

    /// Adds an operation and returns `self` for chaining.
    pub fn with_operation(mut self, op: Operation) -> Self {
        self.operations.push(op);
        self
    }

    /// Adds an equation and returns `self` for chaining.
    pub fn with_equation(mut self, eq: Equation) -> Self {
        self.equations.push(eq);
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn operations(&self) -> &[Operation] {
        &self.operations
    }

    pub fn equations(&self) -> &[Equation] {
        &self.equations
    }

    /// Finds an operation by name.
    pub fn find_operation(&self, name: &str) -> Option<&Operation> {
        self.operations.iter().find(|op| op.name() == name)
    }

    /// Checks that every operation referenced in equations is declared,
    /// and that argument counts match declared arities.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        for eq in &self.equations {
            self.validate_term(eq.lhs(), &mut errors);
            self.validate_term(eq.rhs(), &mut errors);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            errors.dedup();
            Err(errors)
        }
    }

    fn validate_term(&self, term: &super::term::Term, errors: &mut Vec<String>) {
        use super::term::Term;
        match term {
            Term::Var(_) => {}
            Term::App { op, args } => {
                match self.find_operation(op) {
                    None => {
                        errors.push(format!("undeclared operation: {}", op));
                    }
                    Some(decl) if decl.arity() != args.len() => {
                        errors.push(format!(
                            "arity mismatch for {}: declared {}, got {}",
                            op,
                            decl.arity(),
                            args.len()
                        ));
                    }
                    _ => {}
                }
                for arg in args {
                    self.validate_term(arg, errors);
                }
            }
        }
    }
}

impl fmt::Display for Structure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Structure: {}", self.name)?;
        writeln!(f, "  Operations:")?;
        for op in &self.operations {
            writeln!(f, "    {}", op)?;
        }
        writeln!(f, "  Equations:")?;
        for eq in &self.equations {
            writeln!(f, "    {}", eq)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::term::Term;

    // ── helpers ──────────────────────────────────────────────

    fn var(name: &str) -> Term {
        Term::var(name)
    }

    fn mul(a: Term, b: Term) -> Term {
        Term::app("mul", vec![a, b])
    }

    fn inv(a: Term) -> Term {
        Term::app("inv", vec![a])
    }

    fn e() -> Term {
        Term::constant("e")
    }

    fn build_group() -> Structure {
        let (x, y, z) = (var("x"), var("y"), var("z"));

        Structure::new("Group")
            .with_operation(Operation::binary("mul"))
            .with_operation(Operation::unary("inv"))
            .with_operation(Operation::nullary("e"))
            // associativity: mul(mul(x,y),z) = mul(x,mul(y,z))
            .with_equation(Equation::new(
                "associativity",
                mul(mul(x.clone(), y.clone()), z.clone()),
                mul(x.clone(), mul(y.clone(), z.clone())),
            ))
            // right identity: mul(x, e) = x
            .with_equation(Equation::new(
                "right_identity",
                mul(x.clone(), e()),
                x.clone(),
            ))
            // left identity: mul(e, x) = x
            .with_equation(Equation::new(
                "left_identity",
                mul(e(), x.clone()),
                x.clone(),
            ))
            // right inverse: mul(x, inv(x)) = e
            .with_equation(Equation::new(
                "right_inverse",
                mul(x.clone(), inv(x.clone())),
                e(),
            ))
            // left inverse: mul(inv(x), x) = e
            .with_equation(Equation::new(
                "left_inverse",
                mul(inv(x.clone()), x.clone()),
                e(),
            ))
    }

    // ── tests ───────────────────────────────────────────────

    #[test]
    fn test_group_structure() {
        let group = build_group();

        assert_eq!(group.name(), "Group");
        assert_eq!(group.operations().len(), 3);
        assert_eq!(group.equations().len(), 5);

        // Check operations
        assert_eq!(group.find_operation("mul").unwrap().arity(), 2);
        assert_eq!(group.find_operation("inv").unwrap().arity(), 1);
        assert_eq!(group.find_operation("e").unwrap().arity(), 0);

        // Validate arity consistency
        assert!(group.validate().is_ok());
    }

    #[test]
    fn test_group_display() {
        let group = build_group();
        let text = format!("{}", group);
        assert!(text.contains("Group"));
        assert!(text.contains("mul/2"));
        assert!(text.contains("associativity"));
    }

    #[test]
    fn test_validation_catches_undeclared_op() {
        let s = Structure::new("Bad")
            .with_operation(Operation::binary("add"))
            .with_equation(Equation::new(
                "oops",
                Term::app("mul", vec![Term::var("x"), Term::var("y")]),
                Term::var("x"),
            ));
        let errs = s.validate().unwrap_err();
        assert!(errs.iter().any(|e| e.contains("undeclared operation: mul")));
    }

    #[test]
    fn test_validation_catches_arity_mismatch() {
        let s = Structure::new("Bad")
            .with_operation(Operation::binary("mul"))
            .with_equation(Equation::new(
                "oops",
                Term::app("mul", vec![Term::var("x")]), // arity 1, expected 2
                Term::var("x"),
            ));
        let errs = s.validate().unwrap_err();
        assert!(errs.iter().any(|e| e.contains("arity mismatch")));
    }
}
