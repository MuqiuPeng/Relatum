//! The relational network - storage and query of relations.
//!
//! A network is a collection of nodes connected by relations.
//! It provides efficient indexing for both forward and backward traversal.

use std::collections::{HashMap, HashSet};

use crate::node::Node;
use crate::relation::Relation;

/// A network of nodes connected by relations.
///
/// The network maintains bidirectional indexes for efficient querying:
/// - Forward index: given a source node, find all target nodes
/// - Backward index: given a target node, find all source nodes
#[derive(Debug, Clone)]
pub struct Network {
    /// Forward index: from -> {to1, to2, ...}
    outgoing: HashMap<Node, HashSet<Node>>,
    /// Backward index: to -> {from1, from2, ...}
    incoming: HashMap<Node, HashSet<Node>>,
    /// Set of all nodes in the network
    nodes: HashSet<Node>,
    /// Counter for generating new node IDs
    next_id: u64,
    /// Total number of relations
    relation_count: usize,
}

impl Network {
    /// Creates a new empty network.
    pub fn new() -> Self {
        Network {
            outgoing: HashMap::new(),
            incoming: HashMap::new(),
            nodes: HashSet::new(),
            next_id: 0,
            relation_count: 0,
        }
    }

    /// Creates a new node and adds it to the network.
    ///
    /// Returns the newly created node.
    pub fn create_node(&mut self) -> Node {
        let node = Node::new(self.next_id);
        self.next_id += 1;
        self.nodes.insert(node);
        node
    }

    /// Adds an existing node to the network.
    ///
    /// Returns true if the node was newly added, false if it already existed.
    pub fn add_node(&mut self, node: Node) -> bool {
        if node.id() >= self.next_id {
            self.next_id = node.id() + 1;
        }
        self.nodes.insert(node)
    }

    /// Creates a relation between two nodes.
    ///
    /// Both nodes will be added to the network if they don't already exist.
    /// Returns the created relation.
    pub fn relate(&mut self, from: Node, to: Node) -> Relation {
        // Ensure both nodes exist
        self.add_node(from);
        self.add_node(to);

        // Add to forward index
        let targets = self.outgoing.entry(from).or_default();
        let is_new = targets.insert(to);

        // Add to backward index
        self.incoming.entry(to).or_default().insert(from);

        // Update relation count only if this is a new relation
        if is_new {
            self.relation_count += 1;
        }

        Relation::new(from, to)
    }

    /// Removes a relation between two nodes.
    ///
    /// Returns true if the relation existed and was removed.
    /// Cleans up empty entry sets so the maps do not accumulate empty buckets.
    pub fn unrelate(&mut self, from: Node, to: Node) -> bool {
        let removed = self
            .outgoing
            .get_mut(&from)
            .map(|targets| targets.remove(&to))
            .unwrap_or(false);

        if removed {
            if let Some(sources) = self.incoming.get_mut(&to) {
                sources.remove(&from);
            }
            self.relation_count -= 1;
            if self.outgoing.get(&from).map(|s| s.is_empty()).unwrap_or(false) {
                self.outgoing.remove(&from);
            }
            if self.incoming.get(&to).map(|s| s.is_empty()).unwrap_or(false) {
                self.incoming.remove(&to);
            }
        }

        removed
    }

    /// Checks if a node exists in the network.
    pub fn contains_node(&self, node: Node) -> bool {
        self.nodes.contains(&node)
    }

    /// Checks if a relation exists between two nodes.
    pub fn contains_relation(&self, from: Node, to: Node) -> bool {
        self.outgoing
            .get(&from)
            .map(|targets| targets.contains(&to))
            .unwrap_or(false)
    }

