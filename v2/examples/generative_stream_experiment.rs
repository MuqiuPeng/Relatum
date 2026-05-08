//! Experiment: does an indefinitely-generating stream sustain
//! v2's cognitive activity, or does the system still settle into
//! single-shot mode?
//!
//! GenerativeDiamondEnvironment emits a fresh-token 4-node
//! diamond poset every 100 polls forever. Each phase introduces
//! 4 new tokens (`g{phase}_0..3`) and 9 edges (4 self-loops + 5
//! diamond covers).
//!
//! Hypothesis: runtime stays active longer than on a finite
//! stream because `is_clean_subgraph`-failing instances get
//! refreshed each phase, but it eventually saturates because:
//!   - axiom shape grammar is hard-coded (same axioms fire
//!     repeatedly, no new ones discovered)
//!   - theory member set converges
//!   - patterns at sizes 2-5 reach their canonical-form
//!     repertoire and stop being novel
//!
//! Snapshot every 500 ticks for 5000 ticks total, measuring
//! axioms / theories / patterns / total-pattern-instances / drive.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, Environment, Event, RuleBasedScheduler,
    },
    R, RSet,
};

const PHASE_SIZE: u64 = 100;
const HORIZON_TICKS: u64 = 5000;
const SNAPSHOT_INTERVAL: u64 = 500;

/// Emits a fresh-token 4-node diamond poset every PHASE_SIZE
/// polls. Phases start at poll = 1 + N * PHASE_SIZE.
struct GenerativeDiamondEnvironment {
    polled_count: u64,
    phase_size: u64,
    next_phase: u64,
}

impl GenerativeDiamondEnvironment {
    fn new(phase_size: u64) -> Self {
        Self { polled_count: 0, phase_size, next_phase: 0 }
    }
}

impl Environment for GenerativeDiamondEnvironment {
    fn poll(&mut self) -> Vec<Event> {
        self.polled_count += 1;
        let phase_start = self.next_phase * self.phase_size + 1;
        if self.polled_count < phase_start {
            return Vec::new();
        }
        let phase = self.next_phase;
        self.next_phase += 1;
        let n: [String; 4] = std::array::from_fn(|i| format!("g{}_{}", phase, i));
        let mut events = Vec::new();
        for i in 0..4 {
            events.push(Event::AddEdge(R::new(&n[i][..], &n[i][..])));
        }
        events.push(Event::AddEdge(R::new(&n[0][..], &n[1][..])));
        events.push(Event::AddEdge(R::new(&n[0][..], &n[2][..])));
        events.push(Event::AddEdge(R::new(&n[0][..], &n[3][..])));
        events.push(Event::AddEdge(R::new(&n[1][..], &n[3][..])));
        events.push(Event::AddEdge(R::new(&n[2][..], &n[3][..])));
        events
    }
}

#[derive(Debug, Clone)]
struct Snapshot {
    tick: u64,
    axioms: usize,
    theories: usize,
    patterns: usize,
    total_pattern_instances: usize,
    episodes: usize,
    unexplained_count: usize,
    unexplained_ratio: f64,
    distinct_canonicals: usize,
}

