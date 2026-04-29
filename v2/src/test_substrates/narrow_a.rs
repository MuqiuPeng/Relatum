//! "Narrow regime A only" substrate — used for D.3.1 to expose
//! signal disagreement between primary-rate and cross-precision.
//!
//! Just OQ#1's regime A (diamond posets across 5 phases), no
//! bipartite/equivalence/marker injection. Theories discovered
//! here will have high primary-rate (axioms work perfectly on
//! diamond data) but their cross-precision profiles can differ
//! depending on which axioms they encode.

use crate::R;
use crate::runtime::Event;

/// Build a narrow stream with only diamond posets (5 phases × 100 ticks).
pub fn build_narrow_a_stream() -> Vec<(u64, Event)> {
    let mut schedule = Vec::new();
    let regime_a_phases: [&[&str]; 5] = [
        &["a1", "a2", "a3", "a4"],
        &["a5", "a6", "a7", "a8"],
        &["a9", "a10", "a11", "a12"],
        &["a13", "a14", "a15", "a16"],
        &["a17", "a18", "a19", "a20"],
    ];
    for (i, ns) in regime_a_phases.iter().enumerate() {
        let off = 1 + (i as u64) * 100;
        for k in 0..4 {
            schedule.push((off + k as u64, Event::AddEdge(R::new(ns[k], ns[k]))));
        }
        schedule.push((off + 7, Event::AddEdge(R::new(ns[0], ns[1]))));
        schedule.push((off + 11, Event::AddEdge(R::new(ns[0], ns[2]))));
        schedule.push((off + 15, Event::AddEdge(R::new(ns[0], ns[3]))));
        schedule.push((off + 19, Event::AddEdge(R::new(ns[1], ns[3]))));
        schedule.push((off + 23, Event::AddEdge(R::new(ns[2], ns[3]))));
    }
    schedule
}
