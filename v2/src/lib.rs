//! Relatum v2
//!
//! Core primitive: `R(x, y)` — a binary directed relation with no pre-assigned meaning.
//! All structure (objects, types, meaning) emerges from abstraction over R instances.
//!
//! Ontological commitments: see `docs/constitution.md`.

use std::collections::HashSet;

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