fn snapshot(rt: &AutonomousRuntime, tick: u64) -> Snapshot {
    let drive = rt.rset.unexplained_drive_signal();
    let total_pattern_instances: usize = rt.rset.patterns().iter()
        .map(|p| rt.rset.instances_of(p).len())
        .sum();
    Snapshot {
        tick,
        axioms: rt.rset.axioms().len(),
        theories: rt.rset.theories().len(),
        patterns: rt.rset.patterns().len(),
        total_pattern_instances,
        episodes: rt.memory.episodes.len(),
        unexplained_count: drive.unexplained_count,
        unexplained_ratio: drive.unexplained_ratio,
        distinct_canonicals: drive.distinct_canonicals,
    }
}

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" Generative-stream experiment");
    println!("════════════════════════════════════════════════════════");
    println!(" Stream: fresh-token diamond poset every {} polls,",
             PHASE_SIZE);
    println!(" indefinitely. Horizon: {} ticks.", HORIZON_TICKS);
    println!();
    println!(" Hypothesis: runtime stays active longer than on finite");
    println!(" stream but eventually saturates as canonical repertoire");
    println!(" exhausts.");

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(GenerativeDiamondEnvironment::new(PHASE_SIZE));
    rt.scheduler = Box::new(RuleBasedScheduler::default());

    let mut snapshots: Vec<Snapshot> = Vec::new();
    let mut current_tick: u64 = 0;
    while current_tick < HORIZON_TICKS {
        let next_target = (current_tick + SNAPSHOT_INTERVAL).min(HORIZON_TICKS);
        let step = next_target - current_tick;
        rt.run_bounded(step);
        current_tick = next_target;
        snapshots.push(snapshot(&rt, current_tick));
    }

    println!();
    println!(" Time series (every {} ticks):", SNAPSHOT_INTERVAL);
    println!();
    println!(
        " {:>6} {:>4} {:>4} {:>4} {:>9} {:>5} {:>10} {:>5}",
        "tick", "axs", "ths", "pat", "pat_inst", "eps", "unexpl",
        "buckets",
    );
    println!(" {}", "─".repeat(66));
    let mut prev_episodes = 0usize;
    let mut prev_pat_inst = 0usize;
    for s in &snapshots {
        let eps_delta = s.episodes - prev_episodes;
        let inst_delta = s.total_pattern_instances as i64 - prev_pat_inst as i64;
        prev_episodes = s.episodes;
        prev_pat_inst = s.total_pattern_instances;
        let unexpl_str = format!(
            "{} ({:.0}%)",
            s.unexplained_count, s.unexplained_ratio * 100.0,
        );
        println!(
            " {:>6} {:>4} {:>4} {:>4} {:>9} {:>5} {:>10} {:>5}",
            s.tick,
            s.axioms,
            s.theories,
            s.patterns,
            format!("{:+}/{}", inst_delta, s.total_pattern_instances),
            format!("{:+}", eps_delta),
            unexpl_str,
            s.distinct_canonicals,
        );
    }

    // Summary.
    let final_s = snapshots.last().unwrap();
    let mid_s = &snapshots[snapshots.len() / 2];
    let second_half_eps = final_s.episodes.saturating_sub(mid_s.episodes);
    let second_half_inst = final_s.total_pattern_instances
        .saturating_sub(mid_s.total_pattern_instances);

    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Summary");
    println!("════════════════════════════════════════════════════════");
    println!();
    println!(" Final state at tick {}:", final_s.tick);
    println!("   axioms        = {}", final_s.axioms);
    println!("   theories      = {}", final_s.theories);
    println!("   patterns      = {}", final_s.patterns);
    println!("   pattern insts = {}", final_s.total_pattern_instances);
    println!("   episodes      = {}", final_s.episodes);
    println!("   unexplained R = {} ({:.0}%)",
             final_s.unexplained_count,
             final_s.unexplained_ratio * 100.0);
    println!();
    println!(" Second-half (tick {} → {}) activity:",
             mid_s.tick, final_s.tick);
    println!("   episodes added         = {}", second_half_eps);
    println!("   pattern instances added = {}", second_half_inst);

    println!();
    println!(" Verdict:");
    if second_half_eps == 0 && second_half_inst == 0 {
        println!("   FULLY SATURATED — runtime is in deep sleep through");
        println!("   the second half despite stream still feeding events.");
        println!("   Stream injection alone does not sustain cognition.");
    } else if second_half_eps > 0 && second_half_inst > 0 {
        println!("   ACTIVE — runtime keeps minting + dispatching across");
        println!("   the second half. Generative stream sustains some");
        println!("   activity beyond the single-shot phase.");
    } else if second_half_eps > 0 {
        println!("   PARTIAL — runtime still dispatches but no new pattern");
        println!("   instances accumulate. Saturation in pattern path,");
        println!("   not in scheduler.");
    } else {
        println!("   ANOMALOUS — pattern instances accrue without episodes;");
        println!("   should not happen.");
    }

    println!();
    println!("--- end ---");
}
