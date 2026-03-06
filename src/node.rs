//! Node representation in the relational network.
//!
//! A node represents a position in the relational structure.
//! It carries no inherent meaning - its identity is purely positional.

use std::fmt;

/// A node (position) in the relational network.
///
/// Nodes are identified by unique integer IDs. They have no inherent properties
/// other than their identity - all meaning emerges from their relations.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Node(u64);

impl Node {
    /// Creates a new node with the given ID.
    #[inline]
    pub const fn new(id: u64) -> Self {
        Node(id)
    }

    /// Returns the underlying ID of this node.
    #[inline]
    pub const fn id(self) -> u64 {
        self.0
    }
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Node({})", self.0)
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "n{}", self.0)
    }
}

impl From<u64> for Node {
    #[inline]
    fn from(id: u64) -> Self {
        Node(id)
    }
}

impl From<Node> for u64 {
    #[inline]
    fn from(node: Node) -> Self {
        node.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let node = Node::new(42);
        assert_eq!(node.id(), 42);
    }

    #[test]
    fn test_node_equality() {
        let a = Node::new(1);
        let b = Node::new(1);
        let c = Node::new(2);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_node_ordering() {
        let a = Node::new(1);
        let b = Node::new(2);
        assert!(a < b);
    }

    #[test]
    fn test_node_display() {
        let node = Node::new(5);
        assert_eq!(format!("{}", node), "n5");
        assert_eq!(format!("{:?}", node), "Node(5)");
    }

    #[test]
    fn test_node_conversion() {
        let node: Node = 10u64.into();
        let id: u64 = node.into();
        assert_eq!(id, 10);
    }
}
