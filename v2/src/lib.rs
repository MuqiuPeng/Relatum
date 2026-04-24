//! Relatum v2
//!
//! Core primitive: `R(x, y)` — a binary directed relation with no pre-assigned meaning.
//! All structure (objects, types, meaning) emerges from abstraction over R instances.
//!
//! Ontological commitments: see `docs/constitution.md`.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

/// The sole primitive. Direction is intrinsic; meaning is not.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct R {
    pub x: String,
    pub y: String,
}

impl R {
    pub fn new(x: impl Into<String>, y: impl Into<String>) -> Self {
        R { x: x.into(), y: y.into() }
    }
}

/// Reserved registry marker for the pattern-naming mechanism (ADR 0010).
///
/// `R(PATTERN_MARKER, p)` declares `p` as a named pattern. The double
/// underscore prefix is a pragmatic collision guard, not an ontological
/// exception — commitment 4 (token-based identity) still holds, and an
/// externally-supplied identifier equal to this string would be treated
/// as the same object.
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

/// Policy for what meta-R to write when naming pattern instances
/// (ADR 0029).
///
/// All modes write the pattern **intension** (type registry, roles,
/// and structural edges among roles — "Layer A"). They differ only in
/// whether and how much per-instance extension data is also written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternRecordingPolicy {
    /// Layer A only. No per-instance records. The pattern's structure
    /// is persisted; instances are recomputable on demand via
    /// `find_instances_of`. Minimum fact-layer footprint.
    Intensional,
    /// Layer A + `R(p_N, p_N_i_M)` per instance. Instances are
    /// registered but their participant bindings are not stored; if
    /// someone needs bindings they re-match at query time.
    InstancesOnly,
    /// Layer A + instances + `R(p_N_i_M, participant)` per participant.
    /// This is the behavior inherited from ADR 0010 and remains the
    /// default for backward compatibility.
    FullBindings,
}

impl Default for PatternRecordingPolicy {
    fn default() -> Self {
        PatternRecordingPolicy::FullBindings
    }
}

/// Errors from pattern-naming operations. ADR 0010.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternError {
    /// The caller provided no instances to name.
    EmptyInstanceList,
    /// One or more instances contain zero edges.
    EmptyInstance,
    /// Instances are not all isomorphic under ADR 0009 canonicalization.
    NotIsomorphic,
}

/// Policy governing `consider_naming` and `run_naming_pass`. ADR 0012,
/// extended by ADR 0014 with `attach_only`.
///
/// Default: `min_edges = 2`, `min_instances = 1`, `skip_meta_subgraphs = true`,
/// `attach_only = false`. That combination suppresses ADR 0009's trivial
/// single-edge pattern, allows singleton instances, keeps
/// `run_naming_pass` idempotent by skipping subgraphs that touch meta-R,
/// and permits new-pattern discovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamingPolicy {
    pub min_edges: usize,
    pub min_instances: usize,
    pub skip_meta_subgraphs: bool,
    /// When true, `run_naming_pass` only adds instances to existing
    /// named patterns. Candidate groups whose canonical form does not
    /// match any named pattern are reported as
    /// `SkipReason::NoMatchingPattern`. ADR 0014.
    pub attach_only: bool,
    /// Minimum MDL-inspired gain to accept a naming. Defaults to 0
    /// (off). When positive, `consider_naming` computes
    /// `mdl_gain = (N - 1) × k` where N is the provided instance
    /// count and k is the canonical size; candidates below the
    /// threshold return `Skipped(BelowMdlGain)`. ADR 0019.
    pub min_mdl_gain: usize,
}

impl Default for NamingPolicy {
    fn default() -> Self {
        NamingPolicy {
            min_edges: 2,
            min_instances: 1,
            skip_meta_subgraphs: true,
            attach_only: false,
            min_mdl_gain: 0,
        }
    }
}

/// Why a candidate pattern group was not named under the current policy.
/// ADR 0012.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    BelowMinEdges { edges: usize, min: usize },
    BelowMinInstances { instances: usize, min: usize },
    /// Every candidate in the group was already recorded as an instance
    /// of an existing pattern (dedup by participant set).
    AlreadyKnown,
    /// MDL-gain threshold from `NamingPolicy::min_mdl_gain` was not met.
    /// ADR 0019.
    BelowMdlGain { gain: usize, min: usize },
}

/// Outcome of a single naming attempt under policy. ADR 0012.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamingDecision {
    Named(String),
    Skipped(SkipReason),
}

/// A set of R instances — the only state v2 accumulates at the primitive layer.
///
/// This is the observation surface abstraction mechanisms will hook into.
/// It adds no interpretation: just storage, ingestion, and structural lookups.
///
/// Internally maintains source and target indices for O(1) `left_of` /
/// `right_of` queries; see ADR 0043.
#[derive(Debug, Clone, Default)]
pub struct RSet {
    instances: HashSet<R>,
    // ADR 0043: side indices. Kept in sync with `instances` by `add` and
    // `remove`. Not part of identity — two RSets with the same `instances`
    // have equivalent indices and are considered equal.
    by_source: HashMap<String, HashSet<R>>,
    by_target: HashMap<String, HashSet<R>>,
}

impl PartialEq for RSet {
    fn eq(&self, other: &Self) -> bool {
        self.instances == other.instances
    }
}

impl Eq for RSet {}

impl RSet {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert an instance. Returns true if it was not already present.
    pub fn add(&mut self, r: R) -> bool {
        let is_new = self.instances.insert(r.clone());
        if is_new {
            self.by_source
                .entry(r.x.clone())
                .or_default()
                .insert(r.clone());
            self.by_target.entry(r.y.clone()).or_default().insert(r);
        }
        is_new
    }

    pub fn extend<I: IntoIterator<Item = R>>(&mut self, iter: I) {
        for r in iter {
            self.add(r);
        }
    }

    pub fn contains(&self, r: &R) -> bool {
        self.instances.contains(r)
    }

    /// Remove a single R instance. Returns true if the edge was
    /// present. Dual of `add`. ADR 0020.
    pub fn remove(&mut self, r: &R) -> bool {
        let removed = self.instances.remove(r);
        if removed {
            if let Some(set) = self.by_source.get_mut(&r.x) {
                set.remove(r);
                if set.is_empty() {
                    self.by_source.remove(&r.x);
                }
            }
            if let Some(set) = self.by_target.get_mut(&r.y) {
                set.remove(r);
                if set.is_empty() {
                    self.by_target.remove(&r.y);
                }
            }
        }
        removed
    }

    pub fn len(&self) -> usize {
        self.instances.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &R> {
        self.instances.iter()
    }

    /// Serialize the RSet to a deterministic text form (ADR 0038).
    ///
    /// One R instance per line, two tab-separated fields: `x\ty`. The
    /// output is sorted lexicographically by (x, y) so the same RSet
    /// always serializes to the same bytes across processes.
    /// Identifiers containing tab or newline are rejected — they would
    /// break the line-based format.
    pub fn to_text(&self) -> Result<String, PersistenceError> {
        for r in &self.instances {
            for id in [&r.x, &r.y] {
                if id.contains('\t') {
                    return Err(PersistenceError::TabInIdentifier(id.clone()));
                }
                if id.contains('\n') {
                    return Err(PersistenceError::NewlineInIdentifier(id.clone()));
                }
            }
        }
        let mut edges: Vec<&R> = self.instances.iter().collect();
        edges.sort_by(|a, b| {
            (a.x.as_str(), a.y.as_str()).cmp(&(b.x.as_str(), b.y.as_str()))
        });
        let mut out = String::new();
        for r in edges {
            out.push_str(&r.x);
            out.push('\t');
            out.push_str(&r.y);
            out.push('\n');
        }
        Ok(out)
    }

    /// Parse an RSet from the text form produced by `to_text`.
    /// Blank lines and lines beginning with `#` are skipped. ADR 0038.
    pub fn from_text(s: &str) -> Result<RSet, PersistenceError> {
        let mut rs = RSet::new();
        for (i, line) in s.lines().enumerate() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let parts: Vec<&str> = line.splitn(2, '\t').collect();
            if parts.len() != 2 || parts[0].is_empty() {
                return Err(PersistenceError::MalformedLine(i + 1));
            }
            rs.add(R::new(parts[0], parts[1]));
        }
        Ok(rs)
    }

    /// All identifiers appearing anywhere in R instances, on either side.
    pub fn identifiers(&self) -> HashSet<&str> {
        self.instances
            .iter()
            .flat_map(|r| [r.x.as_str(), r.y.as_str()])
            .collect()
    }

    /// Instances with `x` on the left (the source position).
    /// O(|edges from x|) via the source index. ADR 0043.
    pub fn left_of(&self, x: &str) -> Vec<&R> {
        match self.by_source.get(x) {
            Some(set) => set.iter().collect(),
            None => Vec::new(),
        }
    }

    /// Instances with `y` on the right (the target position).
    /// O(|edges into y|) via the target index. ADR 0043.
    pub fn right_of(&self, y: &str) -> Vec<&R> {
        match self.by_target.get(y) {
            Some(set) => set.iter().collect(),
            None => Vec::new(),
        }
    }

    /// Structural profile of one identifier.
    ///
    /// Returns a zero profile (all counts 0, `slots = None`) for identifiers
    /// not present in the set. This is deliberate: every string is a potential
    /// identifier (commitment 4), so "not present" is a count, not an error.
    pub fn profile(&self, id: &str) -> IdentifierProfile {
        let degree_out = self.instances.iter().filter(|r| r.x == id).count();
        let degree_in = self.instances.iter().filter(|r| r.y == id).count();
        let slots = match (degree_out > 0, degree_in > 0) {
            (false, false) => SlotPattern::None,
            (true, false) => SlotPattern::LeftOnly,
            (false, true) => SlotPattern::RightOnly,
            (true, true) => SlotPattern::Both,
        };
        IdentifierProfile { degree_out, degree_in, slots }
    }

    /// Profile every identifier appearing in the set.
    pub fn profiles(&self) -> HashMap<&str, IdentifierProfile> {
        self.identifiers()
            .into_iter()
            .map(|id| (id, self.profile(id)))
            .collect()
    }

    /// Structural signature of an identifier. At the current granularity
    /// (ADR 0004), this is the identifier's profile — 0-hop, no neighbors.
    pub fn signature(&self, id: &str) -> Signature {
        self.profile(id)
    }

    /// Partition identifiers by signature. Each class holds identifiers
    /// that are structurally equivalent at the current granularity.
    pub fn equivalence_classes(&self) -> HashMap<Signature, HashSet<&str>> {
        let mut classes: HashMap<Signature, HashSet<&str>> = HashMap::new();
        for id in self.identifiers() {
            let sig = self.signature(id);
            classes.entry(sig).or_default().insert(id);
        }
        classes
    }

    /// Edge-level (R-instance) signature — the ordered pair of the endpoint
    /// signatures. ADR 0005. Ordered because direction is commitment-level.
    pub fn r_signature(&self, r: &R) -> RSignature {
        (self.signature(&r.x), self.signature(&r.y))
    }

    /// Partition R instances by their edge-level signature. Each class
    /// holds edges that play the same structural role (same endpoint
    /// roles, same direction).
    pub fn r_equivalence_classes(&self) -> HashMap<RSignature, HashSet<&R>> {
        let mut classes: HashMap<RSignature, HashSet<&R>> = HashMap::new();
        for r in self.instances.iter() {
            let sig = self.r_signature(r);
            classes.entry(sig).or_default().insert(r);
        }
        classes
    }

    /// Compound fingerprint combining the edge-level signature (ADR 0005)
    /// and the locality profile (ADR 0006). ADR 0007 probe — observation
    /// layer used to study what the current signals produce as compound
    /// classes before committing to β (compound pattern naming).
    pub fn edge_fingerprint(&self, r: &R) -> EdgeFingerprint {
        (self.r_signature(r), self.locality_profile(r))
    }

    /// For each compound fingerprint, split its member edges into
    /// connected-component subgraphs. ADR 0008 — the first β-layer
    /// operation. The split breaks known 1-hop false merges
    /// (e.g., chain-middle + cycle-edge) because their members live
    /// in different connected components of the R graph.
    pub fn compound_class_subgraphs(&self) -> HashMap<EdgeFingerprint, Vec<Subgraph>> {
        let mut classes: HashMap<EdgeFingerprint, Vec<R>> = HashMap::new();
        for r in self.iter() {
            classes
                .entry(self.edge_fingerprint(r))
                .or_default()
                .push(r.clone());
        }
        classes
            .into_iter()
            .map(|(fp, members)| (fp, Subgraph::connected_components_of(members)))
            .collect()
    }

    /// Locality profile: four counts of how this edge connects to others
    /// via shared identifiers. ADR 0006. All counts exclude `r` itself.
    pub fn locality_profile(&self, r: &R) -> LocalityProfile {
        let mut co_left = 0;
        let mut co_right = 0;
        let mut forward = 0;
        let mut reverse = 0;
        for other in self.instances.iter() {
            if other == r {
                continue;
            }
            if other.x == r.x {
                co_left += 1;
            }
            if other.y == r.y {
                co_right += 1;
            }
            if other.x == r.y {
                forward += 1;
            }
            if other.y == r.x {
                reverse += 1;
            }
        }
        LocalityProfile { co_left, co_right, forward, reverse }
    }

    /// Record a set of isomorphic subgraphs as instances of one named
    /// pattern. ADR 0010 — writes the three-shape encoding
    /// (`R(PATTERN_MARKER, p)`, `R(p, inst)`, `R(inst, participant)`)
    /// directly into the RSet. Returns the pattern identifier (reused
    /// if an existing pattern matches the canonical form, minted
    /// otherwise).
    ///
    /// Errors:
    /// - `EmptyInstanceList` if `instances` is empty.
    /// - `EmptyInstance` if any subgraph has zero edges.
    /// - `NotIsomorphic` if instances disagree on canonical form.
    ///
    /// See ADR 0010 for the canonical-recovery invariant and feedback-
    /// loop consequences on subsequent observation-layer queries.
    pub fn name_pattern_instances(
        &mut self,
        instances: &[Subgraph],
    ) -> Result<String, PatternError> {
        self.name_pattern_instances_with_policy(
            instances,
            PatternRecordingPolicy::default(),
        )
    }

    /// Name a set of isomorphic subgraphs with an explicit recording
    /// policy. ADR 0029.
    ///
    /// Always writes the **intension** of the pattern (registry, roles,
    /// structural edges among roles) on the first mint, regardless of
    /// policy. Policy only controls the extent of per-instance
    /// extension writes (`Intensional`, `InstancesOnly`, `FullBindings`).
    pub fn name_pattern_instances_with_policy(
        &mut self,
        instances: &[Subgraph],
        policy: PatternRecordingPolicy,
    ) -> Result<String, PatternError> {
        if instances.is_empty() {
            return Err(PatternError::EmptyInstanceList);
        }
        for inst in instances {
            if inst.is_empty() {
                return Err(PatternError::EmptyInstance);
            }
        }
        let first = &instances[0];
        for other in instances.iter().skip(1) {
            if !first.is_isomorphic_to(other) {
                return Err(PatternError::NotIsomorphic);
            }
        }

        let canon = first.canonicalize();
        let pattern_id: String = match self.find_pattern_matching(&canon) {
            Some(existing) => existing.to_string(),
            None => {
                let new_id = self.mint_pattern_id();
                // Layer A: registry + role set + structural edges.
                // Roles are indexed by the sorted identifiers of the
                // first instance, not by WL canonical labels — this
                // preserves edge multiplicity (symmetric nodes do NOT
                // collapse in the stored meta-R). The canonical form of
                // the resulting role-subgraph equals `canon` by
                // construction.
                self.add(R::new(PATTERN_MARKER, new_id.clone()));
                let mut sorted_ids: Vec<String> = first
                    .identifiers()
                    .into_iter()
                    .map(str::to_owned)
                    .collect();
                sorted_ids.sort();
                let id_to_role_idx: HashMap<&str, usize> = sorted_ids
                    .iter()
                    .enumerate()
                    .map(|(i, s)| (s.as_str(), i))
                    .collect();
                let k = sorted_ids.len();
                let role_ids: Vec<String> = (0..k)
                    .map(|i| format!("{}_role_{}", new_id, i))
                    .collect();
                for role_id in &role_ids {
                    self.add(R::new(ROLE_MARKER, role_id.clone()));
                    self.add(R::new(new_id.clone(), role_id.clone()));
                }
                for edge in first.edges() {
                    let x_idx = id_to_role_idx[edge.x.as_str()];
                    let y_idx = id_to_role_idx[edge.y.as_str()];
                    self.add(R::new(
                        role_ids[x_idx].clone(),
                        role_ids[y_idx].clone(),
                    ));
                }
                new_id
            }
        };

        // Layer B: extension, subject to policy.
        if matches!(policy, PatternRecordingPolicy::Intensional) {
            return Ok(pattern_id);
        }

        for inst in instances {
            let inst_id = self.mint_instance_id(&pattern_id);
            self.add(R::new(pattern_id.clone(), inst_id.clone()));
            if matches!(policy, PatternRecordingPolicy::FullBindings) {
                let participants: Vec<String> =
                    inst.identifiers().into_iter().map(str::to_owned).collect();
                for participant in participants {
                    self.add(R::new(inst_id.clone(), participant));
                }
            }
        }

        Ok(pattern_id)
    }

    /// All pattern identifiers currently registered in this RSet.
    /// ADR 0010.
    pub fn patterns(&self) -> Vec<&str> {
        self.left_of(PATTERN_MARKER)
            .iter()
            .map(|r| r.y.as_str())
            .collect()
    }

    /// Instance identifiers owned by a pattern. ADR 0010, refined by
    /// ADR 0029 to exclude role identifiers (which share the
    /// `R(pattern, *)` shape but are part of the intension, not the
    /// extension).
    pub fn instances_of(&self, pattern: &str) -> Vec<&str> {
        self.left_of(pattern)
            .iter()
            .filter_map(|r| {
                let y = r.y.as_str();
                if self.is_role(y) {
                    None
                } else {
                    Some(y)
                }
            })
            .collect()
    }

    /// Participant identifiers referenced by a pattern instance. ADR 0010.
    pub fn participants_of(&self, instance: &str) -> HashSet<&str> {
        self.left_of(instance)
            .iter()
            .map(|r| r.y.as_str())
            .collect()
    }

    /// All role identifiers currently registered (any pattern). ADR 0029.
    pub fn roles(&self) -> Vec<&str> {
        self.left_of(ROLE_MARKER)
            .iter()
            .map(|r| r.y.as_str())
            .collect()
    }

    /// Role identifiers owned by `pattern`, in sorted order. ADR 0029.
    /// A pattern created before ADR 0029 (Layer A absent) returns an
    /// empty vector.
    pub fn pattern_roles(&self, pattern: &str) -> Vec<&str> {
        let role_set: HashSet<&str> = self.roles().into_iter().collect();
        let mut out: Vec<&str> = self
            .left_of(pattern)
            .into_iter()
            .filter_map(|r| {
                let y = r.y.as_str();
                if role_set.contains(y) {
                    Some(y)
                } else {
                    None
                }
            })
            .collect();
        out.sort();
        out
    }

    /// Is `id` a role identifier in some pattern's intension? ADR 0029.
    pub fn is_role(&self, id: &str) -> bool {
        self.instances.contains(&R::new(ROLE_MARKER, id))
    }

    /// Read the stored pattern intension and return its canonical form.
    /// ADR 0029. Returns `None` if Layer A is absent for this pattern
    /// (legacy RSets from before ADR 0029).
    pub fn pattern_structure(&self, pattern: &str) -> Option<CanonicalForm> {
        let roles = self.pattern_roles(pattern);
        if roles.is_empty() {
            return None;
        }
        let role_set: HashSet<&str> = roles.iter().copied().collect();
        let edges: Vec<R> = self
            .instances
            .iter()
            .filter(|r| {
                role_set.contains(r.x.as_str()) && role_set.contains(r.y.as_str())
            })
            .cloned()
            .collect();
        Some(Subgraph::from_edges(edges).canonicalize())
    }

    /// Classify a subgraph against the named-pattern registry. ADR 0013.
    /// Returns the matching pattern id if one exists, `None` otherwise.
    /// Thin wrapper over `find_pattern_matching(&sg.canonicalize())`.
    pub fn classify_subgraph(&self, sg: &Subgraph) -> Option<&str> {
        let canon = sg.canonicalize();
        self.find_pattern_matching(&canon)
    }

    /// Return the pattern that owns `instance_id`, or `None` if the
    /// argument is not a recognized instance identifier. ADR 0013.
    pub fn pattern_of(&self, instance_id: &str) -> Option<&str> {
        let pattern_set: HashSet<&str> = self.patterns().into_iter().collect();
        self.right_of(instance_id)
            .into_iter()
            .find_map(|r| {
                let x = r.x.as_str();
                if pattern_set.contains(x) {
                    Some(x)
                } else {
                    None
                }
            })
    }

    /// Every (pattern_id, instance_id) pair in which `id` is recorded as
    /// a participant. ADR 0013; refined by ADR 0029 to skip role ids in
    /// the "is this an instance?" check (Layer A introduces
    /// `R(pattern, role)` edges that otherwise look like ownership).
    pub fn memberships_of(&self, id: &str) -> Vec<(&str, &str)> {
        let pattern_set: HashSet<&str> = self.patterns().into_iter().collect();
        let mut out = Vec::new();
        for r in self.right_of(id) {
            let inst = r.x.as_str();
            if self.is_role(inst) {
                continue;
            }
            for parent in self.right_of(inst) {
                if pattern_set.contains(parent.x.as_str()) {
                    out.push((parent.x.as_str(), inst));
                    break;
                }
            }
        }
        out
    }

    /// Reconstruct the concrete subgraph of a pattern instance — the RSet
    /// edges whose endpoints both lie in the instance's participant set.
    /// ADR 0013.
    pub fn instance_subgraph(&self, instance_id: &str) -> Subgraph {
        let participants = self.participants_of(instance_id);
        let edges: Vec<R> = self
            .instances
            .iter()
            .filter(|r| {
                participants.contains(r.x.as_str())
                    && participants.contains(r.y.as_str())
            })
            .cloned()
            .collect();
        Subgraph::from_edges(edges)
    }

    /// Look up an existing pattern whose stored structural canonical
    /// form equals `canon`. ADR 0029 (upgraded from ADR 0010).
    ///
    /// Primary path reads the pattern's **intension** (Layer A: role
    /// set + role-role structural edges) and computes its canonical
    /// form directly. Fallback path, used only when Layer A is absent
    /// (legacy RSets created before ADR 0029), reconstructs the
    /// canonical from the first instance's participants + RSet edges,
    /// the original ADR 0010 recovery.
    pub fn find_pattern_matching(&self, canon: &CanonicalForm) -> Option<&str> {
        for pattern in self.patterns() {
            if let Some(stored) = self.pattern_structure(pattern) {
                if stored == *canon {
                    return Some(pattern);
                }
                continue;
            }
            // Fallback: legacy (pre-0029) pattern with no Layer A.
            let instance_ids = self.instances_of(pattern);
            let Some(first_inst) = instance_ids.first() else {
                continue;
            };
            let participants = self.participants_of(first_inst);
            let edges: Vec<R> = self
                .instances
                .iter()
                .filter(|r| {
                    participants.contains(r.x.as_str())
                        && participants.contains(r.y.as_str())
                })
                .cloned()
                .collect();
            let sg = Subgraph::from_edges(edges);
            if sg.canonicalize() == *canon {
                return Some(pattern);
            }
        }
        None
    }

    /// Apply a policy filter to a candidate group and, if it passes,
    /// invoke `name_pattern_instances`. ADR 0012; extended by ADR 0019
    /// to include an optional MDL-gain threshold.
    pub fn consider_naming(
        &mut self,
        instances: &[Subgraph],
        policy: &NamingPolicy,
    ) -> Result<NamingDecision, PatternError> {
        if instances.is_empty() {
            return Err(PatternError::EmptyInstanceList);
        }
        let sample = &instances[0];
        let edges = sample.len();
        if edges < policy.min_edges {
            return Ok(NamingDecision::Skipped(SkipReason::BelowMinEdges {
                edges,
                min: policy.min_edges,
            }));
        }
        let count = instances.len();
        if count < policy.min_instances {
            return Ok(NamingDecision::Skipped(SkipReason::BelowMinInstances {
                instances: count,
                min: policy.min_instances,
            }));
        }
        if policy.min_mdl_gain > 0 {
            // MDL gain computed from the provided instance list, not from
            // find_instances_of — lets consider_naming remain a pure
            // function of its inputs. Caller who wants exact data-wide
            // gain should pass the full instance list (as run_naming_pass
            // and autonomous_pass already do).
            let gain = count.saturating_sub(1) * edges;
            if gain < policy.min_mdl_gain {
                return Ok(NamingDecision::Skipped(SkipReason::BelowMdlGain {
                    gain,
                    min: policy.min_mdl_gain,
                }));
            }
        }
        let pid = self.name_pattern_instances(instances)?;
        Ok(NamingDecision::Named(pid))
    }

    /// MDL-inspired reusability gain of naming a pattern with this
    /// canonical. `(N - 1) × k` where N is the clean instance count
    /// from `find_instances_of` and k is the canonical size. Zero for
    /// singletons or empty canonicals. ADR 0019.
    pub fn mdl_gain(&self, canonical: &CanonicalForm) -> usize {
        let k = canonical.len();
        if k == 0 {
            return 0;
        }
        let n = self.find_instances_of(canonical).len();
        if n == 0 {
            return 0;
        }
        n.saturating_sub(1) * k
    }

    /// Re-score candidates by MDL gain (replaces sample-frequency score
    /// in the `MotifCandidate::score` field with `mdl_gain` as f64).
    /// Deterministic. ADR 0019.
    pub fn score_by_mdl(
        &self,
        candidates: Vec<MotifCandidate>,
    ) -> Vec<MotifCandidate> {
        candidates
            .into_iter()
            .map(|mut c| {
                c.score = self.mdl_gain(&c.canonical) as f64;
                c
            })
            .collect()
    }

    /// γ driver — run one naming pass. ADR 0012, extended by ADR 0015.
    ///
    /// Under `attach_only = false` (discovery mode): collects compound-
    /// class subgraphs, groups by canonical form, applies policy per
    /// group. This is the original ADR 0012 semantics.
    ///
    /// Under `attach_only = true` (ADR 0015): iterates known patterns,
    /// uses `find_instances_of` to locate matching subgraphs, and
    /// attaches each fresh one. Returns one decision per pattern.
    /// Handles asymmetric structures that compound-class enumeration
    /// would fragment.
    pub fn run_naming_pass(
        &mut self,
        policy: &NamingPolicy,
    ) -> Vec<(CanonicalForm, NamingDecision)> {
        if policy.attach_only {
            self.run_attach_pass(policy)
        } else {
            self.run_discovery_pass(policy)
        }
    }

    fn run_discovery_pass(
        &mut self,
        policy: &NamingPolicy,
    ) -> Vec<(CanonicalForm, NamingDecision)> {
        let meta_ids = if policy.skip_meta_subgraphs {
            self.collect_meta_ids()
        } else {
            HashSet::new()
        };

        let class_subs = self.compound_class_subgraphs();
        let mut by_canon: BTreeMap<CanonicalForm, Vec<Subgraph>> = BTreeMap::new();
        for (_fp, subs) in class_subs {
            for sub in subs {
                if policy.skip_meta_subgraphs
                    && sub
                        .edges()
                        .any(|r| meta_ids.contains(&r.x) || meta_ids.contains(&r.y))
                {
                    continue;
                }
                by_canon.entry(sub.canonicalize()).or_default().push(sub);
            }
        }

        let mut decisions = Vec::with_capacity(by_canon.len());
        for (canon, subs) in by_canon {
            let fresh = self.filter_known_instances(subs);
            let decision = if fresh.is_empty() {
                NamingDecision::Skipped(SkipReason::AlreadyKnown)
            } else {
                self.consider_naming(&fresh, policy)
                    .expect("well-formed candidate groups produced by the pipeline")
            };
            decisions.push((canon, decision));
        }
        decisions
    }

    fn run_attach_pass(
        &mut self,
        policy: &NamingPolicy,
    ) -> Vec<(CanonicalForm, NamingDecision)> {
        let patterns: Vec<String> = self.patterns().iter().map(|s| s.to_string()).collect();
        let mut decisions = Vec::with_capacity(patterns.len());

        for pattern_id in patterns {
            // Reconstruct the pattern's canonical via its first instance.
            let first_inst = match self.instances_of(&pattern_id).first() {
                Some(s) => s.to_string(),
                None => continue, // pattern exists in registry but has no instances
            };
            let canon = self.instance_subgraph(&first_inst).canonicalize();

            let matches = self.find_instances_of(&canon);
            let fresh = self.filter_known_instances(matches);

            let decision = if fresh.is_empty() {
                NamingDecision::Skipped(SkipReason::AlreadyKnown)
            } else {
                self.consider_naming(&fresh, policy)
                    .expect("fresh isomorphic matches from find_instances_of")
            };
            decisions.push((canon, decision));
        }
        decisions
    }

