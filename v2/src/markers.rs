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

/// Marker for axiom shape families — sets of registered axioms that
/// share a structural sub-component. ADR 0068 / Phase Beta-1.
///
/// `R(SHAPE_FAMILY_MARKER, shape_N)` declares `shape_N` as a named
/// shape family. The family's members (the axioms sharing the
/// structure) are stored as `R(shape_N, ax_id)` for each member.
///
/// The first family kind discovered is **shared premise**: axioms
/// with byte-identical premise edge sets (after canonicalization)
/// but possibly different conclusions. On OQ#1, the 4 noise axioms
/// `ax_tpl_v3_p0-0_p1-2_c{0-1, 0-2, 1-0, 2-0}` all share premise
/// `[p0-0, p1-2]`; this marker lets the system *name* that shared
/// structure as a first-class meta-R object.
///
/// This is the first marker that lets the system extend its
/// structural vocabulary beyond template instances — a `shape_N`
/// is a structurally-derived ABSTRACTION over axioms, not an
/// instance of any pre-defined shape. Commitment 3 (types as
/// meta-R) gets a richer realization here than for previous
/// markers: the type itself is discovered, not declared.
pub const SHAPE_FAMILY_MARKER: &str = "__shape_family__";

/// Marker for **nested** axiom shape families — meta-families
/// whose members are themselves shape families (B.1) that share a
/// structural sub-component. ADR 0068 / Phase Beta-1.6 (Direction
/// B.6).
///
/// `R(META_SHAPE_FAMILY_MARKER, meta_X)` declares `meta_X` as a
/// nested family. Members are stored as `R(meta_X, shape_id)` for
/// each shape family that shares the relevant sub-component.
///
/// First nested family kind: **shared premise edge across premise
/// families**. When two or more `shape_premise_*` families have
/// premise edge sets that share an individual edge (e.g., both
/// contain `p1-2`), they form a meta-family
/// `meta_premise_p1-2`.
///
/// This is a second-order abstraction: Layer 1 is "axioms grouped
/// by structure" (Beta-1); Layer 2 is "structures grouped by
/// commonality" (Beta-1.6). Type instances at this layer are
/// derived from rset's *existing meta-R structure*, not from raw
/// axioms — recursive structural abstraction.
pub const META_SHAPE_FAMILY_MARKER: &str = "__meta_shape_family__";

/// Marker for **layer-3** abstraction: super-meta-families whose
/// members are themselves nested shape families (B.6) that share
/// a member shape family (B.1). ADR 0068 / Phase Beta-1.7 (B.7).
///
/// `R(SUPER_META_SHAPE_FAMILY_MARKER, super_X)` declares `super_X`
/// as a layer-3 family. Members are `R(super_X, meta_id)` for
/// each nested family that contains the relevant shape family.
///
/// On OQ#1, two nested families `meta_premise_p0-1` and
/// `meta_premise_p1-2` both contain `shape_premise_p0-1_p1-2`.
/// A B.7 super-meta-family captures this overlap structurally.
///
/// Layer hierarchy:
///   L0 — data (R)
///   L1 — axioms (templates)
///   L1.5 — theories (axiom conjunctions)
///   L2 — shape families (axioms grouped by structure) — Beta-1
///   L3 — nested families (families grouped by shared edge) — B.6
///   **L4 — super-meta-families (nested grouped by shared member) — B.7**
///
/// Each layer derives instances from the layer below. The marker
/// itself is declared (compile-time); the instance population is
/// data-derived.
pub const SUPER_META_SHAPE_FAMILY_MARKER: &str = "__super_meta_shape_family__";

/// ADR 0070 — Family kind tag ids. Each shape family (L2/L3/L4)
/// is tagged with a kind id describing the structural similarity
/// predicate that grouped its members. Currently inferred from id
/// prefix; ADR 0070 also writes the explicit edge
/// `R(<family_id>, <kind_id>)` during discovery so the kind
/// becomes queryable from rset rather than parsed from the id
/// string.
///
/// Kinds are themselves meta-R tokens (registered in
/// `collect_meta_ids`); they don't pollute data-id accounting.
pub const KIND_MARKER: &str = "__family_kind__";

/// L2 kind: members share the canonicalized premise edge set.
/// Beta-1 / ADR 0068 introduction.
pub const KIND_PREMISE_SHARED: &str = "kind_premise_shared";

/// L2 kind: members share the canonicalized conclusion edge.
/// B.3 introduction.
pub const KIND_CONCLUSION_SHARED: &str = "kind_conclusion_shared";

/// L3 kind: L2 families that share a single individual premise
/// edge. B.6 introduction.
pub const KIND_PREMISE_EDGE_SHARED: &str = "kind_premise_edge_shared";

/// L3 kind: L2 families that share a member axiom (cross-cutting
/// overlap between premise and conclusion families). B.8.1
/// introduction; promoted to lib via ADR 0070 Step 2.
pub const KIND_MEMBER_OVERLAP: &str = "kind_member_overlap";

/// L4 kind: L3 nested families that share a member L2 family.
/// B.7 introduction.
pub const KIND_MEMBER_L2_SHARED: &str = "kind_member_l2_shared";

/// ADR 0074 / Phase Emergence-1 — Concept marker.
///
/// `R(CONCEPT_MARKER, concept_X)` declares `concept_X` as a
/// **second-order abstraction** over shape families: a named
/// pattern of *which shape-families co-occur* across Signal-class
/// theories. This is the first marker that lets the system mint
/// a noun for "shape-shape co-occurrence" — distinct from a
/// theory (instance-bound axiom collection) and from a shape
/// family (one canonicalized shape, ADR 0068/0070).
///
/// Companion edges (all written by `register_concept`):
/// - `R(concept_X, HAS_CONSTITUENT_SHAPE) + R(HAS_CONSTITUENT_SHAPE, sf_Y)`
///   for each constituent shape family
/// - `R(concept_X, ATTESTED_IN_THEORY) + R(ATTESTED_IN_THEORY, t_Z)`
///   for each theory the concept fired in at mint time
/// - `R(concept_X, CROSS_PRECISION_AT_MINT) + R(CROSS_PRECISION_AT_MINT, "<value>")`
///   recording the cross-precision aggregate at mint time
///
/// Concepts are retractable; lifecycle states are computed at
/// query time (Live / Stale / Validated / Falsified).
pub const CONCEPT_MARKER: &str = "__concept__";

/// ADR 0074 — chain marker for the concept's constituent shape
/// families. Used as the middle node in the chain
/// `R(concept_X, HAS_CONSTITUENT_SHAPE) + R(HAS_CONSTITUENT_SHAPE, sf_Y)`.
/// Same direction-as-role convention as PREMISE_MARKER (ADR 0032).
pub const HAS_CONSTITUENT_SHAPE: &str = "__has_constituent_shape__";

/// ADR 0074 — chain marker for theories where the concept was
/// attested at mint time. `R(concept_X, ATTESTED_IN_THEORY) +
/// R(ATTESTED_IN_THEORY, t_Z)`.
pub const ATTESTED_IN_THEORY: &str = "__attested_in_theory__";

/// ADR 0074 — chain marker for the cross-precision value recorded
/// at concept-mint time. The value is encoded as a stringified f64
/// (`"0.9421"`). `R(concept_X, CROSS_PRECISION_AT_MINT) +
/// R(CROSS_PRECISION_AT_MINT, "<value>")`.
pub const CROSS_PRECISION_AT_MINT: &str = "__concept_xprec_mint__";
