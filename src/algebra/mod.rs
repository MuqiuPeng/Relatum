//! Algebraic structure description layer.
//!
//! This module provides a minimal framework for expressing algebraic structures
//! (groups, rings, fields, etc.) in terms of their **operations** and **equations**.
//!
//! # Design philosophy
//!
//! - **Relations first, not objects first**: structures are defined by the
//!   equations (relations) that their operations must satisfy.
//! - **Declarative only**: this layer describes *what* a structure is,
//!   it does not perform computation or inference.
//! - **Global registry**: all operations live in a shared [`OpRegistry`],
//!   so that multiple structures can coexist and interact with globally
//!   unique [`OperationId`]s.
//! - **Separation of concerns**: the registry owns operation identity;
//!   structures are pure axiom containers.
//!
//! # Core types
//!
//! | Type | Role |
//! |------|------|
//! | [`OpRegistry`] | Global operation registry — allocates unique [`OperationId`]s |
//! | [`Operation`] | Named operator with arity (e.g., `mul/2`, `inv/1`, `e/0`) |
//! | [`Term`] | Symbolic expression tree (variables, constants, nested applications) |
//! | [`Equation`] | Axiom asserting two terms are equal (e.g., associativity) |
//! | [`Structure`] | A named set of axioms referencing globally-registered operations |
//!
//! # Example: defining a group
//!
//! ```
//! use relatum::algebra::*;
//!
//! let mut reg = OpRegistry::new();
//!
//! // Operations are declared in the global registry
//! let mul = reg.declare_operation("mul", 2).unwrap();
//! let inv = reg.declare_operation("inv", 1).unwrap();
//! let e   = reg.declare_operation("e", 0).unwrap();
//!
//! let (x, y, z) = (Term::var("x"), Term::var("y"), Term::var("z"));
//!
//! // Structure is a pure axiom container
//! let group = Structure::new("Group")
//!     .with_equation(Equation::new(
//!         "associativity",
//!         Term::app(mul, vec![
//!             Term::app(mul, vec![x.clone(), y.clone()]),
//!             z.clone(),
//!         ]),
//!         Term::app(mul, vec![
//!             x.clone(),
//!             Term::app(mul, vec![y.clone(), z.clone()]),
//!         ]),
//!     ))
//!     .with_equation(Equation::new(
//!         "right_identity",
//!         Term::app(mul, vec![x.clone(), Term::constant(e)]),
//!         x.clone(),
//!     ));
//!
//! assert!(group.validate(&reg).is_ok());
//! ```

pub mod builders;
pub mod closure;
pub mod equation;
pub mod operation;
pub mod parser;
pub mod registry;
pub mod structure;
pub mod term;

pub use closure::{ClosureEngine, ClosureResult, DerivedCategory};
pub use equation::Equation;
pub use operation::{Arity, Operation, OperationId};
pub use parser::Parser;
pub use registry::{OpRegistry, RegistryError};
pub use structure::Structure;
pub use term::Term;
