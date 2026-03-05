//! Iterator utilities for traversing relational structures.
//!
//! This module provides additional iterator adapters and utilities
//! for working with relations and nodes.

use crate::network::Network;
use crate::node::Node;
use crate::relation::Relation;

/// Extension trait for Network iteration patterns.
pub trait NetworkIterExt {
    /// Returns an iterator over relations as tuples of node IDs.
    fn relation_pairs(&self) -> impl Iterator<Item = (u64, u64)> + '_;

    /// Returns an iterator over node IDs.
    fn node_ids(&self) -> impl Iterator<Item = u64> + '_;
}

impl NetworkIterExt for Network {
    fn relation_pairs(&self) -> impl Iterator<Item = (u64, u64)> + '_ {
        self.relations().map(|r| (r.source().id(), r.target().id()))
    }

    fn node_ids(&self) -> impl Iterator<Item = u64> + '_ {
        self.nodes().map(|n| n.id())
    }
}

/// A path through a relational network (a sequence of connected nodes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    nodes: Vec<Node>,
}

impl Path {
    /// Creates a new empty path.
    pub fn new() -> Self {
        Path { nodes: Vec::new() }
    }

    /// Creates a path starting from a single node.
    pub fn from_node(node: Node) -> Self {
        Path { nodes: vec![node] }
    }

    /// Extends the path with a new node.
    pub fn push(&mut self, node: Node) {
        self.nodes.push(node);
    }

    /// Returns the nodes in this path.
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// Returns the length of the path (number of nodes).
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns true if the path is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns an iterator over the relations implied by this path.
    pub fn relations(&self) -> impl Iterator<Item = Relation> + '_ {
        self.nodes.windows(2).map(|w| Relation::new(w[0], w[1]))
    }

    /// Returns the starting node, if any.
    pub fn start(&self) -> Option<Node> {
        self.nodes.first().copied()
    }

    /// Returns the ending node, if any.
    pub fn end(&self) -> Option<Node> {
        self.nodes.last().copied()
    }
}

impl Default for Path {
    fn default() -> Self {
        Self::new()
    }
}

impl FromIterator<Node> for Path {
    fn from_iter<T: IntoIterator<Item = Node>>(iter: T) -> Self {
        Path {
            nodes: iter.into_iter().collect(),
        }
    }
}

impl IntoIterator for Path {
    type Item = Node;
    type IntoIter = std::vec::IntoIter<Node>;

    fn into_iter(self) -> Self::IntoIter {
        self.nodes.into_iter()
    }
}

impl<'a> IntoIterator for &'a Path {
    type Item = &'a Node;
    type IntoIter = std::slice::Iter<'a, Node>;

    fn into_iter(self) -> Self::IntoIter {
        self.nodes.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_creation() {
        let node = Node::new(1);
        let path = Path::from_node(node);
        assert_eq!(path.len(), 1);
        assert_eq!(path.start(), Some(node));
        assert_eq!(path.end(), Some(node));
    }

    #[test]
    fn test_path_push() {
        let mut path = Path::new();
        path.push(Node::new(1));
        path.push(Node::new(2));
        path.push(Node::new(3));

        assert_eq!(path.len(), 3);
        assert_eq!(path.start(), Some(Node::new(1)));
        assert_eq!(path.end(), Some(Node::new(3)));
    }

    #[test]
    fn test_path_relations() {
        let path: Path = [Node::new(1), Node::new(2), Node::new(3)]
            .into_iter()
            .collect();

        let rels: Vec<_> = path.relations().collect();
        assert_eq!(rels.len(), 2);
        assert_eq!(rels[0], Relation::new(Node::new(1), Node::new(2)));
        assert_eq!(rels[1], Relation::new(Node::new(2), Node::new(3)));
    }

    #[test]
    fn test_network_iter_ext() {
        let mut net = Network::new();
        let a = net.create_node();
        let b = net.create_node();
        net.relate(a, b);

        let ids: Vec<_> = net.node_ids().collect();
        assert!(ids.contains(&0));
        assert!(ids.contains(&1));

        let pairs: Vec<_> = net.relation_pairs().collect();
        assert_eq!(pairs.len(), 1);
        assert!(pairs.contains(&(0, 1)));
    }
}
