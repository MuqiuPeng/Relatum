//! Scheduling — intrinsic drive enters episode selection (M3).
//!
//! v3 M1–M2 ran with fixed external schedules: tests hand-picked which
//! mechanism + seed to generate. M3 introduces a scheduling loop
//! above the simulator and fingerprint engine: a `Scheduler` decides
//! which candidate to generate next, optionally informed by an
//! externally-computed "drive" scalar.
//!
//! The drive signal is left abstract — the M3 constitution says
//! intrinsic drive *may* inform scheduling without dictating *what*
//! that signal is. The natural choice for L4 / T5 is **prediction
//! error**: where the T5 chain-composition predictor disagrees with
//! the observed AC fingerprint, the substrate has something to learn.
//! The integration test at the bottom wires this concretely.
//!
//! Two schedulers ship in M3:
//!
//! - `RoundRobinScheduler`: drive-blind baseline. Cycles the candidate
//!   pool uniformly until the budget is exhausted.
//! - `DriveSeekingScheduler`: cold-start explores each candidate once,
//!   then prefers candidates with the highest recorded drive. Ties
//!   broken by oldest last-use to avoid starvation.

/// Episode-selection interface.
///
/// `next` issues the next candidate index, or `None` when the budget
/// is exhausted. `record_drive` reports an externally-computed signal
/// after a candidate was generated and processed.
pub trait Scheduler {
    fn next(&mut self) -> Option<usize>;
    fn record_drive(&mut self, idx: usize, drive: f64);
    fn pool_size(&self) -> usize;
}

/// Drive-blind round-robin. Useful as a control / baseline.
pub struct RoundRobinScheduler {
    n: usize,
    pos: usize,
    budget: usize,
    issued: usize,
}

impl RoundRobinScheduler {
    pub fn new(n_candidates: usize, budget: usize) -> Self {
        assert!(n_candidates > 0, "non-empty pool");
        RoundRobinScheduler {
            n: n_candidates,
            pos: 0,
            budget,
            issued: 0,
        }
    }
}

impl Scheduler for RoundRobinScheduler {
    fn next(&mut self) -> Option<usize> {
        if self.issued >= self.budget {
            return None;
        }
        let idx = self.pos;
        self.pos = (self.pos + 1) % self.n;
        self.issued += 1;
        Some(idx)
    }

    fn record_drive(&mut self, _idx: usize, _drive: f64) {}

    fn pool_size(&self) -> usize {
        self.n
    }
}

/// Drive-seeking scheduler. Cold-start explores every candidate once,
/// then issues whichever candidate has the highest recorded drive.
/// Ties broken by oldest last-use so a strictly-dominant candidate
/// does not entirely starve its peers when their drives later catch up.
pub struct DriveSeekingScheduler {
    n: usize,
    budget: usize,
    issued: usize,
    drive: Vec<f64>,
    use_count: Vec<usize>,
    last_used: Vec<u64>,
    step: u64,
}

impl DriveSeekingScheduler {
    pub fn new(n_candidates: usize, budget: usize) -> Self {
        assert!(n_candidates > 0, "non-empty pool");
        DriveSeekingScheduler {
            n: n_candidates,
            budget,
            issued: 0,
            drive: vec![0.0; n_candidates],
            use_count: vec![0; n_candidates],
            last_used: vec![0; n_candidates],
            step: 0,
        }
    }

    pub fn use_count(&self, idx: usize) -> usize {
        self.use_count[idx]
    }

    pub fn drive_of(&self, idx: usize) -> f64 {
        self.drive[idx]
    }
}

impl Scheduler for DriveSeekingScheduler {
    fn next(&mut self) -> Option<usize> {
        if self.issued >= self.budget {
            return None;
        }
        self.step += 1;

        // Phase 1: cold-start exploration — issue every untouched
        // candidate before doing anything driven.
        if let Some(idx) = (0..self.n).find(|i| self.use_count[*i] == 0) {
            self.use_count[idx] += 1;
            self.last_used[idx] = self.step;
            self.issued += 1;
            return Some(idx);
        }

        // Phase 2: exploitation. Pick the candidate with the highest
        // drive; tie-break by oldest last-use.
        let best = (0..self.n)
            .max_by(|a, b| {
                let cmp = self.drive[*a]
                    .partial_cmp(&self.drive[*b])
                    .unwrap_or(std::cmp::Ordering::Equal);
                if cmp != std::cmp::Ordering::Equal {
                    cmp
                } else {
                    // Lower last_used wins (older preferred), so we
                    // reverse to make `max_by` pick it.
                    self.last_used[*b].cmp(&self.last_used[*a])
                }
            })
            .expect("non-empty pool");

        self.use_count[best] += 1;
        self.last_used[best] = self.step;
        self.issued += 1;
        Some(best)
    }

    fn record_drive(&mut self, idx: usize, drive: f64) {
        self.drive[idx] = drive;
    }

