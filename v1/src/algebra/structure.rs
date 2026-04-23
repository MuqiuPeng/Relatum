//! Algebraic structures as explicit theories.
//!
//! A `Structure` is a named theory that **adopts** a set of globally-registered
//! operations and imposes equational axioms on them. Operations are declared in
//! a shared [`OpRegistry`]; the structure records which of those operations it
//! uses (its *signature*) and what equations they must satisfy.
//!
//! This gives the semantics: **global registration, local adoption, local axioms**.

use std::collections::BTreeSet;

use super::equation::Equation;
use super::operation::{Arity, OperationId};
use super::registry::OpRegistry;
use super::term::Term;

/// An algebraic structure: a named theory with an explicit operation signature
/// and equational axioms.
///
/// - **Operations** are declared in the global [`OpRegistry`] and then *adopted*
///   into the structure via [`adopt_operation`](Self::adopt_operation).
/// - **Equations** reference only adopted operations; [`validate`](Self::validate)
///   enforces this.
///
/// This two-level design (global registry + local adoption) lets multiple
/// structures share the same [`OperationId`]s while each maintaining a clear
/// boundary of which operations belong to its theory.
#[derive(Debug, Clone)]
pub struct Structure {
    name: String,
    operations: BTreeSet<OperationId>,
    equations: Vec<Equation>,
}

impl Structure {
    pub fn new(name: impl Into<String>) -> Self {
        Structure {
            name: name.into(),
            operations: BTreeSet::new(),
            equations: Vec::new(),
        }
    }

    // ── Operation adoption ────────────────────────────────────

    /// Adopts a globally-registered operation into this structure's signature.
    /// Returns `self` for chaining.
    pub fn with_operation(mut self, id: OperationId) -> Self {
        self.operations.insert(id);
        self
    }

    /// Adopts a globally-registered operation into this structure's signature.
    pub fn adopt_operation(&mut self, id: OperationId) {
        self.operations.insert(id);
    }

    /// The set of operations explicitly adopted by this structure.
    pub fn operations(&self) -> &BTreeSet<OperationId> {
        &self.operations
    }

    // ── Equations ─────────────────────────────────────────────

    /// Adds an equation and returns `self` for chaining.
    pub fn with_equation(mut self, eq: Equation) -> Self {
        self.equations.push(eq);
        self
    }

    /// Adds an equation by mutable reference.
    pub fn add_equation(&mut self, eq: Equation) {
        self.equations.push(eq);
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn equations(&self) -> &[Equation] {
        &self.equations
    }

    /// Returns the deduplicated, sorted list of operation ids that actually
    /// appear in this structure's equations.
    ///
    /// This is a computed scan — for the explicitly declared signature, use
    /// [`operations()`](Self::operations).
    pub fn referenced_operations(&self) -> Vec<OperationId> {
        let mut ops = Vec::new();
        for eq in &self.equations {
            collect_op_ids(eq.lhs(), &mut ops);
            collect_op_ids(eq.rhs(), &mut ops);
        }
        ops.sort_by_key(|id| id.0);
        ops.dedup();
        ops
    }

    // ── Validation ────────────────────────────────────────────

    /// Validates the structure against the registry.
    ///
    /// Two layers of checks:
    /// 1. Every adopted operation exists in the registry.
    /// 2. Every operation used in equations is adopted, exists in the registry,
    ///    and has the correct arity.
    pub fn validate(&self, reg: &OpRegistry) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Layer 1: adopted operations must exist in the registry
        for &id in &self.operations {
            if reg.get_operation(id).is_none() {
                errors.push(format!(
                    "adopted operation {} is not registered",
                    id
                ));
            }
        }

        // Layer 2: equation terms — registry + arity + adoption
        for eq in &self.equations {
            validate_term(reg, &self.operations, eq.lhs(), &mut errors);
            validate_term(reg, &self.operations, eq.rhs(), &mut errors);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            errors.sort();
            errors.dedup();
            Err(errors)
        }
    }

    /// Formats the structure using the registry for operation names.
    pub fn display(&self, reg: &OpRegistry) -> String {
        use std::fmt::Write;
        let mut s = String::new();
        writeln!(s, "Structure: {}", self.name).unwrap();
        writeln!(s, "  Operations:").unwrap();
        for &id in &self.operations {
            if let Some(op) = reg.get_operation(id) {
                writeln!(s, "    {}", op).unwrap();
            }
        }
        writeln!(s, "  Equations:").unwrap();
        for eq in &self.equations {
            writeln!(s, "    {}: {}", eq.name(), reg.format_equation(eq)).unwrap();
        }
        s
    }
}

fn collect_op_ids(term: &Term, ops: &mut Vec<OperationId>) {
    match term {
        Term::Var(_) => {}
        Term::App { op, args } => {
            ops.push(*op);
            for arg in args {
                collect_op_ids(arg, ops);
            }
        }
    }
}

