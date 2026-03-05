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
//! - **Extensible foundation**: designed to support future layers like
//!   rewrite engines, e-graphs, or symbolic reasoning.
//!
//! # Core types
//!
//! | Type | Role |
//! |------|------|
//! | [`Operation`] | Named operator with fixed arity (e.g., `mul/2`, `inv/1`, `e/0`) |
//! | [`Term`] | Symbolic expression tree (variables, constants, nested applications) |
//! | [`Equation`] | Axiom asserting two terms are equal (e.g., associativity) |
//! | [`Structure`] | A complete algebraic structure: operations + equations |
//!
//! # Example: defining a group
//!
//! ```
//! use relatum::algebra::*;
//!
//! let (x, y, z) = (Term::var("x"), Term::var("y"), Term::var("z"));
//!
//! let mut group = Structure::new("Group");
//! let mul = group.declare_operation("mul", 2);
//! let inv = group.declare_operation("inv", 1);
//! let e = group.declare_operation("e", 0);
//!
//! group = group
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
//! assert!(group.validate().is_ok());
//! ```

pub mod equation;
pub mod operation;
pub mod structure;
pub mod term;

pub use equation::Equation;
pub use operation::{Operation, OperationId};
pub use structure::Structure;
pub use term::Term;
