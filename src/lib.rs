//! # Relatum
//!
//! A relation-centered foundation for mathematical structures.
//!
//! ## Philosophy
//!
//! Traditional systems define objects first, then relations between them.
//! Relatum inverts this: **relations are the fundamental building blocks**,
//! and objects (nodes) are simply positions within the relational structure.
//!
//! ## Core Concepts
//!
//! - **Node**: A position in the relational network, identified by an integer ID.
//!   Nodes have no inherent properties - their meaning emerges from relations.
//!
//! - **Relation**: A directed connection between two nodes. This is the most
//!   basic structural element in the system.
//!
//! - **Network**: A collection of nodes and relations, with efficient indexing
//!   for traversal in both directions.
//!
//! ## Example
//!
//! ```rust
//! use relatum::{Network, Node, Relation};
//!
//! // Create a new relational network
//! let mut net = Network::new();
//!
//! // Create nodes (positions in the network)
//! let a = net.create_node();
//! let b = net.create_node();
//! let c = net.create_node();
//!
//! // Establish relations between nodes
//! net.relate(a, b);
//! net.relate(b, c);
//! net.relate(a, c);
//!
//! // Query the structure
//! assert!(net.contains_relation(a, b));
//! assert_eq!(net.out_degree(a), 2);
//!
//! // Traverse outgoing relations
//! for target in net.outgoing(a) {
//!     println!("a -> {}", target);
//! }
//! ```

pub mod algebra;
pub mod iter;
pub mod network;
pub mod node;
pub mod relation;

// Re-export core types at crate root
pub use network::Network;
pub use node::Node;
pub use relation::Relation;

// Re-export iterator utilities
pub use iter::{NetworkIterExt, Path};