fn validate_term(
    reg: &OpRegistry,
    adopted: &BTreeSet<OperationId>,
    term: &Term,
    errors: &mut Vec<String>,
) {
    match term {
        Term::Var(_) => {}
        Term::App { op, args } => {
            match reg.get_operation(*op) {
                None => {
                    errors.push(format!("invalid operation id: {}", op.0));
                }
                Some(decl) => {
                    // Check adoption
                    if !adopted.contains(op) {
                        errors.push(format!(
                            "operation '{}' ({}) is used in an equation but not adopted by this structure",
                            decl.name(),
                            op
                        ));
                    }
                    // Check arity
                    if !decl.arity().accepts(args.len()) {
                        let expected = match decl.arity() {
                            Arity::Exact(n) => format!("exactly {}", n),
                            Arity::AtLeast(n) => format!("at least {}", n),
                        };
                        errors.push(format!(
                            "arity mismatch for '{}': expects {} arguments, got {}",
                            decl.name(),
                            expected,
                            args.len()
                        ));
                    }
                }
            }
            for arg in args {
                validate_term(reg, adopted, arg, errors);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::builders;
    use crate::algebra::operation::Arity;
    use crate::algebra::registry::OpRegistry;
    use crate::algebra::term::Term;

    fn var(name: &str) -> Term {
        Term::var(name)
    }

    #[test]
    fn test_group_structure() {
        let mut reg = OpRegistry::new();
        let group = builders::group(&mut reg).unwrap();

        assert_eq!(group.name(), "Group");
        assert_eq!(group.referenced_operations().len(), 3);
        assert_eq!(group.equations().len(), 5);

        assert_eq!(
            reg.find_operation("mul").unwrap().arity(),
            Arity::Exact(2)
        );
        assert_eq!(
            reg.find_operation("inv").unwrap().arity(),
            Arity::Exact(1)
        );
        assert_eq!(reg.find_operation("e").unwrap().arity(), Arity::Exact(0));

        assert!(group.validate(&reg).is_ok());
    }

    #[test]
    fn test_group_display() {
        let mut reg = OpRegistry::new();
        let group = builders::group(&mut reg).unwrap();
        let text = group.display(&reg);
        assert!(text.contains("Group"));
        assert!(text.contains("mul/2"));
        assert!(text.contains("associativity"));
    }

    #[test]
    fn test_validation_catches_invalid_op_id() {
        let reg = OpRegistry::new();
        let bad_id = crate::algebra::operation::OperationId(99);
        let s = Structure::new("Bad")
            .with_operation(bad_id)
            .with_equation(Equation::new(
                "oops",
                Term::app(bad_id, vec![var("x"), var("y")]),
                var("x"),
            ));
        let errs = s.validate(&reg).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("not registered")));
    }

    #[test]
    fn test_validation_catches_arity_mismatch() {
        let mut reg = OpRegistry::new();
        let mul = reg.declare_operation("mul", 2).unwrap();
        let s = Structure::new("Bad")
            .with_operation(mul)
            .with_equation(Equation::new(
                "oops",
                Term::app(mul, vec![var("x")]), // arity 1, expected 2
                var("x"),
            ));
        let errs = s.validate(&reg).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("arity mismatch")));
    }

    #[test]
    fn test_validation_catches_unadopted_operation() {
        let mut reg = OpRegistry::new();
        let mul = reg.declare_operation("mul", 2).unwrap();
        // Structure does NOT adopt mul
        let s = Structure::new("Bad").with_equation(Equation::new(
            "oops",
            Term::app(mul, vec![var("x"), var("y")]),
            var("x"),
        ));
        let errs = s.validate(&reg).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("not adopted")));
    }

    #[test]
    fn test_variadic_operation_valid() {
        let mut reg = OpRegistry::new();
        let add = reg.declare_variadic_operation("add", 2).unwrap();
        assert_eq!(reg.get_operation(add).unwrap().arity(), Arity::AtLeast(2));

        let mut s = Structure::new("Variadic");
        s.adopt_operation(add);
        s.add_equation(Equation::new(
            "binary",
            Term::app(add, vec![var("x"), var("y")]),
            var("z"),
        ));
        s.add_equation(Equation::new(
            "ternary",
            Term::app(add, vec![var("x"), var("y"), var("z")]),
            var("w"),
        ));
        s.add_equation(Equation::new(
            "quaternary",
            Term::app(add, vec![var("a"), var("b"), var("c"), var("d")]),
            var("w"),
        ));

        assert!(s.validate(&reg).is_ok());
    }

    #[test]
    fn test_variadic_operation_too_few_args() {
        let mut reg = OpRegistry::new();
        let add = reg.declare_variadic_operation("add", 2).unwrap();

        let mut s = Structure::new("Bad");
        s.adopt_operation(add);
        s.add_equation(Equation::new(
            "oops",
            Term::app(add, vec![var("x")]),
            var("y"),
        ));

        let errs = s.validate(&reg).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("at least 2")));
    }

    #[test]
    fn test_referenced_operations_deduped() {
        let mut reg = OpRegistry::new();
        let mul = reg.declare_operation("mul", 2).unwrap();
        let e = reg.declare_operation("e", 0).unwrap();

        let s = Structure::new("Test")
            .with_operation(mul)
            .with_operation(e)
            .with_equation(Equation::new(
                "left_id",
                Term::app(mul, vec![Term::constant(e), var("x")]),
                var("x"),
            ))
            .with_equation(Equation::new(
                "right_id",
                Term::app(mul, vec![var("x"), Term::constant(e)]),
                var("x"),
            ));

        let ops = s.referenced_operations();
        assert_eq!(ops.len(), 2);
        assert!(ops.contains(&mul));
        assert!(ops.contains(&e));
    }

    #[test]
    fn test_operations_vs_referenced() {
        let mut reg = OpRegistry::new();
        let mul = reg.declare_operation("mul", 2).unwrap();
        let e = reg.declare_operation("e", 0).unwrap();
        let inv = reg.declare_operation("inv", 1).unwrap();

        // Adopt all three but only use mul and e in equations
        let s = Structure::new("Test")
            .with_operation(mul)
            .with_operation(e)
            .with_operation(inv)
            .with_equation(Equation::new(
                "right_id",
                Term::app(mul, vec![var("x"), Term::constant(e)]),
                var("x"),
            ));

        // operations() includes all adopted (3)
        assert_eq!(s.operations().len(), 3);
        // referenced_operations() only includes those in equations (2)
        assert_eq!(s.referenced_operations().len(), 2);
        // validation passes — unused adopted ops are fine
        assert!(s.validate(&reg).is_ok());
    }
}
