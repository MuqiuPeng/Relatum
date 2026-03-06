//! Relation representation - the fundamental building block.
//!
//! A relation connects two nodes, representing the most basic structural element
//! in the system. Relations are directed: they go from one node to another.

use std::fmt;

use crate::node::Node;

/// A directed binary relation between two nodes.
///
/// This is the fundamental structural element of the system.
/// A relation connects a source node to a target node; use [`source`](Relation::source)
/// and [`target`](Relation::target) for access.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Relation {
    from: Node,
    to: Node,
}

impl Relation {
    /// Creates a new relation from one node to another.
    #[inline]
    pub const fn new(from: Node, to: Node) -> Self {
        Relation { from, to }
    }

    /// Returns the source node of this relation.
    #[inline]
    pub const fn source(&self) -> Node {
        self.from
    }

    /// Returns the target node of this relation.
    #[inline]
    pub const fn target(&self) -> Node {
        self.to
    }

    /// Returns the reverse of this relation (swapping from and to).
    #[inline]
    pub const fn reverse(&self) -> Self {
        Relation {
            from: self.to,
            to: self.from,
        }
    }

    /// Returns true if this is a self-relation (from == to).
    #[inline]
    pub fn is_reflexive(&self) -> bool {
        self.from == self.to
    }
}

impl fmt::Debug for Relation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Relation({:?} -> {:?})", self.from, self.to)
    }
}

impl fmt::Display for Relation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} -> {}", self.from, self.to)
    }
}

impl From<(Node, Node)> for Relation {
    #[inline]
    fn from((from, to): (Node, Node)) -> Self {
        Relation::new(from, to)
    }
}

impl From<(u64, u64)> for Relation {
    #[inline]
    fn from((from, to): (u64, u64)) -> Self {
        Relation::new(Node::new(from), Node::new(to))
    }
}

impl From<Relation> for (Node, Node) {
    #[inline]
    fn from(rel: Relation) -> Self {
        (rel.from, rel.to)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relation_creation() {
        let a = Node::new(1);
        let b = Node::new(2);
        let rel = Relation::new(a, b);
        assert_eq!(rel.source(), a);
        assert_eq!(rel.target(), b);
    }

    #[test]
    fn test_relation_reverse() {
        let rel = Relation::from((1, 2));
        let rev = rel.reverse();
        assert_eq!(rev.source().id(), 2);
        assert_eq!(rev.target().id(), 1);
    }

    #[test]
    fn test_relation_reflexive() {
        let self_rel = Relation::from((1, 1));
        let other_rel = Relation::from((1, 2));
        assert!(self_rel.is_reflexive());
        assert!(!other_rel.is_reflexive());
    }

    #[test]
    fn test_relation_equality() {
        let r1 = Relation::from((1, 2));
        let r2 = Relation::from((1, 2));
        let r3 = Relation::from((2, 1));
        assert_eq!(r1, r2);
        assert_ne!(r1, r3);
    }

    #[test]
    fn test_relation_display() {
        let rel = Relation::from((1, 2));
        assert_eq!(format!("{}", rel), "n1 -> n2");
    }
}