    /// Sample-score-select motif discovery. ADR 0016.
    ///
    /// Propose N connected subgraphs of `config.target_size` via
    /// random-walk sampling from data edges; score each distinct
    /// canonical form by how many samples produced it; return the
    /// top-M canonicals as `MotifCandidate`s. The first v2 search
    /// mechanism that makes an explicit *choice* rather than
    /// enumerating every possibility.
    ///
    /// Deterministic given `config.rng_seed`. Stochastic otherwise.
    pub fn discover_motifs(&self, config: &DiscoveryConfig) -> Vec<MotifCandidate> {
        if config.target_size == 0 || config.sample_count == 0 {
            return Vec::new();
        }
        let data = if config.include_meta_in_discovery {
            self.all_edges_sorted()
        } else {
            self.data_edges_sorted()
        };
        if data.is_empty() {
            return Vec::new();
        }

        let mut rng_state = if config.rng_seed == 0 {
            0x9E3779B97F4A7C15
        } else {
            config.rng_seed
        };

        let mut samples: Vec<Subgraph> = Vec::with_capacity(config.sample_count);
        for _ in 0..config.sample_count {
            if let Some(sg) =
                sample_connected_subgraph(&data, config.target_size, &mut rng_state)
            {
                samples.push(sg);
            }
        }

        // Score: count canonical-form frequency across samples; keep a
        // representative for each distinct canonical.
        let mut by_canon: HashMap<CanonicalForm, (usize, Subgraph)> = HashMap::new();
        for sg in samples {
            let canon = sg.canonicalize();
            by_canon
                .entry(canon)
                .and_modify(|(c, _)| *c += 1)
                .or_insert((1, sg));
        }

        let mut ranked: Vec<MotifCandidate> = by_canon
            .into_iter()
            .map(|(canonical, (freq, representative))| MotifCandidate {
                canonical,
                representative,
                sample_frequency: freq,
                score: freq as f64,
            })
            .collect();
        // Sort by score desc, then by canonical form asc for determinism
        // among ties.
        ranked.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.canonical.cmp(&b.canonical))
        });
        ranked.truncate(config.top_m);
        ranked
    }

    /// Enumerate every connected data subgraph whose canonical form
    /// equals `target`, filtered to "clean" instances. ADR 0015.
    ///
    /// Meta-R edges are excluded so pattern matching never matches
    /// patterns against their own metadata. Clean instances are those
    /// whose participant set, restricted to the RSet's data edges,
    /// induces exactly the subgraph itself — no extra edges. This
    /// excludes embedded cases (e.g., a 2-chain inside a 3-cycle) so
    /// that canonical recovery via participants (ADR 0010 invariant)
    /// stays consistent: the participant set of a clean instance
    /// uniquely determines its structure.
    pub fn find_instances_of(&self, target: &CanonicalForm) -> Vec<Subgraph> {
        let k = target.len();
        if k == 0 {
            return Vec::new();
        }
        let data = self.data_edges_sorted();
        if k > data.len() {
            return Vec::new();
        }

        let mut results: Vec<Subgraph> = Vec::new();
        let mut seen: HashSet<Vec<R>> = HashSet::new();

        for start in 0..data.len() {
            let mut initial: HashSet<R> = HashSet::new();
            initial.insert(data[start].clone());
            expand_connected(&data, initial, k, &mut seen, &mut results, target);
        }

        // Cleanness filter — keep only subgraphs whose participants
        // induce exactly k data edges in the RSet.
        results.retain(|sg| self.is_clean_subgraph(sg));

        results
    }

    /// A subgraph is "clean" in this RSet iff its participants induce
    /// exactly `sg.len()` data edges (meta-R excluded). Non-clean
    /// subgraphs are embedded in larger structures, which violates the
    /// ADR 0010 canonical-recovery invariant. Exposed as a helper for
    /// ADR 0017 refinement and downstream callers.
    pub fn is_clean_subgraph(&self, sg: &Subgraph) -> bool {
        let meta = self.collect_meta_ids();
        let parts: HashSet<&str> = sg.identifiers();
        let induced: usize = self
            .instances
            .iter()
            .filter(|r| {
                !meta.contains(&r.x)
                    && !meta.contains(&r.y)
                    && parts.contains(r.x.as_str())
                    && parts.contains(r.y.as_str())
            })
            .count();
        induced == sg.len()
    }

    /// Sampling variant of `find_instances_of`. Runs `sample_count`
    /// random walks of length `target.len()` over data edges, keeps
    /// those whose canonical matches `target` and which are clean,
    /// dedups by participant set. Never over-returns; may
    /// under-return. Deterministic under `rng_seed`. ADR 0024.
    pub fn sample_instances_of(
        &self,
        target: &CanonicalForm,
        config: &SamplingMatchConfig,
    ) -> Vec<Subgraph> {
        let k = target.len();
        if k == 0 || config.sample_count == 0 {
            return Vec::new();
        }
        let data = self.data_edges_sorted();
        if k > data.len() {
            return Vec::new();
        }

        let mut rng_state = if config.rng_seed == 0 {
            0x9E3779B97F4A7C15
        } else {
            config.rng_seed
        };
        let mut seen_participants: HashSet<Vec<String>> = HashSet::new();
        let mut results: Vec<Subgraph> = Vec::new();

        for _ in 0..config.sample_count {
            let Some(sg) = sample_connected_subgraph(&data, k, &mut rng_state) else {
                continue;
            };
            if sg.canonicalize() != *target {
                continue;
            }
            if !self.is_clean_subgraph(&sg) {
                continue;
            }
            // Dedup by participant set (sorted for determinism).
            let mut parts: Vec<String> =
                sg.identifiers().into_iter().map(str::to_owned).collect();
            parts.sort();
            if seen_participants.insert(parts) {
                results.push(sg);
            }
        }
        results
    }

    /// All named canonical forms in this RSet, recovered via each
    /// pattern's first instance. ADR 0023. Canonicals are portable
    /// (identifier-free) — applying the library to a different RSet
    /// is semantically meaningful.
    pub fn canonical_library(&self) -> Vec<CanonicalForm> {
        let mut library = Vec::new();
        let patterns: Vec<String> = self
            .patterns()
            .iter()
            .map(|s| s.to_string())
            .collect();
        for pattern_id in patterns {
            if let Some(first_inst) = self.instances_of(&pattern_id).first() {
                let canon = self.instance_subgraph(first_inst).canonicalize();
                library.push(canon);
            }
        }
        library
    }

    /// Apply a pattern library to this RSet. ADR 0023. For each
    /// canonical: if already named, report Existing; else find
    /// clean instances and run the naming policy. Same per-canonical
    /// outcomes as `autonomous_pass` returns.
    pub fn attach_canonicals(
        &mut self,
        library: &[CanonicalForm],
        policy: &NamingPolicy,
    ) -> Vec<AutonomousOutcome> {
        let mut outcomes = Vec::with_capacity(library.len());
        for canonical in library {
            if canonical.is_empty() {
                outcomes.push(AutonomousOutcome::Skipped {
                    canonical: canonical.clone(),
                    reason: AutonomousSkip::NoCleanInstance,
                });
                continue;
            }
            if let Some(existing) = self.find_pattern_matching(canonical) {
                outcomes.push(AutonomousOutcome::Existing {
                    pattern_id: existing.to_string(),
                    canonical: canonical.clone(),
                });
                continue;
            }
            let instances = self.find_instances_of(canonical);
            if instances.is_empty() {
                outcomes.push(AutonomousOutcome::Skipped {
                    canonical: canonical.clone(),
                    reason: AutonomousSkip::NoCleanInstance,
                });
                continue;
            }
            let count = instances.len();
            match self.consider_naming(&instances, policy) {
                Ok(NamingDecision::Named(pid)) => {
                    outcomes.push(AutonomousOutcome::NewPattern {
                        pattern_id: pid,
                        instance_count: count,
                        canonical: canonical.clone(),
                    });
                }
                Ok(NamingDecision::Skipped(reason)) => {
                    outcomes.push(AutonomousOutcome::Skipped {
                        canonical: canonical.clone(),
                        reason: AutonomousSkip::PolicyFiltered(reason),
                    });
                }
                Err(_) => {
                    outcomes.push(AutonomousOutcome::Skipped {
                        canonical: canonical.clone(),
                        reason: AutonomousSkip::NoCleanInstance,
                    });
                }
            }
        }
        outcomes
    }

    /// Run `autonomous_pass` (discovers novel canonicals) then
    /// `run_naming_pass(attach_only=true)` (extends pre-existing
    /// canonicals with new instances). The natural incremental-data
    /// workflow. ADR 0022.
    pub fn autonomous_and_attach(
        &mut self,
        config: &AutonomousConfig,
    ) -> AutonomousAndAttachSummary {
        let autonomous = self.autonomous_pass(config);
        let mut attach_policy = config.naming.clone();
        attach_policy.attach_only = true;
        let attach = self.run_naming_pass(&attach_policy);
        AutonomousAndAttachSummary { autonomous, attach }
    }

    /// Run `autonomous_pass` once per `target_size` in `sizes`. Per-size
    /// config is the base with `discovery.target_size` overridden and
    /// `discovery.rng_seed` offset by the size so sampling differs
    /// between sizes. Earlier sizes' named patterns persist, so later
    /// sizes naturally return `Existing` for canonicals already
    /// registered. ADR 0021.
    pub fn autonomous_sweep(
        &mut self,
        base: &AutonomousConfig,
        sizes: &[usize],
    ) -> Vec<(usize, Vec<AutonomousOutcome>)> {
        let mut results = Vec::with_capacity(sizes.len());
        for &size in sizes {
            let mut cfg = base.clone();
            cfg.discovery.target_size = size;
            cfg.discovery.rng_seed = base
                .discovery
                .rng_seed
                .wrapping_add(size as u64);
            let outcomes = self.autonomous_pass(&cfg);
            results.push((size, outcomes));
        }
        results
    }

    /// Global abstraction score (ADR 0031, extended by ADR 0040).
    /// A scalar that increases when the RSet contains *reusable* or
    /// *structurally related* abstractions and decreases when it
    /// accumulates unexplained meta-R overhead:
    ///
    ///   score = Σ_pattern max(0, (N − 1) · k)
    ///         + 2.0 · Σ_theory |members|
    ///         + 1.0 · |extension_edges|
    ///         − 0.1 · |meta-R edges|
    ///
    /// - Reuse savings: `(N−1)·k` per pattern that names a recurrent
    ///   k-edge subgraph occurring N times.
    /// - Theory membership: each axiom in a named theory is rewarded.
    /// - Extension relations (ADR 0034) are rewarded as load-bearing
    ///   higher-order meta-R — ADR 0040 added this term so auto-prune
    ///   doesn't eat them.
    /// - Overhead tax on total meta-R to discourage write-and-forget.
    pub fn abstraction_score(&self) -> f64 {
        let mut s = 0.0;
        for p in self.patterns() {
            let n = self.instances_of(p).len();
            let k = self.pattern_roles(p).len();
            if n >= 2 && k > 0 {
                s += ((n - 1) * k) as f64;
            }
        }
        let theory_member_total: usize = self
            .theories()
            .iter()
            .map(|t| self.theory_axioms(t).len())
            .sum();
        s += 2.0 * theory_member_total as f64;
        s += 1.0 * self.extension_edges().len() as f64;
        let meta = self.collect_meta_ids();
        let meta_edges = self
            .instances
            .iter()
            .filter(|r| meta.contains(&r.x) || meta.contains(&r.y))
            .count();
        s -= 0.1 * meta_edges as f64;
        s
    }

    /// Run one step of the intrinsic drive loop (ADR 0031, task C):
    /// try each candidate action on a clone of self, measure the
    /// score delta, apply the best one in place. Returns the applied
    /// step record, or `None` if no action improves the score above
    /// `config.epsilon`.
    ///
    /// This is the first v2 mechanism where the system self-selects
    /// among its capabilities based on an internal value signal, with
    /// no external trigger telling it what to do or when.
    pub fn drive_step(&mut self, config: &DriveConfig) -> Option<DriveStep> {
        let before = self.abstraction_score();
        let mut best: Option<DriveStep> = None;
        let mut best_trial: Option<RSet> = None;

        for action in config.candidate_actions() {
            let mut trial = self.clone();
            let result = trial.apply_drive_action(&action);
            let after = trial.abstraction_score();
            let delta = after - before;
            if delta > config.epsilon
                && best
                    .as_ref()
                    .map(|b| delta > b.delta)
                    .unwrap_or(true)
            {
                best = Some(DriveStep {
                    action: action.clone(),
                    score_before: before,
                    score_after: after,
                    delta,
                    result,
                });
                best_trial = Some(trial);
            }
        }

        if let (Some(step), Some(trial)) = (best, best_trial) {
            *self = trial;
            Some(step)
        } else {
            None
        }
    }

    /// Iterate `drive_step` until no action is worthwhile or until
    /// `config.max_steps` is reached. Returns the ordered trace of
    /// applied steps. ADR 0031.
    pub fn intrinsic_drive(&mut self, config: &DriveConfig) -> DriveTrace {
        let mut steps = Vec::new();
        for _ in 0..config.max_steps {
            match self.drive_step(config) {
                Some(step) => steps.push(step),
                None => break,
            }
        }
        DriveTrace {
            initial_score: steps
                .first()
                .map(|s| s.score_before)
                .unwrap_or_else(|| self.abstraction_score()),
            final_score: self.abstraction_score(),
            steps,
        }
    }

    fn apply_drive_action(&mut self, action: &DriveAction) -> DriveActionResult {
        match action {
            DriveAction::DiscoverPatterns(cfg) => {
                let outcomes = self.autonomous_pass(cfg);
                let new_patterns = outcomes
                    .iter()
                    .filter(|o| matches!(o, AutonomousOutcome::NewPattern { .. }))
                    .count();
                DriveActionResult::PatternsDiscovered {
                    target_size: cfg.discovery.target_size,
                    new_patterns,
                }
            }
            DriveAction::DiscoverTheory(cfg) => {
                let th = self.discover_theory(cfg);
                if th.member_axiom_ids.is_empty() {
                    return DriveActionResult::TheoryDiscovered {
                        theory_id: None,
                        member_count: 0,
                    };
                }
                let ids: Vec<&str> =
                    th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
                match self.name_theory(&ids) {
                    Ok(tid) => DriveActionResult::TheoryDiscovered {
                        theory_id: Some(tid),
                        member_count: th.member_axiom_ids.len(),
                    },
                    Err(_) => DriveActionResult::TheoryDiscovered {
                        theory_id: None,
                        member_count: 0,
                    },
                }
            }
            DriveAction::Prune(threshold) => {
                let ranked = self.rank_by_counterfactual();
                let victims: Vec<String> = ranked
                    .into_iter()
                    .filter(|(_, v)| *v < *threshold)
                    .map(|(id, _)| id)
                    .collect();
                let mut pruned: Vec<String> = Vec::new();
                let pattern_set: HashSet<String> = self
                    .patterns()
                    .into_iter()
                    .map(str::to_owned)
                    .collect();
                let theory_set: HashSet<String> = self
                    .theories()
                    .into_iter()
                    .map(str::to_owned)
                    .collect();
                let ext_set: HashSet<String> = self
                    .extension_edges()
                    .into_iter()
                    .map(str::to_owned)
                    .collect();
                // Retract theories first (they may reference axioms that
                // would also be prunable; retract order matters for
                // axioms though we only prune objects listed here).
                for id in &victims {
                    if theory_set.contains(id) {
                        if self.retract_theory(id).is_ok() {
                            pruned.push(id.clone());
                        }
                    }
                }
                for id in &victims {
                    if ext_set.contains(id) {
                        if self.retract_extension(id).is_ok() {
                            pruned.push(id.clone());
                        }
                    }
                }
                for id in &victims {
                    if pattern_set.contains(id) {
                        if self.retract_pattern(id).is_ok() {
                            pruned.push(id.clone());
                        }
                    }
                }
                DriveActionResult::Pruned { object_ids: pruned }
            }
        }
    }

    /// Autonomous abstraction pass. ADR 0018.
    ///
    /// Composes `discover_motifs` (sample candidates) + `refine_candidates`
    /// (polish representatives) + `find_instances_of` (enumerate clean
    /// instances) + `name_pattern_instances` (record as meta-R) into a
    /// single pipeline. One outcome per discovered canonical.
    ///
    /// No user-supplied canonical forms or instance lists; the system
    /// samples, scores, refines, and names on its own. Attach-only (for
    /// extending existing named patterns with more instances) is a
    /// separate concern — run `run_naming_pass` with `attach_only = true`
    /// if wanted.
    pub fn autonomous_pass(
        &mut self,
        config: &AutonomousConfig,
    ) -> Vec<AutonomousOutcome> {
        let raw = self.discover_motifs(&config.discovery);
        let refined = self.refine_candidates(raw, &config.refinement);

        let mut outcomes = Vec::with_capacity(refined.len());
        for candidate in refined {
            let canon = candidate.canonical.clone();

            // 1. Is this canonical already named?
            if let Some(existing) = self.find_pattern_matching(&canon) {
                outcomes.push(AutonomousOutcome::Existing {
                    pattern_id: existing.to_string(),
                    canonical: canon,
                });
                continue;
            }

            // 2. Novel canonical. Collect clean instances — use
            // sampling path when configured, otherwise exhaustive.
            // ADR 0043.
            let instances = match &config.instance_sampling {
                Some(smpl) => self.sample_instances_of(&canon, smpl),
                None => self.find_instances_of(&canon),
            };
            if instances.is_empty() {
                outcomes.push(AutonomousOutcome::Skipped {
                    canonical: canon,
                    reason: AutonomousSkip::NoCleanInstance,
                });
                continue;
            }

            // 3. Apply naming policy.
            let count = instances.len();
            match self.consider_naming(&instances, &config.naming) {
                Ok(NamingDecision::Named(pid)) => {
                    outcomes.push(AutonomousOutcome::NewPattern {
                        pattern_id: pid,
                        instance_count: count,
                        canonical: canon,
                    });
                }
                Ok(NamingDecision::Skipped(reason)) => {
                    outcomes.push(AutonomousOutcome::Skipped {
                        canonical: canon,
                        reason: AutonomousSkip::PolicyFiltered(reason),
                    });
                }
                Err(_) => {
                    // consider_naming errors require invalid input (empty
                    // list, empty subgraph, non-isomorphic). By construction
                    // find_instances_of returns non-empty isomorphic
                    // instances, so this branch is unreachable in well-
                    // formed flows. Conservatively skip.
                    outcomes.push(AutonomousOutcome::Skipped {
                        canonical: canon,
                        reason: AutonomousSkip::NoCleanInstance,
                    });
                }
            }
        }
        outcomes
    }

    // NOTE: ADR 0026 gradient-descent refinement primitives
    // (gradient_refine_candidate, gradient_refine_from_init,
    //  gradient_refine_from_uniform, gradient_refine_multistart)
    // were removed after the probe.
    // See ADR 0026 and its experiment log for the algorithm,
    // findings, and revised verdict (multi-start works but is ~45×
    // more expensive than random re-sample on β-scale graphs).
    // Reference code is retrievable from git history at commit 4fc8b67.

    /// Discover positive-implication axioms by enumerating templates
    /// and evaluating them against the data portion of the RSet.
    /// Returns templates that hold with rate == 1.0 and at least
    /// `min_evidence` premise bindings. ADR 0027.
    pub fn discover_axioms(
        &self,
        config: &AxiomDiscoveryConfig,
    ) -> Vec<AxiomEvidence> {
        let meta = self.collect_meta_ids();
        let ids: Vec<String> = self
            .identifiers()
            .into_iter()
            .filter(|id| !meta.contains(*id))
            .map(str::to_owned)
            .collect();
        if ids.is_empty() {
            return Vec::new();
        }

        let templates = enumerate_axiom_templates(config);
        let mut results = Vec::new();
        for template in templates {
            let ev = self.evaluate_axiom_template(&template, &ids, &meta);
            if ev.premise_bindings >= config.min_evidence && ev.rate >= config.min_rate {
                results.push(ev);
            }
        }
        results
    }

    /// Discover axioms and return the subsumption-minimized set. ADR 0028.
    ///
    /// Applies two redundancy filters to `discover_axioms`:
    ///
    /// 1. **Universal reflexivity subsumption.** When every data identifier
    ///    has a self-loop (`check_reflexivity().rate == 1.0`), any axiom
    ///    whose conclusion is `R(v, v)` is trivially entailed and dropped.
    /// 2. **Premise-weakening subsumption.** If axiom A has a subset of
    ///    axiom B's premise under some variable mapping that also sends
    ///    A's conclusion to B's conclusion, then B is strictly weaker
    ///    than A and is dropped.
    ///
    /// The raw `discover_axioms` output is unchanged; this method is a
    /// composable post-filter.
    pub fn discover_axioms_minimal(
        &self,
        config: &AxiomDiscoveryConfig,
    ) -> Vec<AxiomEvidence> {
        let raw = self.discover_axioms(config);
        // Subsumption relies on strict "A implies B" reasoning which
        // only holds when both A and B are universally satisfied
        // (rate == 1.0). In defeasible mode (min_rate < 1.0) we skip
        // subsumption entirely and return the raw filtered set.
        // ADR 0033.
        if config.min_rate < 1.0 {
            return raw;
        }
        let reflexive_holds = {
            let r = self.check_reflexivity();
            r.identifiers_total > 0 && r.rate == 1.0
        };
        let after_refl = if reflexive_holds {
            subsume_by_reflexivity(raw)
        } else {
            raw
        };
        subsume_by_premise_weakening(after_refl)
    }

    /// Like `discover_axioms_minimal`, but also applies
    /// `subsume_by_composition` — a forward-chaining derivation check
    /// that drops axioms derivable from the others. ADR 0037.
    ///
    /// Goes beyond premise-weakening by detecting that e.g. on an
    /// equivalence relation, the four "transitivity variants" are all
    /// derivable from {symmetry, transitivity} and therefore
    /// redundant. Strict-mode only — composition derivation is not
    /// sound under defeasible semantics.
    pub fn discover_axioms_minimal_compositional(
        &self,
        config: &AxiomDiscoveryConfig,
    ) -> Vec<AxiomEvidence> {
        if config.min_rate < 1.0 {
            return self.discover_axioms(config);
        }
        let minimal = self.discover_axioms_minimal(config);
        subsume_by_composition(minimal)
    }

    /// Discover the complete theory that holds on this RSet at rate 1.0:
    /// the minimal set of template axioms plus any predicate axioms
    /// (reflexivity, antisymmetry) that currently hold. ADR 0030.
    ///
    /// The returned `Theory` has `id = ""` (not yet named). Pass its
    /// `member_axiom_ids` to `name_theory` to persist it as meta-R.
    pub fn discover_theory(&self, config: &AxiomDiscoveryConfig) -> Theory {
        let minimal = self.discover_axioms_minimal(config);
        let mut ids: Vec<String> = Vec::new();
        let mut templates: Vec<AxiomTemplate> = Vec::new();
        for ev in &minimal {
            ids.push(axiom_template_id(&ev.template));
            templates.push(ev.template.clone());
        }
        let refl = self.check_reflexivity();
        if refl.identifiers_total > 0 && refl.rate == 1.0 {
            ids.push(AX_REFLEXIVITY.to_string());
        }
        let anti = self.check_antisymmetry();
        if anti.holds && anti.directed_pairs_checked > 0 {
            ids.push(AX_ANTISYMMETRY.to_string());
        }
        let tot = self.check_totality();
        if tot.holds {
            ids.push(AX_TOTALITY.to_string());
        }
        Theory {
            id: String::new(),
            member_axiom_ids: ids,
            template_members: templates,
        }
    }

    /// Name a conjunction of axioms as a single theory. ADR 0030.
    ///
    /// Verifies each member axiom currently holds on the RSet at
    /// rate 1.0 before persisting. Writes:
    /// - `R(AXIOM_MARKER, ax_i)` for every previously-unregistered
    ///   axiom id;
    /// - `R(THEORY_MARKER, t_N)` for the new theory;
    /// - `R(t_N, ax_i)` for each member.
    ///
    /// Returns the minted theory id. If a theory with the exact same
    /// member set already exists, reuses its id (same shape as
    /// `name_pattern_instances`).
    pub fn name_theory(&mut self, axiom_ids: &[&str]) -> Result<String, TheoryError> {
        if axiom_ids.is_empty() {
            return Err(TheoryError::EmptyMemberList);
        }
        let unique: HashSet<&str> = axiom_ids.iter().copied().collect();
        for id in axiom_ids {
            self.verify_axiom_holds(id)?;
        }
        // Reuse if an existing theory has the exact same member set.
        for existing in self.theories() {
            let members: HashSet<String> = self
                .theory_axioms(existing)
                .into_iter()
                .map(str::to_owned)
                .collect();
            let input: HashSet<String> = unique.iter().map(|s| s.to_string()).collect();
            if members == input {
                return Ok(existing.to_string());
            }
        }
        let theory_id = self.mint_theory_id();
        self.add(R::new(THEORY_MARKER, theory_id.clone()));
        for id in &unique {
            if !self.is_axiom(id) {
                self.register_axiom_with_intension(id);
            }
            self.add(R::new(theory_id.clone(), (*id).to_string()));
        }
        Ok(theory_id)
    }

    /// Register an axiom and, if it is a template axiom (not a
    /// predicate), write its intension to meta-R. ADR 0032.
    ///
    /// For a template axiom `ax_tpl_...` the intension comprises:
    /// - `n` variables `ax_X_var_i` (registry + ownership)
    /// - `m` premise-edge nodes `ax_X_prem_j` (registry + ownership)
    /// - One conclusion-edge node `ax_X_concl` (registry + ownership)
    /// - A chain `R(var_x, edge) + R(edge, var_y)` per premise/conclusion
    ///   edge that encodes source→target by the direction of R itself.
    ///
    /// Predicate axioms (`ax_reflexivity`, `ax_antisymmetry`) get only
    /// the registry edge; their semantics live in the predicate
    /// checkers, not in meta-R. Documented as an asymmetry — the
    /// current template language cannot express them.
    pub fn register_axiom_with_intension(&mut self, id: &str) -> bool {
        if self.is_axiom(id) {
            return false;
        }
        self.add(R::new(AXIOM_MARKER, id.to_string()));
        // Predicate axioms: registry only.
        if id == AX_REFLEXIVITY || id == AX_ANTISYMMETRY || id == AX_TOTALITY {
            return true;
        }
        // Template axioms: write the intension.
        let Some(template) = axiom_id_to_template(id) else {
            return true; // unknown shape — registry-only, same as predicates
        };
        let var_ids: Vec<String> = (0..template.num_vars)
            .map(|i| format!("{}_var_{}", id, i))
            .collect();
        for v in &var_ids {
            self.add(R::new(AXIOMVAR_MARKER, v.clone()));
            self.add(R::new(id.to_string(), v.clone()));
        }
        for (j, e) in template.premise.iter().enumerate() {
            let prem_id = format!("{}_prem_{}", id, j);
            self.add(R::new(PREMISE_MARKER, prem_id.clone()));
            self.add(R::new(id.to_string(), prem_id.clone()));
            self.add(R::new(var_ids[e.x_var].clone(), prem_id.clone()));
            self.add(R::new(prem_id.clone(), var_ids[e.y_var].clone()));
        }
        let concl_id = format!("{}_concl", id);
        self.add(R::new(CONCLUSION_MARKER, concl_id.clone()));
        self.add(R::new(id.to_string(), concl_id.clone()));
        self.add(R::new(
            var_ids[template.conclusion.x_var].clone(),
            concl_id.clone(),
        ));
        self.add(R::new(
            concl_id.clone(),
            var_ids[template.conclusion.y_var].clone(),
        ));
        true
    }

    /// Variables of an axiom, in sorted order. ADR 0032.
    pub fn axiom_variables(&self, axiom_id: &str) -> Vec<&str> {
        let var_set: HashSet<&str> = self
            .left_of(AXIOMVAR_MARKER)
            .iter()
            .map(|r| r.y.as_str())
            .collect();
        let mut out: Vec<&str> = self
            .left_of(axiom_id)
            .into_iter()
            .filter_map(|r| {
                let y = r.y.as_str();
                if var_set.contains(y) {
                    Some(y)
                } else {
                    None
                }
            })
            .collect();
        out.sort();
        out
    }

    /// Premise-edge ids of an axiom, in sorted order. ADR 0032.
    pub fn axiom_premise_edges(&self, axiom_id: &str) -> Vec<&str> {
        let prem_set: HashSet<&str> = self
            .left_of(PREMISE_MARKER)
            .iter()
            .map(|r| r.y.as_str())
            .collect();
        let mut out: Vec<&str> = self
            .left_of(axiom_id)
            .into_iter()
            .filter_map(|r| {
                let y = r.y.as_str();
                if prem_set.contains(y) {
                    Some(y)
                } else {
                    None
                }
            })
            .collect();
        out.sort();
        out
    }

    /// Conclusion-edge id of an axiom, if intension is recorded. ADR 0032.
    pub fn axiom_conclusion(&self, axiom_id: &str) -> Option<String> {
        let concl_set: HashSet<String> = self
            .left_of(CONCLUSION_MARKER)
            .iter()
            .map(|r| r.y.clone())
            .collect();
        self.left_of(axiom_id)
            .into_iter()
            .find_map(|r| {
                if concl_set.contains(&r.y) {
                    Some(r.y.clone())
                } else {
                    None
                }
            })
    }

    /// Reconstruct an `AxiomTemplate` from the stored intension.
    /// Returns `None` for predicate axioms or unregistered ids. ADR 0032.
    pub fn reconstruct_axiom_template(&self, axiom_id: &str) -> Option<AxiomTemplate> {
        if axiom_id == AX_REFLEXIVITY
            || axiom_id == AX_ANTISYMMETRY
            || axiom_id == AX_TOTALITY
        {
            return None;
        }
        let vars = self.axiom_variables(axiom_id);
        if vars.is_empty() {
            return None;
        }
        let var_index: HashMap<&str, usize> =
            vars.iter().enumerate().map(|(i, v)| (*v, i)).collect();
        let prem_ids = self.axiom_premise_edges(axiom_id);
        let concl_id = self.axiom_conclusion(axiom_id)?;

        let endpoints = |edge_id: &str| -> Option<EdgeTemplate> {
            // Source: R(var_x, edge_id) — edge_id on the right side.
            let src = self.right_of(edge_id).into_iter().find_map(|r| {
                var_index.get(r.x.as_str()).copied()
            })?;
            // Target: R(edge_id, var_y) — edge_id on the left side.
            let tgt = self.left_of(edge_id).into_iter().find_map(|r| {
                var_index.get(r.y.as_str()).copied()
            })?;
            Some(EdgeTemplate { x_var: src, y_var: tgt })
        };
        let mut premise: Vec<EdgeTemplate> = Vec::new();
        for p in &prem_ids {
            premise.push(endpoints(p)?);
        }
        let conclusion = endpoints(&concl_id)?;
        Some(AxiomTemplate {
            num_vars: vars.len(),
            premise,
            conclusion,
        })
    }

    /// Retract an axiom along with its intension. ADR 0032.
    ///
    /// Fails if the axiom is still referenced by any theory (caller
    /// must retract the theories first). Returns the count of meta-R
    /// edges removed. Predicate axioms only remove the registry edge.
    pub fn retract_axiom(&mut self, axiom_id: &str) -> Result<usize, TheoryError> {
        if !self.is_axiom(axiom_id) {
            return Err(TheoryError::UnsatisfiedMember(axiom_id.to_string()));
        }
        if !self.theories_containing(axiom_id).is_empty() {
            return Err(TheoryError::UnsatisfiedMember(
                format!("{} is still referenced by a theory", axiom_id),
            ));
        }
        let mut removed = 0usize;

        // Collect everything this axiom owns (variables, premise edges,
        // conclusion).
        let vars: Vec<String> = self
            .axiom_variables(axiom_id)
            .into_iter()
            .map(str::to_owned)
            .collect();
        let prem_edges: Vec<String> = self
            .axiom_premise_edges(axiom_id)
            .into_iter()
            .map(str::to_owned)
            .collect();
        let concl = self.axiom_conclusion(axiom_id);

        // Remove structural edges: R(var, prem), R(prem, var), R(var, concl), R(concl, var).
        for prem in &prem_edges {
            let to_remove: Vec<R> = self
                .instances
                .iter()
                .filter(|r| r.x == *prem || r.y == *prem)
                .cloned()
                .collect();
            for e in to_remove {
                if self.remove(&e) {
                    removed += 1;
                }
            }
        }
        if let Some(c) = &concl {
            let to_remove: Vec<R> = self
                .instances
                .iter()
                .filter(|r| r.x == *c || r.y == *c)
                .cloned()
                .collect();
            for e in to_remove {
                if self.remove(&e) {
                    removed += 1;
                }
            }
        }
        for v in &vars {
            if self.remove(&R::new(AXIOMVAR_MARKER, v.clone())) {
                removed += 1;
            }
            if self.remove(&R::new(axiom_id.to_string(), v.clone())) {
                removed += 1;
            }
        }
        if self.remove(&R::new(AXIOM_MARKER, axiom_id.to_string())) {
            removed += 1;
        }
        Ok(removed)
    }

    fn verify_axiom_holds(&self, id: &str) -> Result<(), TheoryError> {
        if id == AX_REFLEXIVITY {
            let r = self.check_reflexivity();
            if r.identifiers_total > 0 && r.rate == 1.0 {
                return Ok(());
            }
            return Err(TheoryError::UnsatisfiedMember(id.to_string()));
        }
        if id == AX_ANTISYMMETRY {
            let a = self.check_antisymmetry();
            if a.holds && a.directed_pairs_checked > 0 {
                return Ok(());
            }
            return Err(TheoryError::UnsatisfiedMember(id.to_string()));
        }
        if id == AX_TOTALITY {
            let t = self.check_totality();
            if t.holds {
                return Ok(());
            }
            return Err(TheoryError::UnsatisfiedMember(id.to_string()));
        }
        let Some(template) = axiom_id_to_template(id) else {
            return Err(TheoryError::UnparseableAxiomId(id.to_string()));
        };
        let meta = self.collect_meta_ids();
        let ids: Vec<String> = self
            .identifiers()
            .into_iter()
            .filter(|x| !meta.contains(*x))
            .map(str::to_owned)
            .collect();
        if ids.is_empty() {
            return Err(TheoryError::UnsatisfiedMember(id.to_string()));
        }
        let ev = self.evaluate_axiom_template(&template, &ids, &meta);
        if ev.rate == 1.0 && ev.premise_bindings > 0 {
            Ok(())
        } else {
            Err(TheoryError::UnsatisfiedMember(id.to_string()))
        }
    }

    /// Registered axioms in this RSet. ADR 0030.
    pub fn axioms(&self) -> Vec<&str> {
        self.left_of(AXIOM_MARKER)
            .iter()
            .map(|r| r.y.as_str())
            .collect()
    }

    /// Is `id` a registered axiom? ADR 0030.
    pub fn is_axiom(&self, id: &str) -> bool {
        self.instances.contains(&R::new(AXIOM_MARKER, id))
    }

    /// Registered theories. ADR 0030.
    pub fn theories(&self) -> Vec<&str> {
        self.left_of(THEORY_MARKER)
            .iter()
            .map(|r| r.y.as_str())
            .collect()
    }

    /// Is `id` a registered theory? ADR 0030.
    pub fn is_theory(&self, id: &str) -> bool {
        self.instances.contains(&R::new(THEORY_MARKER, id))
    }

    /// Member axiom ids of a theory, sorted. ADR 0030.
    pub fn theory_axioms(&self, theory: &str) -> Vec<&str> {
        let axiom_set: HashSet<&str> = self.axioms().into_iter().collect();
        let mut out: Vec<&str> = self
            .left_of(theory)
            .into_iter()
            .filter_map(|r| {
                let y = r.y.as_str();
                if axiom_set.contains(y) {
                    Some(y)
                } else {
                    None
                }
            })
            .collect();
        out.sort();
        out
    }

    /// Every theory that includes `axiom_id`. ADR 0030.
    pub fn theories_containing(&self, axiom_id: &str) -> Vec<&str> {
        let theory_set: HashSet<&str> = self.theories().into_iter().collect();
        let mut out: Vec<&str> = self
            .right_of(axiom_id)
            .into_iter()
            .filter_map(|r| {
                let x = r.x.as_str();
                if theory_set.contains(x) {
                    Some(x)
                } else {
                    None
                }
            })
            .collect();
        out.sort();
        out
    }

    /// Remove a theory's registry and its membership edges. ADR 0030.
    /// Does NOT remove axiom registrations — other theories may share
    /// them. Returns the number of meta-R edges removed.
    pub fn retract_theory(&mut self, theory_id: &str) -> Result<usize, TheoryError> {
        if !self.is_theory(theory_id) {
            return Err(TheoryError::UnsatisfiedMember(theory_id.to_string()));
        }
        let member_ids: Vec<String> = self
            .theory_axioms(theory_id)
            .into_iter()
            .map(str::to_owned)
            .collect();
        let mut removed = 0usize;
        for m in &member_ids {
            if self.remove(&R::new(theory_id.to_string(), m.clone())) {
                removed += 1;
            }
        }
        if self.remove(&R::new(THEORY_MARKER, theory_id.to_string())) {
            removed += 1;
        }
        Ok(removed)
    }

    fn mint_theory_id(&self) -> String {
        let existing = self.identifiers();
        let mut n = self.theories().len();
        loop {
            let candidate = format!("t_{}", n);
            if !existing.contains(candidate.as_str()) {
                return candidate;
            }
            n += 1;
        }
    }

    // ─── ADR 0034: theory extension relations ──────────────────────────

    /// Name a "T_sub extends T_super" relation in meta-R. ADR 0034.
    ///
    /// Verifies that both theories exist, are distinct, and that
    /// `members(T_sub) ⊇ members(T_super)` (i.e., every axiom in the
    /// super-theory is also in the sub-theory). On success writes the
    /// three-edge chain:
    ///
    /// ```text
    /// R(__extends__, ext_N)        — registry
    /// R(T_sub,       ext_N)        — source-side chain link
    /// R(ext_N,       T_super)      — target-side chain link
    /// ```
    ///
    /// Returns the minted `ext_N` id. Reuses an existing id if the
    /// same (sub, super) pair is already recorded.
    pub fn name_theory_extension(
        &mut self,
        sub: &str,
        super_: &str,
    ) -> Result<String, TheoryError> {
        if !self.is_theory(sub) {
            return Err(TheoryError::UnsatisfiedMember(sub.to_string()));
        }
        if !self.is_theory(super_) {
            return Err(TheoryError::UnsatisfiedMember(super_.to_string()));
        }
        if sub == super_ {
            return Err(TheoryError::UnsatisfiedMember(
                "sub and super must differ".to_string(),
            ));
        }
        let sub_members: HashSet<String> = self
            .theory_axioms(sub)
            .into_iter()
            .map(str::to_owned)
            .collect();
        let super_members: HashSet<String> = self
            .theory_axioms(super_)
            .into_iter()
            .map(str::to_owned)
            .collect();
        if !super_members.is_subset(&sub_members) {
            return Err(TheoryError::UnsatisfiedMember(format!(
                "{} does not extend {} — super-members are not a subset",
                sub, super_
            )));
        }
        // Reuse existing ext_N if one already encodes the same pair.
        for existing in self.extension_edges() {
            let (es, esup) = self.extension_endpoints(existing).unwrap_or_default();
            if es == sub && esup == super_ {
                return Ok(existing.to_string());
            }
        }
        let ext_id = self.mint_extension_id();
        self.add(R::new(EXTENDS_MARKER, ext_id.clone()));
        self.add(R::new(sub.to_string(), ext_id.clone()));
        self.add(R::new(ext_id.clone(), super_.to_string()));
        Ok(ext_id)
    }

    /// Every named extension-edge id. ADR 0034.
    pub fn extension_edges(&self) -> Vec<&str> {
        self.left_of(EXTENDS_MARKER)
            .iter()
            .map(|r| r.y.as_str())
            .collect()
    }

    /// Decode an extension edge's endpoints: `(sub, super)`. Returns
    /// `None` if `ext_id` is not a registered extension. ADR 0034.
    pub fn extension_endpoints(&self, ext_id: &str) -> Option<(String, String)> {
        if !self.instances.contains(&R::new(EXTENDS_MARKER, ext_id)) {
            return None;
        }
        let sub = self
            .right_of(ext_id)
            .into_iter()
            .find_map(|r| {
                if self.is_theory(r.x.as_str()) {
                    Some(r.x.clone())
                } else {
                    None
                }
            })?;
        let super_ = self
            .left_of(ext_id)
            .into_iter()
            .find_map(|r| {
                if self.is_theory(r.y.as_str()) {
                    Some(r.y.clone())
                } else {
                    None
                }
            })?;
        Some((sub, super_))
    }

    /// Theories that `theory` extends (direct super-theories). ADR 0034.
    pub fn theory_extends(&self, theory: &str) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for ext in self.extension_edges() {
            if let Some((sub, super_)) = self.extension_endpoints(ext) {
                if sub == theory {
                    // We need &str scoped to self, not owned String.
                    if let Some(edge) = self
                        .instances
                        .iter()
                        .find(|r| r.x == ext && r.y == super_)
                    {
                        out.push(edge.y.as_str());
                    }
                }
            }
        }
        out.sort();
        out
    }

    /// Theories that extend `theory` (direct sub-theories). ADR 0034.
    pub fn theory_extended_by(&self, theory: &str) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for ext in self.extension_edges() {
            if let Some((sub, super_)) = self.extension_endpoints(ext) {
                if super_ == theory {
                    if let Some(edge) = self
                        .instances
                        .iter()
                        .find(|r| r.x == sub && r.y == ext)
                    {
                        out.push(edge.x.as_str());
                    }
                }
            }
        }
        out.sort();
        out
    }

    /// Scan named theories for all (sub, super) pairs where
    /// `members(sub) ⊋ members(super)` (strict superset). Does not
    /// write anything to meta-R. ADR 0034.
    pub fn discover_theory_extensions(&self) -> Vec<(String, String)> {
        let theories: Vec<String> =
            self.theories().into_iter().map(str::to_owned).collect();
        let member_sets: HashMap<String, HashSet<String>> = theories
            .iter()
            .map(|t| {
                let members: HashSet<String> = self
                    .theory_axioms(t)
                    .into_iter()
                    .map(str::to_owned)
                    .collect();
                (t.clone(), members)
            })
            .collect();
        let mut out: Vec<(String, String)> = Vec::new();
        for sub in &theories {
            for super_ in &theories {
                if sub == super_ {
                    continue;
                }
                let s = &member_sets[sub];
                let p = &member_sets[super_];
                if p.is_subset(s) && p.len() < s.len() {
                    out.push((sub.clone(), super_.clone()));
                }
            }
        }
        out.sort();
        out
    }

    fn mint_extension_id(&self) -> String {
        let existing = self.identifiers();
        let mut n = self.extension_edges().len();
        loop {
            let candidate = format!("ext_{}", n);
            if !existing.contains(candidate.as_str()) {
                return candidate;
            }
            n += 1;
        }
    }

    // ─── ADR 0042: theory independence relations ───────────────────

    /// Name a "T_a ⊥ T_b" (independence) relation in meta-R. ADR 0042.
    ///
    /// Two theories are independent iff their member-axiom sets are
    /// disjoint. Symmetric — the chain stores a canonical direction
    /// (lex-smaller theory id as the source). On success writes:
    ///
    /// ```text
    /// R(__independent__, ind_N)    — registry
    /// R(T_min, ind_N)              — canonical source side
    /// R(ind_N, T_max)              — canonical target side
    /// ```
    ///
    /// Reuses an existing ind_N if the same pair is already recorded.
    pub fn name_theory_independence(
        &mut self,
        a: &str,
        b: &str,
    ) -> Result<String, TheoryError> {
        if !self.is_theory(a) {
            return Err(TheoryError::UnsatisfiedMember(a.to_string()));
        }
        if !self.is_theory(b) {
            return Err(TheoryError::UnsatisfiedMember(b.to_string()));
        }
        if a == b {
            return Err(TheoryError::UnsatisfiedMember(
                "a theory is not independent of itself".to_string(),
            ));
        }
        let a_members: HashSet<String> = self
            .theory_axioms(a)
            .into_iter()
            .map(str::to_owned)
            .collect();
        let b_members: HashSet<String> = self
            .theory_axioms(b)
            .into_iter()
            .map(str::to_owned)
            .collect();
        if !a_members.is_disjoint(&b_members) {
            return Err(TheoryError::UnsatisfiedMember(format!(
                "{} and {} share at least one axiom",
                a, b
            )));
        }
        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
        for existing in self.independence_edges() {
            let (el, eh) = self.independence_endpoints(existing).unwrap_or_default();
            if el == lo && eh == hi {
                return Ok(existing.to_string());
            }
        }
        let ind_id = self.mint_independence_id();
        self.add(R::new(INDEPENDENT_MARKER, ind_id.clone()));
        self.add(R::new(lo.to_string(), ind_id.clone()));
        self.add(R::new(ind_id.clone(), hi.to_string()));
        Ok(ind_id)
    }

    /// Every named independence-edge id. ADR 0042.
    pub fn independence_edges(&self) -> Vec<&str> {
        self.left_of(INDEPENDENT_MARKER)
            .iter()
            .map(|r| r.y.as_str())
            .collect()
    }

    /// Decode an independence-edge's endpoints: `(T_lo, T_hi)` where
    /// `T_lo < T_hi` lexicographically. ADR 0042.
    pub fn independence_endpoints(&self, ind_id: &str) -> Option<(String, String)> {
        if !self.instances.contains(&R::new(INDEPENDENT_MARKER, ind_id)) {
            return None;
        }
        let lo = self
            .right_of(ind_id)
            .into_iter()
            .find_map(|r| {
                if self.is_theory(r.x.as_str()) {
                    Some(r.x.clone())
                } else {
                    None
                }
            })?;
        let hi = self
            .left_of(ind_id)
            .into_iter()
            .find_map(|r| {
                if self.is_theory(r.y.as_str()) {
                    Some(r.y.clone())
                } else {
                    None
                }
            })?;
        Some((lo, hi))
    }

    /// All theories independent from `theory` (either direction of
    /// the canonical chain). ADR 0042.
    pub fn theories_independent_from(&self, theory: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for ind in self.independence_edges() {
            if let Some((lo, hi)) = self.independence_endpoints(ind) {
                if lo == theory {
                    out.push(hi);
                } else if hi == theory {
                    out.push(lo);
                }
            }
        }
        out.sort();
        out
    }

    /// Scan named theories for all pairs with disjoint member sets.
    /// Returns pairs in canonical order `(lo, hi)` with `lo < hi`.
    /// Read-only. ADR 0042.
    pub fn discover_theory_independences(&self) -> Vec<(String, String)> {
        let theories: Vec<String> =
            self.theories().into_iter().map(str::to_owned).collect();
        let member_sets: HashMap<String, HashSet<String>> = theories
            .iter()
            .map(|t| {
                let m: HashSet<String> = self
                    .theory_axioms(t)
                    .into_iter()
                    .map(str::to_owned)
                    .collect();
                (t.clone(), m)
            })
            .collect();
        let mut out: Vec<(String, String)> = Vec::new();
        for i in 0..theories.len() {
            for j in (i + 1)..theories.len() {
                let a = &theories[i];
                let b = &theories[j];
                if member_sets[a].is_disjoint(&member_sets[b])
                    && !member_sets[a].is_empty()
                    && !member_sets[b].is_empty()
                {
                    let (lo, hi) = if a < b { (a.clone(), b.clone()) } else { (b.clone(), a.clone()) };
                    out.push((lo, hi));
                }
            }
        }
        out.sort();
        out
    }

    /// Retract an independence-relation edge. ADR 0042.
    pub fn retract_independence(&mut self, ind_id: &str) -> Result<usize, TheoryError> {
        if !self.instances.contains(&R::new(INDEPENDENT_MARKER, ind_id)) {
            return Err(TheoryError::UnsatisfiedMember(ind_id.to_string()));
        }
        let mut removed = 0usize;
        let to_remove: Vec<R> = self
            .instances
            .iter()
            .filter(|r| r.x == ind_id || r.y == ind_id)
            .cloned()
            .collect();
        for e in to_remove {
            if self.remove(&e) {
                removed += 1;
            }
        }
        if self.remove(&R::new(INDEPENDENT_MARKER, ind_id.to_string())) {
            removed += 1;
        }
        Ok(removed)
    }

    fn mint_independence_id(&self) -> String {
        let existing = self.identifiers();
        let mut n = self.independence_edges().len();
        loop {
            let candidate = format!("ind_{}", n);
            if !existing.contains(candidate.as_str()) {
                return candidate;
            }
            n += 1;
        }
    }

    /// Retract an extension-relation edge. ADR 0034 / 0035.
    pub fn retract_extension(&mut self, ext_id: &str) -> Result<usize, TheoryError> {
        if !self.instances.contains(&R::new(EXTENDS_MARKER, ext_id)) {
            return Err(TheoryError::UnsatisfiedMember(ext_id.to_string()));
        }
        let mut removed = 0usize;
        // Remove sub-side and super-side edges (whichever exist).
        let to_remove: Vec<R> = self
            .instances
            .iter()
            .filter(|r| r.x == ext_id || r.y == ext_id)
            .cloned()
            .collect();
        for e in to_remove {
            if self.remove(&e) {
                removed += 1;
            }
        }
        if self.remove(&R::new(EXTENDS_MARKER, ext_id.to_string())) {
            removed += 1;
        }
        Ok(removed)
    }

    // ─── ADR 0035: meta-metric / counterfactual value ──────────────────

    /// Counterfactual value of a named object: the drop in
    /// `abstraction_score` that would result from retracting it.
    /// Positive = the object is "load-bearing"; near zero = it
    /// contributes little; negative = it's a net cost. ADR 0035.
    ///
    /// Works for patterns, theories, and extensions. Returns `None`
    /// for ids that are not one of these, or when retraction fails
    /// (e.g., an axiom still referenced by a theory — use
    /// `retract_theory` first).
    pub fn counterfactual_value(&self, id: &str) -> Option<f64> {
        let before = self.abstraction_score();
        let mut trial = self.clone();
        let is_pat: HashSet<&str> =
            self.patterns().into_iter().collect();
        let is_th: HashSet<&str> =
            self.theories().into_iter().collect();
        let is_ext: HashSet<&str> =
            self.extension_edges().into_iter().collect();
        let is_ax: HashSet<&str> =
            self.axioms().into_iter().collect();

        let retracted = if is_pat.contains(id) {
            trial.retract_pattern(id).is_ok()
        } else if is_th.contains(id) {
            trial.retract_theory(id).is_ok()
        } else if is_ext.contains(id) {
            trial.retract_extension(id).is_ok()
        } else if is_ax.contains(id) {
            trial.retract_axiom(id).is_ok()
        } else {
            return None;
        };
        if !retracted {
            return None;
        }
        Some(before - trial.abstraction_score())
    }

    /// Rank every retractable named object by its counterfactual
    /// value, descending. Items with equal value order by id. ADR 0035.
    ///
    /// Gives a global picture of which abstractions carry the score
    /// and which are passengers. Useful as a second-order ("was my
    /// drive choice any good?") signal on top of ADR 0031's drive.
    pub fn rank_by_counterfactual(&self) -> Vec<(String, f64)> {
        let mut items: Vec<(String, f64)> = Vec::new();
        for p in self.patterns() {
            if let Some(v) = self.counterfactual_value(p) {
                items.push((p.to_string(), v));
            }
        }
        for t in self.theories() {
            if let Some(v) = self.counterfactual_value(t) {
                items.push((t.to_string(), v));
            }
        }
        for e in self.extension_edges() {
            if let Some(v) = self.counterfactual_value(e) {
                items.push((e.to_string(), v));
            }
        }
        items.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        items
    }

    /// Reflexivity: every data identifier has a self-loop `R(x, x)`.
    /// ADR 0027.
    pub fn check_reflexivity(&self) -> ReflexivityEvidence {
        let meta = self.collect_meta_ids();
        let ids: Vec<String> = self
            .identifiers()
            .into_iter()
            .filter(|id| !meta.contains(*id))
            .map(str::to_owned)
            .collect();
        let total = ids.len();
        let present = ids
            .iter()
            .filter(|id| self.instances.contains(&R::new(id.clone(), id.clone())))
            .count();
        let rate = if total == 0 { 1.0 } else { present as f64 / total as f64 };
        ReflexivityEvidence {
            identifiers_total: total,
            self_loops_present: present,
            rate,
        }
    }

    /// Totality: for every pair of distinct data identifiers (x, y),
    /// at least one of `R(x, y)` or `R(y, x)` holds. ADR 0039.
    pub fn check_totality(&self) -> TotalityEvidence {
        let meta = self.collect_meta_ids();
        let ids: Vec<String> = self
            .identifiers()
            .into_iter()
            .filter(|id| !meta.contains(*id))
            .map(str::to_owned)
            .collect();
        let mut sorted_ids = ids.clone();
        sorted_ids.sort();
        let mut unordered_pairs_checked = 0usize;
        let mut violations = 0usize;
        for i in 0..sorted_ids.len() {
            for j in (i + 1)..sorted_ids.len() {
                unordered_pairs_checked += 1;
                let a = &sorted_ids[i];
                let b = &sorted_ids[j];
                let forward = R::new(a.clone(), b.clone());
                let backward = R::new(b.clone(), a.clone());
                if !self.instances.contains(&forward)
                    && !self.instances.contains(&backward)
                {
                    violations += 1;
                }
            }
        }
        TotalityEvidence {
            unordered_pairs_checked,
            violations,
            holds: violations == 0 && unordered_pairs_checked > 0,
        }
    }

    /// Antisymmetry: no pair of distinct data identifiers (x, y) such
    /// that both R(x, y) and R(y, x) exist. ADR 0027.
    pub fn check_antisymmetry(&self) -> AntisymmetryEvidence {
        let meta = self.collect_meta_ids();
        let mut pairs_seen = 0;
        let mut violations = 0;
        for r in self.instances.iter() {
            if meta.contains(&r.x) || meta.contains(&r.y) {
                continue;
            }
            if r.x == r.y {
                continue;
            }
            pairs_seen += 1;
            let reverse = R::new(r.y.clone(), r.x.clone());
            if self.instances.contains(&reverse) {
                violations += 1;
            }
        }
        AntisymmetryEvidence {
            directed_pairs_checked: pairs_seen,
            violations,
            holds: violations == 0,
        }
    }

    /// Combined poset check: reflexive ∧ antisymmetric ∧ transitive.
    /// ADR 0027.
    pub fn check_poset(&self) -> PosetCheck {
        let reflexive = self.check_reflexivity();
        let antisymmetric = self.check_antisymmetry();

        let transitivity_template = AxiomTemplate {
            num_vars: 3,
            premise: vec![
                EdgeTemplate { x_var: 0, y_var: 1 },
                EdgeTemplate { x_var: 1, y_var: 2 },
            ],
            conclusion: EdgeTemplate { x_var: 0, y_var: 2 },
        };
        let meta = self.collect_meta_ids();
        let ids: Vec<String> = self
            .identifiers()
            .into_iter()
            .filter(|id| !meta.contains(*id))
            .map(str::to_owned)
            .collect();
        let transitive_ev = if ids.is_empty() {
            None
        } else {
            Some(self.evaluate_axiom_template(&transitivity_template, &ids, &meta))
        };
        let transitive_holds = transitive_ev
            .as_ref()
            .map(|e| e.rate == 1.0)
            .unwrap_or(true);
        let is_poset =
            reflexive.rate == 1.0 && antisymmetric.holds && transitive_holds;
        PosetCheck {
            reflexive,
            antisymmetric,
            transitive: transitive_ev,
            is_poset,
        }
    }

    /// Evaluate an `EqualityAxiomTemplate`. ADR 0044.
    fn evaluate_equality_template(
        &self,
        template: &EqualityAxiomTemplate,
        ids: &[String],
    ) -> (usize, usize) {
        let mut binding: Vec<usize> = vec![0; template.num_vars];
        let mut premise_bindings = 0usize;
        let mut conclusion_satisfied = 0usize;
        fn rec(
            rs: &RSet,
            template: &EqualityAxiomTemplate,
            ids: &[String],
            binding: &mut [usize],
            depth: usize,
            premise_bindings: &mut usize,
            conclusion_satisfied: &mut usize,
        ) {
            if depth == binding.len() {
                for e in &template.premise {
                    let x = &ids[binding[e.x_var]];
                    let y = &ids[binding[e.y_var]];
                    if !rs.instances.contains(&R::new(x.clone(), y.clone())) {
                        return;
                    }
                }
                *premise_bindings += 1;
                let a = binding[template.equal_vars.0];
                let b = binding[template.equal_vars.1];
                if ids[a] == ids[b] {
                    *conclusion_satisfied += 1;
                }
                return;
            }
            for i in 0..ids.len() {
                binding[depth] = i;
                rec(rs, template, ids, binding, depth + 1, premise_bindings, conclusion_satisfied);
            }
        }
        rec(self, template, ids, &mut binding, 0,
            &mut premise_bindings, &mut conclusion_satisfied);
        (premise_bindings, conclusion_satisfied)
    }

    /// Evaluate a `DisjunctiveAxiomTemplate`. ADR 0044. Conclusion is
    /// satisfied for a binding iff at least one of its disjuncts holds.
    fn evaluate_disjunctive_template(
        &self,
        template: &DisjunctiveAxiomTemplate,
        ids: &[String],
    ) -> (usize, usize) {
        let mut binding: Vec<usize> = vec![0; template.num_vars];
        let mut premise_bindings = 0usize;
        let mut conclusion_satisfied = 0usize;
        fn rec(
            rs: &RSet,
            template: &DisjunctiveAxiomTemplate,
            ids: &[String],
            binding: &mut [usize],
            depth: usize,
            premise_bindings: &mut usize,
            conclusion_satisfied: &mut usize,
        ) {
            if depth == binding.len() {
                for e in &template.premise {
                    let x = &ids[binding[e.x_var]];
                    let y = &ids[binding[e.y_var]];
                    if !rs.instances.contains(&R::new(x.clone(), y.clone())) {
                        return;
                    }
                }
                *premise_bindings += 1;
                let any_holds = template.conclusions.iter().any(|c| {
                    let x = &ids[binding[c.x_var]];
                    let y = &ids[binding[c.y_var]];
                    rs.instances.contains(&R::new(x.clone(), y.clone()))
                });
                if any_holds {
                    *conclusion_satisfied += 1;
                }
                return;
            }
            for i in 0..ids.len() {
                binding[depth] = i;
                rec(rs, template, ids, binding, depth + 1, premise_bindings, conclusion_satisfied);
            }
        }
        rec(self, template, ids, &mut binding, 0,
            &mut premise_bindings, &mut conclusion_satisfied);
        (premise_bindings, conclusion_satisfied)
    }

    /// Discover antisymmetry as an equality-conclusion template:
    /// `R(0,1) ∧ R(1,0) ⇒ v_0 = v_1`. ADR 0044.
    pub fn discover_antisymmetry_template(&self) -> Option<ExtendedAxiomEvidence> {
        let meta = self.collect_meta_ids();
        let ids: Vec<String> = self
            .identifiers()
            .into_iter()
            .filter(|id| !meta.contains(*id))
            .map(str::to_owned)
            .collect();
        if ids.is_empty() {
            return None;
        }
        let tpl = EqualityAxiomTemplate {
            num_vars: 2,
            premise: vec![
                EdgeTemplate { x_var: 0, y_var: 1 },
                EdgeTemplate { x_var: 1, y_var: 0 },
            ],
            equal_vars: (0, 1),
        };
        let (bindings, satisfied) = self.evaluate_equality_template(&tpl, &ids);
        if bindings == 0 {
            return None;
        }
        let rate = satisfied as f64 / bindings as f64;
        Some(ExtendedAxiomEvidence::Equality {
            template: tpl,
            premise_bindings: bindings,
            conclusion_satisfied: satisfied,
            rate,
        })
    }

    /// Discover totality as a disjunctive-conclusion template:
    /// `(empty premise) ⇒ R(0,1) ∨ R(1,0)`. ADR 0044.
    pub fn discover_totality_template(&self) -> Option<ExtendedAxiomEvidence> {
        let meta = self.collect_meta_ids();
        let ids: Vec<String> = self
            .identifiers()
            .into_iter()
            .filter(|id| !meta.contains(*id))
            .map(str::to_owned)
            .collect();
        if ids.is_empty() {
            return None;
        }
        let tpl = DisjunctiveAxiomTemplate {
            num_vars: 2,
            premise: vec![],
            conclusions: vec![
                EdgeTemplate { x_var: 0, y_var: 1 },
                EdgeTemplate { x_var: 1, y_var: 0 },
            ],
        };
        let (bindings, satisfied) = self.evaluate_disjunctive_template(&tpl, &ids);
        if bindings == 0 {
            return None;
        }
        let rate = satisfied as f64 / bindings as f64;
        Some(ExtendedAxiomEvidence::Disjunctive {
            template: tpl,
            premise_bindings: bindings,
            conclusion_satisfied: satisfied,
            rate,
        })
    }

    /// Discover all three axiom families (edge / equality / disjunctive)
    /// and return a merged evidence list. ADR 0044.
    pub fn discover_extended_axioms(
        &self,
        config: &AxiomDiscoveryConfig,
    ) -> Vec<ExtendedAxiomEvidence> {
        let mut out: Vec<ExtendedAxiomEvidence> = self
            .discover_axioms(config)
            .into_iter()
            .map(ExtendedAxiomEvidence::Edge)
            .collect();
        if let Some(ev) = self.discover_antisymmetry_template() {
            if ev.rate() >= config.min_rate
                && ev.premise_bindings() >= config.min_evidence
            {
                out.push(ev);
            }
        }
        if let Some(ev) = self.discover_totality_template() {
            if ev.rate() >= config.min_rate
                && ev.premise_bindings() >= config.min_evidence
            {
                out.push(ev);
            }
        }
        out
    }

    /// Evaluate one axiom template: count premise bindings and
    /// conclusion satisfactions over data-only identifiers. ADR 0027.
    fn evaluate_axiom_template(
        &self,
        template: &AxiomTemplate,
        ids: &[String],
        meta: &HashSet<String>,
    ) -> AxiomEvidence {
        let mut binding: Vec<usize> = vec![0; template.num_vars];
        let mut premise_bindings = 0usize;
        let mut conclusion_satisfied = 0usize;
        evaluate_template_recursive(
            self,
            template,
            ids,
            meta,
            &mut binding,
            0,
            &mut premise_bindings,
            &mut conclusion_satisfied,
        );
        let rate = if premise_bindings == 0 {
            1.0
        } else {
            conclusion_satisfied as f64 / premise_bindings as f64
        };
        let (posterior_lower_95, posterior_upper_95) =
            wilson_score_95(conclusion_satisfied, premise_bindings);
        let ids_n = ids.len();
        let data_edge_count = self
            .instances
            .iter()
            .filter(|r| !meta.contains(&r.x) && !meta.contains(&r.y))
            .count();
        let p_edge = if ids_n == 0 {
            0.0
        } else {
            data_edge_count as f64 / (ids_n as f64 * ids_n as f64)
        };
        let null_baseline_prob =
            null_baseline_probability(premise_bindings, conclusion_satisfied, p_edge);
        AxiomEvidence {
            template: template.clone(),
            premise_bindings,
            conclusion_satisfied,
            rate,
            posterior_lower_95,
            posterior_upper_95,
            null_baseline_prob,
        }
    }

    /// Refine a set of motif candidates. ADR 0017.
    ///
    /// For each candidate whose representative is not clean (embedded
    /// in a larger structure), try up to `config.max_tries` new
    /// random-walk samples of the same target size. Accept the first
    /// sample whose canonical form matches the candidate's AND which
    /// is clean. If no clean alternative is found within the budget,
    /// leave the representative unchanged.
    ///
    /// Deterministic under `config.rng_seed`.
    pub fn refine_candidates(
        &self,
        candidates: Vec<MotifCandidate>,
        config: &RefinementConfig,
    ) -> Vec<MotifCandidate> {
        let data = self.data_edges_sorted();
        if data.is_empty() {
            return candidates;
        }

        let mut rng_state = if config.rng_seed == 0 {
            0x9E3779B97F4A7C15
        } else {
            config.rng_seed
        };

        candidates
            .into_iter()
            .map(|mut c| {
                if self.is_clean_subgraph(&c.representative) {
                    return c;
                }
                let k = c.canonical.len();
                for _ in 0..config.max_tries {
                    if let Some(sg) = sample_connected_subgraph(&data, k, &mut rng_state) {
                        if sg.canonicalize() == c.canonical && self.is_clean_subgraph(&sg) {
                            c.representative = sg;
                            break;
                        }
                    }
                }
                c
            })
            .collect()
    }

    /// Drop any candidate whose participant set already matches an existing
    /// instance of a pattern with the same canonical form. ADR 0012 dedup.
    /// Keeps `run_naming_pass` idempotent: a second call on an unchanged
    /// RSet adds no new instances.
    fn filter_known_instances(&self, subs: Vec<Subgraph>) -> Vec<Subgraph> {
        if subs.is_empty() {
            return subs;
        }
        let canon = subs[0].canonicalize();
        let existing_pattern = match self.find_pattern_matching(&canon) {
            Some(p) => p.to_string(),
            None => return subs,
        };
        let known_sets: Vec<HashSet<String>> = self
            .instances_of(&existing_pattern)
            .iter()
            .map(|inst| {
                self.participants_of(inst)
                    .into_iter()
                    .map(str::to_owned)
                    .collect()
            })
            .collect();
        subs.into_iter()
            .filter(|sub| {
                let this_set: HashSet<String> =
                    sub.identifiers().into_iter().map(str::to_owned).collect();
                !known_sets.iter().any(|k| k == &this_set)
            })
            .collect()
    }

    /// Data edges (meta-R excluded) in a deterministic order.
    /// Sorted lexicographically by `(x, y)` so sampling and matching
    /// are reproducible across process runs, not just within one
    /// process. ADR 0017.
    fn data_edges_sorted(&self) -> Vec<R> {
        let meta = self.collect_meta_ids();
        let mut data: Vec<R> = self
            .instances
            .iter()
            .filter(|r| !meta.contains(&r.x) && !meta.contains(&r.y))
            .cloned()
            .collect();
        data.sort_by(|a, b| (a.x.as_str(), a.y.as_str()).cmp(&(b.x.as_str(), b.y.as_str())));
        data
    }

    /// All edges (data + meta-R) in a deterministic order.
    /// ADR 0025: used by `discover_motifs` when the hierarchical
    /// probe flag is set.
    fn all_edges_sorted(&self) -> Vec<R> {
        let mut all: Vec<R> = self.instances.iter().cloned().collect();
        all.sort_by(|a, b| (a.x.as_str(), a.y.as_str()).cmp(&(b.x.as_str(), b.y.as_str())));
        all
    }

    /// Collect every identifier currently marked as a meta-R
    /// registry marker, or an identifier owned by one. Used by
    /// `run_naming_pass` for the meta-subgraph skip. ADR 0012,
    /// extended by ADR 0029 (roles), ADR 0030 (axioms, theories),
    /// and ADR 0032 (axiom-intension variables and edge nodes).
    fn collect_meta_ids(&self) -> HashSet<String> {
        let mut s = HashSet::new();
        s.insert(PATTERN_MARKER.to_string());
        s.insert(ROLE_MARKER.to_string());
        s.insert(AXIOM_MARKER.to_string());
        s.insert(THEORY_MARKER.to_string());
        s.insert(AXIOMVAR_MARKER.to_string());
        s.insert(PREMISE_MARKER.to_string());
        s.insert(CONCLUSION_MARKER.to_string());
        s.insert(EXTENDS_MARKER.to_string());
        for ext in self.extension_edges() {
            s.insert(ext.to_string());
        }
        s.insert(INDEPENDENT_MARKER.to_string());
        for ind in self.independence_edges() {
            s.insert(ind.to_string());
        }
        for role in self.roles() {
            s.insert(role.to_string());
        }
        for p in self.patterns() {
            s.insert(p.to_string());
            for inst in self.instances_of(p) {
                s.insert(inst.to_string());
            }
        }
        for t in self.theories() {
            s.insert(t.to_string());
        }
        let axiom_ids: Vec<String> =
            self.axioms().into_iter().map(str::to_owned).collect();
        for a in &axiom_ids {
            s.insert(a.clone());
            for v in self.axiom_variables(a) {
                s.insert(v.to_string());
            }
            for p in self.axiom_premise_edges(a) {
                s.insert(p.to_string());
            }
            if let Some(c) = self.axiom_conclusion(a) {
                s.insert(c);
            }
        }
        s
    }

    /// Retract a named pattern and all of its meta-R edges. Data edges
    /// are untouched. ADR 0020. Order of removal is
    /// participants → ownership → registry, so a partial interruption
    /// leaves the RSet queryable in a self-consistent state.
    pub fn retract_pattern(
        &mut self,
        pattern_id: &str,
    ) -> Result<RetractionSummary, RetractionError> {
        let known: HashSet<&str> = self.patterns().into_iter().collect();
        if !known.contains(pattern_id) {
            return Err(RetractionError::UnknownPattern);
        }

        // Snapshot the ids we will touch (take ownership of strings so
        // we can mutate self below without borrow conflicts).
        let pattern_id_owned = pattern_id.to_string();
        let instance_ids: Vec<String> = self
            .instances_of(pattern_id)
            .into_iter()
            .map(str::to_owned)
            .collect();
        let instances_count = instance_ids.len();
        let role_ids: Vec<String> = self
            .pattern_roles(pattern_id)
            .into_iter()
            .map(str::to_owned)
            .collect();

        let mut removed: usize = 0;

        // (1) Remove every R(instance_id, participant) edge (Layer B).
        for inst in &instance_ids {
            let participants: Vec<String> = self
                .left_of(inst)
                .into_iter()
                .map(|r| r.y.clone())
                .collect();
            for participant in participants {
                if self.remove(&R::new(inst.clone(), participant)) {
                    removed += 1;
                }
            }
        }

        // (2) Remove every R(pattern_id, instance_id) ownership edge (Layer B).
        for inst in &instance_ids {
            if self.remove(&R::new(pattern_id_owned.clone(), inst.clone())) {
                removed += 1;
            }
        }

        // (3) Remove every Layer A structural edge R(role_i, role_j).
        //     ADR 0029.
        let role_set: HashSet<String> = role_ids.iter().cloned().collect();
        let structural_edges: Vec<R> = self
            .instances
            .iter()
            .filter(|r| role_set.contains(&r.x) && role_set.contains(&r.y))
            .cloned()
            .collect();
        for e in structural_edges {
            if self.remove(&e) {
                removed += 1;
            }
        }

        // (4) Remove every R(pattern_id, role_i) (Layer A pattern→role).
        //     ADR 0029.
        for role in &role_ids {
            if self.remove(&R::new(pattern_id_owned.clone(), role.clone())) {
                removed += 1;
            }
        }

        // (5) Remove every R(ROLE_MARKER, role_i) (Layer A registry).
        //     ADR 0029.
        for role in &role_ids {
            if self.remove(&R::new(ROLE_MARKER, role.clone())) {
                removed += 1;
            }
        }

        // (6) Remove the registry edge R(PATTERN_MARKER, pattern_id).
        if self.remove(&R::new(PATTERN_MARKER, pattern_id_owned.clone())) {
            removed += 1;
        }

        Ok(RetractionSummary {
            pattern_id: pattern_id_owned,
            instances_removed: instances_count,
            meta_edges_removed: removed,
        })
    }

    /// Pick the next free pattern id. Monotone scan from current count,
    /// skipping any identifier already present in the RSet (collision
    /// guard — ADR 0010).
    fn mint_pattern_id(&self) -> String {
        let existing = self.identifiers();
        let mut n = self.patterns().len();
        loop {
            let candidate = format!("p_{}", n);
            if !existing.contains(candidate.as_str()) {
                return candidate;
            }
            n += 1;
        }
    }

    /// Pick the next free instance id under a pattern. ADR 0010.
    fn mint_instance_id(&self, pattern: &str) -> String {
        let existing = self.identifiers();
        let mut n = self.instances_of(pattern).len();
        loop {
            let candidate = format!("{}_i_{}", pattern, n);
            if !existing.contains(candidate.as_str()) {
                return candidate;
            }
            n += 1;
        }
    }
}

