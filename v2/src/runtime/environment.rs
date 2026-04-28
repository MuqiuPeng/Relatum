//! Environment trait + the two baseline implementations
//! (`NoOpEnvironment`, `SyntheticStreamEnvironment`), the `Event`
//! enum produced by environments, and the `should_wake` predicate
//! that lifts the runtime out of `Sleeping`. ADR 0052 / A3, B0.

use crate::R;

#[derive(Debug, Clone)]
pub enum Event {
    AddEdge(R),
    RemoveEdge(R),
    Tick,
}

pub trait Environment {
    fn poll(&mut self) -> Vec<Event>;
}

pub struct NoOpEnvironment;

impl Environment for NoOpEnvironment {
    fn poll(&mut self) -> Vec<Event> {
        Vec::new()
    }
}

/// Replay events from a fixed schedule of `(target_poll_index, Event)`
/// pairs. The poll index is 1-based — the first call to `poll`
/// returns all events scheduled for `<= 1`. ADR 0052 § Phase B / B0.
///
/// Use case: drip-feed an evolving graph into the runtime to verify
/// it responds correctly to a temporal sequence.
pub struct SyntheticStreamEnvironment {
    schedule: Vec<(u64, Event)>,
    polled_count: u64,
}

impl SyntheticStreamEnvironment {
    pub fn new(schedule: Vec<(u64, Event)>) -> Self {
        let mut s = schedule;
        s.sort_by_key(|(t, _)| *t);
        Self { schedule: s, polled_count: 0 }
    }

    pub fn polled_count(&self) -> u64 {
        self.polled_count
    }

    pub fn remaining(&self) -> usize {
        self.schedule.len()
    }
}

impl Environment for SyntheticStreamEnvironment {
    fn poll(&mut self) -> Vec<Event> {
        self.polled_count += 1;
        let target = self.polled_count;
        let mut out = Vec::new();
        // Drain events whose tick <= current poll index. Keep those
        // in the future. Sorted schedule means we can stop early once
        // we hit a future-tick event.
        while !self.schedule.is_empty() && self.schedule[0].0 <= target {
            let (_, ev) = self.schedule.remove(0);
            out.push(ev);
        }
        out
    }
}

/// Wake predicate: any data-mutating event lifts the runtime out of
/// `Sleeping`. Bare `Tick` is informational and does NOT wake — it
/// preserves "no signal, stay asleep" semantics. ADR 0052 / A3.
///
/// Ordering: this predicate is evaluated *after* `Environment::poll`
/// but *before* `apply_events`, so any data-mutating event arriving
/// in this tick will both wake the runtime AND modify the rset on
/// the same pass — the next iteration (or the rest of this
/// iteration) sees a dirty frontier.
pub fn should_wake(events: &[Event]) -> bool {
    events.iter().any(|e| matches!(e, Event::AddEdge(_) | Event::RemoveEdge(_)))
}
