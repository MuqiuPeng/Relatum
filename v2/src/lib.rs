//! Relatum v2
//!
//! Core primitive: `R(x, y)` — a binary directed relation with no pre-assigned meaning.
//! All structure (objects, types, meaning) emerges from abstraction over R instances.
//!
//! Ontological commitments: see `docs/constitution.md`.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

pub mod runtime;

mod axiom_ids;
mod markers;
mod stats;
mod types_axiom_drive;
mod types_runtime;

pub use axiom_ids::{
    axiom_id_to_template, axiom_template_id, disjunctive_axiom_id,
    disjunctive_id_to_template, equality_axiom_id, equality_id_to_template,
};
pub use markers::*;
pub use stats::{null_baseline_probability, wilson_score_95};
pub use types_axiom_drive::*;
pub use types_runtime::*;

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
    // ADR 0066 Addendum 5+ — monotonic version counter incremented on
    // every successful add/remove. Used by external caches (e.g.,
    // forward_apply_axiom result caches in the runtime layer) to invalidate
    // when the rset has changed. Not part of identity (PartialEq compares
    // instances only).
    version: u64,
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
            self.version = self.version.wrapping_add(1);
        }
        is_new
    }

    /// Monotonic version counter incremented on every successful
    /// add/remove. External caches (e.g., forward_apply_axiom
    /// caches) read this to detect rset changes. ADR 0066
    /// Addendum 5+ perf path.
    pub fn version(&self) -> u64 {
        self.version
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
            self.version = self.version.wrapping_add(1);
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
        let data = if config.include_meta_in_discovery {
            self.all_edges_sorted()
        } else {
            self.data_edges_sorted()
        };
        self.discover_motifs_from_edges(config, &data)
    }

    /// Build the canonical meta-meta subset from a list of M1
    /// markers: each marker plus the named subjects it currently
    /// anchors (the `r.x` of every `R(x, marker)`). The same
    /// construction the runtime uses for `DiscoverMetaMetaPatterns`,
    /// exposed here so callers and tests don't have to reimplement
    /// it. ADR 0054 / Phase D0.
    pub fn meta_meta_subset(
        &self,
        markers: &[&str],
    ) -> HashSet<String> {
        let mut subset: HashSet<String> = HashSet::new();
        for m in markers {
            subset.insert((*m).to_string());
            for r in self.right_of(m) {
                subset.insert(r.x.clone());
            }
        }
        subset
    }

    /// Run `discover_motifs` against a filtered edge view: every data
    /// edge plus every meta edge with at least one endpoint in `subset`.
    /// ADR 0054 / Phase D0. Pure data edges (both endpoints
    /// non-meta) are always included; meta edges are included iff one
    /// of their endpoints sits in `subset`. Other meta edges are
    /// dropped so the meta-meta hypothesis space stays scoped to the
    /// markers and named objects the caller cares about.
    pub fn discover_motifs_with_meta_subset(
        &self,
        config: &DiscoveryConfig,
        subset: &HashSet<String>,
    ) -> Vec<MotifCandidate> {
        let data = self.edges_with_meta_subset_sorted(subset);
        self.discover_motifs_from_edges(config, &data)
    }

    fn discover_motifs_from_edges(
        &self,
        config: &DiscoveryConfig,
        data: &[R],
    ) -> Vec<MotifCandidate> {
        if config.target_size == 0
            || config.sample_count == 0
            || data.is_empty()
        {
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
                sample_connected_subgraph(data, config.target_size, &mut rng_state)
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

    /// Edges visible to the meta-subset filter.
    /// See `discover_motifs_with_meta_subset`. ADR 0054.
    pub(crate) fn edges_with_meta_subset_sorted(
        &self,
        subset: &HashSet<String>,
    ) -> Vec<R> {
        let meta = self.collect_meta_ids();
        let mut out: Vec<R> = self
            .instances
            .iter()
            .filter(|r| {
                let x_meta = meta.contains(&r.x);
                let y_meta = meta.contains(&r.y);
                if !x_meta && !y_meta {
                    true
                } else {
                    subset.contains(&r.x) || subset.contains(&r.y)
                }
            })
            .cloned()
            .collect();
        out.sort_by(|a, b| {
            (a.x.as_str(), a.y.as_str()).cmp(&(b.x.as_str(), b.y.as_str()))
        });
        out
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

    /// Like `find_instances_of`, but the universe of edges is the
    /// meta-subset view: data edges plus meta edges anchored to
    /// `subset`. Used by Phase D0+ loop closure to find clean
    /// instances of a meta-meta-pattern's canonical form.
    /// ADR 0054 / Phase D0+.
    pub fn find_instances_of_with_meta_subset(
        &self,
        target: &CanonicalForm,
        subset: &HashSet<String>,
    ) -> Vec<Subgraph> {
        let k = target.len();
        if k == 0 {
            return Vec::new();
        }
        let data = self.edges_with_meta_subset_sorted(subset);
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

        results.retain(|sg| {
            self.is_clean_subgraph_with_meta_subset(sg, subset)
        });

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

    /// Cleanness check under the meta-subset edge view. The induced
    /// edge set is computed against the same filter semantics as
    /// `discover_motifs_with_meta_subset`: data edges always count;
    /// meta edges count only when at least one endpoint is in
    /// `subset`. Used by `find_instances_of_with_meta_subset` so a
    /// meta-meta-pattern's instances are validated against the same
    /// hypothesis space the discovery used. ADR 0054 / Phase D0+.
    pub fn is_clean_subgraph_with_meta_subset(
        &self,
        sg: &Subgraph,
        subset: &HashSet<String>,
    ) -> bool {
        let meta = self.collect_meta_ids();
        let parts: HashSet<&str> = sg.identifiers();
        let induced: usize = self
            .instances
            .iter()
            .filter(|r| {
                let x_meta = meta.contains(&r.x);
                let y_meta = meta.contains(&r.y);
                let visible = if !x_meta && !y_meta {
                    true
                } else {
                    subset.contains(&r.x) || subset.contains(&r.y)
                };
                visible
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

    /// Produce a config auto-tuned to this RSet's scale and density.
    /// ADR 0051.
    ///
    /// Reads the number of data edges and identifiers and adjusts
    /// `pattern_sizes`, `sample_count`, and `instance_sampling` so
    /// that the drive is likely to finish in bounded time. Honors
    /// every caller-specified field except the ones it adapts.
    ///
    /// Rules (all applied to a clone of `base`; original untouched):
    /// - If data_edges > 300: enable `instance_sampling` with
    ///   sample_count proportional to edges.
    /// - If data_edges < pattern_size: drop that size from
    ///   `pattern_sizes` (can't discover k-edge patterns with < k
    ///   data edges).
    /// - `discovery_config.sample_count` scaled by edge count (more
    ///   edges → explore more candidates, capped at 1000).
    pub fn adaptive_drive_config(&self, base: DriveConfig) -> DriveConfig {
        let meta = self.collect_meta_ids();
        let data_edges = self
            .instances
            .iter()
            .filter(|r| !meta.contains(&r.x) && !meta.contains(&r.y))
            .count();
        let mut cfg = base;
        // Drop pattern sizes that can't fit.
        cfg.pattern_sizes.retain(|&k| data_edges >= k);
        // Scale discovery sample_count with edge count.
        cfg.discovery_config.sample_count = (data_edges * 2).clamp(50, 1000);
        // Enable instance_sampling when the graph is large enough
        // that exhaustive enumeration starts biting.
        if data_edges > 300 && cfg.instance_sampling.is_none() {
            cfg.instance_sampling = Some(SamplingMatchConfig {
                sample_count: (data_edges * 2).clamp(100, 2000),
                rng_seed: cfg.discovery_config.rng_seed,
            });
        }
        cfg
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
            if ev.premise_bindings >= config.min_evidence
                && ev.rate >= config.min_rate
                && ev.posterior_lower_95 >= config.min_posterior_lower
                && ev.null_baseline_prob <= config.max_null_baseline
            {
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

    // ─── ADR 0061 H1.1 — action-sequence promotion ──────────

    /// All named action-sequence pairs (length-2 only) as
    /// `(seq_id, prefix_name, suffix_name)`. Triples (length-3,
    /// ADR 0062 / Phase H1.4) are excluded — those use
    /// `action_sequence_triples()`. ADR 0061 / Phase H1.1.
    pub fn action_sequence_pairs(&self) -> Vec<(String, String, String)> {
        let mut out: Vec<(String, String, String)> = Vec::new();
        let seq_edges = self.left_of(ACTION_SEQ_MARKER);
        for seq_edge in seq_edges {
            let seq_id = seq_edge.y.to_string();
            let step_0_id = format!("{}_step_0", seq_id);
            let step_1_id = format!("{}_step_1", seq_id);
            let step_2_id = format!("{}_step_2", seq_id);
            // Skip triples — return only sequences without step_2.
            if !self.left_of(&step_2_id).is_empty() {
                continue;
            }
            let prefix = self.left_of(&step_0_id).first().map(
                |r| r.y.to_string(),
            );
            let suffix = self.left_of(&step_1_id).first().map(
                |r| r.y.to_string(),
            );
            if let (Some(p), Some(s)) = (prefix, suffix) {
                out.push((seq_id, p, s));
            }
        }
        out.sort();
        out
    }

    /// All named action-sequence triples (length-3 only) as
    /// `(seq_id, step_0, step_1, step_2)`. ADR 0062 / Phase H1.4.
    pub fn action_sequence_triples(
        &self,
    ) -> Vec<(String, String, String, String)> {
        let mut out: Vec<(String, String, String, String)> = Vec::new();
        let seq_edges = self.left_of(ACTION_SEQ_MARKER);
        for seq_edge in seq_edges {
            let seq_id = seq_edge.y.to_string();
            let step_0_id = format!("{}_step_0", seq_id);
            let step_1_id = format!("{}_step_1", seq_id);
            let step_2_id = format!("{}_step_2", seq_id);
            let s0 = self.left_of(&step_0_id).first().map(|r| r.y.to_string());
            let s1 = self.left_of(&step_1_id).first().map(|r| r.y.to_string());
            let s2 = self.left_of(&step_2_id).first().map(|r| r.y.to_string());
            if let (Some(a), Some(b), Some(c)) = (s0, s1, s2) {
                out.push((seq_id, a, b, c));
            }
        }
        out.sort();
        out
    }

    /// True iff a triple `(a, b, c)` action-sequence has been
    /// promoted to a named meta-R chain. ADR 0062 / Phase H1.4.
    pub fn has_action_sequence_triple(
        &self,
        a: &str,
        b: &str,
        c: &str,
    ) -> bool {
        self.action_sequence_triples()
            .iter()
            .any(|(_, x, y, z)| x == a && y == b && z == c)
    }

    /// Mint a new triple action-sequence id and write its 7-edge
    /// meta-R chain. Idempotent: returns the existing seq id if
    /// `(a, b, c)` is already named. ADR 0062 / Phase H1.4.
    pub fn name_action_sequence_triple(
        &mut self,
        a: &str,
        b: &str,
        c: &str,
    ) -> String {
        for (seq_id, x, y, z) in self.action_sequence_triples() {
            if x == a && y == b && z == c {
                return seq_id;
            }
        }
        let mut n = self.left_of(ACTION_SEQ_MARKER).len();
        let seq_id = loop {
            let candidate = format!("seq_{}", n);
            let existing: HashSet<&str> = self.identifiers();
            if !existing.contains(candidate.as_str()) {
                break candidate;
            }
            n += 1;
        };
        let step_0 = format!("{}_step_0", seq_id);
        let step_1 = format!("{}_step_1", seq_id);
        let step_2 = format!("{}_step_2", seq_id);
        self.add(R::new(ACTION_SEQ_MARKER, seq_id.clone()));
        self.add(R::new(seq_id.clone(), step_0.clone()));
        self.add(R::new(seq_id.clone(), step_1.clone()));
        self.add(R::new(seq_id.clone(), step_2.clone()));
        self.add(R::new(step_0, a.to_string()));
        self.add(R::new(step_1, b.to_string()));
        self.add(R::new(step_2, c.to_string()));
        seq_id
    }

    /// Retract a named triple action-sequence's 7-edge chain.
    /// Returns count of edges removed (0 if not named).
    /// ADR 0062 / Phase H1.4.
    pub fn retract_action_sequence_triple(
        &mut self,
        a: &str,
        b: &str,
        c: &str,
    ) -> usize {
        let seq_id = match self
            .action_sequence_triples()
            .into_iter()
            .find(|(_, x, y, z)| x == a && y == b && z == c)
        {
            Some((id, _, _, _)) => id,
            None => return 0,
        };
        let mut removed = 0usize;
        let step_0 = format!("{}_step_0", seq_id);
        let step_1 = format!("{}_step_1", seq_id);
        let step_2 = format!("{}_step_2", seq_id);
        if self.remove(&R::new(step_0.clone(), a.to_string())) {
            removed += 1;
        }
        if self.remove(&R::new(step_1.clone(), b.to_string())) {
            removed += 1;
        }
        if self.remove(&R::new(step_2.clone(), c.to_string())) {
            removed += 1;
        }
        if self.remove(&R::new(seq_id.clone(), step_0)) {
            removed += 1;
        }
        if self.remove(&R::new(seq_id.clone(), step_1)) {
            removed += 1;
        }
        if self.remove(&R::new(seq_id.clone(), step_2)) {
            removed += 1;
        }
        if self.remove(&R::new(ACTION_SEQ_MARKER, seq_id)) {
            removed += 1;
        }
        removed
    }

    /// True iff the (prefix, suffix) action-pair has already been
    /// promoted to a named meta-R sequence. ADR 0061 / Phase H1.1.
    pub fn has_action_sequence_pair(
        &self,
        prefix: &str,
        suffix: &str,
    ) -> bool {
        self.action_sequence_pairs()
            .iter()
            .any(|(_, p, s)| p == prefix && s == suffix)
    }

    /// Retract a named action-sequence pair's full meta-R chain.
    /// Returns the count of edges removed (0 if the pair wasn't
    /// named). ADR 0062 / Phase H1.3.
    ///
    /// Removes:
    /// - `R(ACTION_SEQ_MARKER, seq_N)` registry edge
    /// - `R(seq_N, seq_N_step_0)` and `R(seq_N, seq_N_step_1)`
    ///   ownership edges
    /// - `R(seq_N_step_0, prefix_name)` and
    ///   `R(seq_N_step_1, suffix_name)` step-value edges
    pub fn retract_action_sequence_pair(
        &mut self,
        prefix: &str,
        suffix: &str,
    ) -> usize {
        let seq_id = match self
            .action_sequence_pairs()
            .into_iter()
            .find(|(_, p, s)| p == prefix && s == suffix)
        {
            Some((id, _, _)) => id,
            None => return 0,
        };
        let mut removed = 0usize;
        let step_0 = format!("{}_step_0", seq_id);
        let step_1 = format!("{}_step_1", seq_id);
        if self.remove(&R::new(step_0.clone(), prefix.to_string())) {
            removed += 1;
        }
        if self.remove(&R::new(step_1.clone(), suffix.to_string())) {
            removed += 1;
        }
        if self.remove(&R::new(seq_id.clone(), step_0)) {
            removed += 1;
        }
        if self.remove(&R::new(seq_id.clone(), step_1)) {
            removed += 1;
        }
        if self.remove(&R::new(ACTION_SEQ_MARKER, seq_id)) {
            removed += 1;
        }
        removed
    }

    /// Mint a new action-sequence id and write its meta-R chain.
    /// Idempotent: returns the existing seq id if `(prefix, suffix)`
    /// is already named. ADR 0061 / Phase H1.1.
    pub fn name_action_sequence_pair(
        &mut self,
        prefix: &str,
        suffix: &str,
    ) -> String {
        // Idempotent shortcut.
        for (seq_id, p, s) in self.action_sequence_pairs() {
            if p == prefix && s == suffix {
                return seq_id;
            }
        }
        // Mint a fresh seq id.
        let mut n = self.left_of(ACTION_SEQ_MARKER).len();
        let seq_id = loop {
            let candidate = format!("seq_{}", n);
            let existing: HashSet<&str> = self.identifiers();
            if !existing.contains(candidate.as_str()) {
                break candidate;
            }
            n += 1;
        };
        let step_0 = format!("{}_step_0", seq_id);
        let step_1 = format!("{}_step_1", seq_id);
        self.add(R::new(ACTION_SEQ_MARKER, seq_id.clone()));
        self.add(R::new(seq_id.clone(), step_0.clone()));
        self.add(R::new(seq_id.clone(), step_1.clone()));
        self.add(R::new(step_0, prefix.to_string()));
        self.add(R::new(step_1, suffix.to_string()));
        seq_id
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
        // Try the three template families in order: edge, equality,
        // disjunctive. First that parses succeeds; others return None.
        // ADR 0047.
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
        if let Some(template) = axiom_id_to_template(id) {
            let ev = self.evaluate_axiom_template(&template, &ids, &meta);
            if ev.rate == 1.0 && ev.premise_bindings > 0 {
                return Ok(());
            }
            return Err(TheoryError::UnsatisfiedMember(id.to_string()));
        }
        if let Some(template) = equality_id_to_template(id) {
            let (bindings, satisfied) =
                self.evaluate_equality_template(&template, &ids);
            if bindings > 0 && satisfied == bindings {
                return Ok(());
            }
            return Err(TheoryError::UnsatisfiedMember(id.to_string()));
        }
        if let Some(template) = disjunctive_id_to_template(id) {
            let (bindings, satisfied) =
                self.evaluate_disjunctive_template(&template, &ids);
            if bindings > 0 && satisfied == bindings {
                return Ok(());
            }
            return Err(TheoryError::UnsatisfiedMember(id.to_string()));
        }
        Err(TheoryError::UnparseableAxiomId(id.to_string()))
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
        // ADR 0053 / Phase C1. Cascade the experience-with edge if
        // this theory had been promoted.
        if self.remove(&R::new(theory_id.to_string(), ESTABLISHED_MARKER)) {
            removed += 1;
        }
        // ADR 0053 / Phase C2. For each axiom this theory used to
        // include, demote the SHARED_AXIOM marker when the surviving
        // theory count drops below 2.
        for m in &member_ids {
            if self.theories_containing(m).len() < 2
                && self.remove(&R::new(m.clone(), SHARED_AXIOM_MARKER))
            {
                removed += 1;
            }
        }
        Ok(removed)
    }

    /// Remove a single membership edge `R(theory_id, axiom_id)` from a
    /// theory, leaving the theory itself and its other members intact.
    /// ADR 0066 Phase Alpha-3+++. Counterexample-guided specialization:
    /// theory survives, only the failing axiom is detached. Axiom
    /// remains globally registered; other theories that share it are
    /// unaffected. Cascades the SHARED_AXIOM_MARKER demotion when the
    /// axiom's theory count drops below 2, mirroring `retract_theory`.
    /// Returns the number of meta-R edges removed.
    pub fn retract_theory_member(
        &mut self,
        theory_id: &str,
        axiom_id: &str,
    ) -> Result<usize, TheoryError> {
        if !self.is_theory(theory_id) {
            return Err(TheoryError::UnsatisfiedMember(theory_id.to_string()));
        }
        if !self
            .instances
            .contains(&R::new(theory_id.to_string(), axiom_id.to_string()))
        {
            return Err(TheoryError::UnsatisfiedMember(axiom_id.to_string()));
        }
        let mut removed = 0usize;
        if self.remove(&R::new(theory_id.to_string(), axiom_id.to_string())) {
            removed += 1;
        }
        if self.theories_containing(axiom_id).len() < 2
            && self.remove(&R::new(axiom_id.to_string(), SHARED_AXIOM_MARKER))
        {
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

    // ─── ADR 0046: theory parallel relations ───────────────────────

    /// Name a "T_a || T_b" (parallel) relation in meta-R. ADR 0046.
    ///
    /// Two theories are parallel iff they share a nonempty member
    /// subset but neither is a subset of the other — i.e. they have
    /// common ground but diverge. Symmetric — chain stores canonical
    /// direction (`T_lo, par_N, T_hi` with `T_lo < T_hi` lex).
    pub fn name_theory_parallel(
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
                "a theory is not parallel to itself".to_string(),
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
        let intersection: HashSet<_> = a_members.intersection(&b_members).collect();
        if intersection.is_empty() {
            return Err(TheoryError::UnsatisfiedMember(format!(
                "{} and {} have no shared axioms (use independence instead)",
                a, b
            )));
        }
        if a_members.is_subset(&b_members) || b_members.is_subset(&a_members) {
            return Err(TheoryError::UnsatisfiedMember(format!(
                "{} and {} are in an extends relation, not parallel",
                a, b
            )));
        }
        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
        for existing in self.parallel_edges() {
            let (el, eh) = self.parallel_endpoints(existing).unwrap_or_default();
            if el == lo && eh == hi {
                return Ok(existing.to_string());
            }
        }
        let par_id = self.mint_parallel_id();
        self.add(R::new(PARALLEL_MARKER, par_id.clone()));
        self.add(R::new(lo.to_string(), par_id.clone()));
        self.add(R::new(par_id.clone(), hi.to_string()));
        Ok(par_id)
    }

    /// Every named parallel-edge id. ADR 0046.
    pub fn parallel_edges(&self) -> Vec<&str> {
        self.left_of(PARALLEL_MARKER)
            .iter()
            .map(|r| r.y.as_str())
            .collect()
    }

    /// Decode a parallel-edge's endpoints: `(T_lo, T_hi)` where
    /// `T_lo < T_hi` lexicographically. ADR 0046.
    pub fn parallel_endpoints(&self, par_id: &str) -> Option<(String, String)> {
        if !self.instances.contains(&R::new(PARALLEL_MARKER, par_id)) {
            return None;
        }
        let lo = self
            .right_of(par_id)
            .into_iter()
            .find_map(|r| {
                if self.is_theory(r.x.as_str()) {
                    Some(r.x.clone())
                } else {
                    None
                }
            })?;
        let hi = self
            .left_of(par_id)
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

    /// All theories parallel to `theory`. ADR 0046.
    pub fn theories_parallel_to(&self, theory: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for par in self.parallel_edges() {
            if let Some((lo, hi)) = self.parallel_endpoints(par) {
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

    /// Scan named theories for all parallel pairs. Read-only. ADR 0046.
    pub fn discover_theory_parallels(&self) -> Vec<(String, String)> {
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
                let am = &member_sets[a];
                let bm = &member_sets[b];
                let shares = !am.is_disjoint(bm);
                let neither_subset =
                    !am.is_subset(bm) && !bm.is_subset(am);
                if shares && neither_subset {
                    let (lo, hi) = if a < b { (a.clone(), b.clone()) } else { (b.clone(), a.clone()) };
                    out.push((lo, hi));
                }
            }
        }
        out.sort();
        out
    }

    /// Retract a parallel-relation edge. ADR 0046.
    pub fn retract_parallel(&mut self, par_id: &str) -> Result<usize, TheoryError> {
        if !self.instances.contains(&R::new(PARALLEL_MARKER, par_id)) {
            return Err(TheoryError::UnsatisfiedMember(par_id.to_string()));
        }
        let mut removed = 0usize;
        let to_remove: Vec<R> = self
            .instances
            .iter()
            .filter(|r| r.x == par_id || r.y == par_id)
            .cloned()
            .collect();
        for e in to_remove {
            if self.remove(&e) {
                removed += 1;
            }
        }
        if self.remove(&R::new(PARALLEL_MARKER, par_id.to_string())) {
            removed += 1;
        }
        Ok(removed)
    }

    fn mint_parallel_id(&self) -> String {
        let existing = self.identifiers();
        let mut n = self.parallel_edges().len();
        loop {
            let candidate = format!("par_{}", n);
            if !existing.contains(candidate.as_str()) {
                return candidate;
            }
            n += 1;
        }
    }

    // ─── ADR 0049: theory relation classifier + neighborhood ────────

    /// Classify the structural relation between two named theories.
    /// ADR 0049. The five cases partition every pair of distinct
    /// theories with named members:
    ///
    /// - `Equal`: same member set (but distinct ids — should be rare,
    ///   since `name_theory` reuses id on match; left in the enum for
    ///   completeness).
    /// - `Extends`: `a`'s members ⊋ `b`'s members.
    /// - `ExtendedBy`: `b`'s members ⊋ `a`'s members.
    /// - `Independent`: empty intersection.
    /// - `Parallel`: non-empty intersection, neither is subset.
    ///
    /// Returns `None` if either id is not a named theory.
    pub fn classify_theory_pair(
        &self,
        a: &str,
        b: &str,
    ) -> Option<TheoryRelationKind> {
        if !self.is_theory(a) || !self.is_theory(b) {
            return None;
        }
        if a == b {
            return Some(TheoryRelationKind::Equal);
        }
        let am: HashSet<String> = self
            .theory_axioms(a)
            .into_iter()
            .map(str::to_owned)
            .collect();
        let bm: HashSet<String> = self
            .theory_axioms(b)
            .into_iter()
            .map(str::to_owned)
            .collect();
        if am == bm {
            return Some(TheoryRelationKind::Equal);
        }
        if am.is_disjoint(&bm) {
            return Some(TheoryRelationKind::Independent);
        }
        let a_super = bm.is_subset(&am);
        let b_super = am.is_subset(&bm);
        if a_super && !b_super {
            return Some(TheoryRelationKind::Extends);
        }
        if b_super && !a_super {
            return Some(TheoryRelationKind::ExtendedBy);
        }
        Some(TheoryRelationKind::Parallel)
    }

    /// Summarize the structural neighborhood of a theory. ADR 0049.
    /// Groups every other named theory by their relation to `theory`.
    pub fn theory_neighborhood(&self, theory: &str) -> Option<TheoryNeighborhood> {
        if !self.is_theory(theory) {
            return None;
        }
        let mut out = TheoryNeighborhood::default();
        for other in self.theories() {
            if other == theory {
                continue;
            }
            match self.classify_theory_pair(theory, other) {
                Some(TheoryRelationKind::Equal) => out.equal.push(other.to_string()),
                Some(TheoryRelationKind::Extends) => out.extends.push(other.to_string()),
                Some(TheoryRelationKind::ExtendedBy) => out.extended_by.push(other.to_string()),
                Some(TheoryRelationKind::Independent) => out.independent.push(other.to_string()),
                Some(TheoryRelationKind::Parallel) => out.parallel.push(other.to_string()),
                None => {}
            }
        }
        out.equal.sort();
        out.extends.sort();
        out.extended_by.sort();
        out.independent.sort();
        out.parallel.sort();
        Some(out)
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

    /// Forward-apply a single named axiom. Returns the raw set of
    /// `R(σ(c.x), σ(c.y))` instances produced under every variable
    /// substitution σ : 0..num_vars → data_ids that satisfies every
    /// premise edge. ADR 0058 / Phase G1.0.
    ///
    /// Returns the empty set when:
    /// - `axiom_id` is not a registered axiom;
    /// - the axiom is a predicate axiom (reflexivity / antisymmetry
    ///   / totality) — those have specialised semantics outside the
    ///   template-based forward-apply scope of G1.0;
    /// - the template carries equality or disjunctive premise
    ///   constraints (G1.1 / G1.2 deferred — `reconstruct_axiom_template`
    ///   returns `None` for those today).
    ///
    /// Substitution domain is data identifiers only (commitment 3:
    /// types are meta-R, not subject to axiomatic prediction). The
    /// caller decides whether to subtract `rset.instances` to keep
    /// only "predictions that aren't yet observed".
    pub fn forward_apply_axiom(&self, axiom_id: &str) -> HashSet<R> {
        let template = match self.reconstruct_axiom_template(axiom_id) {
            Some(t) => t,
            None => return HashSet::new(),
        };
        if template.num_vars == 0 {
            return HashSet::new();
        }
        let meta = self.collect_meta_ids();
        let data_ids = self.compute_data_ids(&meta);
        if data_ids.is_empty() {
            return HashSet::new();
        }
        self.forward_apply_axiom_with_data_ids_inner(&template, &data_ids)
    }

    /// Performance-amortized variant of `forward_apply_axiom`. The
    /// caller precomputes `data_ids` (sorted, deterministic) and
    /// passes it in. Useful when forward-applying many axioms in a
    /// single tick: caller calls `compute_data_ids` once, then
    /// invokes this method per axiom.
    ///
    /// Behavior is byte-identical to `forward_apply_axiom` for the
    /// same `axiom_id`; this just removes the per-call recomputation
    /// of `meta_ids` and `data_ids`. Performance fix motivated by
    /// Phase Alpha-4 perf diagnosis (forward_apply_axiom is the
    /// dominant per-tick cost on long substrate runs).
    pub fn forward_apply_axiom_with_data_ids(
        &self,
        axiom_id: &str,
        data_ids: &[String],
    ) -> HashSet<R> {
        let template = match self.reconstruct_axiom_template(axiom_id) {
            Some(t) => t,
            None => return HashSet::new(),
        };
        if template.num_vars == 0 || data_ids.is_empty() {
            return HashSet::new();
        }
        self.forward_apply_axiom_with_data_ids_inner(&template, data_ids)
    }

    fn forward_apply_axiom_with_data_ids_inner(
        &self,
        template: &AxiomTemplate,
        data_ids: &[String],
    ) -> HashSet<R> {
        let mut binding: Vec<usize> = vec![0; template.num_vars];
        let mut out: HashSet<R> = HashSet::new();
        forward_apply_recursive(self, template, data_ids, &mut binding, 0, &mut out);
        out
    }

    /// Compute the sorted vector of data identifiers (non-meta).
    /// Caller passes a precomputed `meta_ids` set so this can be
    /// amortized across multiple axiom forward-applies in the same
    /// tick. Result is deterministic (sorted lexicographically).
    pub fn compute_data_ids(&self, meta: &HashSet<String>) -> Vec<String> {
        let mut data_ids_set: HashSet<String> = HashSet::new();
        for r in &self.instances {
            if !meta.contains(&r.x) {
                data_ids_set.insert(r.x.clone());
            }
            if !meta.contains(&r.y) {
                data_ids_set.insert(r.y.clone());
            }
        }
        let mut data_ids: Vec<String> = data_ids_set.into_iter().collect();
        data_ids.sort();
        data_ids
    }

    /// Forward-apply every named axiom. Union of
    /// `forward_apply_axiom` over `self.axioms()`. ADR 0058.
    pub fn forward_apply_all(&self) -> HashSet<R> {
        let mut out: HashSet<R> = HashSet::new();
        for ax in self.axioms() {
            out.extend(self.forward_apply_axiom(ax));
        }
        out
    }

    /// Data edges that are explained by NEITHER any named pattern's
    /// Layer B coverage NOR any axiom's forward-application
    /// prediction. ADR 0059 / Phase G1.4.
    ///
    /// Tighter sibling of `uncovered_data_edges` (Phase G0): the
    /// G0 metric only credits structural Layer B coverage; G1.4
    /// also subtracts the axiomatic prediction set. An edge is
    /// "unexplained" iff:
    ///   1. it's a data edge (both endpoints non-meta), AND
    ///   2. it doesn't sit between two participants of any named
    ///      pattern's Layer B instance (Phase G0 condition), AND
    ///   3. it isn't predicted by any named axiom's forward-apply
    ///      output (Phase G1.4 addition).
    ///
    /// Once axioms exist that predict the rset's content,
    /// `unexplained_data_edges()` shrinks even when no patterns
    /// have been Layer-B-named — the runtime now considers
    /// axiomatic prediction a valid explanation.
    pub fn unexplained_data_edges(&self) -> HashSet<R> {
        let mut out = self.uncovered_data_edges();
        let predicted = self.forward_apply_all();
        for edge in &predicted {
            out.remove(edge);
        }
        out
    }

    /// Data edges not in any named pattern's Layer B instance binding.
    /// ADR 0057 / Phase G0.
    ///
    /// "Covered" by a pattern means: the edge's two endpoints both
    /// appear as participants of some `R(pattern, instance_N)` chain
    /// (i.e., both endpoints are right-hand sides of `R(instance_N,
    /// participant)` edges for the same `instance_N`). Data edges
    /// failing this for every named pattern are *uncovered* — the
    /// runtime has not yet built any abstraction that explains them
    /// in pattern terms. The signal feeds the Phase G0 anomaly
    /// scheduler hooks. Patterns named with the Intensional policy
    /// have no Layer B and therefore cover nothing by this measure;
    /// that's the intended behaviour (Intensional patterns abstract
    /// shape, not data).
    pub fn uncovered_data_edges(&self) -> HashSet<R> {
        let meta = self.collect_meta_ids();
        let mut covered_groups: Vec<HashSet<String>> = Vec::new();
        for p in self.patterns() {
            for inst_id in self.instances_of(p) {
                let participants: HashSet<String> = self
                    .left_of(inst_id)
                    .iter()
                    .map(|r| r.y.clone())
                    .collect();
                if participants.len() >= 2 {
                    covered_groups.push(participants);
                }
            }
        }
        self.instances
            .iter()
            .filter(|r| !meta.contains(&r.x) && !meta.contains(&r.y))
            .filter(|r| {
                !covered_groups
                    .iter()
                    .any(|g| g.contains(&r.x) && g.contains(&r.y))
            })
            .cloned()
            .collect()
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
    pub(crate) fn collect_meta_ids(&self) -> HashSet<String> {
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
        s.insert(PARALLEL_MARKER.to_string());
        for par in self.parallel_edges() {
            s.insert(par.to_string());
        }
        s.insert(ESTABLISHED_MARKER.to_string());
        s.insert(SHARED_AXIOM_MARKER.to_string());
        s.insert(ACTION_SEQ_MARKER.to_string());
        // Per-sequence step ids and registry edges sit under
        // ACTION_SEQ_MARKER's left-of set.
        for seq in self.left_of(ACTION_SEQ_MARKER) {
            s.insert(seq.y.to_string());
            for step_edge in self.left_of(seq.y.as_str()) {
                s.insert(step_edge.y.to_string());
            }
        }
        // ADR 0064 / Phase H2.1.0 — DRIVE_MARKER and
        // PENALTY_MARKER are meta-R class markers; the
        // `drive_<id>` tokens registered under them are also
        // meta-R, not data.
        s.insert(DRIVE_MARKER.to_string());
        s.insert(PENALTY_MARKER.to_string());
        for drive in self.left_of(DRIVE_MARKER) {
            s.insert(drive.y.to_string());
        }
        for drive in self.left_of(PENALTY_MARKER) {
            s.insert(drive.y.to_string());
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

        // (7) Remove the experience-with edge R(pattern_id, ESTABLISHED_MARKER)
        //     if it was promoted via Phase C0. ADR 0053.
        if self.remove(&R::new(pattern_id_owned.clone(), ESTABLISHED_MARKER)) {
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
        // The most recent signature for each node — used by ADR 0055 to
        // project the converged WL-1 result into a direction-preserving
        // canonical label.
        let mut last_sigs: Vec<(u32, Vec<u32>, Vec<u32>)> = (0..n)
            .map(|i| (labels[i], Vec::new(), Vec::new()))
            .collect();

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
            last_sigs = sigs.clone();
            let next = rank_labels(&sigs);
            if next == labels {
                break;
            }
            labels = next;
        }

        // ADR 0055: project converged signatures to global hashes
        // rather than local ranks. This preserves direction-sensitive
        // content the WL-1 signature already carries — fan-in and
        // fan-out have different signatures, so they get different
        // hashes, so their canonical edge lists differ.
        let hashes: Vec<u64> =
            last_sigs.iter().map(signature_hash).collect();

        let mut canonical: Vec<(u64, u64)> = self
            .edges
            .iter()
            .map(|r| (
                hashes[id_to_index[r.x.as_str()]],
                hashes[id_to_index[r.y.as_str()]],
            ))
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
pub(crate) fn canonicalize_template(tpl: AxiomTemplate) -> AxiomTemplate {
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

/// Forward-apply enumerator. Mirror of `evaluate_template_recursive`
/// but inserts a predicted `R(σ(c.x), σ(c.y))` into `out` for every
/// σ that satisfies every premise edge. ADR 0058.
fn forward_apply_recursive(
    rs: &RSet,
    template: &AxiomTemplate,
    ids: &[String],
    binding: &mut [usize],
    depth: usize,
    out: &mut HashSet<R>,
) {
    if depth == binding.len() {
        // All variables bound; verify any premises not already
        // verified by the early-termination logic below. (In
        // practice all premises will have been verified by the
        // time we get here, but the leaf check is kept as a
        // safety net for premises whose variables are bound
        // simultaneously at depth = binding.len().)
        for e in &template.premise {
            let x = &ids[binding[e.x_var]];
            let y = &ids[binding[e.y_var]];
            if !rs.instances.contains(&R::new(x.clone(), y.clone())) {
                return;
            }
        }
        // Emit conclusion under σ.
        let cx = &ids[binding[template.conclusion.x_var]];
        let cy = &ids[binding[template.conclusion.y_var]];
        out.insert(R::new(cx.clone(), cy.clone()));
        return;
    }
    // ADR 0066 Addendum 7 — Option D: early premise termination.
    // After binding[depth] is set, check any premise whose
    // variables are all already bound (i.e., max var index <=
    // depth). If unsatisfied, prune this branch immediately
    // instead of letting the recursion explore all sub-branches
    // before discovering at the leaf that the premise was
    // violated.
    //
    // Prune impact: for axioms like transitivity (R(x,y) ∧ R(y,z)
    // ⇒ R(x,z)), the first premise R(x,y) is fully bound by
    // depth=1 (when y is bound). Branches where R(x,y) doesn't
    // hold get cut here, skipping the entire z-recursion. Total
    // operations: N * |children(x)| * |children(y)| instead of
    // N^3 — substantial saving when graph is sparse.
    for i in 0..ids.len() {
        binding[depth] = i;
        // Early-termination check: are any premises fully bound now?
        let mut prune = false;
        for e in &template.premise {
            if e.x_var <= depth && e.y_var <= depth {
                let x = &ids[binding[e.x_var]];
                let y = &ids[binding[e.y_var]];
                if !rs.instances.contains(&R::new(x.clone(), y.clone())) {
                    prune = true;
                    break;
                }
            }
        }
        if prune {
            continue;
        }
        forward_apply_recursive(rs, template, ids, binding, depth + 1, out);
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
/// Deterministic FNV-1a hash of a converged WL-1 node signature.
/// Used by `Subgraph::canonicalize` to project signatures into the
/// canonical label space without losing direction-sensitive content
/// (ADR 0055). Hand-rolled FNV-1a so the result is stable across
/// Rust versions and platforms — `std`'s default hasher seeds may
/// vary in future, and we want canonical labels to be a function of
/// the signature alone.
fn signature_hash(sig: &(u32, Vec<u32>, Vec<u32>)) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut h: u64 = FNV_OFFSET;
    let mut update = |byte: u8, h: &mut u64| {
        *h ^= byte as u64;
        *h = h.wrapping_mul(FNV_PRIME);
    };
    let mut hash_u32 = |v: u32, h: &mut u64| {
        for b in v.to_le_bytes() {
            update(b, h);
        }
    };
    hash_u32(sig.0, &mut h);
    update(0xfe, &mut h); // separator between fields
    for v in &sig.1 {
        hash_u32(*v, &mut h);
        update(0xfd, &mut h); // separator between elements
    }
    update(0xfe, &mut h);
    for v in &sig.2 {
        hash_u32(*v, &mut h);
        update(0xfd, &mut h);
    }
    h
}

fn rank_labels<T: Ord + Clone>(sigs: &[T]) -> Vec<u32> {
    let mut sorted_unique: Vec<T> = sigs.to_vec();
    sorted_unique.sort();
    sorted_unique.dedup();
    sigs.iter()
        .map(|s| sorted_unique.binary_search(s).unwrap() as u32)
        .collect()
}

/// Canonical form of a subgraph: sorted edge list over stable labels.
/// See `Subgraph::canonicalize`. ADR 0009 (form), ADR 0055 (label width).
///
/// Each label is a `u64` derived from a global FNV-1a hash of the
/// node's converged WL-1 signature. The widening from `u32` (rank
/// indices, ADR 0009) to `u64` (signature hashes, ADR 0055) is what
/// distinguishes fan-in from fan-out at small sizes; see
/// `Subgraph::canonicalize` for the projection.
pub type CanonicalForm = Vec<(u64, u64)>;


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
mod tests;
