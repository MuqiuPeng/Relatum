//! Reserved meta-R registry markers + small types adjacent to them
//! (TheoryRelationKind, TheoryNeighborhood, PersistenceError).
//!
//! ADR 0010, 0029, 0030, 0032, 0034, 0038, 0042, 0046, 0049, 0052, 0053,
//! 0061, 0064. Each marker is a string token used as the *left* of an
//! `R(MARKER, x)` edge to declare `x` as a member of that meta-R class.
//! All markers begin and end with double underscores by convention; this
//! is a pragmatic collision guard, not an ontological exception.

pub const PATTERN_MARKER: &str = "__pattern__";

/// Reserved registry marker for role identifiers in a pattern's
/// *intension* (ADR 0029). `R(ROLE_MARKER, p_N_role_i)` declares
/// `p_N_role_i` as a role in pattern `p_N`; the structural edges among
/// roles then encode the pattern's canonical form explicitly. Same
/// collision-guard caveats as `PATTERN_MARKER`.
pub const ROLE_MARKER: &str = "__role__";

/// Reserved registry marker for axiom identifiers (ADR 0030).
/// `R(AXIOM_MARKER, ax_X)` declares `ax_X` as a named axiom.
/// In the A-phase this is a pure registry; the axiom's *intension*
/// (premise / conclusion / role structure) is deferred to ADR 0031+.
pub const AXIOM_MARKER: &str = "__axiom__";

/// Reserved registry marker for theory identifiers (ADR 0030).
/// A theory is a conjunction of axioms that jointly hold on the RSet.
/// `R(THEORY_MARKER, t_N)` declares `t_N` as a theory;
/// `R(t_N, ax_i)` lists its members.
pub const THEORY_MARKER: &str = "__theory__";

/// Stable axiom id for universal reflexivity: every data identifier
/// has a self-loop `R(x, x)`. Not a template axiom (premise "x is an
/// identifier" has no edge), so it has a fixed string id. ADR 0030.
pub const AX_REFLEXIVITY: &str = "ax_reflexivity";

/// Stable axiom id for antisymmetry: no pair of distinct identifiers
/// (x, y) such that both `R(x, y)` and `R(y, x)` hold. Not a template
/// axiom (conclusion "x = y" is not an edge). ADR 0030.
pub const AX_ANTISYMMETRY: &str = "ax_antisymmetry";

/// Stable axiom id for totality: every pair of distinct data
/// identifiers (x, y) satisfies `R(x, y) ∨ R(y, x)`. Not a template
/// axiom (disjunctive conclusion). ADR 0039.
pub const AX_TOTALITY: &str = "ax_totality";

/// Reserved registry marker for axiom-variable identifiers (ADR 0032).
/// `R(AXIOMVAR_MARKER, ax_X_var_i)` declares `ax_X_var_i` as the i-th
/// variable slot in axiom `ax_X`'s intension.
pub const AXIOMVAR_MARKER: &str = "__axiomvar__";

/// Reserved registry marker for an axiom's premise-edge identifiers
/// (ADR 0032). `R(PREMISE_MARKER, ax_X_prem_j)` declares `ax_X_prem_j`
/// as a premise edge of `ax_X`. The source and target variables are
/// encoded by the chain `R(var_x, prem_j)` + `R(prem_j, var_y)`.
pub const PREMISE_MARKER: &str = "__premise__";

/// Reserved registry marker for an axiom's conclusion-edge identifier
/// (ADR 0032). `R(CONCLUSION_MARKER, ax_X_concl)` declares the
/// conclusion of `ax_X`. Same source/target chain convention as the
/// premise marker.
pub const CONCLUSION_MARKER: &str = "__conclusion__";

/// ADR 0049: the structural relation between two named theories
/// given their member-axiom sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TheoryRelationKind {
    /// Same member set.
    Equal,
    /// `a`'s members strictly contain `b`'s.
    Extends,
    /// `b`'s members strictly contain `a`'s.
    ExtendedBy,
    /// Empty intersection.
    Independent,
    /// Non-empty intersection, neither is a subset of the other.
    Parallel,
}

/// ADR 0049: summary of a theory's neighbors, grouped by relation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TheoryNeighborhood {
    pub equal: Vec<String>,
    pub extends: Vec<String>,
    pub extended_by: Vec<String>,
    pub independent: Vec<String>,
    pub parallel: Vec<String>,
}

/// Errors returned by `RSet::to_text` and `RSet::from_text`. ADR 0038.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersistenceError {
    /// Serialization format is line-based TSV; tab characters in an
    /// identifier would break parsing. Names the offending identifier.
    TabInIdentifier(String),
    /// Newlines would break line-based parsing. Names the identifier.
    NewlineInIdentifier(String),
    /// A line in the input does not split into exactly two tab-separated
    /// fields. Reports the 1-based line number.
    MalformedLine(usize),
}