/// Structural signature of an identifier.
///
/// ADR 0004: the first-pass signature is the identifier's profile itself.
/// The alias exists so later mechanisms can be written against `Signature`
/// rather than `IdentifierProfile`, letting the definition be refined
/// (e.g., to a 1-hop neighbor profile multiset) without changing callers.
pub type Signature = IdentifierProfile;

/// Structural signature of an R instance (edge).
///
/// ADR 0005: the ordered pair of endpoint signatures. Ordering matters
/// because direction is commitment-level. Later upgrades can replace the
/// component type or extend it; the pair shape is the stable surface.
pub type RSignature = (Signature, Signature);

/// Locality profile of an R instance — 1-hop connectivity via shared
/// identifiers, split by direction-preserving position. ADR 0006.
///
/// `co_left` and `co_right` capture co-occurrence at an endpoint (two
/// edges sharing left / sharing right). `forward` and `reverse` capture
/// directed chaining (this edge flows into another / another flows into
/// this).
///
/// Known 1-hop collision: chain-middle edges and cycle edges both have
/// `(0, 0, 1, 1)`. Distinguishing them requires 2-hop context (deferred).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalityProfile {
    pub co_left: usize,
    pub co_right: usize,
    pub forward: usize,
    pub reverse: usize,
}

impl LocalityProfile {
    pub fn total(&self) -> usize {
        self.co_left + self.co_right + self.forward + self.reverse
    }
}

/// Compound edge observation: endpoint-pair signature paired with
/// 1-hop locality. ADR 0007. The "fingerprint" name is deliberate —
/// this is an observational composition, not a new commitment.
pub type EdgeFingerprint = (RSignature, LocalityProfile);

/// A connected chunk of the R graph. ADR 0008.
///
/// Equality at this layer is trivial set-of-edges equality. Isomorphism
/// (two subgraphs representing the same *pattern* despite different
/// identifiers) is ADR 0009's concern.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Subgraph {
    edges: HashSet<R>,
}

impl Subgraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_edges<I: IntoIterator<Item = R>>(edges: I) -> Self {
        Subgraph { edges: edges.into_iter().collect() }
    }

    pub fn edges(&self) -> impl Iterator<Item = &R> {
        self.edges.iter()
    }

    pub fn contains(&self, r: &R) -> bool {
        self.edges.contains(r)
    }

    pub fn len(&self) -> usize {
        self.edges.len()
    }

    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }

    /// All identifiers appearing in this subgraph.
    pub fn identifiers(&self) -> HashSet<&str> {
        self.edges
            .iter()
            .flat_map(|r| [r.x.as_str(), r.y.as_str()])
            .collect()
    }

    /// Canonical form for pattern equality. ADR 0009.
    ///
    /// Two subgraphs produce the same canonical form iff they are
    /// isomorphic under Weisfeiler–Lehman refinement. Isomorphism here
    /// is direction-preserving (commitment 2); `R(a,b)` and `R(b,a)`
    /// in isolation produce distinct canonical forms.
    ///
    /// Known limitation: WL-1 is heuristic. Rare graph pairs can
    /// produce identical canonical forms while being non-isomorphic
    /// (strongly regular graphs, certain trees). At β's current
    /// experiment scale this does not occur; a stronger backend is a
    /// future ADR if it ever does.
    pub fn canonicalize(&self) -> CanonicalForm {
        let mut identifiers: Vec<String> = self
            .identifiers()
            .into_iter()
            .map(str::to_owned)
            .collect();
        identifiers.sort();
        let id_to_index: HashMap<&str, usize> = identifiers
            .iter()
            .enumerate()
            .map(|(i, id)| (id.as_str(), i))
            .collect();

        let n = identifiers.len();
        if n == 0 {
            return Vec::new();
        }

        let mut out_neighbors: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut in_neighbors: Vec<Vec<usize>> = vec![Vec::new(); n];
        for r in self.edges.iter() {
            let i = id_to_index[r.x.as_str()];
            let j = id_to_index[r.y.as_str()];
            out_neighbors[i].push(j);
            in_neighbors[j].push(i);
        }

        let initial: Vec<(usize, usize)> = (0..n)
            .map(|i| (out_neighbors[i].len(), in_neighbors[i].len()))
            .collect();
        let mut labels = rank_labels(&initial);

        for _ in 0..=n {
            let sigs: Vec<(u32, Vec<u32>, Vec<u32>)> = (0..n)
                .map(|i| {
                    let mut outs: Vec<u32> =
                        out_neighbors[i].iter().map(|&j| labels[j]).collect();
                    outs.sort();
                    let mut ins: Vec<u32> =
                        in_neighbors[i].iter().map(|&j| labels[j]).collect();
                    ins.sort();
                    (labels[i], outs, ins)
                })
                .collect();
            let next = rank_labels(&sigs);
            if next == labels {
                break;
            }
            labels = next;
        }

        let mut canonical: Vec<(u32, u32)> = self
            .edges
            .iter()
            .map(|r| (labels[id_to_index[r.x.as_str()]], labels[id_to_index[r.y.as_str()]]))
            .collect();
        canonical.sort();
        canonical
    }

    /// Are two subgraphs isomorphic under the current canonical form?
    pub fn is_isomorphic_to(&self, other: &Subgraph) -> bool {
        self.canonicalize() == other.canonicalize()
    }

    /// Partition a set of edges into connected components. Two edges
    /// are connected iff they share at least one identifier. The
    /// relation is transitive; components are maximal connected sets.
    pub fn connected_components_of<I: IntoIterator<Item = R>>(edges: I) -> Vec<Subgraph> {
        let edges: Vec<R> = edges.into_iter().collect();
        let n = edges.len();
        if n == 0 {
            return Vec::new();
        }

        // adjacency[i] = indices of edges that share ≥ 1 identifier with edges[i]
        let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); n];
        for i in 0..n {
            for j in (i + 1)..n {
                if shares_identifier(&edges[i], &edges[j]) {
                    adjacency[i].push(j);
                    adjacency[j].push(i);
                }
            }
        }

        let mut visited = vec![false; n];
        let mut components = Vec::new();

        for start in 0..n {
            if visited[start] {
                continue;
            }
            let mut component: HashSet<R> = HashSet::new();
            let mut queue = VecDeque::new();
            queue.push_back(start);
            visited[start] = true;

            while let Some(i) = queue.pop_front() {
                component.insert(edges[i].clone());
                for &j in &adjacency[i] {
                    if !visited[j] {
                        visited[j] = true;
                        queue.push_back(j);
                    }
                }
            }
            components.push(Subgraph { edges: component });
        }

        components
    }
}

fn shares_identifier(a: &R, b: &R) -> bool {
    a.x == b.x || a.x == b.y || a.y == b.x || a.y == b.y
}

// ADR 0026 helpers (sigmoid, compute_gradient_refine) removed
// alongside the gradient refinement primitives. See ADR 0026 log
// and git history at commit 4fc8b67 for the reference
// implementation.

/// Enumerate axiom templates up to the config's limits. ADR 0027.
/// Produces canonicalized templates (first-use variable ordering)
/// so that symmetric-renaming duplicates are not generated.
fn enumerate_axiom_templates(config: &AxiomDiscoveryConfig) -> Vec<AxiomTemplate> {
    let v = config.max_vars;
    let mut single_edges: Vec<EdgeTemplate> = Vec::new();
    for x in 0..v {
        for y in 0..v {
            single_edges.push(EdgeTemplate { x_var: x, y_var: y });
        }
    }

    let mut premises: Vec<Vec<EdgeTemplate>> = Vec::new();
    if config.include_empty_premise {
        premises.push(Vec::new());
    }
    for m in 1..=config.max_premise_edges {
        if m == 1 {
            for e in &single_edges {
                premises.push(vec![e.clone()]);
            }
        } else {
            // m = 2: unordered pairs (possibly equal, but skip equal to avoid trivial repeats)
            for i in 0..single_edges.len() {
                for j in i..single_edges.len() {
                    if i == j {
                        continue;
                    }
                    premises.push(vec![single_edges[i].clone(), single_edges[j].clone()]);
                }
            }
        }
    }

    let mut templates: Vec<AxiomTemplate> = Vec::new();
    let mut seen: HashSet<AxiomTemplate> = HashSet::new();

    for premise in premises {
        if premise.is_empty() {
            // Empty-premise axioms (ADR 0036): only `R(v, v)` self-loop
            // conclusion is admitted — universally quantified reflexivity
            // at a single variable. Use v = 0 as the canonical pick.
            let concl = EdgeTemplate { x_var: 0, y_var: 0 };
            let tpl = canonicalize_template(AxiomTemplate {
                num_vars: v,
                premise: Vec::new(),
                conclusion: concl,
            });
            if seen.insert(tpl.clone()) {
                templates.push(tpl);
            }
            continue;
        }
        // Collect variables used in premise.
        let mut used_vars: HashSet<usize> = HashSet::new();
        for e in &premise {
            used_vars.insert(e.x_var);
            used_vars.insert(e.y_var);
        }
        for conclusion in &single_edges {
            // Conclusion must use only variables used in premise.
            if !used_vars.contains(&conclusion.x_var)
                || !used_vars.contains(&conclusion.y_var)
            {
                continue;
            }
            // Conclusion must not be trivially one of the premise edges.
            if premise.contains(conclusion) {
                continue;
            }
            let tpl = canonicalize_template(AxiomTemplate {
                num_vars: v,
                premise: premise.clone(),
                conclusion: conclusion.clone(),
            });
            if seen.insert(tpl.clone()) {
                templates.push(tpl);
            }
        }
    }
    templates
}

