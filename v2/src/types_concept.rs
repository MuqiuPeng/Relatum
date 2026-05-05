//! Phase Emergence-1 — Concept lifting types. ADR 0074.
//!
//! A **concept** is a second-order abstraction: a subset of
//! shape-families (size ≥ 2) that co-occur across Signal-class
//! theories. Distinct from theories (instance-bound axiom
//! collections) and from shape families (one canonicalized shape).
//!
//! Lifecycle:
//! - Proposed: produced by `propose_concept_candidates`; not yet
//!   validated, not yet registered
//! - Validated: passed cross-precision floor; eligible for register
//! - Live: registered as meta-R; constituents all still exist
//! - Stale: registered, but ≥1 constituent shape family was
//!   retracted after mint
//! - Falsified: registered, re-validated, fell below floor
//!
//! Stale and Falsified are computed at query time, not stored.

/// A candidate concept produced by mining shape-family
/// co-occurrence in Signal-class theories.
#[derive(Debug, Clone, PartialEq)]
pub struct ConceptCandidate {
    /// Hash-based stable id, deterministic from sorted constituents.
    pub id: String,
    /// Optional dash-joined human-readable alias (sorted constituents).
    pub alias: Option<String>,
    /// Sorted shape-family ids constituting the concept.
    pub constituent_shapes: Vec<String>,
    /// Theories where all constituent shape-families co-occurred.
    pub theories_attested: Vec<String>,
    /// Aggregate cross-precision after validation, or None if not
    /// yet validated.
    pub aggregate_cross_precision: Option<f64>,
}

/// Status of a registered concept, computed at query time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConceptStatus {
    /// All constituent shape families exist; concept is current.
    Live,
    /// At least one constituent shape family was retracted; the
    /// concept is logically broken but its meta-R edges remain
    /// until explicit retraction.
    Stale,
    /// The concept was re-validated recently and passed the floor.
    /// Reserved for future re-validation API; currently equal to
    /// Live for newly-registered concepts.
    Validated,
    /// The concept was re-validated and fell below the floor. The
    /// meta-R remains; explicit retraction must follow. Reserved
    /// for future re-validation API.
    Falsified,
}

/// Mining configuration for `propose_concept_candidates`.
#[derive(Debug, Clone)]
pub struct ConceptMiningConfig {
    /// A candidate must co-occur in at least this many theories.
    /// Default: 2 (the smallest non-degenerate co-occurrence).
    pub min_theories: usize,
    /// If true, only Signal-class theories count toward
    /// `min_theories`. Default: true (filters concepts grounded
    /// in noise).
    pub require_signal_only: bool,
    /// Cross-precision aggregate floor for `validate_concept`.
    /// Default: 0.80.
    pub validation_floor: f64,
    /// Cap on the size of constituent_shapes. Default: 4. Larger
    /// concepts are conjectural and likely to be subsumed by
    /// smaller validated ones.
    pub max_candidate_size: usize,
}

impl Default for ConceptMiningConfig {
    fn default() -> Self {
        Self {
            min_theories: 2,
            require_signal_only: true,
            validation_floor: 0.80,
            max_candidate_size: 4,
        }
    }
}

/// Errors from `register_concept`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConceptRegistrationError {
    /// The candidate has not been validated (aggregate is None).
    NotValidated,
    /// The candidate's id already exists as a registered concept.
    AlreadyRegistered(String),
    /// One or more constituent shape families do not exist in the
    /// rset at register time.
    UnknownConstituent(String),
    /// One or more attested theories do not exist in the rset at
    /// register time.
    UnknownTheory(String),
    /// constituent_shapes is empty or below 2.
    DegenerateConstituents,
}

/// Compute a deterministic concept id from sorted constituent
/// shape-family ids. Uses a 16-hex-char hash for collision safety.
pub fn concept_id_from_constituents(sorted_shapes: &[String]) -> String {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    let mut h = DefaultHasher::new();
    for s in sorted_shapes {
        s.hash(&mut h);
        // Separator hashed in to disambiguate ["ab", "c"] from ["a", "bc"].
        "|".hash(&mut h);
    }
    format!("concept_{:016x}", h.finish())
}

/// Compute a human-readable alias from sorted constituent shape
/// ids (dash-joined, prefixed with "concept_alias_").
pub fn concept_alias_from_constituents(sorted_shapes: &[String]) -> String {
    let cleaned: Vec<String> = sorted_shapes
        .iter()
        .map(|s| s.replace("shape_", "").replace("_", "-"))
        .collect();
    format!("concept_alias_{}", cleaned.join("__"))
}
