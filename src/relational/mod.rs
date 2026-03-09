//! Pure relational closure engine.
//!
//! A minimal logic system that operates exclusively on **terms** and
//! **relations**, with no predefined mathematical semantics. Equality is not
//! a built-in concept — declare it with [`ClosureEngine::define_equivalence`]
//! to get symmetry, transitivity, reflexivity, and congruence.
//!
//! # Core types
//!
//! | Type | Role |
//! |------|------|
//! | [`Term`] | Symbolic expression — variable or function application |
//! | [`Relation`] | Named connection between terms |
//! | [`Rule`] | Inference rule: premises ⊢ conclusions |
//! | [`ClosureEngine`] | Declares entities/relations, derives closure |
//!
//! # Example
//!
//! ```
//! use relatum::relational::*;
//!
//! let mut engine = ClosureEngine::new();
//!
//! // Declare entities
//! let a = engine.define_constant("a");
//! let b = engine.define_constant("b");
//! let c = engine.define_constant("c");
//!
//! // Declare equiv as a full equivalence relation
//! engine.define_equivalence("equiv");
//!
//! // Add ground facts
//! engine.add_fact(Relation::binary("equiv", a.clone(), b.clone()));
//! engine.add_fact(Relation::binary("equiv", b, c.clone()));
//!
//! let result = engine.derive_closure();
//!
//! // Transitivity derived equiv(a, c); symmetry derived equiv(c, a)
//! assert!(result.facts.contains(&Relation::binary("equiv", a.clone(), c.clone())));
//! assert!(result.facts.contains(&Relation::binary("equiv", c, a)));
//! assert!(result.saturated);
//! ```

pub mod engine;
pub mod relation;
pub mod rule;
pub mod term;

pub use engine::{ClosureEngine, ClosureResult, RelationDef};
pub use relation::Relation;
pub use rule::{RelationPattern, Rule, Substitution};
pub use term::Term;

// ── Future extension interfaces (not yet implemented) ────────

/// Categorize relations by type (e.g., "equivalence", "order", "membership").
pub trait RelationClassifier {
    fn classify(&self, relation: &Relation) -> Option<&str>;
}

/// Assign priorities to rules for ordered application.
pub trait RulePriority {
    fn priority(&self, rule: &Rule) -> i32;
}

/// Normalize terms before comparison (e.g., canonical ordering of arguments).
pub trait TermNormalizer {
    fn normalize(&self, term: &Term) -> Term;
}

/// Observe engine events for visualization or debugging.
pub trait ClosureObserver {
    fn on_fact_derived(&mut self, fact: &Relation, source: &str);
    fn on_round_complete(&mut self, round: usize, new_count: usize);
}