    /// Returns an iterator over all nodes connected from the given node.
    ///
    /// These are the targets of relations where `node` is the source.
    pub fn outgoing(&self, node: Node) -> impl Iterator<Item = Node> + '_ {
        self.outgoing
            .get(&node)
            .into_iter()
            .flat_map(|set| set.iter().copied())
    }

    /// Returns an iterator over all nodes that connect to the given node.
    ///
    /// These are the sources of relations where `node` is the target.
    pub fn incoming(&self, node: Node) -> impl Iterator<Item = Node> + '_ {
        self.incoming
            .get(&node)
            .into_iter()
            .flat_map(|set| set.iter().copied())
    }

    /// Returns an iterator over all nodes in the network.
    pub fn nodes(&self) -> impl Iterator<Item = Node> + '_ {
        self.nodes.iter().copied()
    }

    /// Returns an iterator over all relations in the network.
    pub fn relations(&self) -> impl Iterator<Item = Relation> + '_ {
        self.outgoing
            .iter()
            .flat_map(|(&from, targets)| targets.iter().map(move |&to| Relation::new(from, to)))
    }

    /// Returns the number of nodes in the network.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns the number of relations in the network.
    pub fn relation_count(&self) -> usize {
        self.relation_count
    }

    /// Returns the number of outgoing relations from a node.
    pub fn out_degree(&self, node: Node) -> usize {
        self.outgoing.get(&node).map(|s| s.len()).unwrap_or(0)
    }

    /// Returns the number of incoming relations to a node.
    pub fn in_degree(&self, node: Node) -> usize {
        self.incoming.get(&node).map(|s| s.len()).unwrap_or(0)
    }

    /// Returns true if the network is empty (no nodes or relations).
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Clears all nodes and relations from the network.
    ///
    /// Does not reset the next node id; nodes created after `clear()` will
    /// continue from the previous counter, preserving global id uniqueness.
    pub fn clear(&mut self) {
        self.outgoing.clear();
        self.incoming.clear();
        self.nodes.clear();
        self.relation_count = 0;
    }
}

impl Default for Network {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_node() {
        let mut net = Network::new();
        let n1 = net.create_node();
        let n2 = net.create_node();
        assert_eq!(n1.id(), 0);
        assert_eq!(n2.id(), 1);
        assert_eq!(net.node_count(), 2);
    }

    #[test]
    fn test_relate() {
        let mut net = Network::new();
        let a = net.create_node();
        let b = net.create_node();

        let rel = net.relate(a, b);
        assert_eq!(rel.source(), a);
        assert_eq!(rel.target(), b);
        assert!(net.contains_relation(a, b));
        assert!(!net.contains_relation(b, a));
        assert_eq!(net.relation_count(), 1);
    }

    #[test]
    fn test_unrelate() {
        let mut net = Network::new();
        let a = net.create_node();
        let b = net.create_node();

        net.relate(a, b);
        assert!(net.contains_relation(a, b));

        assert!(net.unrelate(a, b));
        assert!(!net.contains_relation(a, b));
        assert_eq!(net.relation_count(), 0);

        // Removing non-existent relation
        assert!(!net.unrelate(a, b));
    }

    #[test]
    fn test_outgoing_incoming() {
        let mut net = Network::new();
        let a = net.create_node();
        let b = net.create_node();
        let c = net.create_node();

        net.relate(a, b);
        net.relate(a, c);
        net.relate(b, c);

        let out_a: Vec<_> = net.outgoing(a).collect();
        assert_eq!(out_a.len(), 2);
        assert!(out_a.contains(&b));
        assert!(out_a.contains(&c));

        let in_c: Vec<_> = net.incoming(c).collect();
        assert_eq!(in_c.len(), 2);
        assert!(in_c.contains(&a));
        assert!(in_c.contains(&b));
    }

    #[test]
    fn test_degree() {
        let mut net = Network::new();
        let a = net.create_node();
        let b = net.create_node();
        let c = net.create_node();

        net.relate(a, b);
        net.relate(a, c);
        net.relate(b, c);

        assert_eq!(net.out_degree(a), 2);
        assert_eq!(net.in_degree(a), 0);
        assert_eq!(net.out_degree(c), 0);
        assert_eq!(net.in_degree(c), 2);
    }

    #[test]
    fn test_duplicate_relation() {
        let mut net = Network::new();
        let a = net.create_node();
        let b = net.create_node();

        net.relate(a, b);
        net.relate(a, b); // duplicate

        assert_eq!(net.relation_count(), 1);
    }

    #[test]
    fn test_self_relation() {
        let mut net = Network::new();
        let a = net.create_node();

        net.relate(a, a);
        assert!(net.contains_relation(a, a));
        assert_eq!(net.out_degree(a), 1);
        assert_eq!(net.in_degree(a), 1);
    }

    #[test]
    fn test_clear() {
        let mut net = Network::new();
        let a = net.create_node();
        let b = net.create_node();
        net.relate(a, b);

        net.clear();
        assert!(net.is_empty());
        assert_eq!(net.node_count(), 0);
        assert_eq!(net.relation_count(), 0);

        // New nodes should continue from where we left off
        let c = net.create_node();
        assert_eq!(c.id(), 2);
    }

    #[test]
    fn test_iterate_relations() {
        let mut net = Network::new();
        let a = net.create_node();
        let b = net.create_node();
        let c = net.create_node();

        net.relate(a, b);
        net.relate(b, c);

        let rels: Vec<_> = net.relations().collect();
        assert_eq!(rels.len(), 2);
    }
}
