//! Global operation registry for algebraic structures.
//!
//! All [`Structure`](super::structure::Structure)s share a single `OpRegistry`
//! so that [`OperationId`]s are globally unique and operations from different
//! structures can coexist in the same term or equation.

use std::collections::HashMap;
use std::fmt;

use super::equation::Equation;
use super::operation::{Arity, Operation, OperationId};
use super::term::Term;

/// Error from registry operations (e.g. declaring an operation with conflicting arity).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    /// Same operation name declared again with a different arity.
    OperationArityConflict {
        name: String,
        existing: Arity,
        requested: Arity,
    },
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistryError::OperationArityConflict {
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

impl std::error::Error for RegistryError {}

/// Global registry of algebraic operations.
///
/// Operations are registered via [`declare_operation`](OpRegistry::declare_operation),
/// which returns an [`OperationId`] for use in [`Term`] and [`Equation`].
/// The same name with the same arity is deduplicated (returns the existing id);
/// the same name with a different arity is an error.
#[derive(Debug, Clone)]
pub struct OpRegistry {
    operations: Vec<Operation>,
    op_lookup: HashMap<String, OperationId>,
}

impl OpRegistry {
    pub fn new() -> Self {
        OpRegistry {
            operations: Vec::new(),
            op_lookup: HashMap::new(),
        }
    }

    /// Declares an operation by name and exact arity.
    pub fn declare_operation(
        &mut self,
        name: &str,
        arity: usize,
    ) -> Result<OperationId, RegistryError> {
        self.declare_operation_with_arity(name, Arity::Exact(arity))
    }

    /// Declares a variadic operation that accepts `min` or more arguments.
    pub fn declare_variadic_operation(
        &mut self,
        name: &str,
        min: usize,
    ) -> Result<OperationId, RegistryError> {
        self.declare_operation_with_arity(name, Arity::AtLeast(min))
    }

    /// Declares an operation with an explicit [`Arity`] constraint.
    pub fn declare_operation_with_arity(
        &mut self,
        name: &str,
        arity: Arity,
    ) -> Result<OperationId, RegistryError> {
        if let Some(&id) = self.op_lookup.get(name) {
            let existing_arity = self.operations[id.0 as usize].arity();
            if existing_arity != arity {
                return Err(RegistryError::OperationArityConflict {
                    name: name.to_string(),
                    existing: existing_arity,
                    requested: arity,
                });
            }
            return Ok(id);
        }
        let id = OperationId(self.operations.len() as u32);
        self.operations.push(Operation::with_arity(name, arity));
        self.op_lookup.insert(name.to_string(), id);
        Ok(id)
    }

    pub fn operations(&self) -> &[Operation] {
        &self.operations
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

    /// Formats a term using declared operation names instead of raw ids.
    pub fn format_term(&self, term: &Term) -> String {
        match term {
            Term::Var(name) => name.clone(),
            Term::App { op, args } => {
                let op_name = self
                    .get_operation(*op)
                    .map(|o| o.name().to_string())
                    .unwrap_or_else(|| format!("#{}", op.0));
                if args.is_empty() {
                    op_name
                } else {
                    let arg_strs: Vec<String> =
                        args.iter().map(|a| self.format_term(a)).collect();
                    format!("{}({})", op_name, arg_strs.join(", "))
                }
            }
        }
    }

    /// Formats an equation using declared operation names.
    pub fn format_equation(&self, eq: &Equation) -> String {
        format!(
            "{} = {}",
            self.format_term(eq.lhs()),
            self.format_term(eq.rhs())
        )
    }
}

impl Default for OpRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_declare_operation() {
        let mut reg = OpRegistry::new();
        let mul = reg.declare_operation("mul", 2).unwrap();
        let inv = reg.declare_operation("inv", 1).unwrap();
        let e = reg.declare_operation("e", 0).unwrap();

        assert_eq!(mul.id(), 0);
        assert_eq!(inv.id(), 1);
        assert_eq!(e.id(), 2);

        assert_eq!(reg.operations().len(), 3);
        assert_eq!(
            reg.find_operation("mul").unwrap().arity(),
            Arity::Exact(2)
        );
    }

    #[test]
    fn test_declare_operation_dedup() {
        let mut reg = OpRegistry::new();
        let a = reg.declare_operation("mul", 2).unwrap();
        let b = reg.declare_operation("mul", 2).unwrap();
        assert_eq!(a, b);
        assert_eq!(reg.operations().len(), 1);
    }

    #[test]
    fn test_declare_operation_arity_conflict() {
        let mut reg = OpRegistry::new();
        assert!(reg.declare_operation("mul", 2).is_ok());
        let err = reg.declare_operation("mul", 1).unwrap_err();
        assert_eq!(
            err,
            RegistryError::OperationArityConflict {
                name: "mul".to_string(),
                existing: Arity::Exact(2),
                requested: Arity::Exact(1),
            }
        );
    }

    #[test]
    fn test_variadic_operation() {
        let mut reg = OpRegistry::new();
        let add = reg.declare_variadic_operation("add", 2).unwrap();
        assert_eq!(
            reg.get_operation(add).unwrap().arity(),
            Arity::AtLeast(2)
        );
    }

    #[test]
    fn test_variadic_conflict_with_exact() {
        let mut reg = OpRegistry::new();
        reg.declare_operation("mul", 2).unwrap();
        let err = reg.declare_variadic_operation("mul", 2).unwrap_err();
        assert_eq!(
            err,
            RegistryError::OperationArityConflict {
                name: "mul".to_string(),
                existing: Arity::Exact(2),
                requested: Arity::AtLeast(2),
            }
        );
    }

    #[test]
    fn test_format_term() {
        let mut reg = OpRegistry::new();
        let mul = reg.declare_operation("mul", 2).unwrap();
        let e = reg.declare_operation("e", 0).unwrap();

        let term = Term::app(mul, vec![Term::var("x"), Term::constant(e)]);
        assert_eq!(reg.format_term(&term), "mul(x, e)");
    }

    #[test]
    fn test_format_equation() {
        let mut reg = OpRegistry::new();
        let mul = reg.declare_operation("mul", 2).unwrap();
        let e = reg.declare_operation("e", 0).unwrap();

        let eq = Equation::new(
            "right_id",
            Term::app(mul, vec![Term::var("x"), Term::constant(e)]),
            Term::var("x"),
        );
        assert_eq!(reg.format_equation(&eq), "mul(x, e) = x");
    }

    #[test]
    fn test_cross_structure_ids_unique() {
        let mut reg = OpRegistry::new();

        // "Group" operations
        let mul = reg.declare_operation("mul", 2).unwrap();
        let inv = reg.declare_operation("inv", 1).unwrap();
        let e = reg.declare_operation("e", 0).unwrap();

        // "Ring" adds more operations — same registry
        let add = reg.declare_operation("add", 2).unwrap();
        let zero = reg.declare_operation("zero", 0).unwrap();
        let neg = reg.declare_operation("neg", 1).unwrap();

        // "mul" from ring reuses the same id (same arity)
        let mul2 = reg.declare_operation("mul", 2).unwrap();
        assert_eq!(mul, mul2);

        // All ids are distinct
        let ids = [mul, inv, e, add, zero, neg];
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                assert_ne!(ids[i], ids[j], "ids[{}] == ids[{}]", i, j);
            }
        }
        assert_eq!(reg.operations().len(), 6);
    }
}
