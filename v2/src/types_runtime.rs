//! Runtime/autonomous configuration types: sampling, refinement,
//! autonomous pass config, retraction errors and summaries, outcome
//! enums. Pure data — no methods on RSet. ADR 0021, 0022, 0024,
//! 0017, 0020.

use crate::{
    CanonicalForm, DiscoveryConfig, NamingDecision, NamingPolicy, SkipReason,
};

/// Configuration for sampling-based `sample_instances_of`. ADR 0024.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SamplingMatchConfig {
    pub sample_count: usize,
    pub rng_seed: u64,
}

/// Configuration for representative refinement. ADR 0017.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefinementConfig {
    /// Per-candidate re-sampling budget.
    pub max_tries: usize,
    pub rng_seed: u64,
}

/// Configuration for the autonomous abstraction pass. ADR 0018,
/// extended by ADR 0043 with `instance_sampling`.
#[derive(Debug, Clone)]
pub struct AutonomousConfig {
    pub discovery: DiscoveryConfig,
    pub refinement: RefinementConfig,
    pub naming: NamingPolicy,
    /// When `Some(cfg)`, `autonomous_pass` uses `sample_instances_of`
    /// (ADR 0024) to collect instances of a novel canonical, instead
    /// of the exhaustive `find_instances_of`. This trades complete
    /// enumeration for tractability on large graphs; may under-report
    /// instance counts. `None` (default) keeps the exhaustive path.
    /// ADR 0043.
    pub instance_sampling: Option<SamplingMatchConfig>,
}

/// Why a candidate was skipped by the autonomous pass. ADR 0018.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutonomousSkip {
    /// No clean instance of this canonical exists in the current data.
    NoCleanInstance,
    /// Naming policy (min_edges / min_instances / attach_only) filtered
    /// the candidate out.
    PolicyFiltered(SkipReason),
}

/// Error from `retract_pattern`. ADR 0020.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetractionError {
    UnknownPattern,
}

/// Summary of a successful retraction. ADR 0020.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetractionSummary {
    pub pattern_id: String,
    pub instances_removed: usize,
    pub meta_edges_removed: usize,
}

/// ADR 0070 Step 2 — Outcome of `retract_shape_family`.
///
/// Field semantics differ by layer (per ADR 0070 §4.4):
/// - **L2**: retraction cascades into axiom-level cleanup
///   (`theory_memberships_detached` + `axioms_globally_retracted`).
/// - **L3+**: retraction removes the family's own meta-R but does
///   NOT cascade into member families (`member_links_removed`
///   counts the per-member edges severed). Underlying L2 / L3
///   abstractions remain available for re-aggregation.
///
/// `structural_edges_removed` counts the family's own marker +
/// kind-tag edges (always 1 + (1 if kind tag was present)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeFamilyRetractionSummary {
    pub family_id: String,
    pub layer: crate::FamilyLayer,
    /// L2 only: number of (theory, axiom) membership edges removed.
    pub theory_memberships_detached: usize,
    /// L2 only: number of axioms whose registration was retracted.
    pub axioms_globally_retracted: usize,
    /// L3+ only: number of `R(family_id, member_id)` edges removed.
    pub member_links_removed: usize,
    /// All layers: marker + kind-tag edges removed for the family
    /// itself.
    pub structural_edges_removed: usize,
}

/// ADR 0070 Step 2 — Errors from `retract_shape_family`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShapeFamilyRetractionError {
    /// The id is not registered as a family at any layer.
    UnknownFamily,
    /// L2 retraction failed mid-cascade because a member axiom
    /// could not be retracted (e.g., still referenced by an
    /// unrelated theory or registry inconsistency). Contains the
    /// axiom id and the upstream error message.
    AxiomRetractFailed { axiom_id: String, cause: String },
}

/// Combined outcome of `autonomous_and_attach`. ADR 0022.
#[derive(Debug, Clone)]
pub struct AutonomousAndAttachSummary {
    pub autonomous: Vec<AutonomousOutcome>,
    pub attach: Vec<(CanonicalForm, NamingDecision)>,
}

/// Outcome of a single candidate in an autonomous pass. ADR 0018.
#[derive(Debug, Clone)]
pub enum AutonomousOutcome {
    /// A new pattern was created from a novel canonical.
    NewPattern {
        pattern_id: String,
        instance_count: usize,
        canonical: CanonicalForm,
    },
    /// The candidate's canonical matches an already-named pattern;
    /// autonomous pass reports but takes no action. Use
    /// `run_naming_pass` with `attach_only = true` to extend it.
    Existing {
        pattern_id: String,
        canonical: CanonicalForm,
    },
    /// The candidate was rejected.
    Skipped {
        canonical: CanonicalForm,
        reason: AutonomousSkip,
    },
}