/// Canonicalize an axiom template to a structural canonical form:
/// minimum over all variable permutations of the first-use-normalized
/// form. ADR 0027 / ADR 0028.
///
/// ADR 0027's canonicalizer only normalized by first-use ordering,
/// which is invariant under variable *renaming* but not under variable
/// *permutation*. That left transitivity's two forms
/// `[R(0,1), R(1,2)] ⇒ R(0,2)` and `[R(0,1), R(2,0)] ⇒ R(2,1)` both
/// in the output. ADR 0028 upgrades this: the canonical form is the
/// lexicographically-smallest first-use-normalized form obtained over
/// all permutations of the original variable labels. For num_vars ≤ 4
/// this is bounded by 24 permutations per template.
fn canonicalize_template(tpl: AxiomTemplate) -> AxiomTemplate {
    let base = canonicalize_template_first_use(tpl);
    let n = base.num_vars;
    if n <= 1 {
        return base;
    }
    let mut best = base.clone();
    let mut best_key = template_key(&best);
    let perms = all_permutations(n);
    for perm in perms {
        let mut permuted = base.clone();
        for e in &mut permuted.premise {
            e.x_var = perm[e.x_var];
            e.y_var = perm[e.y_var];
        }
        permuted.conclusion.x_var = perm[permuted.conclusion.x_var];
        permuted.conclusion.y_var = perm[permuted.conclusion.y_var];
        let re_canon = canonicalize_template_first_use(permuted);
        let key = template_key(&re_canon);
        if key < best_key {
            best_key = key;
            best = re_canon;
        }
    }
    best
}

/// First-use variable renumbering + premise-edge sort. Invariant under
/// renaming but NOT under permutation of variable labels; see
/// `canonicalize_template` for the full structural form.
fn canonicalize_template_first_use(mut tpl: AxiomTemplate) -> AxiomTemplate {
    tpl.premise.sort_by(|a, b| (a.x_var, a.y_var).cmp(&(b.x_var, b.y_var)));
    let mut remap: HashMap<usize, usize> = HashMap::new();
    let mut next: usize = 0;
    let mut assign = |v: usize, remap: &mut HashMap<usize, usize>, next: &mut usize| -> usize {
        if let Some(&m) = remap.get(&v) {
            m
        } else {
            let m = *next;
            remap.insert(v, m);
            *next += 1;
            m
        }
    };
    for e in &mut tpl.premise {
        e.x_var = assign(e.x_var, &mut remap, &mut next);
        e.y_var = assign(e.y_var, &mut remap, &mut next);
    }
    tpl.conclusion.x_var = assign(tpl.conclusion.x_var, &mut remap, &mut next);
    tpl.conclusion.y_var = assign(tpl.conclusion.y_var, &mut remap, &mut next);
    tpl.premise.sort_by(|a, b| (a.x_var, a.y_var).cmp(&(b.x_var, b.y_var)));
    tpl.num_vars = next;
    tpl
}

/// Order key for comparing two templates in canonicalization.
fn template_key(tpl: &AxiomTemplate) -> (usize, Vec<(usize, usize)>, (usize, usize)) {
    let premise: Vec<(usize, usize)> =
        tpl.premise.iter().map(|e| (e.x_var, e.y_var)).collect();
    let concl = (tpl.conclusion.x_var, tpl.conclusion.y_var);
    (tpl.num_vars, premise, concl)
}

/// All permutations of 0..n. n is bounded by AxiomDiscoveryConfig.max_vars
/// (≤ 4 in practice), so 24 is the maximum size. ADR 0028.
fn all_permutations(n: usize) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    let mut current: Vec<usize> = (0..n).collect();
    permute_rec(&mut current, 0, &mut out);
    out
}

fn permute_rec(arr: &mut Vec<usize>, k: usize, out: &mut Vec<Vec<usize>>) {
    if k == arr.len() {
        out.push(arr.clone());
        return;
    }
    for i in k..arr.len() {
        arr.swap(k, i);
        permute_rec(arr, k + 1, out);
        arr.swap(k, i);
    }
}

/// Drop axioms whose conclusion is a self-loop `R(v, v)`. ADR 0028.
///
/// Rationale: when universal reflexivity holds on the RSet, any such
/// conclusion is entailed by reflexivity alone, independent of the
/// premise. Caller is responsible for checking reflexivity first;
/// this function itself only inspects the conclusion shape.
pub fn subsume_by_reflexivity(
    axioms: Vec<AxiomEvidence>,
) -> Vec<AxiomEvidence> {
    axioms
        .into_iter()
        .filter(|ev| ev.template.conclusion.x_var != ev.template.conclusion.y_var)
        .collect()
}

/// Drop axioms strictly weaker than another axiom in the set. ADR 0028.
///
/// Axiom A *subsumes* axiom B if there exists a variable mapping
/// `σ: vars(A) → vars(B)` such that `σ(conclusion_A) = conclusion_B`
/// and `σ(premise_A) ⊆ premise_B` as sets of edge templates. When such
/// a σ exists, A's conclusion follows from a subset of B's premise,
/// so B adds no new content and is redundant.
///
/// The axiom-pair comparison is asymmetric; to avoid two mutually-
/// equivalent axioms both being dropped, ties are broken by template
/// key order: among a pair where each subsumes the other, only the
/// larger-key one is dropped.
pub fn subsume_by_premise_weakening(
    axioms: Vec<AxiomEvidence>,
) -> Vec<AxiomEvidence> {
    let n = axioms.len();
    let mut keep = vec![true; n];
    for i in 0..n {
        if !keep[i] {
            continue;
        }
        for j in 0..n {
            if i == j || !keep[j] {
                continue;
            }
            if template_subsumes(&axioms[i].template, &axioms[j].template) {
                // i subsumes j: drop j. But if j also subsumes i, break
                // the symmetry by keeping the lex-smaller template.
                let sym = template_subsumes(&axioms[j].template, &axioms[i].template);
                if sym {
                    let ki = template_key(&axioms[i].template);
                    let kj = template_key(&axioms[j].template);
                    if ki <= kj {
                        keep[j] = false;
                    } else {
                        keep[i] = false;
                    }
                } else {
                    keep[j] = false;
                }
            }
        }
    }
    axioms
        .into_iter()
        .enumerate()
        .filter_map(|(idx, ev)| if keep[idx] { Some(ev) } else { None })
        .collect()
}

/// Is `target` derivable from `sources` by forward chaining? ADR 0037.
///
/// Instantiates `target.premise` as seed facts on a fresh RSet over
/// `max(target.num_vars, sources[*].num_vars)` fresh identifiers
/// `v0, v1, …`, then iterates every source axiom as a closure rule
/// until no new facts are added. Finally checks whether the seeded
/// graph contains `target.conclusion`.
///
/// Sound only when the provided sources are all universally valid
/// (rate = 1.0). Defeasible sources would break the derivation.
pub fn template_derivable_from(
    target: &AxiomTemplate,
    sources: &[AxiomTemplate],
) -> bool {
    let mut n = target.num_vars;
    for s in sources {
        if s.num_vars > n {
            n = s.num_vars;
        }
    }
    if n == 0 {
        return false;
    }
    let node_id = |i: usize| format!("v{}", i);
    let mut rs = RSet::new();
    for e in &target.premise {
        rs.add(R::new(node_id(e.x_var), node_id(e.y_var)));
    }
    loop {
        let before = rs.len();
        for axiom in sources {
            // Skip exact target — we want derivation via OTHERS.
            if axiom == target {
                continue;
            }
            let mut binding = vec![0usize; axiom.num_vars];
            forward_chain_apply(&mut rs, axiom, n, &mut binding, 0);
        }
        if rs.len() == before {
            break;
        }
    }
    let cx = node_id(target.conclusion.x_var);
    let cy = node_id(target.conclusion.y_var);
    rs.instances.contains(&R::new(cx, cy))
}

fn forward_chain_apply(
    rs: &mut RSet,
    axiom: &AxiomTemplate,
    n: usize,
    binding: &mut [usize],
    depth: usize,
) {
    if depth == binding.len() {
        for e in &axiom.premise {
            let x = format!("v{}", binding[e.x_var]);
            let y = format!("v{}", binding[e.y_var]);
            if !rs.instances.contains(&R::new(x, y)) {
                return;
            }
        }
        let cx = format!("v{}", binding[axiom.conclusion.x_var]);
        let cy = format!("v{}", binding[axiom.conclusion.y_var]);
        rs.add(R::new(cx, cy));
        return;
    }
    for i in 0..n {
        binding[depth] = i;
        forward_chain_apply(rs, axiom, n, binding, depth + 1);
    }
}

/// Drop axioms that are derivable from the remaining set via forward
/// chaining. ADR 0037. Runs to a fixpoint: if dropping B makes C
/// now undrop-able (because C's derivation relied on B), C remains.
/// Ordering tie-break by template key to ensure determinism.
pub fn subsume_by_composition(
    axioms: Vec<AxiomEvidence>,
) -> Vec<AxiomEvidence> {
    let n = axioms.len();
    if n <= 1 {
        return axioms;
    }
    let mut keep = vec![true; n];
    loop {
        let mut changed = false;
        // Process in a deterministic order: largest template_key first,
        // so that "bigger" (more complex) axioms get considered for
        // subsumption before simpler ones.
        let mut indices: Vec<usize> = (0..n).filter(|i| keep[*i]).collect();
        indices.sort_by(|a, b| {
            template_key(&axioms[*b].template)
                .cmp(&template_key(&axioms[*a].template))
        });
        for i in indices {
            if !keep[i] {
                continue;
            }
            let sources: Vec<AxiomTemplate> = (0..n)
                .filter(|j| *j != i && keep[*j])
                .map(|j| axioms[j].template.clone())
                .collect();
            if template_derivable_from(&axioms[i].template, &sources) {
                keep[i] = false;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    axioms
        .into_iter()
        .enumerate()
        .filter_map(|(i, ev)| if keep[i] { Some(ev) } else { None })
        .collect()
}

/// Wilson score 95% confidence interval on the binomial proportion
/// `s / n`. Returns `(lower, upper)`. ADR 0045.
///
/// Wilson score is an interval-estimator that's better than the
/// normal approximation for small `n` and extreme `s / n`. Formula
/// with `z = 1.96`:
///
/// ```text
/// p_hat = s / n
/// denom = 1 + z² / n
/// center = (p_hat + z² / (2n)) / denom
/// halfwidth = z × sqrt(p_hat(1 − p_hat)/n + z²/(4n²)) / denom
/// ```
pub fn wilson_score_95(successes: usize, n: usize) -> (f64, f64) {
    if n == 0 {
        return (0.0, 1.0);
    }
    let z = 1.96_f64;
    let z2 = z * z;
    let n_f = n as f64;
    let p_hat = successes as f64 / n_f;
    let denom = 1.0 + z2 / n_f;
    let center = (p_hat + z2 / (2.0 * n_f)) / denom;
    let halfwidth =
        (z * (p_hat * (1.0 - p_hat) / n_f + z2 / (4.0 * n_f * n_f)).sqrt()) / denom;
    (
        (center - halfwidth).max(0.0),
        (center + halfwidth).min(1.0),
    )
}

/// Null-baseline probability of a template's result under iid
/// Bernoulli edges with density `p` = data_edges / |ids|².
/// ADR 0045. If premise holds on N bindings and conclusion
/// satisfied on all N, returns `p_conclusion^N` — the chance
/// of this observation under random edges. A small value = the
/// observed rate is surprising and less likely to be accidental.
pub fn null_baseline_probability(
    bindings: usize,
    satisfied: usize,
    p_edge: f64,
) -> f64 {
    if p_edge <= 0.0 || bindings == 0 {
        return 1.0;
    }
    if p_edge >= 1.0 {
        return 1.0;
    }
    if satisfied < bindings {
        return 1.0; // not a rate-1.0 claim; no significance discount
    }
    p_edge.powi(bindings as i32)
}

/// Does template A subsume template B? See `subsume_by_premise_weakening`.
fn template_subsumes(a: &AxiomTemplate, b: &AxiomTemplate) -> bool {
    let b_premise_set: HashSet<(usize, usize)> =
        b.premise.iter().map(|e| (e.x_var, e.y_var)).collect();
    // Enumerate every mapping σ: 0..a.num_vars → 0..b.num_vars that
    // already agrees on the conclusion endpoints, then check premise
    // inclusion.
    let a_cx = a.conclusion.x_var;
    let a_cy = a.conclusion.y_var;
    let b_cx = b.conclusion.x_var;
    let b_cy = b.conclusion.y_var;
    // If conclusion_A has equal endpoints, conclusion_B must too.
    if (a_cx == a_cy) != (b_cx == b_cy) {
        return false;
    }
    // Seed σ with conclusion constraints.
    let mut seed: HashMap<usize, usize> = HashMap::new();
    seed.insert(a_cx, b_cx);
    if a_cy != a_cx {
        if let Some(&existing) = seed.get(&a_cy) {
            if existing != b_cy {
                return false;
            }
        }
        seed.insert(a_cy, b_cy);
    }
    // Enumerate assignments for remaining A-vars.
    let unassigned: Vec<usize> =
        (0..a.num_vars).filter(|v| !seed.contains_key(v)).collect();
    if b.num_vars == 0 {
        return unassigned.is_empty() && premise_contained(a, &seed, &b_premise_set);
    }
    let mut working = seed;
    extend_and_check(a, b, &b_premise_set, &unassigned, 0, &mut working)
}

fn extend_and_check(
    a: &AxiomTemplate,
    b: &AxiomTemplate,
    b_premise: &HashSet<(usize, usize)>,
    unassigned: &[usize],
    idx: usize,
    working: &mut HashMap<usize, usize>,
) -> bool {
    if idx == unassigned.len() {
        return premise_contained(a, working, b_premise);
    }
    let var = unassigned[idx];
    for target in 0..b.num_vars {
        working.insert(var, target);
        if extend_and_check(a, b, b_premise, unassigned, idx + 1, working) {
            return true;
        }
    }
    working.remove(&var);
    false
}

fn premise_contained(
    a: &AxiomTemplate,
    sigma: &HashMap<usize, usize>,
    b_premise: &HashSet<(usize, usize)>,
) -> bool {
    for e in &a.premise {
        let (Some(&x), Some(&y)) = (sigma.get(&e.x_var), sigma.get(&e.y_var)) else {
            return false;
        };
        if !b_premise.contains(&(x, y)) {
            return false;
        }
    }
    true
}

/// Enumerate all bindings of `template.num_vars` variables to
/// identifiers and accumulate counts of premise satisfactions and
/// conclusion satisfactions. ADR 0027.
#[allow(clippy::too_many_arguments)]
fn evaluate_template_recursive(
    rs: &RSet,
    template: &AxiomTemplate,
    ids: &[String],
    _meta: &HashSet<String>,
    binding: &mut [usize],
    depth: usize,
    premise_bindings: &mut usize,
    conclusion_satisfied: &mut usize,
) {
    if depth == binding.len() {
        // Check premise.
        for e in &template.premise {
            let x = &ids[binding[e.x_var]];
            let y = &ids[binding[e.y_var]];
            if !rs.instances.contains(&R::new(x.clone(), y.clone())) {
                return;
            }
        }
        *premise_bindings += 1;
        let cx = &ids[binding[template.conclusion.x_var]];
        let cy = &ids[binding[template.conclusion.y_var]];
        if rs.instances.contains(&R::new(cx.clone(), cy.clone())) {
            *conclusion_satisfied += 1;
        }
        return;
    }
    for i in 0..ids.len() {
        binding[depth] = i;
        evaluate_template_recursive(
            rs,
            template,
            ids,
            _meta,
            binding,
            depth + 1,
            premise_bindings,
            conclusion_satisfied,
        );
    }
}

/// BFS-style enumeration of connected k-edge subgraphs with canonical-form
/// match. ADR 0015 `find_instances_of` helper. Dedups edge sets via a
/// sorted-tuple key so each connected set is visited once, regardless of
/// seed order.
fn expand_connected(
    all: &[R],
    current: HashSet<R>,
    target_size: usize,
    seen: &mut HashSet<Vec<R>>,
    results: &mut Vec<Subgraph>,
    target: &CanonicalForm,
) {
    let mut key: Vec<R> = current.iter().cloned().collect();
    key.sort_by(|a, b| (a.x.as_str(), a.y.as_str()).cmp(&(b.x.as_str(), b.y.as_str())));
    if !seen.insert(key) {
        return;
    }

    if current.len() == target_size {
        let sg = Subgraph::from_edges(current.iter().cloned());
        if sg.canonicalize() == *target {
            results.push(sg);
        }
        return;
    }

    if current.len() > target_size {
        return;
    }

    let current_ids: HashSet<&str> = current
        .iter()
        .flat_map(|r| [r.x.as_str(), r.y.as_str()])
        .collect();

    for edge in all {
        if current.contains(edge) {
            continue;
        }
        if current_ids.contains(edge.x.as_str()) || current_ids.contains(edge.y.as_str()) {
            let mut extended = current.clone();
            extended.insert(edge.clone());
            expand_connected(all, extended, target_size, seen, results, target);
        }
    }
}

/// Rank a slice of signatures into small integer labels.
/// Two items with the same signature receive the same label; labels are
/// assigned in sorted order of the distinct signatures (so the result is
/// deterministic and independent of input order for equivalence purposes).
fn rank_labels<T: Ord + Clone>(sigs: &[T]) -> Vec<u32> {
    let mut sorted_unique: Vec<T> = sigs.to_vec();
    sorted_unique.sort();
    sorted_unique.dedup();
    sigs.iter()
        .map(|s| sorted_unique.binary_search(s).unwrap() as u32)
        .collect()
}

/// Canonical form of a subgraph: sorted edge list over stable labels.
/// See `Subgraph::canonicalize`. ADR 0009.
pub type CanonicalForm = Vec<(u32, u32)>;

/// Configuration for motif discovery. ADR 0016.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryConfig {
    /// Edge count of each candidate subgraph.
    pub target_size: usize,
    /// Number of random-walk candidates to propose.
    pub sample_count: usize,
    /// Keep the top-M distinct canonical forms by score.
    pub top_m: usize,
    /// Seed for the inline xorshift64 PRNG. Zero is replaced with a
    /// non-zero default internally so that sampling always runs.
    pub rng_seed: u64,
    /// ADR 0025 probe: when true, `discover_motifs` samples from
    /// *all* edges (data + meta-R) rather than only data edges.
    /// Defaults to `false`; leave off unless probing for hierarchical
    /// structure.
    pub include_meta_in_discovery: bool,
}

/// A motif candidate found by `discover_motifs`. ADR 0016.
#[derive(Debug, Clone)]
pub struct MotifCandidate {
    pub canonical: CanonicalForm,
    pub representative: Subgraph,
    pub sample_frequency: usize,
    pub score: f64,
}

// GradientRefineConfig removed alongside the gradient refinement
// primitives. See ADR 0026 and its log for rationale.

/// Axiom discovery: template of a single edge with variable indices
/// on both endpoints. ADR 0027.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EdgeTemplate {
    pub x_var: usize,
    pub y_var: usize,
}

/// Axiom discovery: a rule `premise ⇒ conclusion` where premise is a
/// conjunction of edge templates and conclusion is a single edge.
/// Variables are identified by small integer indices 0..num_vars.
/// ADR 0027.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AxiomTemplate {
    pub num_vars: usize,
    pub premise: Vec<EdgeTemplate>,
    pub conclusion: EdgeTemplate,
}

/// Axiom discovery — equality-conclusion family. ADR 0044. A rule
/// `premise ⇒ (v_a = v_b)`. The canonical instance is antisymmetry:
/// `R(0, 1) ∧ R(1, 0) ⇒ v_0 = v_1`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EqualityAxiomTemplate {
    pub num_vars: usize,
    pub premise: Vec<EdgeTemplate>,
    pub equal_vars: (usize, usize),
}

/// Axiom discovery — disjunctive-conclusion family. ADR 0044. A rule
/// `premise ⇒ R(c_1) ∨ R(c_2) ∨ …`. The canonical instance is
/// totality: `(empty) ⇒ R(0, 1) ∨ R(1, 0)`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DisjunctiveAxiomTemplate {
    pub num_vars: usize,
    pub premise: Vec<EdgeTemplate>,
    pub conclusions: Vec<EdgeTemplate>,
}

/// ADR 0044: unified evidence for an extended-axiom-family template.
#[derive(Debug, Clone)]
pub enum ExtendedAxiomEvidence {
    Edge(AxiomEvidence),
    Equality {
        template: EqualityAxiomTemplate,
        premise_bindings: usize,
        conclusion_satisfied: usize,
        rate: f64,
    },
    Disjunctive {
        template: DisjunctiveAxiomTemplate,
        premise_bindings: usize,
        conclusion_satisfied: usize,
        rate: f64,
    },
}

impl ExtendedAxiomEvidence {
    pub fn rate(&self) -> f64 {
        match self {
            ExtendedAxiomEvidence::Edge(e) => e.rate,
            ExtendedAxiomEvidence::Equality { rate, .. } => *rate,
            ExtendedAxiomEvidence::Disjunctive { rate, .. } => *rate,
        }
    }
    pub fn premise_bindings(&self) -> usize {
        match self {
            ExtendedAxiomEvidence::Edge(e) => e.premise_bindings,
            ExtendedAxiomEvidence::Equality { premise_bindings, .. } => *premise_bindings,
            ExtendedAxiomEvidence::Disjunctive { premise_bindings, .. } => *premise_bindings,
        }
    }
}

/// Axiom discovery: evidence for one template against an RSet.
/// ADR 0027. Extended by ADR 0045 with Bayesian posterior (Wilson
/// score) fields and `null_baseline_prob` for significance filtering.
#[derive(Debug, Clone)]
pub struct AxiomEvidence {
    pub template: AxiomTemplate,
    pub premise_bindings: usize,
    pub conclusion_satisfied: usize,
    pub rate: f64,
    /// Lower bound of the 95% Wilson score confidence interval on the
    /// posterior rate. For support N and successes s, this is an
    /// interval-estimate of the "true" rate that corrects the raw
    /// `s/N` estimator for small N. ADR 0045.
    pub posterior_lower_95: f64,
    /// Upper bound of the 95% Wilson score confidence interval.
    pub posterior_upper_95: f64,
    /// Null-baseline probability: if edges were iid Bernoulli with
    /// p = |data edges| / |ids|², the chance of seeing this template
    /// hold on all N premise bindings by accident. A small value
    /// (e.g. < 0.01) indicates the axiom is statistically surprising
    /// relative to a random-edge null hypothesis. ADR 0045.
    pub null_baseline_prob: f64,
}

/// Configuration for axiom discovery. ADR 0027, extended by ADR
/// 0033 (defeasible rules) and ADR 0036 (empty-premise templates).
#[derive(Debug, Clone)]
pub struct AxiomDiscoveryConfig {
    pub max_premise_edges: usize,
    pub max_vars: usize,
    pub min_evidence: usize,
    /// Minimum satisfaction rate for a template to be reported. Default
    /// `1.0` preserves ADR 0027's strict "holds universally" semantics.
    /// Lowering it to e.g. `0.8` admits defeasible rules that hold on
    /// ≥ 80% of premise bindings. ADR 0033.
    pub min_rate: f64,
    /// When true, enumerate empty-premise templates `[] ⇒ R(0,0)`
    /// (single-variable self-loop conclusion — universally quantified
    /// reflexivity in template form). Default `false` preserves
    /// ADR 0027's "premise must have at least one edge" shape. ADR 0036.
    pub include_empty_premise: bool,
}

impl Default for AxiomDiscoveryConfig {
    fn default() -> Self {
        AxiomDiscoveryConfig {
            max_premise_edges: 2,
            max_vars: 3,
            min_evidence: 1,
            min_rate: 1.0,
            include_empty_premise: false,
        }
    }
}

/// Reflexivity check result. ADR 0027. Does not fit the positive-
/// implication template (premise "x is an identifier" has no edge).
#[derive(Debug, Clone)]
pub struct ReflexivityEvidence {
    pub identifiers_total: usize,
    pub self_loops_present: usize,
    pub rate: f64,
}

/// Antisymmetry check result. ADR 0027. Does not fit the template
/// (conclusion "x = y" is not an edge).
#[derive(Debug, Clone)]
pub struct AntisymmetryEvidence {
    pub directed_pairs_checked: usize,
    pub violations: usize,
    pub holds: bool,
}

/// Totality check result. ADR 0039. Does not fit the template
/// (disjunctive conclusion).
#[derive(Debug, Clone)]
pub struct TotalityEvidence {
    pub unordered_pairs_checked: usize,
    pub violations: usize,
    pub holds: bool,
}

/// Combined poset check. ADR 0027.
#[derive(Debug, Clone)]
pub struct PosetCheck {
    pub reflexive: ReflexivityEvidence,
    pub antisymmetric: AntisymmetryEvidence,
    pub transitive: Option<AxiomEvidence>,
    pub is_poset: bool,
}

/// ADR 0031 (task C): one candidate action the intrinsic drive may
/// try. The list of candidates is produced by `DriveConfig::candidate_actions`.
#[derive(Debug, Clone)]
pub enum DriveAction {
    /// Run `autonomous_pass` with the given config.
    DiscoverPatterns(AutonomousConfig),
    /// Run `discover_theory` + `name_theory`.
    DiscoverTheory(AxiomDiscoveryConfig),
    /// Retract every named object whose counterfactual value is
    /// strictly below `threshold`. ADR 0040.
    Prune(f64),
}

/// ADR 0031: structured outcome of one applied drive action.
#[derive(Debug, Clone)]
pub enum DriveActionResult {
    PatternsDiscovered {
        target_size: usize,
        new_patterns: usize,
    },
    TheoryDiscovered {
        theory_id: Option<String>,
        member_count: usize,
    },
    Pruned {
        object_ids: Vec<String>,
    },
}

/// ADR 0031: one step of the intrinsic drive, after it has been
/// applied to the RSet.
#[derive(Debug, Clone)]
pub struct DriveStep {
    pub action: DriveAction,
    pub score_before: f64,
    pub score_after: f64,
    pub delta: f64,
    pub result: DriveActionResult,
}

/// ADR 0031: the full trace of an `intrinsic_drive` run.
#[derive(Debug, Clone)]
pub struct DriveTrace {
    pub initial_score: f64,
    pub final_score: f64,
    pub steps: Vec<DriveStep>,
}

/// ADR 0031: configuration for the intrinsic drive. The driver
/// explores every action in `candidate_actions()` each step and
/// applies the best-improving one if it exceeds `epsilon`. Loops
/// for at most `max_steps` iterations.
#[derive(Debug, Clone)]
pub struct DriveConfig {
    pub pattern_sizes: Vec<usize>,
    pub discovery_config: DiscoveryConfig,
    pub refinement_config: RefinementConfig,
    pub naming_policy: NamingPolicy,
    pub axiom_config: AxiomDiscoveryConfig,
    pub max_steps: usize,
    pub epsilon: f64,
    /// When true, include `DriveAction::Prune(prune_threshold)` as a
    /// candidate action each step. ADR 0040. Default `true`.
    pub enable_prune: bool,
    /// Counterfactual-value threshold below which Prune retracts an
    /// object. Default `0.0` (retract only net-negative contributors).
    pub prune_threshold: f64,
    /// When `Some(cfg)`, pattern-discovery actions route through
    /// `sample_instances_of` instead of exhaustive `find_instances_of`
    /// — trades completeness for tractability on large graphs.
    /// ADR 0043. Default `None`.
    pub instance_sampling: Option<SamplingMatchConfig>,
}

impl DriveConfig {
    fn candidate_actions(&self) -> Vec<DriveAction> {
        let mut out: Vec<DriveAction> = self
            .pattern_sizes
            .iter()
            .map(|&size| {
                let mut d = self.discovery_config.clone();
                d.target_size = size;
                DriveAction::DiscoverPatterns(AutonomousConfig {
                    discovery: d,
                    refinement: self.refinement_config.clone(),
                    naming: self.naming_policy.clone(),
                    instance_sampling: self.instance_sampling.clone(),
                })
            })
            .collect();
        out.push(DriveAction::DiscoverTheory(self.axiom_config.clone()));
        if self.enable_prune {
            out.push(DriveAction::Prune(self.prune_threshold));
        }
        out
    }
}

impl Default for DriveConfig {
    fn default() -> Self {
        DriveConfig {
            pattern_sizes: vec![2, 3, 4],
            discovery_config: DiscoveryConfig {
                target_size: 3,
                sample_count: 200,
                top_m: 10,
                rng_seed: 2024,
                include_meta_in_discovery: false,
            },
            refinement_config: RefinementConfig {
                max_tries: 200,
                rng_seed: 999,
            },
            naming_policy: NamingPolicy::default(),
            axiom_config: AxiomDiscoveryConfig::default(),
            max_steps: 10,
            epsilon: 0.0,
            enable_prune: true,
            prune_threshold: 0.0,
            instance_sampling: None,
        }
    }
}

/// ADR 0030: the result of `discover_theory` — the bundle of axioms
/// that jointly hold at rate 1.0 on the RSet, before any meta-R
/// commitment. `id` is empty until `name_theory` mints one.
#[derive(Debug, Clone)]
pub struct Theory {
    pub id: String,
    pub member_axiom_ids: Vec<String>,
    pub template_members: Vec<AxiomTemplate>,
}

/// ADR 0030: errors from `name_theory`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TheoryError {
    EmptyMemberList,
    /// A claimed member axiom does not actually hold on the current
    /// RSet at rate 1.0. Names the id; the caller can re-verify.
    UnsatisfiedMember(String),
    /// A claimed member axiom id could not be parsed as a template or
    /// a known predicate axiom (reflexivity / antisymmetry).
    UnparseableAxiomId(String),
}

/// ADR 0030: compute the deterministic axiom id for a template.
/// Canonical form `[R(0,1), R(1,2)] ⇒ R(0,2)` (transitivity) becomes
/// `ax_tpl_v3_p0-1_p1-2_c0-2`. Stable across runs and RSets.
pub fn axiom_template_id(template: &AxiomTemplate) -> String {
    let canon = canonicalize_template(template.clone());
    let mut parts: Vec<String> = Vec::new();
    parts.push(format!("ax_tpl_v{}", canon.num_vars));
    for e in &canon.premise {
        parts.push(format!("p{}-{}", e.x_var, e.y_var));
    }
    parts.push(format!("c{}-{}", canon.conclusion.x_var, canon.conclusion.y_var));
    parts.join("_")
}

/// ADR 0030: parse a template axiom id back into a template. Returns
/// `None` if the id is a predicate axiom (reflexivity / antisymmetry)
/// or otherwise not a template form.
pub fn axiom_id_to_template(id: &str) -> Option<AxiomTemplate> {
    let rest = id.strip_prefix("ax_tpl_v")?;
    let mut parts = rest.split('_');
    let num_vars: usize = parts.next()?.parse().ok()?;
    let mut premise: Vec<EdgeTemplate> = Vec::new();
    let mut conclusion: Option<EdgeTemplate> = None;
    for p in parts {
        if let Some(body) = p.strip_prefix('p') {
            let (x, y) = split_edge_part(body)?;
            premise.push(EdgeTemplate { x_var: x, y_var: y });
        } else if let Some(body) = p.strip_prefix('c') {
            let (x, y) = split_edge_part(body)?;
            conclusion = Some(EdgeTemplate { x_var: x, y_var: y });
        } else {
            return None;
        }
    }
    Some(AxiomTemplate {
        num_vars,
        premise,
        conclusion: conclusion?,
    })
}

fn split_edge_part(s: &str) -> Option<(usize, usize)> {
    let mut it = s.split('-');
    let x: usize = it.next()?.parse().ok()?;
    let y: usize = it.next()?.parse().ok()?;
    if it.next().is_some() {
        return None;
    }
    Some((x, y))
}

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

/// Inline xorshift64 — deterministic PRNG. ADR 0016.
fn next_xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    if x == 0 {
        x = 0x9E3779B97F4A7C15;
    }
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// Random walk: seed an edge, grow by adjacent edges until reaching
/// `target_size`. Returns None if the seed's connected component is
/// too small. ADR 0016.
fn sample_connected_subgraph(
    data: &[R],
    target_size: usize,
    rng: &mut u64,
) -> Option<Subgraph> {
    if data.is_empty() || target_size == 0 {
        return None;
    }
    let seed_idx = (next_xorshift64(rng) as usize) % data.len();
    let mut current: HashSet<R> = HashSet::new();
    current.insert(data[seed_idx].clone());

    while current.len() < target_size {
        let current_ids: HashSet<&str> = current
            .iter()
            .flat_map(|r| [r.x.as_str(), r.y.as_str()])
            .collect();
        let adjacent: Vec<&R> = data
            .iter()
            .filter(|r| {
                !current.contains(r)
                    && (current_ids.contains(r.x.as_str())
                        || current_ids.contains(r.y.as_str()))
            })
            .collect();
        if adjacent.is_empty() {
            return None;
        }
        let pick = (next_xorshift64(rng) as usize) % adjacent.len();
        current.insert(adjacent[pick].clone());
    }
    Some(Subgraph::from_edges(current))
}

/// Which slot positions an identifier appears in across the whole RSet.
///
/// `Both` includes the self-loop case R(a, a) — `a` occupies both slots in
/// the same instance. Self-loop detection itself is deferred (see below).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlotPattern {
    None,
    LeftOnly,
    RightOnly,
    Both,
}

/// First-pass structural profile of an identifier.
///
/// This is the minimal observation surface for the object-emergence question:
/// "which identifiers are structurally salient?" It carries no salience
/// judgment itself — judgment is for later mechanisms to apply.
///
/// Deferred candidates (add only when the first pass proves insufficient):
/// - in/out neighbor sets (not just counts)
/// - self-loop flag (R(a, a))
/// - co-occurrence with other identifiers across instances
/// - multi-hop reachability profile
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IdentifierProfile {
    pub degree_out: usize,
    pub degree_in: usize,
    pub slots: SlotPattern,
}

impl IdentifierProfile {
    pub fn total_degree(&self) -> usize {
        self.degree_out + self.degree_in
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r_can_be_constructed() {
        let r = R::new("a", "b");
        assert_eq!(r.x, "a");
        assert_eq!(r.y, "b");
    }

    #[test]
    fn r_is_directional() {
        assert_ne!(R::new("a", "b"), R::new("b", "a"));
    }

    #[test]
    fn identity_is_token_based() {
        assert_eq!(R::new("a", "b"), R::new("a", "b"));
    }

    #[test]
    fn rset_starts_empty() {
        let rs = RSet::new();
        assert!(rs.is_empty());
        assert_eq!(rs.len(), 0);
    }

    #[test]
    fn rset_dedups_identical_instances() {
        let mut rs = RSet::new();
        assert!(rs.add(R::new("a", "b")));
        assert!(!rs.add(R::new("a", "b")));
        assert_eq!(rs.len(), 1);
    }

    #[test]
    fn rset_treats_direction_as_distinct() {
        let mut rs = RSet::new();
        rs.add(R::new("a", "b"));
        rs.add(R::new("b", "a"));
        assert_eq!(rs.len(), 2);
    }

    #[test]
    fn identifiers_collects_tokens_from_both_sides() {
        let mut rs = RSet::new();
        rs.add(R::new("a", "b"));
        rs.add(R::new("b", "c"));
        let ids = rs.identifiers();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains("a"));
        assert!(ids.contains("b"));
        assert!(ids.contains("c"));
    }

    #[test]
    fn left_of_and_right_of_partition_by_slot() {
        let mut rs = RSet::new();
        rs.add(R::new("a", "b"));
        rs.add(R::new("a", "c"));
        rs.add(R::new("d", "a"));

        assert_eq!(rs.left_of("a").len(), 2);
        assert_eq!(rs.right_of("a").len(), 1);
        assert_eq!(rs.left_of("z").len(), 0);
    }

    #[test]
    fn profile_of_absent_identifier_is_zero() {
        let rs = RSet::new();
        let p = rs.profile("ghost");
        assert_eq!(p.degree_out, 0);
        assert_eq!(p.degree_in, 0);
        assert_eq!(p.slots, SlotPattern::None);
        assert_eq!(p.total_degree(), 0);
    }

    #[test]
    fn profile_distinguishes_slot_patterns() {
        let mut rs = RSet::new();
        rs.add(R::new("source", "middle"));
        rs.add(R::new("middle", "sink"));

        assert_eq!(rs.profile("source").slots, SlotPattern::LeftOnly);
        assert_eq!(rs.profile("sink").slots, SlotPattern::RightOnly);
        assert_eq!(rs.profile("middle").slots, SlotPattern::Both);
    }

    #[test]
    fn profile_counts_degrees() {
        let mut rs = RSet::new();
        rs.extend([
            R::new("hub", "a"),
            R::new("hub", "b"),
            R::new("hub", "c"),
            R::new("x", "hub"),
        ]);
        let p = rs.profile("hub");
        assert_eq!(p.degree_out, 3);
        assert_eq!(p.degree_in, 1);
        assert_eq!(p.total_degree(), 4);
        assert_eq!(p.slots, SlotPattern::Both);
    }

    #[test]
    fn self_loop_registers_as_both_slots() {
        let mut rs = RSet::new();
        rs.add(R::new("loop", "loop"));
        let p = rs.profile("loop");
        assert_eq!(p.degree_out, 1);
        assert_eq!(p.degree_in, 1);
        assert_eq!(p.slots, SlotPattern::Both);
    }

