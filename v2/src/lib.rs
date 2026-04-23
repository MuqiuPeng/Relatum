//! Relatum v2
//!
//! Core primitive: `R(x, y)` — a binary directed relation with no pre-assigned meaning.
//! All structure (objects, types, meaning) emerges from abstraction over R instances.
//!
//! Ontological commitments: see `docs/constitution.md`.

use std::collections::{HashMap, HashSet, VecDeque};

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
