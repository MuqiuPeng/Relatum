//! Algebraic structures defined by operations and equations.

use std::collections::HashMap;
use std::fmt;

use super::equation::Equation;
use super::operation::{Operation, OperationId};

/// Error from structure operations (e.g. declaring an operation with conflicting arity).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructureError {
    /// Same operation name declared again with a different arity.
    OperationArityConflict {
        name: String,
        existing: usize,
        requested: usize,
    },
}

impl fmt::Display for StructureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StructureError::OperationArityConflict {
                name,
                existing,
                requested,
            } => write!(
                f,
                "operation '{}' already declared with arity {}, requested {}",
                name, existing, requested
            ),
        }
    }
}

impl std::error::Error for StructureError {}

/// An algebraic structure described by its signature (operations) and axioms (equations).
///
/// Operations are registered via [`declare_operation`](Structure::declare_operation), which
/// returns an [`OperationId`] for use in [`Term::app`] and [`Term::constant`].
#[derive(Debug, Clone)]
pub struct Structure {
    name: String,
    /// operations[id.0 as usize] is the operation for that id
    operations: Vec<Operation>,
    /// name -> OperationId for lookup and deduplication
    op_lookup: HashMap<String, OperationId>,
    equations: Vec<Equation>,
}

impl Structure {
    pub fn new(name: impl Into<String>) -> Self {
        Structure {
            name: name.into(),
            operations: Vec::new(),
            op_lookup: HashMap::new(),
            equations: Vec::new(),
        }
    }

    /// Declares an operation by name and arity.
    /// If the name is not yet registered: creates it and returns `Ok(id)`.
    /// If the name exists with the same arity: returns `Ok(existing_id)`.
    /// If the name exists with a different arity: returns `Err(StructureError::OperationArityConflict)`.
    pub fn declare_operation(
        &mut self,
        name: &str,
        arity: usize,
    ) -> Result<OperationId, StructureError> {
        if let Some(&id) = self.op_lookup.get(name) {
            let existing_arity = self.operations[id.0 as usize].arity();
            if existing_arity != arity {
                return Err(StructureError::OperationArityConflict {
                    name: name.to_string(),
                    existing: existing_arity,
                    requested: arity,
                });
            }
            return Ok(id);
        }
        let id = OperationId(self.operations.len() as u32);
        self.operations.push(Operation::new(name, arity));
        self.op_lookup.insert(name.to_string(), id);
        Ok(id)
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

    /// Returns the operation for the given id, if valid.
    pub fn get_operation(&self, id: OperationId) -> Option<&Operation> {
        self.operations.get(id.0 as usize)
    }

    /// Finds an operation by name, returning its id.
    pub fn find_operation_id(&self, name: &str) -> Option<OperationId> {
        self.op_lookup.get(name).copied()
    }

    /// Finds an operation by name.
    pub fn find_operation(&self, name: &str) -> Option<&Operation> {
        self.find_operation_id(name)
            .and_then(|id| self.get_operation(id))
    }

    /// Checks that every operation id in equations is valid and arities match.
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
                match self.get_operation(*op) {
                    None => {
                        errors.push(format!("invalid operation id: {}", op.0));
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

    fn var(name: &str) -> Term {
        Term::var(name)
    }

    fn build_group() -> Structure {
        let (x, y, z) = (var("x"), var("y"), var("z"));

        let mut group = Structure::new("Group");
        let mul = group.declare_operation("mul", 2).unwrap();
        let inv = group.declare_operation("inv", 1).unwrap();
        let e = group.declare_operation("e", 0).unwrap();

        group = group
            .with_equation(Equation::new(
                "associativity",
                Term::app(
                    mul,
                    vec![
                        Term::app(mul, vec![x.clone(), y.clone()]),
                        z.clone(),
                    ],
                ),
                Term::app(
                    mul,
                    vec![
                        x.clone(),
                        Term::app(mul, vec![y.clone(), z.clone()]),
                    ],
                ),
            ))
            .with_equation(Equation::new(
                "right_identity",
                Term::app(mul, vec![x.clone(), Term::constant(e)]),
                x.clone(),
            ))
            .with_equation(Equation::new(
                "left_identity",
                Term::app(mul, vec![Term::constant(e), x.clone()]),
                x.clone(),
            ))
            .with_equation(Equation::new(
                "right_inverse",
                Term::app(mul, vec![x.clone(), Term::app(inv, vec![x.clone()])]),
                Term::constant(e),
            ))
            .with_equation(Equation::new(
                "left_inverse",
                Term::app(mul, vec![Term::app(inv, vec![x.clone()]), x.clone()]),
                Term::constant(e),
            ));
        group
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
    fn test_validation_catches_invalid_op_id() {
        let mut s = Structure::new("Bad");
        s.declare_operation("add", 2).unwrap();
        let bad_id = crate::algebra::operation::OperationId(99);
        s = s.with_equation(Equation::new(
            "oops",
            Term::app(bad_id, vec![Term::var("x"), Term::var("y")]),
            Term::var("x"),
        ));
        let errs = s.validate().unwrap_err();
        assert!(errs.iter().any(|e| e.contains("invalid operation id")));
    }

    #[test]
    fn test_validation_catches_arity_mismatch() {
        let mut s = Structure::new("Bad");
        let mul = s.declare_operation("mul", 2).unwrap();
        s = s.with_equation(Equation::new(
            "oops",
            Term::app(mul, vec![Term::var("x")]), // arity 1, expected 2
            Term::var("x"),
        ));
        let errs = s.validate().unwrap_err();
        assert!(errs.iter().any(|e| e.contains("arity mismatch")));
    }

    #[test]
    fn test_declare_operation_arity_conflict() {
        let mut s = Structure::new("Test");
        assert!(s.declare_operation("mul", 2).is_ok());
        let err = s.declare_operation("mul", 1).unwrap_err();
        assert_eq!(
            err,
            StructureError::OperationArityConflict {
                name: "mul".to_string(),
                existing: 2,
                requested: 1,
            }
        );
        // Same arity again is ok
        assert!(s.declare_operation("mul", 2).is_ok());
    }
}