/// Reserved registry marker for theory-extension relations (ADR 0034).
/// `R(EXTENDS_MARKER, ext_N)` declares `ext_N` as a named "T_sub
/// extends T_super" edge. The two sides are encoded by the chain
/// `R(T_sub, ext_N)` + `R(ext_N, T_super)` — same direction-as-role
/// convention used for axiom premise/conclusion edges.
pub const EXTENDS_MARKER: &str = "__extends__";

/// Reserved registry marker for theory-independence relations (ADR 0042).
/// Two theories are independent iff their member-axiom sets are
/// disjoint. Independence is symmetric; the chain stores one
/// canonical direction `R(T_a, ind_N) + R(ind_N, T_b)` where
/// `T_a < T_b` lexicographically.
pub const INDEPENDENT_MARKER: &str = "__independent__";

/// Reserved registry marker for theory-parallel relations (ADR 0046).
/// Two theories are *parallel* iff they overlap non-trivially but
/// neither extends the other — some shared members, plus members
/// unique to each side. Symmetric; same canonical-direction chain
/// as independence.
pub const PARALLEL_MARKER: &str = "__parallel__";

/// Reserved registry marker for "experience-with" facts about named
/// objects. ADR 0053 / Phase C0. The runtime emits the edge
/// `R(<pattern_id>, ESTABLISHED_MARKER)` once a pattern has been
/// alive long enough AND has contributed to at least one positive-delta
/// episode. Demoted on retract via the `retract_pattern` cascade. This
/// is the M1 / declarativized layer described in ADR 0052; it does
/// NOT replace `PATTERN_MARKER` ("kind-of") — it sits next to it as
/// a second meta-R class about the runtime's *use* of the object.
pub const ESTABLISHED_MARKER: &str = "__established__";

/// Reserved registry marker for axioms whose extension overlaps two
/// or more named theories. ADR 0053 / Phase C2. The runtime emits the
/// edge `R(<axiom_id>, SHARED_AXIOM_MARKER)` whenever
/// `theories_containing(axiom_id).len() >= 2`, and retracts it the
/// moment the count drops below 2 (handled inside `retract_theory`).
/// Encodes ADR 0052's M1 example "theory X persistently uses axiom Y"
/// as a structural fact independent of runtime experience.
pub const SHARED_AXIOM_MARKER: &str = "__shared_axiom__";

/// Reserved registry marker for promoted action-pair sequences
/// discovered by the runtime via H1.0 sequence-stats correlation.
/// ADR 0061 / Phase H1.1. Auto-promoted from `Memory::sequence_stats`
/// when a pair (A, B) crosses (count >= floor) AND
/// (mean post-EP-delta > floor). Stored as a chain:
///
/// ```text
/// R(ACTION_SEQ_MARKER, seq_N)
/// R(seq_N, seq_N_step_0)
/// R(seq_N, seq_N_step_1)
/// R(seq_N_step_0, "<ActionKind name A>")
/// R(seq_N_step_1, "<ActionKind name B>")
/// ```
///
/// The scheduler reads these chains at choose-time to bias frontier
/// item priority when the previous episode's action_kind matches a
/// stored prefix. This is the moment v2's runtime starts encoding
/// learned operational knowledge as first-class meta-R facts.
pub const ACTION_SEQ_MARKER: &str = "__action_seq__";

/// Marker for runtime-registered drives. ADR 0064 / Phase H2.1.0.
/// Each drive registered with `AutonomousRuntime` registers as
/// `R(DRIVE_MARKER, drive_<id>)`. This is the slice that
/// satisfies commitment 3 (types are meta-R instances) for the
/// drive catalogue: drive existence becomes a meta-R fact rather
/// than a compile-time-only construct.
///
/// Companion markers:
/// - `PENALTY_MARKER` — penalty drives also register under this
///   marker, generalizing the compile-time `Drive::is_penalty()`
///   method to a queryable meta-R fact.
///
/// H2.1.0 is registration-only — no behaviour change. The runtime
/// can observe its drive catalogue via
/// `rset.right_of(DRIVE_MARKER)` and penalty status via
/// `rset.right_of(PENALTY_MARKER)`. Existing code paths
/// (`combined_drive_signal`, `normalized_drive_signal`) still use
/// the compile-time `Drive::is_penalty()` method as a fast path.
/// H2.1.1 + later may rewire those queries to read from rset.
pub const DRIVE_MARKER: &str = "__drive__";

/// Marker for penalty drives. Penalty drives register under both
/// `DRIVE_MARKER` and this marker. ADR 0064 / Phase H2.1.0.
pub const PENALTY_MARKER: &str = "__penalty__";