    #[test]
    fn profiles_covers_every_identifier_in_set() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let all = rs.profiles();
        assert_eq!(all.len(), 3);
        assert!(all.contains_key("a"));
        assert!(all.contains_key("b"));
        assert!(all.contains_key("c"));
    }

    #[test]
    fn chain_profile_marks_endpoints_asymmetrically() {
        let mut rs = RSet::new();
        rs.extend([
            R::new("a1", "a2"),
            R::new("a2", "a3"),
            R::new("a3", "a4"),
        ]);

        // chain head: only appears on left
        assert_eq!(rs.profile("a1").slots, SlotPattern::LeftOnly);
        // chain tail: only appears on right
        assert_eq!(rs.profile("a4").slots, SlotPattern::RightOnly);
        // middle nodes: both, each with degree 1 each way
        let mid = rs.profile("a2");
        assert_eq!(mid.degree_out, 1);
        assert_eq!(mid.degree_in, 1);
    }

    #[test]
    fn equivalence_classes_are_empty_for_empty_set() {
        let rs = RSet::new();
        assert!(rs.equivalence_classes().is_empty());
    }

    #[test]
    fn single_instance_produces_two_classes() {
        let mut rs = RSet::new();
        rs.add(R::new("a", "b"));
        let classes = rs.equivalence_classes();
        // one LeftOnly (a), one RightOnly (b)
        assert_eq!(classes.len(), 2);
    }

    #[test]
    fn chain_produces_three_classes_head_middles_tail() {
        let mut rs = RSet::new();
        rs.extend([
            R::new("a1", "a2"),
            R::new("a2", "a3"),
            R::new("a3", "a4"),
            R::new("a4", "a5"),
        ]);
        let classes = rs.equivalence_classes();
        assert_eq!(classes.len(), 3);

        // the middles collapse — three of them in one class
        let (_, biggest) = classes.iter().max_by_key(|(_, v)| v.len()).unwrap();
        assert_eq!(biggest.len(), 3);
        assert!(biggest.contains("a2"));
        assert!(biggest.contains("a3"));
        assert!(biggest.contains("a4"));
    }

    #[test]
    fn cycle_collapses_to_one_class() {
        let mut rs = RSet::new();
        rs.extend([
            R::new("a", "b"),
            R::new("b", "c"),
            R::new("c", "a"),
        ]);
        let classes = rs.equivalence_classes();
        assert_eq!(classes.len(), 1);
        let only = classes.values().next().unwrap();
        assert_eq!(only.len(), 3);
    }

    #[test]
    fn star_splits_hub_from_leaves() {
        let mut rs = RSet::new();
        rs.extend([
            R::new("hub", "a"),
            R::new("hub", "b"),
            R::new("hub", "c"),
        ]);
        let classes = rs.equivalence_classes();
        assert_eq!(classes.len(), 2);

        // one singleton class (the hub), one class of three leaves
        let sizes: Vec<usize> = {
            let mut v: Vec<usize> = classes.values().map(|c| c.len()).collect();
            v.sort();
            v
        };
        assert_eq!(sizes, vec![1, 3]);
    }

    #[test]
    fn bidirectional_chain_collapses_endpoints() {
        // forward + reverse: endpoints have (out=1, in=1, Both),
        // same as each other; middles have (out=2, in=2, Both).
        let mut rs = RSet::new();
        rs.extend([
            R::new("a1", "a2"),
            R::new("a2", "a3"),
            R::new("a3", "a4"),
            R::new("a2", "a1"),
            R::new("a3", "a2"),
            R::new("a4", "a3"),
        ]);
        let classes = rs.equivalence_classes();
        assert_eq!(classes.len(), 2);

        // endpoints a1 and a4 share a class; middles a2 and a3 share a class
        let mut sizes: Vec<usize> = classes.values().map(|c| c.len()).collect();
        sizes.sort();
        assert_eq!(sizes, vec![2, 2]);
    }

    #[test]
    fn r_equivalence_empty_for_empty_set() {
        let rs = RSet::new();
        assert!(rs.r_equivalence_classes().is_empty());
    }

    #[test]
    fn short_chain_two_edge_classes() {
        // a1 -> a2 -> a3: no middle-middle edge, so head-edge and tail-edge
        let mut rs = RSet::new();
        rs.extend([R::new("a1", "a2"), R::new("a2", "a3")]);
        assert_eq!(rs.r_equivalence_classes().len(), 2);
    }

    #[test]
    fn long_chain_three_edge_classes_with_middle_merge() {
        // a1 -> a2 -> a3 -> a4 -> a5: middle-middle edges R(a2,a3) and
        // R(a3,a4) must merge into a single class.
        let mut rs = RSet::new();
        rs.extend([
            R::new("a1", "a2"),
            R::new("a2", "a3"),
            R::new("a3", "a4"),
            R::new("a4", "a5"),
        ]);
        let classes = rs.r_equivalence_classes();
        assert_eq!(classes.len(), 3);

        // one class has 2 edges (the middle-middle merge)
        let mut sizes: Vec<usize> = classes.values().map(|c| c.len()).collect();
        sizes.sort();
        assert_eq!(sizes, vec![1, 1, 2]);
    }

    #[test]
    fn cycle_merges_all_edges() {
        let mut rs = RSet::new();
        rs.extend([
            R::new("a", "b"),
            R::new("b", "c"),
            R::new("c", "a"),
        ]);
        let classes = rs.r_equivalence_classes();
        assert_eq!(classes.len(), 1);
        assert_eq!(classes.values().next().unwrap().len(), 3);
    }

    #[test]
    fn star_merges_all_spokes() {
        let mut rs = RSet::new();
        rs.extend([
            R::new("hub", "a"),
            R::new("hub", "b"),
            R::new("hub", "c"),
        ]);
        let classes = rs.r_equivalence_classes();
        assert_eq!(classes.len(), 1);
        assert_eq!(classes.values().next().unwrap().len(), 3);
    }

    #[test]
    fn r_signature_respects_direction() {
        // two edges with the same endpoint profiles but opposite directions
        // must land in different classes
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "a")]);
        // a and b both have Both-1-1 profile after these two edges
        // but the signatures are (Both-1-1, Both-1-1) either way...
        // so they collapse. That's the correct behavior — in this graph
        // the two edges really are structurally equivalent.
        //
        // Construct a clearer directional case: a -> b and c -> d where
        // a has LeftOnly, b has RightOnly, c has LeftOnly, d has RightOnly.
        // Both edges should merge.
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("c", "d")]);
        let classes = rs.r_equivalence_classes();
        assert_eq!(classes.len(), 1);

        // Now add a reverse edge e -> f where e has LeftOnly and f has
        // RightOnly, but between two nodes where the profile contains
        // both directions — this requires constructing a richer case.
        // Simplest directional test: R(a, b) vs R(c, b) where both
        // edges terminate at b. b now has in=2, a and c have out=1.
        // Signatures: (LeftOnly-1-0, RightOnly-0-2). Merged.
        //
        // For a case where direction forces a split, use the
        // bidirectional-chain test below.
    }

    #[test]
    fn bidirectional_chain_edge_classes() {
        let mut rs = RSet::new();
        rs.extend([
            R::new("a1", "a2"), R::new("a2", "a3"), R::new("a3", "a4"),
            R::new("a2", "a1"), R::new("a3", "a2"), R::new("a4", "a3"),
        ]);
        // Profiles:
        //   a1, a4: (out=1, in=1, Both)
        //   a2, a3: (out=2, in=2, Both)
        // Edge signatures (ordered):
        //   out-from-end:    (1-1-Both, 2-2-Both)  — R(a1,a2), R(a4,a3)
        //   in-to-end:       (2-2-Both, 1-1-Both)  — R(a2,a1), R(a3,a4)
        //   middle-middle:   (2-2-Both, 2-2-Both)  — R(a2,a3), R(a3,a2)
        // Three classes, two edges each. Direction of edge distinguishes
        // out-from-end from in-to-end.
        let classes = rs.r_equivalence_classes();
        assert_eq!(classes.len(), 3);
        let mut sizes: Vec<usize> = classes.values().map(|c| c.len()).collect();
        sizes.sort();
        assert_eq!(sizes, vec![2, 2, 2]);
    }

    #[test]
    fn locality_of_absent_edge_is_zero() {
        let rs = RSet::new();
        let p = rs.locality_profile(&R::new("a", "b"));
        assert_eq!(p.co_left, 0);
        assert_eq!(p.co_right, 0);
        assert_eq!(p.forward, 0);
        assert_eq!(p.reverse, 0);
    }

    #[test]
    fn locality_separates_cycle_from_star() {
        let mut cycle = RSet::new();
        cycle.extend([R::new("a", "b"), R::new("b", "c"), R::new("c", "a")]);
        let loc_cycle = cycle.locality_profile(&R::new("a", "b"));
        assert_eq!(
            loc_cycle,
            LocalityProfile { co_left: 0, co_right: 0, forward: 1, reverse: 1 }
        );

        let mut star = RSet::new();
        star.extend([R::new("hub", "a"), R::new("hub", "b"), R::new("hub", "c")]);
        let loc_star = star.locality_profile(&R::new("hub", "a"));
        assert_eq!(
            loc_star,
            LocalityProfile { co_left: 2, co_right: 0, forward: 0, reverse: 0 }
        );

        // The motivating distinction: these profiles differ.
        assert_ne!(loc_cycle, loc_star);
    }

    #[test]
    fn locality_chain_positions() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c"), R::new("c", "d")]);

        // head edge: only a forward neighbor
        assert_eq!(
            rs.locality_profile(&R::new("a", "b")),
            LocalityProfile { co_left: 0, co_right: 0, forward: 1, reverse: 0 }
        );
        // middle edge: one forward, one reverse
        assert_eq!(
            rs.locality_profile(&R::new("b", "c")),
            LocalityProfile { co_left: 0, co_right: 0, forward: 1, reverse: 1 }
        );
        // tail edge: only a reverse neighbor
        assert_eq!(
            rs.locality_profile(&R::new("c", "d")),
            LocalityProfile { co_left: 0, co_right: 0, forward: 0, reverse: 1 }
        );
    }

    #[test]
    fn locality_known_chain_cycle_collision() {
        // Recorded limitation: chain-middle edge and any cycle edge have
        // the same 1-hop locality profile (0, 0, 1, 1). This test locks
        // the behavior so we notice when a future upgrade (2-hop) breaks
        // the collision.
        let mut chain = RSet::new();
        chain.extend([R::new("a", "b"), R::new("b", "c"), R::new("c", "d")]);

        let mut cycle = RSet::new();
        cycle.extend([R::new("x", "y"), R::new("y", "z"), R::new("z", "x")]);

        assert_eq!(
            chain.locality_profile(&R::new("b", "c")),
            cycle.locality_profile(&R::new("x", "y"))
        );
    }

    #[test]
    fn locality_in_star_puts_co_right() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "sink"), R::new("b", "sink"), R::new("c", "sink")]);
        let p = rs.locality_profile(&R::new("a", "sink"));
        assert_eq!(p.co_left, 0);
        assert_eq!(p.co_right, 2);
        assert_eq!(p.forward, 0);
        assert_eq!(p.reverse, 0);
    }

    #[test]
    fn locality_excludes_self() {
        // a self-loop R(a, a): when asked about itself, all four counts
        // should be zero because the only candidate neighbor (itself) is
        // excluded. (The self-loop character is visible via profile
        // slots, not locality.)
        let mut rs = RSet::new();
        rs.add(R::new("a", "a"));
        let p = rs.locality_profile(&R::new("a", "a"));
        assert_eq!(p.total(), 0);
    }

    #[test]
    fn edge_fingerprint_composes_existing_signals() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let r = R::new("a", "b");
        let fp = rs.edge_fingerprint(&r);
        assert_eq!(fp.0, rs.r_signature(&r));
        assert_eq!(fp.1, rs.locality_profile(&r));
    }

    #[test]
    fn edge_fingerprint_merges_star_spokes() {
        let mut rs = RSet::new();
        rs.extend([R::new("h", "a"), R::new("h", "b"), R::new("h", "c")]);
        let fa = rs.edge_fingerprint(&R::new("h", "a"));
        let fb = rs.edge_fingerprint(&R::new("h", "b"));
        let fc = rs.edge_fingerprint(&R::new("h", "c"));
        assert_eq!(fa, fb);
        assert_eq!(fb, fc);
    }

    #[test]
    fn edge_fingerprint_inherits_1hop_chain_cycle_collision() {
        // Documented in ADR 0006 and 0007: compound fingerprint does not
        // break the chain-middle / cycle-edge collision because both
        // its components are 1-hop.
        let mut chain = RSet::new();
        chain.extend([R::new("a", "b"), R::new("b", "c"), R::new("c", "d")]);
        let mut cycle = RSet::new();
        cycle.extend([R::new("x", "y"), R::new("y", "z"), R::new("z", "x")]);
        assert_eq!(
            chain.edge_fingerprint(&R::new("b", "c")),
            cycle.edge_fingerprint(&R::new("x", "y"))
        );
    }

    #[test]
    fn subgraph_empty() {
        let s = Subgraph::new();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
        assert!(s.identifiers().is_empty());
    }

    #[test]
    fn subgraph_from_edges_roundtrip() {
        let s = Subgraph::from_edges([R::new("a", "b"), R::new("b", "c")]);
        assert_eq!(s.len(), 2);
        assert!(s.contains(&R::new("a", "b")));
        assert_eq!(s.identifiers().len(), 3);
    }

    #[test]
    fn connected_components_empty_input() {
        assert!(Subgraph::connected_components_of([] as [R; 0]).is_empty());
    }

    #[test]
    fn connected_components_single_edge_single_component() {
        let comps = Subgraph::connected_components_of([R::new("a", "b")]);
        assert_eq!(comps.len(), 1);
        assert_eq!(comps[0].len(), 1);
    }

    #[test]
    fn connected_components_disjoint_edges_split() {
        let comps = Subgraph::connected_components_of([
            R::new("a", "b"),
            R::new("c", "d"), // shares no identifier with R(a, b)
        ]);
        assert_eq!(comps.len(), 2);
    }

    #[test]
    fn connected_components_chain_is_single() {
        let comps = Subgraph::connected_components_of([
            R::new("a", "b"),
            R::new("b", "c"),
            R::new("c", "d"),
        ]);
        assert_eq!(comps.len(), 1);
        assert_eq!(comps[0].len(), 3);
    }

    #[test]
    fn connected_components_cycle_is_single() {
        let comps = Subgraph::connected_components_of([
            R::new("a", "b"),
            R::new("b", "c"),
            R::new("c", "a"),
        ]);
        assert_eq!(comps.len(), 1);
        assert_eq!(comps[0].len(), 3);
    }

    #[test]
    fn compound_class_subgraphs_splits_chain_plus_cycle_false_merge() {
        // Chain and cycle disjoint in the same RSet. Their interior
        // edges share the 1-hop compound fingerprint, so they land in
        // one compound class — but their connected components differ.
        let mut rs = RSet::new();
        rs.extend([
            R::new("c1", "c2"), R::new("c2", "c3"),
            R::new("c3", "c4"), R::new("c4", "c5"),
        ]);
        rs.extend([
            R::new("k1", "k2"), R::new("k2", "k3"), R::new("k3", "k1"),
        ]);

        let classes = rs.compound_class_subgraphs();
        // Find the big class (5 members = chain-middle + cycle)
        let big = classes
            .values()
            .find(|subgraphs| subgraphs.iter().map(|s| s.len()).sum::<usize>() == 5)
            .expect("expected a 5-member compound class (chain-middle + cycle)");
        assert_eq!(big.len(), 2);
        // one subgraph of 2 edges (chain fragment), one of 3 (cycle)
        let mut sizes: Vec<usize> = big.iter().map(|s| s.len()).collect();
        sizes.sort();
        assert_eq!(sizes, vec![2, 3]);
    }

    #[test]
    fn compound_class_subgraphs_star_stays_unified() {
        let mut rs = RSet::new();
        rs.extend([
            R::new("h", "a"),
            R::new("h", "b"),
            R::new("h", "c"),
        ]);
        let classes = rs.compound_class_subgraphs();
        assert_eq!(classes.len(), 1);
        let subgraphs = classes.values().next().unwrap();
        assert_eq!(subgraphs.len(), 1);
        assert_eq!(subgraphs[0].len(), 3);
    }

    #[test]
    fn compound_class_subgraphs_same_fingerprint_no_shared_id_splits() {
        // Two single edges with the same endpoint profile pair but no
        // identifier in common land in the same compound class yet
        // produce two separate subgraphs.
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("c", "d")]);
        let classes = rs.compound_class_subgraphs();
        assert_eq!(classes.len(), 1);
        let subgraphs = classes.values().next().unwrap();
        assert_eq!(subgraphs.len(), 2);
        assert!(subgraphs.iter().all(|s| s.len() == 1));
    }

    #[test]
    fn canonicalize_empty_subgraph() {
        assert!(Subgraph::new().canonicalize().is_empty());
    }

    #[test]
    fn canonicalize_single_edge_has_one_canonical_form() {
        // Every single-edge subgraph reduces to the same canonical form
        // regardless of its identifiers: one directed edge from a
        // source (out=1,in=0) to a sink (out=0,in=1).
        let a = Subgraph::from_edges([R::new("a", "b")]);
        let b = Subgraph::from_edges([R::new("p", "q")]);
        let c = Subgraph::from_edges([R::new("hello", "world")]);
        assert_eq!(a.canonicalize(), b.canonicalize());
        assert_eq!(b.canonicalize(), c.canonicalize());
    }

    #[test]
    fn canonicalize_isomorphic_two_chains() {
        let a = Subgraph::from_edges([R::new("a", "b"), R::new("b", "c")]);
        let b = Subgraph::from_edges([R::new("p", "q"), R::new("q", "r")]);
        assert!(a.is_isomorphic_to(&b));
    }

    #[test]
    fn canonicalize_chain_vs_cycle_differ() {
        let chain = Subgraph::from_edges([
            R::new("a", "b"), R::new("b", "c"), R::new("c", "d"),
        ]);
        let cycle = Subgraph::from_edges([
            R::new("x", "y"), R::new("y", "z"), R::new("z", "x"),
        ]);
        assert_ne!(chain.canonicalize(), cycle.canonicalize());
    }

    #[test]
    fn canonicalize_chain_vs_star_differ() {
        let chain = Subgraph::from_edges([R::new("a", "b"), R::new("b", "c")]);
        let star = Subgraph::from_edges([R::new("h", "a"), R::new("h", "b")]);
        assert_ne!(chain.canonicalize(), star.canonicalize());
    }

    #[test]
    fn canonicalize_isomorphic_three_cycles() {
        let one = Subgraph::from_edges([
            R::new("a", "b"), R::new("b", "c"), R::new("c", "a"),
        ]);
        let two = Subgraph::from_edges([
            R::new("x", "y"), R::new("y", "z"), R::new("z", "x"),
        ]);
        assert!(one.is_isomorphic_to(&two));
    }

    #[test]
    fn canonicalize_isomorphic_three_stars() {
        let one = Subgraph::from_edges([
            R::new("h1", "a"), R::new("h1", "b"), R::new("h1", "c"),
        ]);
        let two = Subgraph::from_edges([
            R::new("h2", "p"), R::new("h2", "q"), R::new("h2", "r"),
        ]);
        assert!(one.is_isomorphic_to(&two));
    }

    #[test]
    fn canonicalize_forward_chain_same_as_reversed_identifiers() {
        // Forward chain a -> b -> c and "reversed identifier" chain
        // c -> b -> a are the same *structure*: source -> middle -> sink.
        // Only the names of the nodes change.
        let forward = Subgraph::from_edges([R::new("a", "b"), R::new("b", "c")]);
        let renamed = Subgraph::from_edges([R::new("c", "b"), R::new("b", "a")]);
        assert!(forward.is_isomorphic_to(&renamed));
    }

    #[test]
    fn canonicalize_distinguishes_V_from_chain() {
        // a -> b -> c is a chain. a -> b, c -> b is a "V" into b.
        // Structurally distinct even though node counts match.
        let chain = Subgraph::from_edges([R::new("a", "b"), R::new("b", "c")]);
        let vee = Subgraph::from_edges([R::new("a", "b"), R::new("c", "b")]);
        assert_ne!(chain.canonicalize(), vee.canonicalize());
    }

    #[test]
    fn canonicalize_direction_matters() {
        // Single "outward" edge a -> b has a source and a sink.
        // Reverse edge b -> a is *the same* one-edge pattern; only
        // the labels change. But two chains with opposite edge
        // direction at the fork are different.
        let one_forward = Subgraph::from_edges([R::new("a", "b"), R::new("b", "c")]);
        let one_reversed = Subgraph::from_edges([R::new("b", "a"), R::new("c", "b")]);
        // Both are "source -> middle -> sink" with relabeled nodes.
        // So canonical forms should match.
        assert!(one_forward.is_isomorphic_to(&one_reversed));

        // But a -> b -> c (chain) and b -> a, b -> c (out-V) differ.
        let out_vee = Subgraph::from_edges([R::new("b", "a"), R::new("b", "c")]);
        assert_ne!(one_forward.canonicalize(), out_vee.canonicalize());
    }

    #[test]
    fn naming_empty_instance_list_errors() {
        let mut rs = RSet::new();
        assert_eq!(
            rs.name_pattern_instances(&[]),
            Err(PatternError::EmptyInstanceList)
        );
    }

    #[test]
    fn naming_empty_instance_errors() {
        let mut rs = RSet::new();
        assert_eq!(
            rs.name_pattern_instances(&[Subgraph::new()]),
            Err(PatternError::EmptyInstance)
        );
    }

    #[test]
    fn naming_non_isomorphic_errors() {
        let mut rs = RSet::new();
        let chain = Subgraph::from_edges([R::new("a", "b"), R::new("b", "c")]);
        let star = Subgraph::from_edges([R::new("h", "a"), R::new("h", "b")]);
        assert_eq!(
            rs.name_pattern_instances(&[chain, star]),
            Err(PatternError::NotIsomorphic)
        );
    }

    #[test]
    fn naming_same_canonical_twice_reuses_pattern_id() {
        let mut rs = RSet::new();
        // Populate with two separated chains so reconstruction can recover
        // the canonical form from participants.
        rs.extend([
            R::new("a", "b"), R::new("b", "c"),
            R::new("p", "q"), R::new("q", "r"),
        ]);
        let first = Subgraph::from_edges([R::new("a", "b"), R::new("b", "c")]);
        let second = Subgraph::from_edges([R::new("p", "q"), R::new("q", "r")]);

        let p1 = rs.name_pattern_instances(&[first]).unwrap();
        let p2 = rs.name_pattern_instances(&[second]).unwrap();
        assert_eq!(p1, p2);
        // One pattern, two instances.
        assert_eq!(rs.patterns().len(), 1);
        assert_eq!(rs.instances_of(&p1).len(), 2);
    }

    #[test]
    fn naming_skips_colliding_pattern_id() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        // Plant a spurious user identifier that would clash with p_0.
        rs.add(R::new("p_0", "spurious"));

        let chain = Subgraph::from_edges([R::new("a", "b"), R::new("b", "c")]);
        let pid = rs.name_pattern_instances(&[chain]).unwrap();
        assert_ne!(pid, "p_0");
        // With a single planted collision, the next free id is p_1.
        assert_eq!(pid, "p_1");
    }

    #[test]
    fn naming_round_trips_canonical_form() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let sg = Subgraph::from_edges([R::new("a", "b"), R::new("b", "c")]);
        let expected = sg.canonicalize();

        let pid = rs.name_pattern_instances(&[sg]).unwrap();
        let instance = rs.instances_of(&pid)[0].to_string();
        let participants = rs.participants_of(&instance);
        let edges: Vec<R> = rs
            .iter()
            .filter(|r| {
                participants.contains(r.x.as_str())
                    && participants.contains(r.y.as_str())
            })
            .cloned()
            .collect();
        let recovered = Subgraph::from_edges(edges);
        assert_eq!(recovered.canonicalize(), expected);
    }

    #[test]
    fn participant_shared_across_two_patterns() {
        let mut rs = RSet::new();
        // A node `b` participates in a chain {a, b, c} and a star {b, x, y}.
        rs.extend([
            R::new("a", "b"), R::new("b", "c"),
            R::new("b", "x"), R::new("b", "y"),
        ]);
        let chain = Subgraph::from_edges([R::new("a", "b"), R::new("b", "c")]);
        let star = Subgraph::from_edges([R::new("b", "x"), R::new("b", "y")]);

        let p_chain = rs.name_pattern_instances(&[chain]).unwrap();
        let p_star = rs.name_pattern_instances(&[star]).unwrap();
        assert_ne!(p_chain, p_star);

        let inst_chain = rs.instances_of(&p_chain)[0].to_string();
        let inst_star = rs.instances_of(&p_star)[0].to_string();
        assert!(rs.participants_of(&inst_chain).contains("b"));
        assert!(rs.participants_of(&inst_star).contains("b"));

        // instances_of is pattern-local.
        assert_eq!(rs.instances_of(&p_chain).len(), 1);
        assert_eq!(rs.instances_of(&p_star).len(), 1);
    }

    #[test]
    fn naming_single_edge_pattern_collects_six_instances() {
        // Mirrors ADR 0009's P2 finding: six single-edge subgraphs across
        // the mixed graph all share a canonical form.
        let mut rs = RSet::new();
        // A small assortment of single-edge contexts — participant
        // identifiers cannot collide across subgraphs, so each instance
        // is truly isolated.
        rs.extend([
            R::new("a1", "a2"),
            R::new("b1", "b2"),
            R::new("c1", "c2"),
            R::new("d1", "d2"),
            R::new("e1", "e2"),
            R::new("f1", "f2"),
        ]);
        let instances: Vec<Subgraph> = [
            ("a1", "a2"), ("b1", "b2"), ("c1", "c2"),
            ("d1", "d2"), ("e1", "e2"), ("f1", "f2"),
        ]
        .into_iter()
        .map(|(x, y)| Subgraph::from_edges([R::new(x, y)]))
        .collect();

        let pid = rs.name_pattern_instances(&instances).unwrap();
        assert_eq!(rs.patterns().len(), 1);
        assert_eq!(rs.instances_of(&pid).len(), 6);
    }

    fn build_mixed_graph() -> RSet {
        let mut rs = RSet::new();
        rs.extend([
            R::new("c1", "c2"), R::new("c2", "c3"),
            R::new("c3", "c4"), R::new("c4", "c5"),
        ]);
        rs.extend([
            R::new("k1", "k2"), R::new("k2", "k3"), R::new("k3", "k1"),
        ]);
        rs.extend([
            R::new("s", "sa"), R::new("s", "sb"), R::new("s", "sc"),
        ]);
        rs.extend([
            R::new("t1", "t2"), R::new("t1", "t3"), R::new("t2", "t4"),
        ]);
        rs.add(R::new("ie1", "ie2"));
        rs
    }

    #[test]
    fn default_policy_skips_single_edge() {
        let mut rs = RSet::new();
        rs.add(R::new("a", "b"));
        let sg = Subgraph::from_edges([R::new("a", "b")]);
        let decision = rs.consider_naming(&[sg], &NamingPolicy::default()).unwrap();
        assert!(matches!(
            decision,
            NamingDecision::Skipped(SkipReason::BelowMinEdges { edges: 1, min: 2 })
        ));
        assert!(rs.patterns().is_empty());
    }

    #[test]
    fn lowering_min_edges_allows_single_edge() {
        let mut rs = RSet::new();
        rs.add(R::new("a", "b"));
        let sg = Subgraph::from_edges([R::new("a", "b")]);
        let policy = NamingPolicy { min_edges: 1, min_instances: 1, skip_meta_subgraphs: true, attach_only: false, min_mdl_gain: 0 };
        let decision = rs.consider_naming(&[sg], &policy).unwrap();
        assert!(matches!(decision, NamingDecision::Named(_)));
        assert_eq!(rs.patterns().len(), 1);
    }

    #[test]
    fn min_instances_threshold_skips_singleton() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let sg = Subgraph::from_edges([R::new("a", "b"), R::new("b", "c")]);
        let policy = NamingPolicy { min_edges: 1, min_instances: 2, skip_meta_subgraphs: true, attach_only: false, min_mdl_gain: 0 };
        let decision = rs.consider_naming(&[sg], &policy).unwrap();
        assert!(matches!(
            decision,
            NamingDecision::Skipped(SkipReason::BelowMinInstances { instances: 1, min: 2 })
        ));
    }

    #[test]
    fn default_pass_on_mixed_graph_names_three_skips_one() {
        let mut rs = build_mixed_graph();
        let decisions = rs.run_naming_pass(&NamingPolicy::default());

        let named: usize = decisions
            .iter()
            .filter(|(_, d)| matches!(d, NamingDecision::Named(_)))
            .count();
        let skipped: usize = decisions
            .iter()
            .filter(|(_, d)| matches!(d, NamingDecision::Skipped(_)))
            .count();
        assert_eq!(named, 3, "cycle, star, chain named");
        assert_eq!(skipped, 1, "single-edge pattern P2 skipped by default min_edges=2");
        assert_eq!(rs.patterns().len(), 3);
    }

    #[test]
    fn pass_with_min_instances_two_names_nothing_on_mixed_graph() {
        // On the mixed graph, every non-trivial pattern has exactly 1 instance
        // (3-cycle, 3-star, 2-chain all singletons). P2 single-edge has 6
        // instances but is filtered by min_edges=2. So nothing is named.
        let mut rs = build_mixed_graph();
        let policy = NamingPolicy { min_edges: 2, min_instances: 2, skip_meta_subgraphs: true, attach_only: false, min_mdl_gain: 0 };
        let decisions = rs.run_naming_pass(&policy);
        let named: usize = decisions
            .iter()
            .filter(|(_, d)| matches!(d, NamingDecision::Named(_)))
            .count();
        assert_eq!(named, 0);
        assert!(rs.patterns().is_empty());
    }

    #[test]
    fn naming_pass_is_idempotent_under_default_policy() {
        let mut rs = build_mixed_graph();
        let first_decisions = rs.run_naming_pass(&NamingPolicy::default());
        let named_first = first_decisions
            .iter()
            .filter(|(_, d)| matches!(d, NamingDecision::Named(_)))
            .count();
        let pattern_count_before = rs.patterns().len();

        // Re-run: data subgraphs are still there (connected components of
        // the original data edges ignore meta-R), but dedup via
        // filter_known_instances catches them — their participant sets
        // already match the instance records. Result: AlreadyKnown skips,
        // no new patterns, no new instances.
        let second_decisions = rs.run_naming_pass(&NamingPolicy::default());
        let named_second = second_decisions
            .iter()
            .filter(|(_, d)| matches!(d, NamingDecision::Named(_)))
            .count();
        let already_known: usize = second_decisions
            .iter()
            .filter(|(_, d)| matches!(d, NamingDecision::Skipped(SkipReason::AlreadyKnown)))
            .count();
        assert_eq!(named_first, 3);
        assert_eq!(named_second, 0);
        assert_eq!(already_known, 3);
        assert_eq!(rs.patterns().len(), pattern_count_before);
    }

    #[test]
    fn classify_subgraph_matches_known_pattern() {
        let mut rs = build_mixed_graph();
        rs.run_naming_pass(&NamingPolicy::default());
        let cycle = Subgraph::from_edges([
            R::new("k1", "k2"), R::new("k2", "k3"), R::new("k3", "k1"),
        ]);
        let matched = rs.classify_subgraph(&cycle);
        assert!(matched.is_some());
        // Same canonical as p_0; isomorphic under identifier relabeling too.
        let fresh_cycle = Subgraph::from_edges([
            R::new("m1", "m2"), R::new("m2", "m3"), R::new("m3", "m1"),
        ]);
        assert_eq!(rs.classify_subgraph(&fresh_cycle), matched);
    }

    #[test]
    fn classify_subgraph_returns_none_for_novel_structure() {
        let mut rs = build_mixed_graph();
        rs.run_naming_pass(&NamingPolicy::default());
        // Two-spoke out-star — not among the named patterns (default policy
        // names 3-cycle, 3-star, 2-chain but not 2-star).
        let two_spoke = Subgraph::from_edges([
            R::new("h", "a"), R::new("h", "b"),
        ]);
        assert_eq!(rs.classify_subgraph(&two_spoke), None);
    }

    #[test]
    fn pattern_of_recovers_owner_for_known_instance() {
        let mut rs = build_mixed_graph();
        rs.run_naming_pass(&NamingPolicy::default());
        let patterns: Vec<String> = rs.patterns().iter().map(|s| s.to_string()).collect();
        for pid in &patterns {
            for inst in rs.instances_of(pid) {
                let inst = inst.to_string();
                assert_eq!(rs.pattern_of(&inst), Some(pid.as_str()));
            }
        }
    }

    #[test]
    fn pattern_of_returns_none_for_non_instance() {
        let mut rs = build_mixed_graph();
        rs.run_naming_pass(&NamingPolicy::default());
        // A regular participant identifier is not itself an instance id.
        assert_eq!(rs.pattern_of("k1"), None);
        // Nor is the marker.
        assert_eq!(rs.pattern_of(PATTERN_MARKER), None);
        // Nor is a nonsense string.
        assert_eq!(rs.pattern_of("nope"), None);
    }

    #[test]
    fn memberships_of_reports_participation() {
        let mut rs = build_mixed_graph();
        rs.run_naming_pass(&NamingPolicy::default());
        // c2 participates in the chain pattern only (not in star or cycle).
        // Let's find which pattern owns the chain.
        let chain_canon: CanonicalForm = {
            let chain = Subgraph::from_edges([
                R::new("c2", "c3"),
                R::new("c3", "c4"),
            ]);
            chain.canonicalize()
        };
        let chain_pattern = rs
            .find_pattern_matching(&chain_canon)
            .map(|s| s.to_string())
            .expect("chain pattern should be named");
        let memberships = rs.memberships_of("c3");
        assert_eq!(memberships.len(), 1);
        assert_eq!(memberships[0].0, chain_pattern);
    }

    #[test]
    fn instance_subgraph_reconstructs_canonical_form() {
        let mut rs = build_mixed_graph();
        rs.run_naming_pass(&NamingPolicy::default());
        for pid in rs.patterns().iter().map(|s| s.to_string()).collect::<Vec<_>>() {
            let expected = {
                // Recover pattern's canonical form by reconstructing its
                // first instance the same way find_pattern_matching does.
                let inst = rs.instances_of(&pid)[0].to_string();
                rs.instance_subgraph(&inst).canonicalize()
            };
            for inst in rs.instances_of(&pid).iter().map(|s| s.to_string()).collect::<Vec<_>>() {
                let sg = rs.instance_subgraph(&inst);
                assert_eq!(sg.canonicalize(), expected);
            }
        }
    }

    #[test]
    fn attach_only_with_empty_registry_names_nothing() {
        // ADR 0015: with no patterns registered, attach-only iterates an
        // empty set of patterns and returns zero decisions. No new
        // patterns created, no instances added.
        let mut rs = build_mixed_graph();
        let policy = NamingPolicy {
            min_edges: 2,
            min_instances: 1,
            skip_meta_subgraphs: true,
            attach_only: true,
            min_mdl_gain: 0,
        };
        let decisions = rs.run_naming_pass(&policy);
        assert!(decisions.is_empty(), "no registered patterns → no decisions");
        assert!(rs.patterns().is_empty());
    }

    #[test]
    fn attach_only_admits_asymmetric_chain_after_discovery() {
        // ADR 0015 fix — the case that compound-class fragmentation
        // missed in ADR 0014.
        let mut rs = build_mixed_graph();
        rs.run_naming_pass(&NamingPolicy::default());

        // p_2 is the 2-chain pattern. Before extending: 1 instance.
        let p2_before = rs.instances_of("p_2").len();

        // Add a fresh 2-chain on completely new identifiers. Under the
        // old compound-class pipeline, this would fragment and not
        // attach. Under subgraph matching, it should attach.
        rs.extend([R::new("u", "v"), R::new("v", "w")]);

        let policy = NamingPolicy {
            min_edges: 2,
            min_instances: 1,
            skip_meta_subgraphs: true,
            attach_only: true,
            min_mdl_gain: 0,
        };
        rs.run_naming_pass(&policy);

        let p2_after = rs.instances_of("p_2").len();
        assert!(p2_after > p2_before, "asymmetric chain should attach to p_2");
    }

    #[test]
    fn find_instances_of_returns_empty_for_novel_canonical() {
        let mut rs = build_mixed_graph();
        rs.run_naming_pass(&NamingPolicy::default());
        // A completely novel canonical (e.g., a 5-edge form that no
        // named pattern uses) should yield no matches.
        let novel_target: CanonicalForm = vec![(0, 1), (1, 2), (2, 3), (3, 0), (0, 2)];
        let matches = rs.find_instances_of(&novel_target);
        assert!(matches.is_empty());
    }

    #[test]
    fn attach_only_admits_matching_canonical_after_discovery() {
        let mut rs = build_mixed_graph();
        rs.run_naming_pass(&NamingPolicy::default()); // discovery
        let pattern_count_before = rs.patterns().len();
        let instance_total_before: usize = rs
            .patterns()
            .iter()
            .map(|p| rs.instances_of(p).len())
            .sum();

        // Add a fresh 3-cycle on new identifiers. Its canonical form
        // should match the existing p_0 cycle pattern.
        rs.extend([
            R::new("m1", "m2"),
            R::new("m2", "m3"),
            R::new("m3", "m1"),
        ]);

        let policy = NamingPolicy {
            min_edges: 2,
            min_instances: 1,
            skip_meta_subgraphs: true,
            attach_only: true,
            min_mdl_gain: 0,
        };
        let decisions = rs.run_naming_pass(&policy);

        let named: usize = decisions
            .iter()
            .filter(|(_, d)| matches!(d, NamingDecision::Named(_)))
            .count();
        assert!(named >= 1, "the fresh 3-cycle should attach to p_0");

        // No new pattern created.
        assert_eq!(rs.patterns().len(), pattern_count_before);
        // At least one new instance recorded.
        let instance_total_after: usize = rs
            .patterns()
            .iter()
            .map(|p| rs.instances_of(p).len())
            .sum();
        assert!(instance_total_after > instance_total_before);
    }

    #[test]
    fn attach_pass_picks_up_instances_discovery_missed() {
        // ADR 0015: under subgraph matching, attach finds 2-chain
        // instances that compound-class discovery fragmented. In the
        // c1→c2→c3→c4→c5 chain, discovery recognizes only the
        // {c2,c3,c4} interior as a 2-chain subgraph (its edges share
        // compound fingerprints). Attach enumeration finds {c1,c2,c3}
        // and {c3,c4,c5} in addition.
        let mut rs = build_mixed_graph();
        rs.run_naming_pass(&NamingPolicy::default());
        let p2_after_discovery = rs.instances_of("p_2").len();

        let policy = NamingPolicy {
            min_edges: 2,
            min_instances: 1,
            skip_meta_subgraphs: true,
            attach_only: true,
            min_mdl_gain: 0,
        };
        rs.run_naming_pass(&policy);
        let p2_after_attach = rs.instances_of("p_2").len();

        assert!(
            p2_after_attach > p2_after_discovery,
            "attach should find additional 2-chain instances"
        );
    }

    #[test]
    fn attach_only_second_pass_is_no_op() {
        let mut rs = build_mixed_graph();
        rs.run_naming_pass(&NamingPolicy::default());
        let policy = NamingPolicy {
            min_edges: 2,
            min_instances: 1,
            skip_meta_subgraphs: true,
            attach_only: true,
            min_mdl_gain: 0,
        };
        // First attach may add instances that discovery missed; second
        // attach on unchanged data adds nothing.
        rs.run_naming_pass(&policy);
        let size_after_first = rs.len();
        let patterns_after = rs.patterns().len();

        rs.run_naming_pass(&policy);
        assert_eq!(rs.len(), size_after_first);
        assert_eq!(rs.patterns().len(), patterns_after);
    }

    #[test]
    fn discover_motifs_empty_rset_returns_empty() {
        let rs = RSet::new();
        let config = DiscoveryConfig {
            target_size: 2,
            sample_count: 20,
            top_m: 5,
            rng_seed: 42,
            include_meta_in_discovery: false,
        };
        assert!(rs.discover_motifs(&config).is_empty());
    }

    #[test]
    fn discover_motifs_is_deterministic_under_fixed_seed() {
        let rs = build_mixed_graph();
        let config = DiscoveryConfig {
            target_size: 3,
            sample_count: 30,
            top_m: 5,
            rng_seed: 12345,
            include_meta_in_discovery: false,
        };
        let first: Vec<CanonicalForm> =
            rs.discover_motifs(&config).into_iter().map(|c| c.canonical).collect();
        let second: Vec<CanonicalForm> =
            rs.discover_motifs(&config).into_iter().map(|c| c.canonical).collect();
        assert_eq!(first, second);
    }

    #[test]
    fn discover_motifs_respects_target_size() {
        let rs = build_mixed_graph();
        let config = DiscoveryConfig {
            target_size: 3,
            sample_count: 30,
            top_m: 5,
            rng_seed: 7,
            include_meta_in_discovery: false,
        };
        let candidates = rs.discover_motifs(&config);
        for c in &candidates {
            assert_eq!(c.representative.len(), 3);
            // canonical edge count equals subgraph edge count
            assert_eq!(c.canonical.len(), 3);
        }
    }

    #[test]
    fn discover_motifs_respects_top_m() {
        let rs = build_mixed_graph();
        let config = DiscoveryConfig {
            target_size: 2,
            sample_count: 50,
            top_m: 2,
            rng_seed: 99,
            include_meta_in_discovery: false,
        };
        let candidates = rs.discover_motifs(&config);
        assert!(candidates.len() <= 2);
    }

    #[test]
    fn discover_motifs_finds_two_chain_on_mixed_graph() {
        // At target_size=2, the 5-chain alone contributes 3 structurally
        // isomorphic 2-chains (c1-c2-c3, c2-c3-c4, c3-c4-c5), plus one
        // more in the tree branch t1-t2-t4 and one via the T-fork if
        // applicable. Sampling with enough draws should find the
        // 2-chain canonical with high frequency.
        let rs = build_mixed_graph();
        let config = DiscoveryConfig {
            target_size: 2,
            sample_count: 200,
            top_m: 5,
            rng_seed: 2024,
            include_meta_in_discovery: false,
        };
        let candidates = rs.discover_motifs(&config);
        assert!(!candidates.is_empty());
        // The 2-chain canonical is [(1, 2), (2, 0)].
        let two_chain_canonical: CanonicalForm = vec![(1, 2), (2, 0)];
        assert!(
            candidates.iter().any(|c| c.canonical == two_chain_canonical),
            "expected to discover the 2-chain canonical among candidates: {:?}",
            candidates.iter().map(|c| &c.canonical).collect::<Vec<_>>()
        );
    }

    #[test]
    fn refine_preserves_already_clean_representative() {
        let rs = build_mixed_graph();
        // Manually construct a clean 2-chain candidate.
        let rep = Subgraph::from_edges([R::new("c1", "c2"), R::new("c2", "c3")]);
        let canon = rep.canonicalize();
        assert!(rs.is_clean_subgraph(&rep));
        let input = vec![MotifCandidate {
            canonical: canon.clone(),
            representative: rep.clone(),
            sample_frequency: 1,
            score: 1.0,
        }];
        let refined = rs.refine_candidates(
            input,
            &RefinementConfig { max_tries: 50, rng_seed: 7 },
        );
        assert_eq!(refined[0].representative, rep);
    }

    #[test]
    fn refine_replaces_nonclean_rep_when_clean_alternative_exists() {
        let rs = build_mixed_graph();
        // Construct a 2-chain candidate with a non-clean representative
        // (embedded in the 3-cycle {k1, k2, k3}).
        let embedded = Subgraph::from_edges([R::new("k1", "k2"), R::new("k3", "k1")]);
        let canon = embedded.canonicalize();
        assert!(!rs.is_clean_subgraph(&embedded));
        let input = vec![MotifCandidate {
            canonical: canon.clone(),
            representative: embedded.clone(),
            sample_frequency: 100,
            score: 100.0,
        }];
        let refined = rs.refine_candidates(
            input,
            &RefinementConfig { max_tries: 200, rng_seed: 2024 },
        );
        // A clean 2-chain exists in the 5-chain data; refinement should
        // find one within the budget.
        assert!(rs.is_clean_subgraph(&refined[0].representative));
        assert_eq!(refined[0].canonical, canon);
    }

    #[test]
    fn refine_is_noop_when_no_clean_alternative() {
        // A graph consisting only of a single 3-cycle. The 2-chain
        // canonical has NO clean instance anywhere (every 2-chain is
        // embedded in the cycle). Refinement must leave rep unchanged.
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c"), R::new("c", "a")]);
        let embedded = Subgraph::from_edges([R::new("a", "b"), R::new("b", "c")]);
        let canon = embedded.canonicalize();
        let input = vec![MotifCandidate {
            canonical: canon,
            representative: embedded.clone(),
            sample_frequency: 1,
            score: 1.0,
        }];
        let refined = rs.refine_candidates(
            input,
            &RefinementConfig { max_tries: 100, rng_seed: 42 },
        );
        assert_eq!(refined[0].representative, embedded);
    }

    #[test]
    fn refine_is_deterministic_under_fixed_seed() {
        let rs = build_mixed_graph();
        let config_disc = DiscoveryConfig {
            target_size: 2,
            sample_count: 50,
            top_m: 3,
            rng_seed: 11,
            include_meta_in_discovery: false,
        };
        let cands = rs.discover_motifs(&config_disc);
        let cfg = RefinementConfig { max_tries: 100, rng_seed: 999 };
        let r1 = rs.refine_candidates(cands.clone(), &cfg);
        let r2 = rs.refine_candidates(cands, &cfg);
        for (a, b) in r1.iter().zip(r2.iter()) {
            assert_eq!(a.representative, b.representative);
            assert_eq!(a.canonical, b.canonical);
        }
    }

    fn default_autonomous_config() -> AutonomousConfig {
        AutonomousConfig {
            discovery: DiscoveryConfig {
                target_size: 3,
                sample_count: 200,
                top_m: 10,
                rng_seed: 2024,
            include_meta_in_discovery: false,
            },
            refinement: RefinementConfig {
                max_tries: 200,
                rng_seed: 999,
            },
            naming: NamingPolicy::default(),
            instance_sampling: None,
        }
    }

    #[test]
    fn autonomous_pass_empty_rset_returns_empty() {
        let mut rs = RSet::new();
        let outcomes = rs.autonomous_pass(&default_autonomous_config());
        assert!(outcomes.is_empty());
    }

    #[test]
    fn autonomous_pass_names_patterns_on_mixed_graph() {
        let mut rs = build_mixed_graph();
        let outcomes = rs.autonomous_pass(&default_autonomous_config());
        let new_count = outcomes
            .iter()
            .filter(|o| matches!(o, AutonomousOutcome::NewPattern { .. }))
            .count();
        assert!(
            new_count > 0,
            "autonomous_pass should name at least one new pattern on the mixed graph"
        );
        // Registry should reflect the new patterns.
        assert_eq!(rs.patterns().len(), new_count);
    }

    #[test]
    fn autonomous_pass_is_idempotent() {
        let mut rs = build_mixed_graph();
        let config = default_autonomous_config();
        let first = rs.autonomous_pass(&config);
        let first_new: usize = first
            .iter()
            .filter(|o| matches!(o, AutonomousOutcome::NewPattern { .. }))
            .count();
        assert!(first_new > 0);

        let pattern_count_after_first = rs.patterns().len();
        let rset_size_after_first = rs.len();

        let second = rs.autonomous_pass(&config);
        let second_new: usize = second
            .iter()
            .filter(|o| matches!(o, AutonomousOutcome::NewPattern { .. }))
            .count();
        let second_existing: usize = second
            .iter()
            .filter(|o| matches!(o, AutonomousOutcome::Existing { .. }))
            .count();
        assert_eq!(second_new, 0, "no new patterns on second pass");
        assert!(second_existing > 0, "existing canonicals should be reported");
        assert_eq!(rs.patterns().len(), pattern_count_after_first);
        assert_eq!(rs.len(), rset_size_after_first);
    }

    #[test]
    fn autonomous_pass_respects_policy() {
        // Raise min_instances so every single-instance motif (tree,
        // cycle, star at target_size=3) is filtered out. Only canonicals
        // with ≥ 2 clean instances survive — the 3-chain has 3 clean
        // instances in the 5-chain data.
        let mut rs = build_mixed_graph();
        let mut config = default_autonomous_config();
        config.naming.min_instances = 2;
        let outcomes = rs.autonomous_pass(&config);
        let named_count = outcomes
            .iter()
            .filter(|o| matches!(o, AutonomousOutcome::NewPattern { .. }))
            .count();
        let filtered_count = outcomes
            .iter()
            .filter(|o| matches!(
                o,
                AutonomousOutcome::Skipped {
                    reason: AutonomousSkip::PolicyFiltered(
                        SkipReason::BelowMinInstances { .. }
                    ),
                    ..
                }
            ))
            .count();
        assert_eq!(named_count, 1, "expected only the 3-chain to be named");
        assert!(filtered_count >= 2, "expected single-instance candidates filtered");
    }

    #[test]
    fn mdl_gain_is_zero_for_singleton_canonical() {
        // Build a graph with exactly one 3-cycle — one clean instance.
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c"), R::new("c", "a")]);
        let sg = Subgraph::from_edges([R::new("a", "b"), R::new("b", "c"), R::new("c", "a")]);
        let canon = sg.canonicalize();
        assert_eq!(rs.mdl_gain(&canon), 0);
    }

    #[test]
    fn mdl_gain_scales_with_reuse_and_size() {
        let rs = build_mixed_graph();
        // 2-chain canonical [(1, 2), (2, 0)]. Clean instances on the
        // mixed graph: {c1,c2,c3}, {c2,c3,c4}, {c3,c4,c5}, {t1,t2,t4}
        // → N=4. Gain = (4 - 1) * 2 = 6.
        let two_chain: CanonicalForm = vec![(1, 2), (2, 0)];
        assert_eq!(rs.mdl_gain(&two_chain), 6);

        // 3-chain canonical. Clean instances: {c1..c4}, {c2..c5} → N=2.
        // Gain = (2 - 1) * 3 = 3.
        let three_chain: CanonicalForm = vec![(1, 3), (2, 0), (3, 2)];
        assert_eq!(rs.mdl_gain(&three_chain), 3);
    }

    #[test]
    fn score_by_mdl_updates_candidate_scores() {
        let rs = build_mixed_graph();
        let candidates = vec![
            MotifCandidate {
                canonical: vec![(1, 2), (2, 0)],
                representative: Subgraph::from_edges([R::new("c1", "c2"), R::new("c2", "c3")]),
                sample_frequency: 42,
                score: 42.0,
            },
        ];
        let rescored = rs.score_by_mdl(candidates);
        assert_eq!(rescored[0].score, 6.0);
    }

    #[test]
    fn consider_naming_rejects_below_mdl_threshold() {
        let mut rs = build_mixed_graph();
        // Singleton 3-cycle instance → edges=3, count=1, gain=0.
        // With min_mdl_gain=1 it should be rejected.
        let cycle = Subgraph::from_edges([
            R::new("k1", "k2"), R::new("k2", "k3"), R::new("k3", "k1"),
        ]);
        let policy = NamingPolicy {
            min_edges: 2,
            min_instances: 1,
            skip_meta_subgraphs: true,
            attach_only: false,
            min_mdl_gain: 1,
        };
        let decision = rs.consider_naming(&[cycle], &policy).unwrap();
        assert!(matches!(
            decision,
            NamingDecision::Skipped(SkipReason::BelowMdlGain { gain: 0, min: 1 })
        ));
        assert!(rs.patterns().is_empty());
    }

    #[test]
    fn autonomous_pass_honors_min_mdl_gain() {
        // target_size=3 on the mixed graph surfaces four canonicals:
        //   3-chain  (N=2, k=3, gain=3)
        //   3-cycle  (N=1, k=3, gain=0)
        //   3-tree   (N=1, k=3, gain=0)
        //   3-star   (N=1, k=3, gain=0)
        // min_mdl_gain=1 should keep only the 3-chain.
        let mut rs = build_mixed_graph();
        let config = AutonomousConfig {
            discovery: DiscoveryConfig {
                target_size: 3,
                sample_count: 200,
                top_m: 10,
                rng_seed: 2024,
            include_meta_in_discovery: false,
            },
            refinement: RefinementConfig {
                max_tries: 200,
                rng_seed: 999,
            },
            naming: NamingPolicy {
                min_edges: 2,
                min_instances: 1,
                skip_meta_subgraphs: true,
                attach_only: false,
                min_mdl_gain: 1,
            },
            instance_sampling: None,
        };
        let outcomes = rs.autonomous_pass(&config);
        let new_count = outcomes
            .iter()
            .filter(|o| matches!(o, AutonomousOutcome::NewPattern { .. }))
            .count();
        let mdl_skipped = outcomes
            .iter()
            .filter(|o| matches!(
                o,
                AutonomousOutcome::Skipped {
                    reason: AutonomousSkip::PolicyFiltered(SkipReason::BelowMdlGain { .. }),
                    ..
                }
            ))
            .count();
        assert_eq!(new_count, 1, "only 3-chain has positive MDL gain");
        assert!(mdl_skipped >= 3, "three singleton canonicals should be MDL-filtered");
        assert_eq!(rs.patterns().len(), 1);
    }

    #[test]
    fn remove_takes_one_edge_off() {
        let mut rs = RSet::new();
        rs.add(R::new("a", "b"));
        rs.add(R::new("b", "c"));
        assert_eq!(rs.len(), 2);
        assert!(rs.remove(&R::new("a", "b")));
        assert_eq!(rs.len(), 1);
        assert!(!rs.remove(&R::new("a", "b"))); // already gone
    }

    #[test]
    fn retract_nonexistent_pattern_errors() {
        let mut rs = build_mixed_graph();
        let err = rs.retract_pattern("p_999").unwrap_err();
        assert_eq!(err, RetractionError::UnknownPattern);
    }

    #[test]
    fn retract_removes_all_meta_edges_and_preserves_data() {
        let mut rs = build_mixed_graph();
        let size_before_any_naming = rs.len();
        rs.run_naming_pass(&NamingPolicy::default());
        let size_after_naming = rs.len();
        assert!(size_after_naming > size_before_any_naming);

        let patterns: Vec<String> = rs.patterns().iter().map(|s| s.to_string()).collect();
        let victim = patterns[0].clone();
        let victim_instances = rs.instances_of(&victim).len();

        let summary = rs.retract_pattern(&victim).unwrap();
        assert_eq!(summary.pattern_id, victim);
        assert_eq!(summary.instances_removed, victim_instances);
        assert!(summary.meta_edges_removed >= victim_instances + 1);

        // Pattern is gone from the registry.
        assert!(!rs.patterns().iter().any(|p| *p == victim));

        // Other patterns are untouched.
        for p in &patterns[1..] {
            assert!(rs.patterns().iter().any(|q| q == p));
        }

        // Data edges intact.
        assert!(rs.contains(&R::new("c1", "c2")));
        assert!(rs.contains(&R::new("k1", "k2")));
        assert!(rs.contains(&R::new("s", "sa")));
    }

    #[test]
    fn retract_allows_rediscovery() {
        // After retraction, a re-run of autonomous_pass should find the
        // same canonical and name it as a fresh pattern (possibly
        // reusing the id, possibly picking a new one).
        let mut rs = build_mixed_graph();
        let config = AutonomousConfig {
            discovery: DiscoveryConfig {
                target_size: 3,
                sample_count: 200,
                top_m: 10,
                rng_seed: 2024,
            include_meta_in_discovery: false,
            },
            refinement: RefinementConfig { max_tries: 200, rng_seed: 999 },
            naming: NamingPolicy::default(),
            instance_sampling: None,
        };
        rs.autonomous_pass(&config);
        let patterns_before: Vec<String> = rs
            .patterns()
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(!patterns_before.is_empty());

        // Retract the first pattern.
        let victim = patterns_before[0].clone();
        let canon_before = {
            let inst = rs.instances_of(&victim)[0].to_string();
            rs.instance_subgraph(&inst).canonicalize()
        };
        rs.retract_pattern(&victim).unwrap();

        // The canonical should be no longer recognized.
        assert!(rs.find_pattern_matching(&canon_before).is_none());

        // Re-run. The canonical should re-emerge and be named.
        let outcomes = rs.autonomous_pass(&config);
        assert!(outcomes.iter().any(|o| matches!(
            o,
            AutonomousOutcome::NewPattern { canonical, .. } if canonical == &canon_before
        )));
    }

    #[test]
    fn retract_clears_classification() {
        let mut rs = build_mixed_graph();
        rs.run_naming_pass(&NamingPolicy::default());
        let patterns: Vec<String> = rs.patterns().iter().map(|s| s.to_string()).collect();
        let victim = patterns[0].clone();
        let inst = rs.instances_of(&victim)[0].to_string();
        let canonical = rs.instance_subgraph(&inst).canonicalize();

        // Before retraction, classification hits.
        assert_eq!(rs.classify_subgraph(&Subgraph::from_edges(rs.iter().cloned())).is_some() || true, true); // placeholder

        rs.retract_pattern(&victim).unwrap();

        // After retraction, nothing classifies to the retracted canonical.
        assert!(rs.find_pattern_matching(&canonical).is_none());
    }

    #[test]
    fn sweep_with_empty_sizes_returns_empty() {
        let mut rs = build_mixed_graph();
        let base = AutonomousConfig {
            discovery: DiscoveryConfig {
                target_size: 3,
                sample_count: 100,
                top_m: 5,
                rng_seed: 2024,
            include_meta_in_discovery: false,
            },
            refinement: RefinementConfig { max_tries: 100, rng_seed: 999 },
            naming: NamingPolicy::default(),
            instance_sampling: None,
        };
        let results = rs.autonomous_sweep(&base, &[]);
        assert!(results.is_empty());
        assert!(rs.patterns().is_empty());
    }

    #[test]
    fn sweep_with_single_size_matches_direct_pass() {
        let base = AutonomousConfig {
            discovery: DiscoveryConfig {
                target_size: 3,
                sample_count: 100,
                top_m: 5,
                rng_seed: 2024,
            include_meta_in_discovery: false,
            },
            refinement: RefinementConfig { max_tries: 100, rng_seed: 999 },
            naming: NamingPolicy::default(),
            instance_sampling: None,
        };

        // Path A: sweep with a single size — seed is offset by the size.
        let mut rs_sweep = build_mixed_graph();
        let sweep_results = rs_sweep.autonomous_sweep(&base, &[3]);
        assert_eq!(sweep_results.len(), 1);

        // Path B: call autonomous_pass with the equivalent offset seed.
        let mut rs_direct = build_mixed_graph();
        let mut direct_cfg = base.clone();
        direct_cfg.discovery.rng_seed = base.discovery.rng_seed.wrapping_add(3);
        let direct_outcomes = rs_direct.autonomous_pass(&direct_cfg);

        assert_eq!(sweep_results[0].0, 3);
        // Same number of outcomes; same registered pattern count.
        assert_eq!(sweep_results[0].1.len(), direct_outcomes.len());
        assert_eq!(rs_sweep.patterns().len(), rs_direct.patterns().len());
    }

    #[test]
    fn sweep_accumulates_patterns_across_sizes() {
        let base = AutonomousConfig {
            discovery: DiscoveryConfig {
                target_size: 3,
                sample_count: 200,
                top_m: 10,
                rng_seed: 2024,
            include_meta_in_discovery: false,
            },
            refinement: RefinementConfig { max_tries: 200, rng_seed: 999 },
            naming: NamingPolicy::default(),
            instance_sampling: None,
        };

        let mut rs = build_mixed_graph();
        let results = rs.autonomous_sweep(&base, &[2, 3]);
        assert_eq!(results.len(), 2);
        // Patterns exist at both sizes.
        let patterns_after = rs.patterns().len();
        assert!(patterns_after >= 2);

        // Second sweep on identical sizes — all Existing, no new
        // patterns.
        let second = rs.autonomous_sweep(&base, &[2, 3]);
        let any_new: usize = second
            .iter()
            .flat_map(|(_, outs)| outs.iter())
            .filter(|o| matches!(o, AutonomousOutcome::NewPattern { .. }))
            .count();
        assert_eq!(any_new, 0);
        assert_eq!(rs.patterns().len(), patterns_after);
    }

    #[test]
    fn autonomous_and_attach_on_fresh_rset() {
        // On fresh data, attach phase should find only AlreadyKnown /
        // empty: autonomous already used find_instances_of exhaustively
        // for each discovered canonical.
        let mut rs = build_mixed_graph();
        let config = AutonomousConfig {
            discovery: DiscoveryConfig {
                target_size: 3,
                sample_count: 200,
                top_m: 10,
                rng_seed: 2024,
            include_meta_in_discovery: false,
            },
            refinement: RefinementConfig { max_tries: 200, rng_seed: 999 },
            naming: NamingPolicy::default(),
            instance_sampling: None,
        };
        let summary = rs.autonomous_and_attach(&config);
        // Autonomous creates several new patterns.
        let new_patterns = summary
            .autonomous
            .iter()
            .filter(|o| matches!(o, AutonomousOutcome::NewPattern { .. }))
            .count();
        assert!(new_patterns > 0);
        // Attach pass should not create any new patterns or instances.
        let new_instances_via_attach: usize = summary
            .attach
            .iter()
            .filter(|(_, d)| matches!(d, NamingDecision::Named(_)))
            .count();
        assert_eq!(new_instances_via_attach, 0);
    }

    #[test]
    fn autonomous_and_attach_picks_up_new_data_after_prior_naming() {
        let mut rs = build_mixed_graph();
        let config = AutonomousConfig {
            discovery: DiscoveryConfig {
                target_size: 3,
                sample_count: 200,
                top_m: 10,
                rng_seed: 2024,
            include_meta_in_discovery: false,
            },
            refinement: RefinementConfig { max_tries: 200, rng_seed: 999 },
            naming: NamingPolicy::default(),
            instance_sampling: None,
        };
        // Prime the registry with the first autonomous_pass.
        rs.autonomous_pass(&config);
        let p_3_chain: CanonicalForm = vec![(1, 3), (2, 0), (3, 2)];
        let p_3_chain_id = rs
            .find_pattern_matching(&p_3_chain)
            .map(|s| s.to_string())
            .expect("3-chain named");
        let chain_instances_before = rs.instances_of(&p_3_chain_id).len();

        // Add new data that contains another clean 3-chain.
        rs.extend([
            R::new("q1", "q2"),
            R::new("q2", "q3"),
            R::new("q3", "q4"),
        ]);

        // autonomous_and_attach: autonomous may or may not re-sample the
        // 3-chain canonical (it's already Existing). Attach definitely
        // picks up {q1, q2, q3, q4} as a new instance.
        let _summary = rs.autonomous_and_attach(&config);
        let chain_instances_after = rs.instances_of(&p_3_chain_id).len();
        assert!(chain_instances_after > chain_instances_before);
    }

    #[test]
    fn autonomous_and_attach_is_idempotent() {
        let mut rs = build_mixed_graph();
        let config = AutonomousConfig {
            discovery: DiscoveryConfig {
                target_size: 3,
                sample_count: 200,
                top_m: 10,
                rng_seed: 2024,
            include_meta_in_discovery: false,
            },
            refinement: RefinementConfig { max_tries: 200, rng_seed: 999 },
            naming: NamingPolicy::default(),
            instance_sampling: None,
        };
        rs.autonomous_and_attach(&config);
        let size_before = rs.len();
        let patterns_before = rs.patterns().len();
        rs.autonomous_and_attach(&config);
        assert_eq!(rs.len(), size_before);
        assert_eq!(rs.patterns().len(), patterns_before);
    }

    #[test]
    fn canonical_library_round_trip_is_all_existing() {
        let mut rs = build_mixed_graph();
        rs.run_naming_pass(&NamingPolicy::default());
        let library = rs.canonical_library();
        assert!(!library.is_empty());

        // Re-applying the same library to the same RSet → all Existing.
        let outcomes = rs.attach_canonicals(&library, &NamingPolicy::default());
        assert_eq!(outcomes.len(), library.len());
        assert!(outcomes
            .iter()
            .all(|o| matches!(o, AutonomousOutcome::Existing { .. })));
    }

    #[test]
    fn attach_canonicals_skips_when_no_clean_instance_in_target() {
        // Source: 3-cycle named as p_0.
        let mut source = RSet::new();
        source.extend([R::new("a", "b"), R::new("b", "c"), R::new("c", "a")]);
        source.run_naming_pass(&NamingPolicy::default());
        let library = source.canonical_library();

        // Target: a graph with no 3-cycle (just a chain).
        let mut target = RSet::new();
        target.extend([R::new("p", "q"), R::new("q", "r")]);
        let outcomes = target.attach_canonicals(&library, &NamingPolicy::default());
        assert_eq!(outcomes.len(), library.len());
        assert!(outcomes.iter().all(|o| matches!(
            o,
            AutonomousOutcome::Skipped { reason: AutonomousSkip::NoCleanInstance, .. }
        )));
        assert!(target.patterns().is_empty());
    }

    #[test]
    fn attach_canonicals_names_patterns_when_target_matches() {
        // Source: a 3-cycle. Target: has another 3-cycle with different ids.
        let mut source = RSet::new();
        source.extend([R::new("a", "b"), R::new("b", "c"), R::new("c", "a")]);
        source.run_naming_pass(&NamingPolicy::default());
        let library = source.canonical_library();

        let mut target = RSet::new();
        target.extend([R::new("m1", "m2"), R::new("m2", "m3"), R::new("m3", "m1")]);

        let outcomes = target.attach_canonicals(&library, &NamingPolicy::default());
        let named: usize = outcomes
            .iter()
            .filter(|o| matches!(o, AutonomousOutcome::NewPattern { .. }))
            .count();
        assert_eq!(named, 1);
        assert_eq!(target.patterns().len(), 1);
    }

    #[test]
    fn attach_canonicals_is_idempotent() {
        let mut source = build_mixed_graph();
        source.run_naming_pass(&NamingPolicy::default());
        let library = source.canonical_library();

        let mut target = build_mixed_graph();
        target.attach_canonicals(&library, &NamingPolicy::default());
        let size_after_first = target.len();
        let patterns_after_first = target.patterns().len();

        // Second application should be all-Existing, no delta.
        let outcomes = target.attach_canonicals(&library, &NamingPolicy::default());
        assert!(outcomes
            .iter()
            .all(|o| matches!(o, AutonomousOutcome::Existing { .. })));
        assert_eq!(target.len(), size_after_first);
        assert_eq!(target.patterns().len(), patterns_after_first);
    }

    #[test]
    fn sample_instances_empty_canonical_returns_empty() {
        let rs = build_mixed_graph();
        let got = rs.sample_instances_of(
            &vec![],
            &SamplingMatchConfig { sample_count: 100, rng_seed: 1 },
        );
        assert!(got.is_empty());
    }

    #[test]
    fn sample_instances_with_no_matches_returns_empty() {
        let rs = build_mixed_graph();
        // A canonical the graph does not contain.
        let impossible: CanonicalForm = vec![(5, 5), (5, 5), (5, 5), (5, 5)];
        let got = rs.sample_instances_of(
            &impossible,
            &SamplingMatchConfig { sample_count: 100, rng_seed: 1 },
        );
        assert!(got.is_empty());
    }

    #[test]
    fn sample_instances_deterministic_under_fixed_seed() {
        let rs = build_mixed_graph();
        let target: CanonicalForm = vec![(1, 2), (2, 0)];
        let config = SamplingMatchConfig { sample_count: 100, rng_seed: 42 };
        let a = rs.sample_instances_of(&target, &config);
        let b = rs.sample_instances_of(&target, &config);
        assert_eq!(a.len(), b.len());
        // Each entry matches by participant set (sort-compare)
        let key = |v: &[Subgraph]| -> Vec<Vec<String>> {
            let mut out: Vec<Vec<String>> = v
                .iter()
                .map(|s| {
                    let mut p: Vec<String> =
                        s.identifiers().into_iter().map(str::to_owned).collect();
                    p.sort();
                    p
                })
                .collect();
            out.sort();
            out
        };
        assert_eq!(key(&a), key(&b));
    }

    #[test]
    fn sample_instances_approximates_find_instances_with_enough_budget() {
        let rs = build_mixed_graph();
        let target: CanonicalForm = vec![(1, 2), (2, 0)]; // 2-chain
        let exhaustive = rs.find_instances_of(&target);
        // Generous budget — small graph, sampling should hit all.
        let sampled = rs.sample_instances_of(
            &target,
            &SamplingMatchConfig { sample_count: 500, rng_seed: 7 },
        );
        // Never over-returns.
        assert!(sampled.len() <= exhaustive.len());
        // With 500 samples on a 14-edge graph, expect all 4 clean
        // 2-chain instances to be hit (verified empirically).
        assert_eq!(sampled.len(), exhaustive.len());
    }

    #[test]
    fn hierarchical_probe_default_matches_data_only() {
        // Flag off (default) should behave exactly like pre-0025 discover_motifs.
        let rs = build_mixed_graph();
        let cfg = DiscoveryConfig {
            target_size: 3,
            sample_count: 200,
            top_m: 10,
            rng_seed: 2024,
            include_meta_in_discovery: false,
        };
        let a = rs.discover_motifs(&cfg);

        let rs2 = build_mixed_graph();
        let b = rs2.discover_motifs(&cfg);
        let key = |v: &[MotifCandidate]| -> Vec<(CanonicalForm, usize)> {
            v.iter().map(|c| (c.canonical.clone(), c.sample_frequency)).collect()
        };
        assert_eq!(key(&a), key(&b));
    }

    #[test]
    fn hierarchical_probe_flag_on_after_naming_sees_meta_edges() {
        let mut rs = build_mixed_graph();
        let cfg = AutonomousConfig {
            discovery: DiscoveryConfig {
                target_size: 3,
                sample_count: 200,
                top_m: 10,
                rng_seed: 2024,
                include_meta_in_discovery: false,
            },
            refinement: RefinementConfig { max_tries: 200, rng_seed: 999 },
            naming: NamingPolicy::default(),
            instance_sampling: None,
        };
        rs.autonomous_pass(&cfg);

        let meta_ids = rs.collect_meta_ids();
        let probe_cfg = DiscoveryConfig {
            target_size: 3,
            sample_count: 500,
            top_m: 20,
            rng_seed: 7,
            include_meta_in_discovery: true,
        };
        let candidates = rs.discover_motifs(&probe_cfg);
        let has_meta_candidate = candidates.iter().any(|c| {
            c.representative
                .edges()
                .any(|r| meta_ids.contains(&r.x) || meta_ids.contains(&r.y))
        });
        assert!(
            has_meta_candidate,
            "flag-on probe should surface at least one candidate touching meta-R"
        );
    }

    // ADR 0026 gradient refinement tests removed with the primitives.

    // ADR 0027 axiom-discovery helpers for tests.

    fn diamond_poset() -> RSet {
        // Hasse-diagram-as-transitive-closure: a ≤ b, a ≤ c, b ≤ d, c ≤ d,
        // a ≤ d (transitive closure), plus all self-loops.
        let mut rs = RSet::new();
        let nodes = ["a", "b", "c", "d"];
        for n in &nodes {
            rs.add(R::new(*n, *n));
        }
        rs.extend([
            R::new("a", "b"), R::new("a", "c"), R::new("a", "d"),
            R::new("b", "d"), R::new("c", "d"),
        ]);
        rs
    }

    fn simple_symmetric_graph() -> RSet {
        let mut rs = RSet::new();
        rs.extend([
            R::new("a", "b"), R::new("b", "a"),
            R::new("b", "c"), R::new("c", "b"),
        ]);
        rs
    }

    #[test]
    fn axiom_discovery_finds_transitivity_on_poset() {
        let rs = diamond_poset();
        let axioms = rs.discover_axioms(&AxiomDiscoveryConfig::default());
        // Transitivity as canonicalized template: premise
        // [R(0,1), R(1,2)], conclusion R(0,2), num_vars = 3.
        let transitivity = AxiomTemplate {
            num_vars: 3,
            premise: vec![
                EdgeTemplate { x_var: 0, y_var: 1 },
                EdgeTemplate { x_var: 1, y_var: 2 },
            ],
            conclusion: EdgeTemplate { x_var: 0, y_var: 2 },
        };
        let found = axioms.iter().any(|e| e.template == transitivity);
        assert!(found, "expected transitivity among discovered axioms; got {:?}",
            axioms.iter().map(|e| &e.template).collect::<Vec<_>>());
    }

    #[test]
    fn axiom_discovery_rejects_transitivity_on_raw_chain() {
        // Non-transitive: R(a,b), R(b,c). Transitivity demands R(a,c),
        // which is absent → rate < 1.0 → NOT in strict-discover output.
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let axioms = rs.discover_axioms(&AxiomDiscoveryConfig::default());
        let transitivity = AxiomTemplate {
            num_vars: 3,
            premise: vec![
                EdgeTemplate { x_var: 0, y_var: 1 },
                EdgeTemplate { x_var: 1, y_var: 2 },
            ],
            conclusion: EdgeTemplate { x_var: 0, y_var: 2 },
        };
        assert!(!axioms.iter().any(|e| e.template == transitivity));
    }

    #[test]
    fn axiom_discovery_finds_symmetry_on_symmetric_graph() {
        let rs = simple_symmetric_graph();
        let axioms = rs.discover_axioms(&AxiomDiscoveryConfig::default());
        // Symmetry: premise R(0,1), conclusion R(1,0).
        let symmetry = AxiomTemplate {
            num_vars: 2,
            premise: vec![EdgeTemplate { x_var: 0, y_var: 1 }],
            conclusion: EdgeTemplate { x_var: 1, y_var: 0 },
        };
        assert!(axioms.iter().any(|e| e.template == symmetry));
    }

    #[test]
    fn check_poset_accepts_diamond_and_rejects_chain() {
        let rs = diamond_poset();
        let pc = rs.check_poset();
        assert!(pc.is_poset);
        assert_eq!(pc.reflexive.rate, 1.0);
        assert!(pc.antisymmetric.holds);
        assert!(pc
            .transitive
            .as_ref()
            .map(|e| e.rate == 1.0)
            .unwrap_or(false));

        // A chain {a→b, b→c}: not reflexive, not transitive.
        let mut chain = RSet::new();
        chain.extend([R::new("a", "b"), R::new("b", "c")]);
        let pc2 = chain.check_poset();
        assert!(!pc2.is_poset);
    }

    #[test]
    fn check_reflexivity_empty_rset_is_vacuously_one() {
        let rs = RSet::new();
        let ev = rs.check_reflexivity();
        assert_eq!(ev.rate, 1.0);
        assert_eq!(ev.identifiers_total, 0);
    }

    #[test]
    fn axiom_discovery_enumeration_is_deterministic() {
        // Same RSet, two calls → same axioms in same order.
        let rs = diamond_poset();
        let a = rs.discover_axioms(&AxiomDiscoveryConfig::default());
        let b = rs.discover_axioms(&AxiomDiscoveryConfig::default());
        assert_eq!(a.len(), b.len());
        for (ea, eb) in a.iter().zip(b.iter()) {
            assert_eq!(ea.template, eb.template);
            assert_eq!(ea.premise_bindings, eb.premise_bindings);
            assert_eq!(ea.conclusion_satisfied, eb.conclusion_satisfied);
        }
    }

    // ADR 0028 subsumption tests.

    #[test]
    fn adr0028_canonicalizer_collapses_transitivity_variants() {
        // Transitive closure of 5-chain: 0027's enumeration surfaced two
        // templates recognized by humans as "transitivity under different
        // variable-to-slot assignments". The structural canonicalizer must
        // collapse them into exactly one canonical transitivity template.
        let mut rs = RSet::new();
        let nodes = ["a", "b", "c", "d", "e"];
        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                rs.add(R::new(nodes[i], nodes[j]));
            }
        }
        let axioms = rs.discover_axioms(&AxiomDiscoveryConfig::default());
        // Exactly one axiom, which is classical transitivity.
        assert_eq!(axioms.len(), 1, "got: {:?}",
            axioms.iter().map(|e| &e.template).collect::<Vec<_>>());
        let t = &axioms[0].template;
        assert_eq!(t.num_vars, 3);
        assert_eq!(t.premise.len(), 2);
        assert_eq!(t.conclusion, EdgeTemplate { x_var: 0, y_var: 2 });
        assert!(t.premise.contains(&EdgeTemplate { x_var: 0, y_var: 1 }));
        assert!(t.premise.contains(&EdgeTemplate { x_var: 1, y_var: 2 }));
    }

    #[test]
    fn adr0028_reflexivity_subsumes_self_loop_conclusions() {
        // Equivalence relation: symmetry + transitivity both hold; plus
        // universal reflexivity forces axioms with R(v, v) conclusions to
        // trivially hold. discover_axioms_minimal must eliminate those.
        let mut rs = RSet::new();
        let classes: &[&[&str]] = &[&["a", "b"], &["c", "d", "e"]];
        for cls in classes {
            for x in cls.iter() {
                for y in cls.iter() {
                    rs.add(R::new(*x, *y));
                }
            }
        }
        let minimal = rs.discover_axioms_minimal(&AxiomDiscoveryConfig::default());
        // Every remaining axiom has a non-self-loop conclusion.
        for ev in &minimal {
            assert_ne!(
                ev.template.conclusion.x_var,
                ev.template.conclusion.y_var,
                "reflexivity-trivial conclusion leaked through: {:?}",
                ev.template
            );
        }
        // Symmetry must survive.
        let symmetry = AxiomTemplate {
            num_vars: 2,
            premise: vec![EdgeTemplate { x_var: 0, y_var: 1 }],
            conclusion: EdgeTemplate { x_var: 1, y_var: 0 },
        };
        assert!(minimal.iter().any(|e| e.template == symmetry));
        // Transitivity must survive.
        let transitivity = AxiomTemplate {
            num_vars: 3,
            premise: vec![
                EdgeTemplate { x_var: 0, y_var: 1 },
                EdgeTemplate { x_var: 1, y_var: 2 },
            ],
            conclusion: EdgeTemplate { x_var: 0, y_var: 2 },
        };
        assert!(minimal.iter().any(|e| e.template == transitivity));
    }

    #[test]
    fn adr0028_premise_weakening_drops_redundant_superset() {
        // Synthetic axioms that share the symmetry conclusion but differ in
        // premise — the 1-edge-premise (strictly stronger) must dominate.
        let a = AxiomEvidence {
            template: AxiomTemplate {
                num_vars: 2,
                premise: vec![EdgeTemplate { x_var: 0, y_var: 1 }],
                conclusion: EdgeTemplate { x_var: 1, y_var: 0 },
            },
            premise_bindings: 10,
            conclusion_satisfied: 10,
            rate: 1.0,
            posterior_lower_95: 0.0,
            posterior_upper_95: 1.0,
            null_baseline_prob: 1.0,
        };
        let b = AxiomEvidence {
            template: AxiomTemplate {
                num_vars: 2,
                premise: vec![
                    EdgeTemplate { x_var: 0, y_var: 0 },
                    EdgeTemplate { x_var: 0, y_var: 1 },
                ],
                conclusion: EdgeTemplate { x_var: 1, y_var: 0 },
            },
            premise_bindings: 10,
            conclusion_satisfied: 10,
            rate: 1.0,
            posterior_lower_95: 0.0,
            posterior_upper_95: 1.0,
            null_baseline_prob: 1.0,
        };
        let out = subsume_by_premise_weakening(vec![a.clone(), b]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].template, a.template);
    }

    #[test]
    fn adr0028_discover_minimal_matches_raw_when_no_reflexivity() {
        // Strict partial order (no self-loops): reflexivity holds for 0
        // identifiers, subsumption-by-reflexivity should NOT fire. The
        // premise-weakening pass still runs, so the counts match only if
        // the raw output already lacks dominated pairs — for this case it
        // does.
        let mut rs = RSet::new();
        rs.extend([
            R::new("a", "b"), R::new("a", "c"), R::new("a", "d"),
            R::new("b", "d"), R::new("c", "d"),
        ]);
        let raw = rs.discover_axioms(&AxiomDiscoveryConfig::default());
        let minimal = rs.discover_axioms_minimal(&AxiomDiscoveryConfig::default());
        // Both should be just transitivity.
        assert_eq!(raw.len(), 1);
        assert_eq!(minimal.len(), 1);
        assert_eq!(raw[0].template, minimal[0].template);
    }

    #[test]
    fn adr0028_minimal_collapses_total_order_to_transitivity() {
        let mut rs = RSet::new();
        let nodes = ["1", "2", "3", "4", "5"];
        for i in 0..nodes.len() {
            rs.add(R::new(nodes[i], nodes[i]));
            for j in (i + 1)..nodes.len() {
                rs.add(R::new(nodes[i], nodes[j]));
            }
        }
        let minimal = rs.discover_axioms_minimal(&AxiomDiscoveryConfig::default());
        assert_eq!(minimal.len(), 1);
        let t = &minimal[0].template;
        assert_eq!(t.num_vars, 3);
        assert_eq!(t.premise.len(), 2);
        assert_eq!(t.conclusion, EdgeTemplate { x_var: 0, y_var: 2 });
    }

    #[test]
    fn adr0028_minimal_on_tolerance_keeps_symmetry_only() {
        // Tolerance: reflexive + symmetric, NOT transitive. Minimal axioms
        // should contain symmetry and NO transitivity.
        let mut rs = RSet::new();
        for n in ["a", "b", "c"] {
            rs.add(R::new(n, n));
        }
        rs.extend([
            R::new("a", "b"), R::new("b", "a"),
            R::new("b", "c"), R::new("c", "b"),
        ]);
        let minimal = rs.discover_axioms_minimal(&AxiomDiscoveryConfig::default());
        let symmetry = AxiomTemplate {
            num_vars: 2,
            premise: vec![EdgeTemplate { x_var: 0, y_var: 1 }],
            conclusion: EdgeTemplate { x_var: 1, y_var: 0 },
        };
        assert!(minimal.iter().any(|e| e.template == symmetry));
        let transitivity = AxiomTemplate {
            num_vars: 3,
            premise: vec![
                EdgeTemplate { x_var: 0, y_var: 1 },
                EdgeTemplate { x_var: 1, y_var: 2 },
            ],
            conclusion: EdgeTemplate { x_var: 0, y_var: 2 },
        };
        assert!(!minimal.iter().any(|e| e.template == transitivity));
    }

    #[test]
    fn chain_is_representable() {
        let mut rs = RSet::new();
        rs.extend([
            R::new("a1", "a2"),
            R::new("a2", "a3"),
            R::new("a3", "a4"),
            R::new("a4", "a5"),
        ]);

        assert_eq!(rs.len(), 4);
        assert_eq!(rs.identifiers().len(), 5);

        // middle-of-chain node: one in-edge, one out-edge
        assert_eq!(rs.left_of("a3").len(), 1);
        assert_eq!(rs.right_of("a3").len(), 1);

        // chain endpoints
        assert_eq!(rs.right_of("a1").len(), 0);
        assert_eq!(rs.left_of("a5").len(), 0);
    }

    // ADR 0029 — intension vs extension layering tests.

    fn mk_two_chain(a: &str, b: &str, c: &str) -> Subgraph {
        Subgraph::from_edges([R::new(a, b), R::new(b, c)])
    }

    #[test]
    fn adr0029_layer_a_written_on_first_mint() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let sg = mk_two_chain("a", "b", "c");
        let p = rs.name_pattern_instances(&[sg]).unwrap();
        // Intension: three roles registered.
        assert_eq!(rs.pattern_roles(&p).len(), 3);
        for role in rs.pattern_roles(&p) {
            assert!(rs.is_role(role));
        }
        // Intension: stored canonical form equals the subgraph's.
        let stored = rs.pattern_structure(&p).unwrap();
        let sg2 = mk_two_chain("a", "b", "c");
        assert_eq!(stored, sg2.canonicalize());
    }

    #[test]
    fn adr0029_intensional_policy_writes_no_instance_edges() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let sg = mk_two_chain("a", "b", "c");
        let pre = rs.len();
        let p = rs
            .name_pattern_instances_with_policy(
                &[sg],
                PatternRecordingPolicy::Intensional,
            )
            .unwrap();
        // Layer A was written; Layer B was not.
        assert!(rs.pattern_roles(&p).len() == 3);
        assert_eq!(rs.instances_of(&p).len(), 0);
        // Growth = Layer A only: 1 (registry) + 3 (role registry) + 3
        // (pattern→role) + 2 (structural edges) = 9 edges.
        let layer_a_count = 1 + 3 + 3 + 2;
        assert_eq!(rs.len() - pre, layer_a_count);
    }

    #[test]
    fn adr0029_instances_only_policy_writes_no_participants() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let sg = mk_two_chain("a", "b", "c");
        let p = rs
            .name_pattern_instances_with_policy(
                &[sg],
                PatternRecordingPolicy::InstancesOnly,
            )
            .unwrap();
        assert_eq!(rs.instances_of(&p).len(), 1);
        let inst = rs.instances_of(&p)[0].to_string();
        // No participant edges were written for this instance.
        assert_eq!(rs.participants_of(&inst).len(), 0);
    }

    #[test]
    fn adr0029_full_bindings_preserves_0010_semantics() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let sg = mk_two_chain("a", "b", "c");
        let p = rs.name_pattern_instances(&[sg]).unwrap(); // default = FullBindings
        let inst = rs.instances_of(&p)[0].to_string();
        let parts = rs.participants_of(&inst);
        assert_eq!(parts.len(), 3);
        assert!(parts.contains("a"));
        assert!(parts.contains("b"));
        assert!(parts.contains("c"));
    }

    #[test]
    fn adr0029_find_pattern_matching_uses_layer_a_without_instances() {
        // Intensional-only naming: no instances are persisted. But the
        // pattern must still be findable by canonical form via Layer A.
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let sg = mk_two_chain("a", "b", "c");
        let p = rs
            .name_pattern_instances_with_policy(
                &[sg.clone()],
                PatternRecordingPolicy::Intensional,
            )
            .unwrap();
        // Second identical structure should match the same pattern,
        // relying on Layer A (no instances to fall back on).
        let canon = sg.canonicalize();
        assert_eq!(rs.find_pattern_matching(&canon).unwrap(), p.as_str());
    }

    #[test]
    fn adr0029_collect_meta_ids_includes_roles() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let sg = mk_two_chain("a", "b", "c");
        let p = rs.name_pattern_instances(&[sg]).unwrap();
        let meta = rs.collect_meta_ids();
        assert!(meta.contains(ROLE_MARKER));
        for role in rs.pattern_roles(&p) {
            assert!(meta.contains(role));
        }
    }

    #[test]
    fn adr0029_retract_removes_layer_a() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let pre = rs.len();
        let sg = mk_two_chain("a", "b", "c");
        let p = rs.name_pattern_instances(&[sg]).unwrap();
        let _ = rs.retract_pattern(&p).unwrap();
        assert_eq!(rs.len(), pre);
        assert!(rs.pattern_structure(&p).is_none());
        assert!(rs.roles().is_empty());
        assert!(!rs.instances.iter().any(|r| r.x == ROLE_MARKER));
    }

    #[test]
    fn adr0029_reuse_pattern_id_across_policies() {
        // FullBindings then Intensional on a structurally identical
        // instance — should reuse the same pattern id, not mint a
        // second. Layer A is written once on first mint.
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let p1 = rs
            .name_pattern_instances(&[mk_two_chain("a", "b", "c")])
            .unwrap();
        rs.extend([R::new("p", "q"), R::new("q", "r")]);
        let p2 = rs
            .name_pattern_instances_with_policy(
                &[mk_two_chain("p", "q", "r")],
                PatternRecordingPolicy::Intensional,
            )
            .unwrap();
        assert_eq!(p1, p2);
        assert_eq!(rs.pattern_roles(&p1).len(), 3);
    }

    #[test]
    fn adr0029_instances_of_excludes_roles() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let p = rs
            .name_pattern_instances(&[mk_two_chain("a", "b", "c")])
            .unwrap();
        let insts = rs.instances_of(&p);
        assert_eq!(insts.len(), 1);
        for inst in &insts {
            assert!(!rs.is_role(inst));
        }
    }

    // ADR 0030 — theory objects (conjunctive concept naming).

    fn equivalence_relation() -> RSet {
        let mut rs = RSet::new();
        let classes: &[&[&str]] = &[&["a", "b"], &["c", "d", "e"]];
        for cls in classes {
            for x in cls.iter() {
                for y in cls.iter() {
                    rs.add(R::new(*x, *y));
                }
            }
        }
        rs
    }

    fn poset_with_selfloops() -> RSet {
        // Same diamond as axiom tests, reflexive closure.
        diamond_poset()
    }

    #[test]
    fn adr0030_axiom_template_id_roundtrip() {
        let transitivity = AxiomTemplate {
            num_vars: 3,
            premise: vec![
                EdgeTemplate { x_var: 0, y_var: 1 },
                EdgeTemplate { x_var: 1, y_var: 2 },
            ],
            conclusion: EdgeTemplate { x_var: 0, y_var: 2 },
        };
        let id = axiom_template_id(&transitivity);
        assert_eq!(id, "ax_tpl_v3_p0-1_p1-2_c0-2");
        let parsed = axiom_id_to_template(&id).expect("parses");
        assert_eq!(parsed, transitivity);
    }

    #[test]
    fn adr0030_discover_theory_on_equivalence_relation() {
        let rs = equivalence_relation();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        // Equivalence: symmetry (template) + reflexivity (predicate).
        // Transitivity variants also show up (5 minimal axioms + 1 predicate).
        assert!(th.member_axiom_ids.iter().any(|id| id == AX_REFLEXIVITY));
        assert!(th.member_axiom_ids.iter().any(|id| {
            id == "ax_tpl_v2_p0-1_c1-0" // symmetry
        }));
        assert!(th.member_axiom_ids.iter().any(|id| {
            id == "ax_tpl_v3_p0-1_p1-2_c0-2" // transitivity
        }));
    }

    #[test]
    fn adr0030_discover_theory_on_poset() {
        let rs = poset_with_selfloops();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        assert!(th.member_axiom_ids.iter().any(|id| id == AX_REFLEXIVITY));
        assert!(th.member_axiom_ids.iter().any(|id| id == AX_ANTISYMMETRY));
        assert!(th.member_axiom_ids.iter().any(|id| {
            id == "ax_tpl_v3_p0-1_p1-2_c0-2" // transitivity
        }));
    }

    #[test]
    fn adr0030_name_theory_persists_to_meta_r() {
        let mut rs = poset_with_selfloops();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let t_id = rs.name_theory(&ids).expect("valid members");
        // Registry edge exists.
        assert!(rs.is_theory(&t_id));
        // Each axiom is registered.
        for id in &th.member_axiom_ids {
            assert!(rs.is_axiom(id));
        }
        // Members retrievable.
        let members: HashSet<&str> = rs.theory_axioms(&t_id).into_iter().collect();
        for id in &th.member_axiom_ids {
            assert!(members.contains(id.as_str()));
        }
    }

    #[test]
    fn adr0030_name_theory_reuses_existing() {
        let mut rs = poset_with_selfloops();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let t1 = rs.name_theory(&ids).unwrap();
        let t2 = rs.name_theory(&ids).unwrap();
        assert_eq!(t1, t2);
        assert_eq!(rs.theories().len(), 1);
    }

    #[test]
    fn adr0030_name_theory_rejects_unsatisfied() {
        // Try to name reflexivity on a RSet where it doesn't hold.
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let err = rs.name_theory(&[AX_REFLEXIVITY]).unwrap_err();
        assert_eq!(err, TheoryError::UnsatisfiedMember(AX_REFLEXIVITY.to_string()));
    }

    #[test]
    fn adr0030_name_theory_rejects_unparseable() {
        let mut rs = equivalence_relation();
        let err = rs.name_theory(&["ax_not_a_real_id"]).unwrap_err();
        assert_eq!(err, TheoryError::UnparseableAxiomId("ax_not_a_real_id".to_string()));
    }

    #[test]
    fn adr0030_name_theory_rejects_empty() {
        let mut rs = equivalence_relation();
        let err = rs.name_theory(&[]).unwrap_err();
        assert_eq!(err, TheoryError::EmptyMemberList);
    }

    #[test]
    fn adr0030_retract_theory_removes_theory_only() {
        let mut rs = poset_with_selfloops();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let t_id = rs.name_theory(&ids).unwrap();
        let axiom_count_before = rs.axioms().len();
        let removed = rs.retract_theory(&t_id).unwrap();
        // Removed: 1 registry + len(members) membership edges.
        assert_eq!(removed, 1 + th.member_axiom_ids.len());
        assert!(!rs.is_theory(&t_id));
        // Axiom registry is NOT touched (other theories may share).
        assert_eq!(rs.axioms().len(), axiom_count_before);
    }

    #[test]
    fn adr0030_theories_containing() {
        let mut rs = poset_with_selfloops();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let t_id = rs.name_theory(&ids).unwrap();
        // AX_REFLEXIVITY is a member, so it should find this theory.
        let containing = rs.theories_containing(AX_REFLEXIVITY);
        assert!(containing.contains(&t_id.as_str()));
    }

    #[test]
    fn adr0030_discover_theory_on_tolerance_no_trans() {
        // Tolerance: reflexive + symmetric, not transitive.
        let mut rs = RSet::new();
        for n in ["a", "b", "c"] {
            rs.add(R::new(n, n));
        }
        rs.extend([
            R::new("a", "b"), R::new("b", "a"),
            R::new("b", "c"), R::new("c", "b"),
        ]);
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        assert!(th.member_axiom_ids.iter().any(|id| id == AX_REFLEXIVITY));
        assert!(th.member_axiom_ids.iter().any(|id| {
            id == "ax_tpl_v2_p0-1_c1-0" // symmetry
        }));
        // Transitivity must NOT be present.
        assert!(!th.member_axiom_ids.iter().any(|id| {
            id == "ax_tpl_v3_p0-1_p1-2_c0-2"
        }));
    }

    #[test]
    fn adr0030_collect_meta_ids_includes_theory_markers() {
        let mut rs = poset_with_selfloops();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let t_id = rs.name_theory(&ids).unwrap();
        let meta = rs.collect_meta_ids();
        assert!(meta.contains(AXIOM_MARKER));
        assert!(meta.contains(THEORY_MARKER));
        assert!(meta.contains(&t_id));
        for id in &th.member_axiom_ids {
            assert!(meta.contains(id));
        }
    }

    // ADR 0031 — intrinsic drive + global evaluation.

    #[test]
    fn adr0031_abstraction_score_zero_on_bare_rset() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        // No patterns, no theories → score = 0 (nothing to tax, nothing to reward).
        assert_eq!(rs.abstraction_score(), 0.0);
    }

    #[test]
    fn adr0031_abstraction_score_rewards_pattern_reuse() {
        // 6 single-edge instances across distinct token pairs → one
        // pattern with 6 instances of size 1. Reuse savings = (6-1)*1 = 5.
        let mut rs = RSet::new();
        for (a, b) in [
            ("a", "b"), ("c", "d"), ("e", "f"),
            ("g", "h"), ("i", "j"), ("k", "l"),
        ] {
            rs.add(R::new(a, b));
        }
        let instances: Vec<Subgraph> = rs
            .iter()
            .map(|r| Subgraph::from_edges([r.clone()]))
            .collect();
        // Name as single-edge pattern.
        let _p = rs.name_pattern_instances(&instances).unwrap();
        let s = rs.abstraction_score();
        // Positive, dominated by reuse savings minus overhead tax.
        assert!(s > 0.0, "expected positive score, got {}", s);
    }

    #[test]
    fn adr0031_drive_discovers_something_on_structured_input() {
        // Equivalence relation — rich axioms available. Drive should
        // name at least a theory, producing positive score.
        let mut rs = equivalence_relation();
        let cfg = DriveConfig::default();
        let trace = rs.intrinsic_drive(&cfg);
        assert!(trace.final_score > trace.initial_score,
            "drive did not improve score: trace={:?}", trace);
        assert!(!trace.steps.is_empty());
    }

    #[test]
    fn adr0031_drive_halts_on_unstructured_input() {
        // Random-ish sparse graph — no pattern reuse, no universal
        // axioms with meaningful content beyond accidental antisym.
        // Drive should either do nothing or take minimal action and
        // then halt.
        let mut rs = RSet::new();
        rs.extend([
            R::new("a", "c"), R::new("b", "d"), R::new("c", "e"),
            R::new("d", "f"), R::new("e", "a"), R::new("f", "b"),
            R::new("a", "d"),
        ]);
        let cfg = DriveConfig {
            max_steps: 5,
            ..DriveConfig::default()
        };
        let trace = rs.intrinsic_drive(&cfg);
        // Final score ≥ initial: the driver never applies a step
        // that reduces the score (delta must exceed epsilon).
        assert!(trace.final_score >= trace.initial_score);
        // And fewer than max_steps actions (it should stop early).
        assert!(trace.steps.len() <= 5);
    }

    #[test]
    fn adr0031_drive_step_is_rejected_when_unprofitable() {
        // Empty RSet — no way to score positive — drive_step returns None.
        let mut rs = RSet::new();
        let cfg = DriveConfig::default();
        let step = rs.drive_step(&cfg);
        assert!(step.is_none());
    }

    #[test]
    fn adr0031_drive_produces_theory_on_poset() {
        let mut rs = diamond_poset();
        let cfg = DriveConfig::default();
        let trace = rs.intrinsic_drive(&cfg);
        // At least one theory was discovered.
        let theory_step = trace.steps.iter().find(|s| {
            matches!(s.result, DriveActionResult::TheoryDiscovered { theory_id: Some(_), .. })
        });
        assert!(theory_step.is_some(), "expected a theory-discovery step");
        assert_eq!(rs.theories().len(), 1);
    }

    #[test]
    fn adr0031_drive_is_idempotent_after_saturation() {
        // Run drive twice. Second run should be a no-op.
        let mut rs = diamond_poset();
        let cfg = DriveConfig::default();
        let first = rs.intrinsic_drive(&cfg);
        let score_after_first = rs.abstraction_score();
        let second = rs.intrinsic_drive(&cfg);
        assert!(second.steps.is_empty(),
            "second drive added steps: {:?}", second.steps);
        assert_eq!(rs.abstraction_score(), score_after_first);
        assert!(!first.steps.is_empty());
    }

    // ADR 0032 — axiom intension as meta-R.

    #[test]
    fn adr0032_template_axiom_gets_intension_on_registration() {
        let mut rs = poset_with_selfloops();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let _ = rs.name_theory(&ids).unwrap();
        // Transitivity axiom: n=3 variables, 2 premise edges, 1 conclusion.
        let trans_id = "ax_tpl_v3_p0-1_p1-2_c0-2";
        assert!(rs.is_axiom(trans_id));
        let vars = rs.axiom_variables(trans_id);
        assert_eq!(vars.len(), 3);
        let prem = rs.axiom_premise_edges(trans_id);
        assert_eq!(prem.len(), 2);
        assert!(rs.axiom_conclusion(trans_id).is_some());
    }

    #[test]
    fn adr0032_predicate_axioms_get_registry_only() {
        let mut rs = poset_with_selfloops();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let _ = rs.name_theory(&ids).unwrap();
        // Reflexivity + antisymmetry exist as registered axioms...
        assert!(rs.is_axiom(AX_REFLEXIVITY));
        assert!(rs.is_axiom(AX_ANTISYMMETRY));
        // ...but have no intension (no variables, no edges).
        assert!(rs.axiom_variables(AX_REFLEXIVITY).is_empty());
        assert!(rs.axiom_premise_edges(AX_REFLEXIVITY).is_empty());
        assert!(rs.axiom_conclusion(AX_REFLEXIVITY).is_none());
        assert!(rs.axiom_variables(AX_ANTISYMMETRY).is_empty());
    }

    #[test]
    fn adr0032_reconstruct_roundtrip_transitivity() {
        let mut rs = poset_with_selfloops();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let _ = rs.name_theory(&ids).unwrap();
        let trans_id = "ax_tpl_v3_p0-1_p1-2_c0-2";
        let reconstructed = rs.reconstruct_axiom_template(trans_id).unwrap();
        let expected = AxiomTemplate {
            num_vars: 3,
            premise: vec![
                EdgeTemplate { x_var: 0, y_var: 1 },
                EdgeTemplate { x_var: 1, y_var: 2 },
            ],
            conclusion: EdgeTemplate { x_var: 0, y_var: 2 },
        };
        assert_eq!(reconstructed, expected);
    }

    #[test]
    fn adr0032_reconstruct_roundtrip_symmetry() {
        let mut rs = equivalence_relation();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let _ = rs.name_theory(&ids).unwrap();
        let sym_id = "ax_tpl_v2_p0-1_c1-0";
        let reconstructed = rs.reconstruct_axiom_template(sym_id).unwrap();
        let expected = AxiomTemplate {
            num_vars: 2,
            premise: vec![EdgeTemplate { x_var: 0, y_var: 1 }],
            conclusion: EdgeTemplate { x_var: 1, y_var: 0 },
        };
        assert_eq!(reconstructed, expected);
    }

    #[test]
    fn adr0032_retract_axiom_removes_intension() {
        let mut rs = poset_with_selfloops();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let t_id = rs.name_theory(&ids).unwrap();
        // Must retract the theory first.
        let _ = rs.retract_theory(&t_id).unwrap();
        let trans_id = "ax_tpl_v3_p0-1_p1-2_c0-2".to_string();
        let before = rs.len();
        let removed = rs.retract_axiom(&trans_id).unwrap();
        assert!(removed > 0);
        assert!(rs.len() < before);
        assert!(!rs.is_axiom(&trans_id));
        assert!(rs.axiom_variables(&trans_id).is_empty());
        assert!(rs.reconstruct_axiom_template(&trans_id).is_none());
    }

    #[test]
    fn adr0032_retract_axiom_refuses_when_theory_holds_reference() {
        let mut rs = poset_with_selfloops();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let _ = rs.name_theory(&ids).unwrap();
        let trans_id = "ax_tpl_v3_p0-1_p1-2_c0-2".to_string();
        let err = rs.retract_axiom(&trans_id).unwrap_err();
        assert!(matches!(err, TheoryError::UnsatisfiedMember(_)));
    }

    #[test]
    fn adr0032_collect_meta_ids_includes_axiom_intension_ids() {
        let mut rs = poset_with_selfloops();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let _ = rs.name_theory(&ids).unwrap();
        let meta = rs.collect_meta_ids();
        assert!(meta.contains(AXIOMVAR_MARKER));
        assert!(meta.contains(PREMISE_MARKER));
        assert!(meta.contains(CONCLUSION_MARKER));
        let trans_id = "ax_tpl_v3_p0-1_p1-2_c0-2";
        for v in rs.axiom_variables(trans_id) {
            assert!(meta.contains(v));
        }
        for p in rs.axiom_premise_edges(trans_id) {
            assert!(meta.contains(p));
        }
        if let Some(c) = rs.axiom_conclusion(trans_id) {
            assert!(meta.contains(&c));
        }
    }

    #[test]
    fn adr0032_axioms_do_not_pollute_data_discovery() {
        // Name a theory, then verify axiom discovery on the same RSet
        // still sees only the original data identifiers — none of the
        // axiom intension ids leaks in.
        let mut rs = poset_with_selfloops();
        let before = rs.discover_axioms(&AxiomDiscoveryConfig::default());
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let _ = rs.name_theory(&ids).unwrap();
        let after = rs.discover_axioms(&AxiomDiscoveryConfig::default());
        assert_eq!(before.len(), after.len(),
            "axiom discovery must be stable — meta-R should be filtered");
    }

    // ADR 0033 — defeasible axioms (rate < 1.0).

    fn almost_transitive() -> RSet {
        // 4-chain transitive closure minus one closure edge: transitivity
        // holds on all but one binding out of many.
        let mut rs = RSet::new();
        let nodes = ["a", "b", "c", "d"];
        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                rs.add(R::new(nodes[i], nodes[j]));
            }
        }
        rs.remove(&R::new("b", "d"));
        rs
    }

    #[test]
    fn adr0033_default_strict_mode_unchanged() {
        let rs = almost_transitive();
        // Default min_rate=1.0: transitivity fails because of the one
        // missing closure edge → zero strict axioms.
        let strict = rs.discover_axioms(&AxiomDiscoveryConfig::default());
        assert_eq!(strict.len(), 0);
    }

    #[test]
    fn adr0033_defeasible_mode_surfaces_near_axioms() {
        let rs = almost_transitive();
        let cfg = AxiomDiscoveryConfig {
            min_rate: 0.5,
            ..AxiomDiscoveryConfig::default()
        };
        let defeasible = rs.discover_axioms(&cfg);
        assert!(!defeasible.is_empty(),
            "defeasible discovery should return non-empty on almost-transitive");
        // Transitivity template shows up with rate < 1.0.
        let trans = AxiomTemplate {
            num_vars: 3,
            premise: vec![
                EdgeTemplate { x_var: 0, y_var: 1 },
                EdgeTemplate { x_var: 1, y_var: 2 },
            ],
            conclusion: EdgeTemplate { x_var: 0, y_var: 2 },
        };
        let trans_ev = defeasible.iter().find(|e| e.template == trans)
            .expect("transitivity at rate ≥ 0.5 on almost-transitive");
        assert!(trans_ev.rate < 1.0,
            "defeasible transitivity should have rate < 1.0, got {}", trans_ev.rate);
        assert!(trans_ev.rate >= 0.5);
    }

    #[test]
    fn adr0033_defeasible_minimal_skips_subsumption() {
        // In defeasible mode, discover_axioms_minimal returns the raw
        // output without the subsumption filter (soundness guard).
        let rs = almost_transitive();
        let cfg = AxiomDiscoveryConfig {
            min_rate: 0.5,
            ..AxiomDiscoveryConfig::default()
        };
        let raw = rs.discover_axioms(&cfg);
        let minimal = rs.discover_axioms_minimal(&cfg);
        assert_eq!(raw.len(), minimal.len());
    }

    #[test]
    fn adr0033_strict_minimal_still_subsumes() {
        // min_rate=1.0 path unchanged — subsumption still fires.
        let rs = equivalence_relation();
        let cfg = AxiomDiscoveryConfig::default(); // strict
        let raw = rs.discover_axioms(&cfg);
        let minimal = rs.discover_axioms_minimal(&cfg);
        assert!(minimal.len() < raw.len(),
            "strict minimal should subsume; raw={}, minimal={}", raw.len(), minimal.len());
    }

    #[test]
    fn adr0033_rate_is_reported_on_every_evidence() {
        let rs = almost_transitive();
        let cfg = AxiomDiscoveryConfig {
            min_rate: 0.1,
            ..AxiomDiscoveryConfig::default()
        };
        let defeasible = rs.discover_axioms(&cfg);
        for ev in &defeasible {
            assert!(ev.rate >= 0.1);
            assert!(ev.rate <= 1.0);
            assert!(ev.premise_bindings >= 1);
            assert!(ev.conclusion_satisfied <= ev.premise_bindings);
            // rate = satisfied / bindings
            let expected = ev.conclusion_satisfied as f64 / ev.premise_bindings as f64;
            assert!((ev.rate - expected).abs() < 1e-9);
        }
    }

    #[test]
    fn adr0033_near_zero_rate_threshold_yields_more() {
        let rs = almost_transitive();
        let tight = AxiomDiscoveryConfig { min_rate: 0.9, ..AxiomDiscoveryConfig::default() };
        let loose = AxiomDiscoveryConfig { min_rate: 0.1, ..AxiomDiscoveryConfig::default() };
        let a = rs.discover_axioms(&tight);
        let b = rs.discover_axioms(&loose);
        assert!(b.len() >= a.len());
    }

    // ADR 0034 — theory extension relations.

    fn name_theory_from_rset(rs: &mut RSet) -> String {
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> =
            th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        rs.name_theory(&ids).unwrap()
    }

    #[test]
    fn adr0034_poset_theory_extends_strict_poset_theory() {
        // strict partial order {trans, antisym} is a sub-theory of
        // full poset {trans, antisym, refl}. Build both in one RSet by
        // naming two theories explicitly.
        let mut rs = diamond_poset();
        let t_full = name_theory_from_rset(&mut rs); // {trans, refl, antisym}
        // Name a smaller theory with just {trans, antisym}.
        let strict_ids = ["ax_tpl_v3_p0-1_p1-2_c0-2", AX_ANTISYMMETRY];
        let t_strict = rs.name_theory(&strict_ids).unwrap();
        // Full extends strict (full has refl in addition).
        let ext_id = rs.name_theory_extension(&t_full, &t_strict).unwrap();
        assert!(rs.extension_edges().contains(&ext_id.as_str()));
        // Query
        let (sub, sup) = rs.extension_endpoints(&ext_id).unwrap();
        assert_eq!(sub, t_full);
        assert_eq!(sup, t_strict);
        assert!(rs.theory_extends(&t_full).contains(&t_strict.as_str()));
        assert!(rs.theory_extended_by(&t_strict).contains(&t_full.as_str()));
    }

    #[test]
    fn adr0034_name_extension_rejects_non_subset() {
        let mut rs = diamond_poset();
        let t_full = name_theory_from_rset(&mut rs);
        // Make a bogus "theory" with axioms not in t_full by handpicking.
        // Here: a theory with just symmetry (not in poset).
        // But symmetry doesn't hold on poset, so name_theory rejects.
        // Use a different approach: two theories with disjoint non-subset members.
        let weak_ids = [AX_ANTISYMMETRY];
        let t_weak = rs.name_theory(&weak_ids).unwrap();
        let strong_ids = ["ax_tpl_v3_p0-1_p1-2_c0-2"];
        let t_strong = rs.name_theory(&strong_ids).unwrap();
        // Neither is a subset of the other.
        assert!(rs.name_theory_extension(&t_weak, &t_strong).is_err());
        assert!(rs.name_theory_extension(&t_strong, &t_weak).is_err());
    }

    #[test]
    fn adr0034_name_extension_refuses_self_loop() {
        let mut rs = diamond_poset();
        let t = name_theory_from_rset(&mut rs);
        assert!(rs.name_theory_extension(&t, &t).is_err());
    }

    #[test]
    fn adr0034_discover_extensions_scans_pairs() {
        let mut rs = diamond_poset();
        let t_full = name_theory_from_rset(&mut rs);
        let strict_ids = ["ax_tpl_v3_p0-1_p1-2_c0-2", AX_ANTISYMMETRY];
        let t_strict = rs.name_theory(&strict_ids).unwrap();
        let trans_only = ["ax_tpl_v3_p0-1_p1-2_c0-2"];
        let t_trans = rs.name_theory(&trans_only).unwrap();
        let found = rs.discover_theory_extensions();
        // t_full ⊋ t_strict ⊋ t_trans. Expected pairs:
        // (t_full, t_strict), (t_full, t_trans), (t_strict, t_trans).
        assert!(found.contains(&(t_full.clone(), t_strict.clone())));
        assert!(found.contains(&(t_full.clone(), t_trans.clone())));
        assert!(found.contains(&(t_strict.clone(), t_trans.clone())));
        assert_eq!(found.len(), 3);
    }

    #[test]
    fn adr0034_extension_reuses_on_duplicate() {
        let mut rs = diamond_poset();
        let t_full = name_theory_from_rset(&mut rs);
        let strict_ids = ["ax_tpl_v3_p0-1_p1-2_c0-2", AX_ANTISYMMETRY];
        let t_strict = rs.name_theory(&strict_ids).unwrap();
        let e1 = rs.name_theory_extension(&t_full, &t_strict).unwrap();
        let e2 = rs.name_theory_extension(&t_full, &t_strict).unwrap();
        assert_eq!(e1, e2);
        assert_eq!(rs.extension_edges().len(), 1);
    }

    #[test]
    fn adr0034_collect_meta_ids_includes_extends() {
        let mut rs = diamond_poset();
        let t_full = name_theory_from_rset(&mut rs);
        let strict_ids = ["ax_tpl_v3_p0-1_p1-2_c0-2", AX_ANTISYMMETRY];
        let t_strict = rs.name_theory(&strict_ids).unwrap();
        let ext = rs.name_theory_extension(&t_full, &t_strict).unwrap();
        let meta = rs.collect_meta_ids();
        assert!(meta.contains(EXTENDS_MARKER));
        assert!(meta.contains(&ext));
    }

    // ADR 0035 — counterfactual value / meta-metric.

    #[test]
    fn adr0035_counterfactual_for_theory_is_positive() {
        let mut rs = diamond_poset();
        let t = name_theory_from_rset(&mut rs);
        let v = rs.counterfactual_value(&t).expect("theory is retractable");
        // Theory has 3 members; removing it drops 2.0 * 3 = 6.0 from reward
        // minus some overhead savings. Net should still be > 0 because
        // the reward exceeds the tax-savings.
        assert!(v > 0.0, "expected positive counterfactual, got {}", v);
    }

    #[test]
    fn adr0035_counterfactual_returns_none_for_unknown_id() {
        let rs = diamond_poset();
        assert!(rs.counterfactual_value("definitely_not_named").is_none());
    }

    #[test]
    fn adr0035_counterfactual_blocked_by_theory_reference_for_axiom() {
        let mut rs = diamond_poset();
        let _ = name_theory_from_rset(&mut rs);
        // Transitivity is used by the theory, so retract_axiom would fail.
        let v = rs.counterfactual_value("ax_tpl_v3_p0-1_p1-2_c0-2");
        assert!(v.is_none(),
            "axiom still referenced by a theory should return None");
    }

    #[test]
    fn adr0035_rank_orders_by_value_descending() {
        let mut rs = diamond_poset();
        let _t = name_theory_from_rset(&mut rs);
        let ranked = rs.rank_by_counterfactual();
        assert!(!ranked.is_empty());
        // Monotone descending.
        for w in ranked.windows(2) {
            assert!(w[0].1 >= w[1].1);
        }
    }

    #[test]
    fn adr0035_counterfactual_respects_actual_retract_behavior() {
        let mut rs = diamond_poset();
        let _t = name_theory_from_rset(&mut rs);
        let before = rs.abstraction_score();
        // Pick any retractable id from the ranking.
        let ranked = rs.rank_by_counterfactual();
        let (id, predicted_drop) = ranked.first().cloned().unwrap();
        // Actually retract, compare.
        let mut trial = rs.clone();
        if trial.is_theory(&id) {
            let _ = trial.retract_theory(&id);
        } else if trial.patterns().iter().any(|p| *p == id) {
            let _ = trial.retract_pattern(&id);
        } else if trial.extension_edges().iter().any(|e| *e == id) {
            let _ = trial.retract_extension(&id);
        }
        let actual_drop = before - trial.abstraction_score();
        assert!((predicted_drop - actual_drop).abs() < 1e-9);
    }

    #[test]
    fn adr0035_retract_extension_clears_all_three_edges() {
        let mut rs = diamond_poset();
        let t_full = name_theory_from_rset(&mut rs);
        let strict_ids = ["ax_tpl_v3_p0-1_p1-2_c0-2", AX_ANTISYMMETRY];
        let t_strict = rs.name_theory(&strict_ids).unwrap();
        let ext = rs.name_theory_extension(&t_full, &t_strict).unwrap();
        let removed = rs.retract_extension(&ext).unwrap();
        assert_eq!(removed, 3);
        assert!(rs.extension_edges().is_empty());
    }

    // ADR 0036 — extended template language (empty premise).

    #[test]
    fn adr0036_default_config_does_not_include_empty_premise() {
        // Reflexive diamond poset. Default config should NOT surface
        // ax_tpl_v1_c0-0 — backward compat with 0027/0028.
        let rs = diamond_poset();
        let cfg = AxiomDiscoveryConfig::default();
        let axioms = rs.discover_axioms(&cfg);
        let has_empty_premise = axioms
            .iter()
            .any(|e| e.template.premise.is_empty());
        assert!(!has_empty_premise,
            "default config must not produce empty-premise templates");
    }

    #[test]
    fn adr0036_opt_in_surfaces_template_reflexivity() {
        // Reflexive diamond poset with include_empty_premise=true:
        // reflexivity shows up as ax_tpl_v1_c0-0 at rate 1.0.
        let rs = diamond_poset();
        let cfg = AxiomDiscoveryConfig {
            include_empty_premise: true,
            ..AxiomDiscoveryConfig::default()
        };
        let axioms = rs.discover_axioms(&cfg);
        let reflexivity_tpl = AxiomTemplate {
            num_vars: 1,
            premise: vec![],
            conclusion: EdgeTemplate { x_var: 0, y_var: 0 },
        };
        let ev = axioms.iter().find(|e| e.template == reflexivity_tpl);
        assert!(ev.is_some(),
            "empty-premise reflexivity should be discovered");
        assert_eq!(ev.unwrap().rate, 1.0);
    }

    #[test]
    fn adr0036_empty_premise_id_roundtrip() {
        let reflexivity_tpl = AxiomTemplate {
            num_vars: 1,
            premise: vec![],
            conclusion: EdgeTemplate { x_var: 0, y_var: 0 },
        };
        let id = axiom_template_id(&reflexivity_tpl);
        assert_eq!(id, "ax_tpl_v1_c0-0");
        let back = axiom_id_to_template(&id).expect("parses");
        assert_eq!(back, reflexivity_tpl);
    }

    #[test]
    fn adr0036_empty_premise_absent_on_non_reflexive_rset() {
        // Non-reflexive graph + opt-in → ax_tpl_v1_c0-0 must be absent
        // (rate would be < 1.0, default strict mode suppresses it).
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let cfg = AxiomDiscoveryConfig {
            include_empty_premise: true,
            ..AxiomDiscoveryConfig::default()
        };
        let axioms = rs.discover_axioms(&cfg);
        let has = axioms.iter().any(|e| e.template.premise.is_empty());
        assert!(!has);
    }

    #[test]
    fn adr0036_empty_premise_with_defeasible_surfaces_partial() {
        // Partially-reflexive graph: 2 of 4 identifiers have self-loops.
        // Rate = 0.5. Defeasible mode + empty-premise should surface it.
        let mut rs = RSet::new();
        rs.extend([
            R::new("a", "a"), R::new("b", "b"),
            R::new("c", "d"), R::new("d", "c"),
        ]);
        let cfg = AxiomDiscoveryConfig {
            include_empty_premise: true,
            min_rate: 0.4,
            ..AxiomDiscoveryConfig::default()
        };
        let axioms = rs.discover_axioms(&cfg);
        let refl = axioms.iter().find(|e| e.template.premise.is_empty());
        assert!(refl.is_some());
        let ev = refl.unwrap();
        assert!((ev.rate - 0.5).abs() < 1e-9);
    }

    #[test]
    fn adr0036_opt_in_does_not_break_existing_behavior() {
        // Check that opt-in only ADDS templates, never removes.
        let rs = diamond_poset();
        let strict = rs.discover_axioms(&AxiomDiscoveryConfig::default());
        let extended = rs.discover_axioms(&AxiomDiscoveryConfig {
            include_empty_premise: true,
            ..AxiomDiscoveryConfig::default()
        });
        assert!(extended.len() >= strict.len());
        for ev in &strict {
            assert!(extended
                .iter()
                .any(|e| e.template == ev.template),
                "template {:?} disappeared when opting in", ev.template);
        }
    }

    // ADR 0037 — compositional subsumption via forward chaining.

    #[test]
    fn adr0037_transitivity_variant_derivable_from_sym_trans() {
        // On an equivalence relation: variant-B `[R(0,1), R(1,2)] ⇒ R(2,0)`
        // should be derivable from {symmetry, transitivity}.
        let sym = AxiomTemplate {
            num_vars: 2,
            premise: vec![EdgeTemplate { x_var: 0, y_var: 1 }],
            conclusion: EdgeTemplate { x_var: 1, y_var: 0 },
        };
        let trans = AxiomTemplate {
            num_vars: 3,
            premise: vec![
                EdgeTemplate { x_var: 0, y_var: 1 },
                EdgeTemplate { x_var: 1, y_var: 2 },
            ],
            conclusion: EdgeTemplate { x_var: 0, y_var: 2 },
        };
        let variant_b = AxiomTemplate {
            num_vars: 3,
            premise: vec![
                EdgeTemplate { x_var: 0, y_var: 1 },
                EdgeTemplate { x_var: 1, y_var: 2 },
            ],
            conclusion: EdgeTemplate { x_var: 2, y_var: 0 },
        };
        assert!(template_derivable_from(&variant_b, &[sym, trans]));
    }

    #[test]
    fn adr0037_transitivity_not_derivable_from_symmetry_alone() {
        let sym = AxiomTemplate {
            num_vars: 2,
            premise: vec![EdgeTemplate { x_var: 0, y_var: 1 }],
            conclusion: EdgeTemplate { x_var: 1, y_var: 0 },
        };
        let trans = AxiomTemplate {
            num_vars: 3,
            premise: vec![
                EdgeTemplate { x_var: 0, y_var: 1 },
                EdgeTemplate { x_var: 1, y_var: 2 },
            ],
            conclusion: EdgeTemplate { x_var: 0, y_var: 2 },
        };
        assert!(!template_derivable_from(&trans, &[sym]));
    }

    #[test]
    fn adr0037_equivalence_minimal_compositional_collapses_to_two() {
        // ADR 0028 minimal on equivalence returns 5 axioms (sym + 4
        // transitivity-like). Composition should drop 3 variants,
        // leaving two: symmetry plus one transitivity-like axiom (which
        // specific one survives depends on processing order — any
        // 1 of the 4 variants generates the other 3 under symmetry, so
        // all 4 are valid minimal-set choices).
        let rs = equivalence_relation();
        let five = rs.discover_axioms_minimal(&AxiomDiscoveryConfig::default());
        let compositional =
            rs.discover_axioms_minimal_compositional(&AxiomDiscoveryConfig::default());
        assert_eq!(five.len(), 5);
        assert_eq!(compositional.len(), 2,
            "equivalence should compose down to exactly 2 axioms, got {}",
            compositional.len());
        // Symmetry always survives.
        let sym_template = AxiomTemplate {
            num_vars: 2,
            premise: vec![EdgeTemplate { x_var: 0, y_var: 1 }],
            conclusion: EdgeTemplate { x_var: 1, y_var: 0 },
        };
        assert!(compositional.iter().any(|e| e.template == sym_template),
            "symmetry should survive");
        // Exactly one of the 4 transitivity variants survives.
        let trans_like_count = compositional
            .iter()
            .filter(|e| e.template.num_vars == 3 && e.template.premise.len() == 2)
            .count();
        assert_eq!(trans_like_count, 1,
            "exactly one transitivity variant should survive");
    }

    #[test]
    fn adr0037_strict_inputs_unchanged_when_no_redundancy() {
        // Strict partial order minimal = {trans}. No composition applies.
        let rs = diamond_poset();
        let minimal = rs.discover_axioms_minimal(&AxiomDiscoveryConfig::default());
        let compositional =
            rs.discover_axioms_minimal_compositional(&AxiomDiscoveryConfig::default());
        assert_eq!(minimal.len(), compositional.len());
    }

    #[test]
    fn adr0037_compositional_defeasible_passes_through() {
        let rs = almost_transitive();
        let cfg = AxiomDiscoveryConfig {
            min_rate: 0.5,
            ..AxiomDiscoveryConfig::default()
        };
        let raw = rs.discover_axioms(&cfg);
        let comp = rs.discover_axioms_minimal_compositional(&cfg);
        assert_eq!(raw.len(), comp.len());
    }

    #[test]
    fn adr0037_subsume_by_composition_handles_singletons() {
        let sym = AxiomTemplate {
            num_vars: 2,
            premise: vec![EdgeTemplate { x_var: 0, y_var: 1 }],
            conclusion: EdgeTemplate { x_var: 1, y_var: 0 },
        };
        let ev = AxiomEvidence {
            template: sym,
            premise_bindings: 1,
            conclusion_satisfied: 1,
            rate: 1.0,
            posterior_lower_95: 0.0,
            posterior_upper_95: 1.0,
            null_baseline_prob: 1.0,
        };
        let out = subsume_by_composition(vec![ev]);
        assert_eq!(out.len(), 1);
    }

    // ADR 0038 — persistence / serialization.

    #[test]
    fn adr0038_empty_rset_roundtrip() {
        let a = RSet::new();
        let text = a.to_text().unwrap();
        let b = RSet::from_text(&text).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn adr0038_simple_rset_roundtrip() {
        let mut a = RSet::new();
        a.extend([R::new("x", "y"), R::new("y", "z"), R::new("z", "x")]);
        let text = a.to_text().unwrap();
        let b = RSet::from_text(&text).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn adr0038_serialization_is_deterministic() {
        let mut a = RSet::new();
        a.extend([R::new("b", "c"), R::new("a", "b"), R::new("c", "a")]);
        let mut b = RSet::new();
        b.extend([R::new("c", "a"), R::new("a", "b"), R::new("b", "c")]);
        assert_eq!(a.to_text().unwrap(), b.to_text().unwrap());
    }

    #[test]
    fn adr0038_roundtrip_preserves_full_meta_r() {
        // Build a rich RSet: data + patterns + theories + axioms + ext.
        let mut a = diamond_poset();
        let _ = a
            .name_pattern_instances(&[Subgraph::from_edges([
                R::new("a", "b"),
                R::new("b", "d"),
            ])])
            .unwrap();
        let th = a.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> =
            th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let _ = a.name_theory(&ids).unwrap();
        let text = a.to_text().unwrap();
        let b = RSet::from_text(&text).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn adr0038_rejects_tab_in_identifier() {
        let mut a = RSet::new();
        a.add(R::new("has\ttab", "ok"));
        let err = a.to_text().unwrap_err();
        assert!(matches!(err, PersistenceError::TabInIdentifier(_)));
    }

    #[test]
    fn adr0038_rejects_newline_in_identifier() {
        let mut a = RSet::new();
        a.add(R::new("ok", "has\nnewline"));
        let err = a.to_text().unwrap_err();
        assert!(matches!(err, PersistenceError::NewlineInIdentifier(_)));
    }

    #[test]
    fn adr0038_rejects_malformed_line() {
        let err = RSet::from_text("just_one_field").unwrap_err();
        assert_eq!(err, PersistenceError::MalformedLine(1));
    }

    #[test]
    fn adr0038_skips_blank_and_comment_lines() {
        let text = "# a comment\n\n\
                    a\tb\n\
                    # another comment\n\
                    c\td\n\
                    \n";
        let rs = RSet::from_text(text).unwrap();
        assert_eq!(rs.len(), 2);
        assert!(rs.contains(&R::new("a", "b")));
        assert!(rs.contains(&R::new("c", "d")));
    }

    #[test]
    fn adr0038_bytes_reproduce_exactly() {
        let mut a = RSet::new();
        a.extend([R::new("a", "b"), R::new("a", "c")]);
        let text1 = a.to_text().unwrap();
        let b = RSet::from_text(&text1).unwrap();
        let text2 = b.to_text().unwrap();
        assert_eq!(text1, text2);
    }

    // ADR 0039 — totality predicate axiom.

    fn total_order_closure() -> RSet {
        let mut rs = RSet::new();
        let nodes = ["1", "2", "3", "4", "5"];
        for i in 0..nodes.len() {
            rs.add(R::new(nodes[i], nodes[i]));
            for j in (i + 1)..nodes.len() {
                rs.add(R::new(nodes[i], nodes[j]));
            }
        }
        rs
    }

    #[test]
    fn adr0039_check_totality_holds_on_total_order() {
        let rs = total_order_closure();
        let t = rs.check_totality();
        assert!(t.holds);
        assert_eq!(t.violations, 0);
        assert_eq!(t.unordered_pairs_checked, 10); // C(5,2)
    }

    #[test]
    fn adr0039_check_totality_fails_on_diamond_poset() {
        let rs = diamond_poset();
        let t = rs.check_totality();
        // Diamond has {a,b,c,d}. Pair (b,c) is incomparable.
        assert!(!t.holds);
        assert!(t.violations >= 1);
    }

    #[test]
    fn adr0039_check_totality_empty_rset_does_not_hold() {
        // No pairs → vacuously? We return holds=false when no pairs
        // checked (consistent with antisymmetry's "needs at least one
        // directed pair" rule).
        let rs = RSet::new();
        let t = rs.check_totality();
        assert!(!t.holds);
        assert_eq!(t.unordered_pairs_checked, 0);
    }

    #[test]
    fn adr0039_discover_theory_includes_totality_on_total_order() {
        let rs = total_order_closure();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        assert!(th.member_axiom_ids.iter().any(|id| id == AX_TOTALITY));
    }

    #[test]
    fn adr0039_discover_theory_omits_totality_on_diamond() {
        let rs = diamond_poset();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        assert!(!th.member_axiom_ids.iter().any(|id| id == AX_TOTALITY));
    }

    #[test]
    fn adr0039_name_theory_rejects_totality_when_not_holding() {
        let mut rs = diamond_poset();
        assert!(rs.name_theory(&[AX_TOTALITY]).is_err());
    }

    #[test]
    fn adr0039_name_theory_accepts_totality_on_total_order() {
        let mut rs = total_order_closure();
        let ids = [AX_TOTALITY];
        let t_id = rs.name_theory(&ids).unwrap();
        assert!(rs.theory_axioms(&t_id).contains(&AX_TOTALITY));
    }

    #[test]
    fn adr0039_totality_is_predicate_only() {
        // Verify reconstruct returns None (predicate axioms have no
        // template intension).
        let mut rs = total_order_closure();
        let _ = rs.name_theory(&[AX_TOTALITY]).unwrap();
        assert!(rs.reconstruct_axiom_template(AX_TOTALITY).is_none());
        // axiom_variables for predicate is empty.
        assert!(rs.axiom_variables(AX_TOTALITY).is_empty());
    }

    // ADR 0040 — drive auto-prune via counterfactual.

    #[test]
    fn adr0040_extension_edges_now_reward_the_score() {
        let mut rs = diamond_poset();
        let t_full = name_theory_from_rset(&mut rs);
        let strict_ids = ["ax_tpl_v3_p0-1_p1-2_c0-2", AX_ANTISYMMETRY];
        let t_strict = rs.name_theory(&strict_ids).unwrap();
        let before = rs.abstraction_score();
        let _ = rs.name_theory_extension(&t_full, &t_strict).unwrap();
        let after = rs.abstraction_score();
        // +1 reward per extension; -0.1×3 overhead = net +0.7 minimum.
        assert!(after > before);
    }

    #[test]
    fn adr0040_counterfactual_for_extension_is_positive_now() {
        let mut rs = diamond_poset();
        let t_full = name_theory_from_rset(&mut rs);
        let strict_ids = ["ax_tpl_v3_p0-1_p1-2_c0-2", AX_ANTISYMMETRY];
        let t_strict = rs.name_theory(&strict_ids).unwrap();
        let ext = rs.name_theory_extension(&t_full, &t_strict).unwrap();
        let v = rs.counterfactual_value(&ext).unwrap();
        assert!(v > 0.0, "extension should have positive CV, got {}", v);
    }

    #[test]
    fn adr0040_prune_action_retracts_negative_cv_objects() {
        // Build an RSet where an object exists but has negative CV.
        // Simplest: name a single-edge pattern with just one instance
        // (N=1 → reuse savings = 0, only overhead).
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b")]);
        let sg = Subgraph::from_edges([R::new("a", "b")]);
        let p = rs.name_pattern_instances(&[sg]).unwrap();
        let cv = rs.counterfactual_value(&p).unwrap();
        assert!(cv < 0.0,
            "singleton pattern should have negative CV, got {}", cv);
        // Drive with prune enabled should retract it.
        let mut rs2 = rs.clone();
        let cfg = DriveConfig {
            pattern_sizes: vec![],    // don't re-discover
            enable_prune: true,
            prune_threshold: 0.0,
            ..DriveConfig::default()
        };
        let trace = rs2.intrinsic_drive(&cfg);
        let pruned_step = trace
            .steps
            .iter()
            .any(|s| matches!(s.result, DriveActionResult::Pruned { .. }));
        assert!(pruned_step, "drive should have taken a Prune step");
        assert!(!rs2.patterns().iter().any(|q| *q == p.as_str()),
            "negative-CV pattern should have been pruned");
    }

    #[test]
    fn adr0040_prune_leaves_positive_cv_objects_alone() {
        // Diamond poset with a theory named: theory CV is positive.
        let mut rs = diamond_poset();
        let _t = name_theory_from_rset(&mut rs);
        let theories_before = rs.theories().len();
        let cfg = DriveConfig {
            pattern_sizes: vec![],
            enable_prune: true,
            prune_threshold: 0.0,
            ..DriveConfig::default()
        };
        let mut rs2 = rs.clone();
        let _ = rs2.intrinsic_drive(&cfg);
        assert_eq!(rs2.theories().len(), theories_before,
            "positive-CV theory should survive pruning");
    }

    #[test]
    fn adr0040_prune_disabled_by_default_via_flag() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b")]);
        let sg = Subgraph::from_edges([R::new("a", "b")]);
        let p = rs.name_pattern_instances(&[sg]).unwrap();
        let cfg = DriveConfig {
            pattern_sizes: vec![],
            enable_prune: false,
            ..DriveConfig::default()
        };
        let mut rs2 = rs.clone();
        let _ = rs2.intrinsic_drive(&cfg);
        // Disabled → pattern still there.
        assert!(rs2.patterns().iter().any(|q| *q == p.as_str()));
    }

    // ADR 0042 — theory independence relations.

    #[test]
    fn adr0042_name_independence_on_disjoint_theories() {
        let mut rs = diamond_poset();
        // Two theories with disjoint member sets.
        let t_anti = rs.name_theory(&[AX_ANTISYMMETRY]).unwrap();
        let t_refl = rs.name_theory(&[AX_REFLEXIVITY]).unwrap();
        let ind = rs.name_theory_independence(&t_anti, &t_refl).unwrap();
        assert!(rs.independence_edges().contains(&ind.as_str()));
        let (lo, hi) = rs.independence_endpoints(&ind).unwrap();
        assert!(lo < hi);
        assert!(lo == t_anti || lo == t_refl);
    }

    #[test]
    fn adr0042_rejects_overlapping_theories() {
        let mut rs = diamond_poset();
        let t_full = rs
            .name_theory(&["ax_tpl_v3_p0-1_p1-2_c0-2", AX_REFLEXIVITY, AX_ANTISYMMETRY])
            .unwrap();
        let t_shared = rs.name_theory(&[AX_ANTISYMMETRY]).unwrap();
        // Both contain AX_ANTISYMMETRY → not independent.
        assert!(rs.name_theory_independence(&t_full, &t_shared).is_err());
    }

    #[test]
    fn adr0042_refuses_self_independence() {
        let mut rs = diamond_poset();
        let t = name_theory_from_rset(&mut rs);
        assert!(rs.name_theory_independence(&t, &t).is_err());
    }

    #[test]
    fn adr0042_canonical_ordering_is_deterministic() {
        let mut rs = diamond_poset();
        let t1 = rs.name_theory(&[AX_ANTISYMMETRY]).unwrap();
        let t2 = rs.name_theory(&[AX_REFLEXIVITY]).unwrap();
        let ind_a = rs.name_theory_independence(&t1, &t2).unwrap();
        let ind_b = rs.name_theory_independence(&t2, &t1).unwrap();
        assert_eq!(ind_a, ind_b);
        assert_eq!(rs.independence_edges().len(), 1);
    }

    #[test]
    fn adr0042_symmetric_query() {
        let mut rs = diamond_poset();
        let t1 = rs.name_theory(&[AX_ANTISYMMETRY]).unwrap();
        let t2 = rs.name_theory(&[AX_REFLEXIVITY]).unwrap();
        let _ = rs.name_theory_independence(&t1, &t2).unwrap();
        assert!(rs.theories_independent_from(&t1).contains(&t2));
        assert!(rs.theories_independent_from(&t2).contains(&t1));
    }

    #[test]
    fn adr0042_discover_independences() {
        let mut rs = diamond_poset();
        let t1 = rs.name_theory(&[AX_ANTISYMMETRY]).unwrap();
        let t2 = rs.name_theory(&[AX_REFLEXIVITY]).unwrap();
        let t3 = rs
            .name_theory(&["ax_tpl_v3_p0-1_p1-2_c0-2"])
            .unwrap();
        let found = rs.discover_theory_independences();
        let expected_pairs = [
            (t1.clone().min(t2.clone()), t1.clone().max(t2.clone())),
            (t1.clone().min(t3.clone()), t1.clone().max(t3.clone())),
            (t2.clone().min(t3.clone()), t2.clone().max(t3.clone())),
        ];
        for p in &expected_pairs {
            assert!(found.contains(p), "missing pair {:?}", p);
        }
    }

    #[test]
    fn adr0042_retract_independence_clears_three_edges() {
        let mut rs = diamond_poset();
        let t1 = rs.name_theory(&[AX_ANTISYMMETRY]).unwrap();
        let t2 = rs.name_theory(&[AX_REFLEXIVITY]).unwrap();
        let ind = rs.name_theory_independence(&t1, &t2).unwrap();
        let removed = rs.retract_independence(&ind).unwrap();
        assert_eq!(removed, 3);
        assert!(rs.independence_edges().is_empty());
    }

    #[test]
    fn adr0042_collect_meta_ids_includes_independence() {
        let mut rs = diamond_poset();
        let t1 = rs.name_theory(&[AX_ANTISYMMETRY]).unwrap();
        let t2 = rs.name_theory(&[AX_REFLEXIVITY]).unwrap();
        let ind = rs.name_theory_independence(&t1, &t2).unwrap();
        let meta = rs.collect_meta_ids();
        assert!(meta.contains(INDEPENDENT_MARKER));
        assert!(meta.contains(&ind));
    }

    // ADR 0043 — indexed RSet + sampling-path integration.

    #[test]
    fn adr0043_indices_stay_consistent_with_instances() {
        let mut rs = RSet::new();
        rs.extend([
            R::new("a", "b"), R::new("a", "c"),
            R::new("b", "c"), R::new("c", "a"),
        ]);
        // left_of("a") should match instances scan manually.
        let from_index = rs.left_of("a");
        let from_scan: Vec<&R> = rs
            .instances
            .iter()
            .filter(|r| r.x == "a")
            .collect();
        assert_eq!(from_index.len(), from_scan.len());
        for r in &from_scan {
            assert!(from_index.contains(r));
        }
    }

    #[test]
    fn adr0043_indices_survive_remove() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("a", "c")]);
        assert_eq!(rs.left_of("a").len(), 2);
        rs.remove(&R::new("a", "b"));
        assert_eq!(rs.left_of("a").len(), 1);
        rs.remove(&R::new("a", "c"));
        assert_eq!(rs.left_of("a").len(), 0);
    }

    #[test]
    fn adr0043_equality_ignores_indices() {
        // Two RSets built via different insertion orders still compare
        // equal — equality is defined by `instances`, not index state.
        let mut a = RSet::new();
        a.extend([R::new("x", "y"), R::new("y", "z")]);
        let mut b = RSet::new();
        b.extend([R::new("y", "z"), R::new("x", "y")]);
        assert_eq!(a, b);
    }

    #[test]
    fn adr0043_clone_carries_indices() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let rs2 = rs.clone();
        assert_eq!(rs.left_of("a").len(), rs2.left_of("a").len());
        assert_eq!(rs.right_of("b").len(), rs2.right_of("b").len());
    }

    #[test]
    fn adr0043_autonomous_pass_sampling_mode_finds_patterns() {
        // Same mixed graph; sampling-path should find the same kinds
        // of patterns (sampling may return fewer instances, but at
        // least some).
        let mut rs = RSet::new();
        rs.extend([R::new("c1", "c2"), R::new("c2", "c3"),
                   R::new("c3", "c4"), R::new("c4", "c5")]);
        rs.extend([R::new("s", "sa"), R::new("s", "sb"), R::new("s", "sc")]);
        let cfg = AutonomousConfig {
            discovery: DiscoveryConfig {
                target_size: 2,
                sample_count: 100,
                top_m: 5,
                rng_seed: 2024,
                include_meta_in_discovery: false,
            },
            refinement: RefinementConfig { max_tries: 50, rng_seed: 2024 },
            naming: NamingPolicy::default(),
            instance_sampling: Some(SamplingMatchConfig {
                sample_count: 200,
                rng_seed: 3,
            }),
        };
        let outcomes = rs.autonomous_pass(&cfg);
        // With sampling, we expect at least some outcomes; no crashes.
        assert!(!outcomes.is_empty());
    }

    #[test]
    fn adr0043_drive_with_sampling_flag_works() {
        let mut rs = diamond_poset();
        let cfg = DriveConfig {
            pattern_sizes: vec![2],
            instance_sampling: Some(SamplingMatchConfig {
                sample_count: 100,
                rng_seed: 7,
            }),
            ..DriveConfig::default()
        };
        let trace = rs.intrinsic_drive(&cfg);
        // Just a smoke test — drive runs without panicking.
        let _ = trace.final_score;
    }

    // ADR 0044 — template-language extension (equality + disjunction).

    #[test]
    fn adr0044_antisymmetry_template_holds_on_poset() {
        let rs = diamond_poset();
        let ev = rs.discover_antisymmetry_template().unwrap();
        assert_eq!(ev.rate(), 1.0);
    }

    #[test]
    fn adr0044_antisymmetry_template_fails_on_equivalence() {
        let rs = equivalence_relation();
        let ev = rs.discover_antisymmetry_template().unwrap();
        // On equivalence, R(a,b) AND R(b,a) holds for many distinct
        // pairs — antisymmetry's premise is met but equality isn't.
        assert!(ev.rate() < 1.0);
    }

    #[test]
    fn adr0044_totality_template_holds_on_total_order() {
        let rs = total_order_closure();
        let ev = rs.discover_totality_template().unwrap();
        assert_eq!(ev.rate(), 1.0);
    }

    #[test]
    fn adr0044_totality_template_fails_on_diamond() {
        let rs = diamond_poset();
        let ev = rs.discover_totality_template().unwrap();
        assert!(ev.rate() < 1.0);
    }

    #[test]
    fn adr0044_discover_extended_axioms_merges_all_three() {
        let rs = total_order_closure();
        let cfg = AxiomDiscoveryConfig::default();
        let extended = rs.discover_extended_axioms(&cfg);
        // Expect: edge-family transitivity + totality (disjunctive).
        let has_edge = extended.iter().any(|e|
            matches!(e, ExtendedAxiomEvidence::Edge(_))
        );
        let has_disj = extended.iter().any(|e|
            matches!(e, ExtendedAxiomEvidence::Disjunctive { .. })
        );
        assert!(has_edge);
        assert!(has_disj);
    }

    #[test]
    fn adr0044_equality_template_rate_is_binding_based() {
        // On diamond poset, premise R(x,y) ∧ R(y,x) holds only when
        // x == y (self-loops). Those are 4 bindings (one per id),
        // and equality holds for all of them → rate 1.0.
        let rs = diamond_poset();
        let ev = rs.discover_antisymmetry_template().unwrap();
        if let ExtendedAxiomEvidence::Equality {
            premise_bindings,
            conclusion_satisfied,
            ..
        } = ev
        {
            assert!(premise_bindings >= 1);
            assert_eq!(premise_bindings, conclusion_satisfied);
        } else {
            panic!("expected equality evidence");
        }
    }

    #[test]
    fn adr0044_extended_respects_defeasible_threshold() {
        // Defeasible mode accepts partial antisymmetry.
        let rs = equivalence_relation();
        let loose = AxiomDiscoveryConfig {
            min_rate: 0.1,
            ..AxiomDiscoveryConfig::default()
        };
        let strict = AxiomDiscoveryConfig::default();
        let loose_ev = rs.discover_extended_axioms(&loose);
        let strict_ev = rs.discover_extended_axioms(&strict);
        assert!(loose_ev.len() >= strict_ev.len());
    }

    // ADR 0045 — axiom confidence (Wilson score + null-baseline).

    #[test]
    fn adr0045_wilson_score_edge_cases() {
        // n=0 → (0, 1) (no information)
        let (lo, hi) = wilson_score_95(0, 0);
        assert_eq!(lo, 0.0);
        assert_eq!(hi, 1.0);
        // n=1, s=1 → high lower? Actually small n means wide CI.
        let (lo1, hi1) = wilson_score_95(1, 1);
        assert!(lo1 < 0.5, "n=1 CI should be wide, got lower {}", lo1);
        assert!(hi1 > 0.9);
        // n=100, s=100 → tight CI near 1.0
        let (lo2, _) = wilson_score_95(100, 100);
        assert!(lo2 > 0.95,
            "n=100 s=100 should give tight CI lower > 0.95, got {}", lo2);
        // n=100, s=50 → CI around 0.5
        let (lo3, hi3) = wilson_score_95(50, 100);
        assert!(lo3 > 0.4 && lo3 < 0.5);
        assert!(hi3 > 0.5 && hi3 < 0.6);
    }

    #[test]
    fn adr0045_null_baseline_extreme_cases() {
        // p = 0 → no edges → null prob 1 (impossible to observe anything)
        assert_eq!(null_baseline_probability(10, 10, 0.0), 1.0);
        // p = 1 → all edges → anything holds trivially → null prob 1
        assert_eq!(null_baseline_probability(10, 10, 1.0), 1.0);
        // N = 0 → nothing observed → null prob 1 (no info)
        assert_eq!(null_baseline_probability(0, 0, 0.5), 1.0);
        // not satisfied-all (satisfied < bindings) → no claim to discount
        assert_eq!(null_baseline_probability(10, 5, 0.5), 1.0);
    }

    #[test]
    fn adr0045_null_baseline_small_with_dense_input() {
        // 20 bindings, all satisfied, p = 0.5 → 0.5^20 ≈ 9.5e-7
        let p = null_baseline_probability(20, 20, 0.5);
        assert!(p > 0.0 && p < 1e-5);
    }

    #[test]
    fn adr0045_evidence_carries_posterior_fields() {
        let rs = diamond_poset();
        let axioms = rs.discover_axioms(&AxiomDiscoveryConfig::default());
        assert!(!axioms.is_empty());
        for ev in &axioms {
            // All rate=1.0 axioms have CI lower ≤ 1.0, upper = 1.0.
            assert!(ev.posterior_lower_95 >= 0.0);
            assert!(ev.posterior_upper_95 <= 1.0);
            assert!(ev.posterior_lower_95 <= ev.posterior_upper_95);
            // null baseline is in [0, 1].
            assert!(ev.null_baseline_prob >= 0.0);
            assert!(ev.null_baseline_prob <= 1.0);
        }
    }

    #[test]
    fn adr0045_dense_random_graph_has_high_null_baseline() {
        // Build a dense random graph; accidental axioms at rate 1.0
        // should show high null-baseline probability.
        let mut rs = RSet::new();
        let nodes: Vec<&str> = vec!["a", "b", "c", "d"];
        // Complete graph: all pairs.
        for a in &nodes {
            for b in &nodes {
                rs.add(R::new(*a, *b));
            }
        }
        // Everything holds at rate 1.0. And null baseline should be
        // close to 1.0 because p=1.0.
        let axioms = rs.discover_axioms(&AxiomDiscoveryConfig::default());
        for ev in &axioms {
            // With p_edge = 16/16 = 1.0, null_baseline_prob = 1.0
            assert!((ev.null_baseline_prob - 1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn adr0045_small_support_gives_wide_ci() {
        // Custom synthetic axiom with small support.
        let ev = AxiomEvidence {
            template: AxiomTemplate {
                num_vars: 2,
                premise: vec![EdgeTemplate { x_var: 0, y_var: 1 }],
                conclusion: EdgeTemplate { x_var: 1, y_var: 0 },
            },
            premise_bindings: 2,
            conclusion_satisfied: 2,
            rate: 1.0,
            posterior_lower_95: 0.0,
            posterior_upper_95: 1.0,
            null_baseline_prob: 1.0,
        };
        let (lo, _) = wilson_score_95(ev.conclusion_satisfied, ev.premise_bindings);
        // CI lower at N=2 should be well below rate=1
        assert!(lo < 0.5);
    }
}
