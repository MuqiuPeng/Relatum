//! # Relatum
//!
//! Equational closure engine for algebraic structures.
//!
//! Relatum lets you define algebraic structures (groups, rings, etc.) by their
//! operations and equations, then computes equational closure over ground facts.
//!
//! ## Quick start
//!
//! ```rust
//! use relatum::algebra::{builders, ClosureEngine, Equation, OpRegistry, Parser, Term};
//!
//! // Create a shared registry and pick a structure
//! let mut reg = OpRegistry::new();
//! let monoid = builders::monoid(&mut reg).unwrap();
//!
//! // Parse a fact against the registry, then hand it to the engine
//! let fact = Parser::new(&reg).parse_equation("seed", "a = a").unwrap();
//! let mut engine = ClosureEngine::new(reg);
//! engine.add_structure(&monoid);
//! engine.add_fact(fact);
//!
//! // Compute closure — derives mul(a, e) = a, mul(e, a) = a, etc.
//! let result = engine.compute_closure(2);
//! println!("{} equivalence classes, {} derived equations",
//!     result.equivalence_classes.len(),
//!     result.derived_equations.len());
//! ```

pub mod algebra;
pub mod relational;

// Primary exports: algebra and closure engine
pub use algebra::{
    builders, ClosureEngine, ClosureResult, Equation, OpRegistry, Parser, RegistryError, Structure,
    Term,
};