    fn pool_size(&self) -> usize {
        self.n
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::ChainBB;
    use crate::{
        estimate_all, fingerprint_similarity, predict_chain_composition, Fingerprint, NodeId,
    };

    #[test]
    fn round_robin_distributes_uniformly() {
        let mut s = RoundRobinScheduler::new(4, 8);
        let mut counts = vec![0; 4];
        while let Some(idx) = s.next() {
            counts[idx] += 1;
        }
        assert_eq!(counts, vec![2, 2, 2, 2]);
    }

    #[test]
    fn round_robin_terminates_at_budget() {
        let mut s = RoundRobinScheduler::new(3, 5);
        let mut n = 0;
        while s.next().is_some() {
            n += 1;
        }
        assert_eq!(n, 5);
    }

    #[test]
    fn round_robin_ignores_drive() {
        let mut s = RoundRobinScheduler::new(2, 4);
        let mut seq = Vec::new();
        while let Some(idx) = s.next() {
            seq.push(idx);
            // Bias drive toward 0 — round robin should ignore.
            s.record_drive(idx, if idx == 0 { 1.0 } else { 0.0 });
        }
        assert_eq!(seq, vec![0, 1, 0, 1]);
    }

    #[test]
    fn drive_seeker_explores_each_candidate_once_in_cold_start() {
        let mut s = DriveSeekingScheduler::new(3, 5);
        let mut explored = vec![false; 3];
        for _ in 0..3 {
            let idx = s.next().unwrap();
            assert!(!explored[idx], "candidate {idx} revisited during exploration");
            explored[idx] = true;
            s.record_drive(idx, 0.0);
        }
        assert!(explored.iter().all(|&x| x));
    }

    #[test]
    fn drive_seeker_prefers_high_drive_after_exploration() {
        let mut s = DriveSeekingScheduler::new(3, 12);
        // Exploration: feed back drives such that candidate 1 is the
        // attractive one.
        for _ in 0..3 {
            let idx = s.next().unwrap();
            s.record_drive(idx, if idx == 1 { 1.0 } else { 0.0 });
        }
        // Exploitation: should now lean heavily toward candidate 1.
        let mut counts = vec![0; 3];
        while let Some(idx) = s.next() {
            counts[idx] += 1;
            s.record_drive(idx, if idx == 1 { 1.0 } else { 0.0 });
        }
        assert!(
            counts[1] > counts[0] && counts[1] > counts[2],
            "high-drive candidate did not dominate: {:?}",
            counts
        );
    }

    #[test]
    fn drive_seeker_terminates_at_budget() {
        let mut s = DriveSeekingScheduler::new(3, 7);
        let mut n = 0;
        while let Some(idx) = s.next() {
            s.record_drive(idx, 0.5);
            n += 1;
        }
        assert_eq!(n, 7);
    }

    /// End-to-end: drive scheduler picks among ChainBB candidates,
    /// drive is T5 prediction error (1 − fingerprint similarity).
    ///
    /// We don't assert which candidate wins (depends on seed-specific
    /// recovery noise) — only that the loop runs end-to-end with the
    /// real T5 pipeline and stays within budget.
    #[test]
    fn drive_seeker_drives_t5_chain_workflow_end_to_end() {
        let candidates: Vec<(u32, u32)> = vec![(1, 1), (2, 3), (3, 4)];
        let mut sched = DriveSeekingScheduler::new(candidates.len(), 10);
        let mut seed: u64 = 0;
        let mut runs = 0;
        while let Some(idx) = sched.next() {
            seed = seed.wrapping_add(1);
            let (lag_ab, lag_bc) = candidates[idx];
            let mut chain = ChainBB::default_triple(
                NodeId::new("N1"),
                NodeId::new("N2"),
                NodeId::new("N3"),
            );
            chain.lag_ab = lag_ab;
            chain.lag_bc = lag_bc;
            let ep = chain.generate("E", 800, seed);
            let fps = estimate_all(&ep);
            let ab = look(&fps, "N1", "N2");
            let bc = look(&fps, "N2", "N3");
            let observed_ac = look(&fps, "N1", "N3");
            let predicted_ac = predict_chain_composition(ab, bc);
            let drive = 1.0 - fingerprint_similarity(&predicted_ac, observed_ac);
            sched.record_drive(idx, drive);
            runs += 1;
        }
        assert_eq!(runs, 10);
        // All three candidates must have been touched at least once
        // (cold-start guarantee).
        for i in 0..candidates.len() {
            assert!(sched.use_count(i) >= 1, "candidate {i} starved");
        }
    }

    fn look<'a>(fps: &'a [Fingerprint], s: &str, t: &str) -> &'a Fingerprint {
        fps.iter()
            .find(|f| f.source == NodeId::new(s) && f.target == NodeId::new(t))
            .expect("fingerprint missing")
    }
}
