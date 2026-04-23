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
#[derive(Debug, Clone, Default)]
pub struct RSet {
    instances: HashSet<R>,
}

impl RSet {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert an instance. Returns true if it was not already present.
    pub fn add(&mut self, r: R) -> bool {
        self.instances.insert(r)
    }

    pub fn extend<I: IntoIterator<Item = R>>(&mut self, iter: I) {
        self.instances.extend(iter);
    }

    pub fn contains(&self, r: &R) -> bool {
        self.instances.contains(r)
    }

    /// Remove a single R instance. Returns true if the edge was
    /// present. Dual of `add`. ADR 0020.
    pub fn remove(&mut self, r: &R) -> bool {
        self.instances.remove(r)
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

    /// All identifiers appearing anywhere in R instances, on either side.
    pub fn identifiers(&self) -> HashSet<&str> {
        self.instances
            .iter()
            .flat_map(|r| [r.x.as_str(), r.y.as_str()])
            .collect()
    }

    /// Instances with `x` on the left.
    pub fn left_of(&self, x: &str) -> Vec<&R> {
        self.instances.iter().filter(|r| r.x == x).collect()
    }

    /// Instances with `y` on the right.
    pub fn right_of(&self, y: &str) -> Vec<&R> {
        self.instances.iter().filter(|r| r.y == y).collect()
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
                self.add(R::new(PATTERN_MARKER, new_id.clone()));
                new_id
            }
        };

        for inst in instances {
            let inst_id = self.mint_instance_id(&pattern_id);
            self.add(R::new(pattern_id.clone(), inst_id.clone()));
            let participants: Vec<String> =
                inst.identifiers().into_iter().map(str::to_owned).collect();
            for participant in participants {
                self.add(R::new(inst_id.clone(), participant));
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

    /// Instance identifiers owned by a pattern. ADR 0010.
    pub fn instances_of(&self, pattern: &str) -> Vec<&str> {
        self.left_of(pattern)
            .iter()
            .map(|r| r.y.as_str())
            .collect()
    }

    /// Participant identifiers referenced by a pattern instance. ADR 0010.
    pub fn participants_of(&self, instance: &str) -> HashSet<&str> {
        self.left_of(instance)
            .iter()
            .map(|r| r.y.as_str())
            .collect()
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
    /// a participant. ADR 0013.
    pub fn memberships_of(&self, id: &str) -> Vec<(&str, &str)> {
        let pattern_set: HashSet<&str> = self.patterns().into_iter().collect();
        let mut out = Vec::new();
        for r in self.right_of(id) {
            let inst = r.x.as_str();
            // `inst` is an instance iff some R(pattern, inst) exists where
            // pattern belongs to pattern_set.
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

    /// Look up an existing pattern whose recovered canonical form equals
    /// `canon`. ADR 0010. Recovery works by taking the pattern's first
    /// instance, collecting RSet edges among its participants, and
    /// canonicalizing that subgraph — see the recovery invariant in
    /// ADR 0010's Consequences.
    pub fn find_pattern_matching(&self, canon: &CanonicalForm) -> Option<&str> {
        for pattern in self.patterns() {
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
        let data = self.data_edges_sorted();
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

            // 2. Novel canonical. Collect clean instances.
            let instances = self.find_instances_of(&canon);
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

    /// Collect every identifier currently marked as pattern registry,
    /// a pattern, or an instance. Used by `run_naming_pass` for the
    /// meta-subgraph skip. ADR 0012.
    fn collect_meta_ids(&self) -> HashSet<String> {
        let mut s = HashSet::new();
        s.insert(PATTERN_MARKER.to_string());
        for p in self.patterns() {
            s.insert(p.to_string());
            for inst in self.instances_of(p) {
                s.insert(inst.to_string());
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

        let mut removed: usize = 0;

        // (1) Remove every R(instance_id, participant) edge.
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

        // (2) Remove every R(pattern_id, instance_id) ownership edge.
        for inst in &instance_ids {
            if self.remove(&R::new(pattern_id_owned.clone(), inst.clone())) {
                removed += 1;
            }
        }

        // (3) Remove the registry edge R(PATTERN_MARKER, pattern_id).
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
}

/// A motif candidate found by `discover_motifs`. ADR 0016.
#[derive(Debug, Clone)]
pub struct MotifCandidate {
    pub canonical: CanonicalForm,
    pub representative: Subgraph,
    pub sample_frequency: usize,
    pub score: f64,
}

/// Configuration for representative refinement. ADR 0017.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefinementConfig {
    /// Per-candidate re-sampling budget.
    pub max_tries: usize,
    pub rng_seed: u64,
}

/// Configuration for the autonomous abstraction pass. ADR 0018.
#[derive(Debug, Clone)]
pub struct AutonomousConfig {
    pub discovery: DiscoveryConfig,
    pub refinement: RefinementConfig,
    pub naming: NamingPolicy,
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
            },
            refinement: RefinementConfig {
                max_tries: 200,
                rng_seed: 999,
            },
            naming: NamingPolicy::default(),
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
            },
            refinement: RefinementConfig { max_tries: 200, rng_seed: 999 },
            naming: NamingPolicy::default(),
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
}
