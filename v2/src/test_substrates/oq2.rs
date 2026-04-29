//! OQ#2 — non-overlapping-regime substrate (different from OQ#1).
//!
//! OQ#1 uses 4 regimes: diamond posets, bipartite, equivalence
//! classes, diamonds+markers. C.2.1 (cross-precision generalization
//! to truly different substrates) needs regimes OQ#1 doesn't have.
//!
//! OQ#2's 3 regimes:
//! - Regime T (tournament-style, 1-1500): every pair of nodes has
//!   a directed edge in one direction, chosen by index. Almost-
//!   transitive but with deliberate violations to break clean
//!   total-order discovery.
//! - Regime L (lattice, 1501-3000): a 4-element diamond lattice
//!   (top, bottom, two middles) with bidirectional refl edges
//!   per element (different from OQ#1's strict poset).
//! - Regime S (star, 3001-4500): central hub node with fan-out
//!   to leaves, then leaves point back. Models bidirectional
//!   communication.
//!
//! Total: 4500 ticks across 3 regimes.

use crate::R;
use crate::runtime::Event;

pub fn build_oq2_stream() -> Vec<(u64, Event)> {
    let mut s = Vec::new();

    // Regime T (1-1500): 5 phases × 6-node tournament (almost
    // transitive but with strategic violations).
    for phase in 0..5 {
        let off = 1 + (phase as u64) * 300;
        let nodes: [String; 6] = std::array::from_fn(|i| {
            format!("t{}_{}", phase, i)
        });
        // Tournament: nodes[i] → nodes[j] if i < j ... mostly.
        let mut t = 0u64;
        for i in 0..6 {
            for j in 0..6 {
                if i == j {
                    continue;
                }
                // Skip one transitive closure edge each phase to
                // create regime variety.
                if phase % 2 == 0 && i == 0 && j == 5 {
                    continue;
                }
                if i < j || (phase % 3 == 0 && i == 5 && j == 0) {
                    s.push((
                        off + t,
                        Event::AddEdge(R::new(&nodes[i][..], &nodes[j][..])),
                    ));
                    t += 1;
                }
            }
        }
    }

    // Regime L (1501-3000): 5 phases × 4-element lattice with
    // self-loops on every element + bidirectional comparability
    // edges between top/bottom and middles.
    for phase in 0..5 {
        let off = 1501 + (phase as u64) * 300;
        let bot = format!("l{}_b", phase);
        let m1 = format!("l{}_m1", phase);
        let m2 = format!("l{}_m2", phase);
        let top = format!("l{}_t", phase);
        // Self-loops.
        for n in [&bot, &m1, &m2, &top] {
            s.push((off, Event::AddEdge(R::new(n.as_str(), n.as_str()))));
        }
        // Lattice ordering: bot ≤ m1, bot ≤ m2, m1 ≤ top, m2 ≤ top.
        s.push((off + 5, Event::AddEdge(R::new(bot.as_str(), m1.as_str()))));
        s.push((off + 6, Event::AddEdge(R::new(bot.as_str(), m2.as_str()))));
        s.push((off + 7, Event::AddEdge(R::new(m1.as_str(), top.as_str()))));
        s.push((off + 8, Event::AddEdge(R::new(m2.as_str(), top.as_str()))));
        // Transitive closure for top.
        s.push((off + 10, Event::AddEdge(R::new(bot.as_str(), top.as_str()))));
    }

    // Regime S (3001-4500): 5 phases × star with bidirectional edges.
    for phase in 0..5 {
        let off = 3001 + (phase as u64) * 300;
        let hub = format!("s{}_hub", phase);
        s.push((off, Event::AddEdge(R::new(hub.as_str(), hub.as_str()))));
        for k in 0..4 {
            let leaf = format!("s{}_l{}", phase, k);
            s.push((
                off + 1 + 2 * k as u64,
                Event::AddEdge(R::new(hub.as_str(), leaf.as_str())),
            ));
            s.push((
                off + 2 + 2 * k as u64,
                Event::AddEdge(R::new(leaf.as_str(), hub.as_str())),
            ));
        }
    }

    s
}
