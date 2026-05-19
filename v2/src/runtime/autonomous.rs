//! AutonomousRuntime — main runtime loop, lifecycle/mode state
//! machine, action dispatch, episode and transition logging, and
//! checkpoint serialization. ADR 0052 / A0–A3 + later.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::{
    AutonomousConfig, AxiomDiscoveryConfig, DiscoveryConfig, NamingPolicy,
    PatternRecordingPolicy, RefinementConfig, RSet, TheoryRelationKind,
    DRIVE_MARKER, ESTABLISHED_MARKER, PENALTY_MARKER, SHARED_AXIOM_MARKER, R,
};

use super::action::{ActionKind, ActionPlan, FrontierTarget, SchedulerDecision};
use super::drive::{
    CompressionDrive, Drive, DriveABState, DriveMix, ModeThrashPenalty,
    PredictionErrorDrive,
};
use super::environment::{should_wake, Environment, Event, NoOpEnvironment};
use super::frontier::{Frontier, FrontierKind};
use super::lifecycle::{BudgetState, LifecycleState, RuntimeMode};
use super::memory::{
    Episode, LifecycleTransition, Memory, ModeTransition, ObjectHistory,
    ObjectHistoryStore, PolicyStats, PredictionState, SequenceStats,
};
use super::persistence::{
    action_kind_to_str, check_no_tab_or_newline, check_reason, lifecycle_to_str, mode_to_str, pair_to_target,
    parse_action_kind, parse_checkpoint, parse_f64, parse_history_lines,
    parse_lifecycle, parse_mode, parse_u32,
    parse_u64, parse_usize, target_to_pair, write_history_section,
};
use super::scheduler::{Scheduler, SchedulerContext, StubScheduler};
use super::scheduler_rule::RuleBasedScheduler;

pub(crate) fn theory_pair_has_relation(rset: &RSet, a: &str, b: &str) -> bool {
    rset.extension_edges().iter().any(|e| {
        if let Some((sub, sup)) = rset.extension_endpoints(e) {
            (sub == a && sup == b) || (sub == b && sup == a)
        } else {
            false
        }
    }) || rset.independence_edges().iter().any(|e| {
        if let Some((lo, hi)) = rset.independence_endpoints(e) {
            (lo == a && hi == b) || (lo == b && hi == a)
        } else {
            false
        }
    }) || rset.parallel_edges().iter().any(|e| {
        if let Some((lo, hi)) = rset.parallel_endpoints(e) {
            (lo == a && hi == b) || (lo == b && hi == a)
        } else {
            false
        }
    })
}

// ─── runtime ───────────────────────────────────────────────────────

pub struct AutonomousRuntime {
    pub rset: RSet,
    pub lifecycle: LifecycleState,
    pub mode: RuntimeMode,
    pub memory: Memory,
    pub scheduler: Box<dyn Scheduler>,
    pub environment: Box<dyn Environment>,
    pub frontier: Frontier,

    pub tick: u64,
    pub episode_counter: u64,
    pub steps_since_last_gain: u64,
    pub budget: BudgetState,
    pub current_score: f64,

    /// Snapshot of `checkpoint_text()` taken on the last entry into
    /// `Sleeping` or `Stopped`. ADR 0052 / A3. Caller persists to disk
    /// at its discretion; the runtime itself does no I/O.
    pub last_checkpoint: Option<String>,

    /// Drive blend with A/B mutation. ADR 0063 / Phase H2.0 step 2.
    /// Shadow-only at step 2: advances per tick but does not yet
    /// govern wake/mode/sleep behaviour. Step 3 wires this into
    /// the gates.
    pub drive_mix: DriveMix,

    /// Registered drives consulted by `combined_drive_signal`.
    /// ADR 0063 / Phase H2.0 step 3a — observability layer that
    /// uses `drive_mix.active_weights()` to blend per-drive
    /// scalars. Still shadow: nothing yet gates on this.
    /// `Box<dyn Drive>` keeps the registry pluggable across
    /// step-3b's wake-gate refactor and any future H2.1 work.
    pub drives: Vec<Box<dyn Drive>>,
}

impl AutonomousRuntime {
    /// Construct with defaults: `StubScheduler`, `NoOpEnvironment`,
    /// empty Frontier (refreshes on first tick). Caller swaps
    /// `scheduler` / `environment` before `run_bounded` as needed.
    pub fn new(rset: RSet) -> Self {
        let current_score = rset.abstraction_score();
        let drives: Vec<Box<dyn Drive>> = vec![
            Box::new(CompressionDrive),
            Box::new(PredictionErrorDrive),
            Box::new(ModeThrashPenalty),
        ];
        let mut rt = Self {
            rset,
            lifecycle: LifecycleState::Running,
            mode: RuntimeMode::Expand,
            memory: Memory::default(),
            scheduler: Box::new(StubScheduler),
            environment: Box::new(NoOpEnvironment),
            frontier: Frontier::default(),
            tick: 0,
            episode_counter: 0,
            steps_since_last_gain: 0,
            budget: BudgetState::new(1),
            current_score,
            last_checkpoint: None,
            drive_mix: DriveMix::default(),
            drives,
        };
        rt.register_drives_in_rset();
        rt
    }

    /// Register each drive as `R(DRIVE_MARKER, drive_<id>)` and,
    /// if penalty, also `R(PENALTY_MARKER, drive_<id>)`.
    ///
    /// **rset-as-source-of-truth (ADR 0064 H2.1.1 cleanup):**
    /// If the DRIVE_MARKER edge for a drive already exists, this
    /// function is a no-op for that drive — including the
    /// PENALTY_MARKER edge. This guarantees that manual edge
    /// retractions (e.g., a future H2.1.2 demotion that retracts
    /// `R(PENALTY_MARKER, drive_X)` to flip the drive's role)
    /// SURVIVE checkpoint round-trip. Without this guard,
    /// `from_checkpoint_text` would re-assert the PENALTY_MARKER
    /// edge on every restore, undoing intentional retractions.
    ///
    /// On first call (empty rset, before any DRIVE_MARKER edges)
    /// the behavior is the original H2.1.0 specification: write
    /// DRIVE_MARKER + (if penalty) PENALTY_MARKER per
    /// `Drive::is_penalty()` boot default.
    pub(crate) fn register_drives_in_rset(&mut self) {
        for drive in &self.drives {
            let drive_id = format!("drive_{}", drive.id());
            let drive_marker_edge =
                R::new(DRIVE_MARKER, drive_id.as_str());
            if self.rset.contains(&drive_marker_edge) {
                // Already registered — rset is the source of
                // truth. Do not re-assert the (possibly retracted)
                // PENALTY_MARKER edge.
                continue;
            }
            self.rset.add(drive_marker_edge);
            if drive.is_penalty() {
                self.rset
                    .add(R::new(PENALTY_MARKER, drive_id.as_str()));
            }
        }
    }

    /// Compute the blended drive signal — `Σ_id (active_weights[id]
    /// * drive.evaluate(rset, memory, tick))`. ADR 0063 / Phase
    /// H2.0 step 3a. Negative drives (penalties) are honoured by
    /// allowing negative `evaluate()` returns; the convention
    /// from H2.0 step 1 is non-negative for the 3 baseline drives,
    /// so the blend is non-negative under that catalogue.
    ///
    /// Step 3a is observability-only: nothing gates on this value
    /// yet. Step 3b will replace the zero-streak anti-stagnation
    /// gate with `combined_drive_signal < threshold`.
    pub fn combined_drive_signal(&self) -> f64 {
        // ADR 0063 OQ #4 resolution: positive drives add their
        // weighted evaluate; penalty drives subtract.
        // ADR 0064 / H2.1.0+ — penalty status is now queried
        // from meta-R (`R(PENALTY_MARKER, drive_<id>)`) as the
        // canonical source of truth. The compile-time
        // `Drive::is_penalty()` method is left as a fast-path
        // fallback but no longer consulted here. This makes
        // penalty status mutable at runtime: retracting the
        // PENALTY_MARKER edge for a drive removes its penalty
        // contribution.
        let weights = self.drive_mix.active_weights();
        let mut total: f64 = 0.0;
        for drive in &self.drives {
            let w = weights.get(drive.id()).copied().unwrap_or(0.0);
            if w == 0.0 {
                continue;
            }
            let signal = drive.evaluate(&self.rset, &self.memory, self.tick);
            if self.is_drive_penalty_via_meta_r(drive.id()) {
                total -= w * signal;
            } else {
                total += w * signal;
            }
        }
        total
    }

    /// Query meta-R for whether a drive is registered as a
    /// penalty: `R(PENALTY_MARKER, drive_<id>)`. Returns false
    /// when the drive isn't registered (defensive default —
    /// treat unknown drives as positive contributors). ADR 0064 /
    /// Phase H2.1.0+.
    fn is_drive_penalty_via_meta_r(&self, drive_id_str: &str) -> bool {
        let drive_id = format!("drive_{}", drive_id_str);
        self.rset.contains(&R::new(PENALTY_MARKER, drive_id.as_str()))
    }

    /// Weight-invariant blended drive signal — positive-drive
    /// contribution minus penalty-drive contribution, divided by
    /// the *positive* drives' weight sum. ADR 0063 / Phase H2.0
    /// step 3b + OQ #4 resolution. Used by the EP anti-stagnation
    /// gate as the "drives say no productive work available"
    /// criterion.
    ///
    /// Why exclude penalty weights from the denominator:
    /// the denominator answers "what's the weight-magnitude scale
    /// of positive activity I'm averaging across"; including
    /// penalty weights would muddy that scale. The numerator
    /// already accounts for penalty contribution by subtraction.
    pub fn normalized_drive_signal(&self) -> f64 {
        // ADR 0064 / H2.1.0+ — penalty status queried from meta-R.
        let weights = self.drive_mix.active_weights();
        let mut positive_weight_sum: f64 = 0.0;
        for drive in &self.drives {
            if self.is_drive_penalty_via_meta_r(drive.id()) {
                continue;
            }
            positive_weight_sum +=
                weights.get(drive.id()).copied().unwrap_or(0.0);
        }
        if positive_weight_sum < f64::EPSILON {
            return 0.0;
        }
        self.combined_drive_signal() / positive_weight_sum
    }

    /// Record a lifecycle transition and update `self.lifecycle`.
    /// Snapshot a checkpoint when entering `Sleeping` or `Stopped`.
    /// No-op if `to == self.lifecycle`. ADR 0052 / A3.
    fn transition_lifecycle(
        &mut self,
        to: LifecycleState,
        reason: &str,
    ) {
        if to == self.lifecycle {
            return;
        }
        let from = self.lifecycle;
        self.memory.record_lifecycle_transition(LifecycleTransition {
            tick: self.tick,
            from,
            to,
            reason: reason.to_string(),
        });
        // B0 / PolicyStats.
        match to {
            LifecycleState::Sleeping => {
                self.memory.policy_stats.sleep_count += 1;
            }
            LifecycleState::Running if from == LifecycleState::Sleeping => {
                self.memory.policy_stats.wake_count += 1;
            }
            LifecycleState::Stopped => {
                self.memory.policy_stats.stop_count += 1;
            }
            _ => {}
        }
        self.lifecycle = to;
        if matches!(to, LifecycleState::Sleeping | LifecycleState::Stopped) {
            if let Ok(cp) = self.checkpoint_text() {
                self.last_checkpoint = Some(cp);
            }
        }
    }

    pub fn run_bounded(&mut self, max_ticks: u64) {
        let start_tick = self.tick;

        while self.tick - start_tick < max_ticks
            && self.lifecycle != LifecycleState::Stopped
        {
            self.tick += 1;
            self.budget.reset_per_tick();

            // 1. Ingest events. Decide wake-on-event before applying so
            //    the predicate's input matches the events we just got.
            let events = self.environment.poll();
            let wake_signal = should_wake(&events);
            if !events.is_empty() {
                self.apply_events(events);
                self.frontier.mark_dirty();
            }

            // 1b. Verify last tick's predictions against the now-
            //     current rset state. ADR 0059 / Phase G1.3.
            self.verify_predictions();

            // 2. Sleeping short-circuit. Wake on any data event
            //    OR (per ADR 0079) on non-empty drive signal at
            //    a mature rset. The drive-wake check is throttled
            //    to once every DRIVE_WAKE_INTERVAL ticks so the
            //    O(unexplained-count) cost of building the drive
            //    signal doesn't dominate idle ticks. ADR 0052 / A3
            //    + ADR 0079.
            const DRIVE_WAKE_INTERVAL: u64 = 25;
            const MATURE_DATA_EDGE_FLOOR: usize = 100;
            // ADR 0080 — wake only if drive has signal AND
            // learning progress at modal canonical is non-trivial.
            // A bucket with no recent mint success at its size
            // doesn't justify waking the runtime.
            // Constants centralized in agent_view (2026-05-11 tuning).
            let drive_wakes = !wake_signal
                && self.lifecycle == LifecycleState::Sleeping
                && self.tick % DRIVE_WAKE_INTERVAL == 0
                && self.rset.axioms().len() >= 1
                && self.rset.iter().count() >= MATURE_DATA_EDGE_FLOOR
                && {
                    let drive = self.rset.unexplained_drive_signal();
                    crate::runtime::drive_should_engage(
                        &drive,
                        &self.memory.episodes,
                        crate::runtime::agent_view::LP_DRIVE_THRESHOLD,
                    )
                };
            if self.lifecycle == LifecycleState::Sleeping {
                if wake_signal {
                    self.transition_lifecycle(
                        LifecycleState::Running,
                        "wake_on_event",
                    );
                } else if drive_wakes {
                    self.transition_lifecycle(
                        LifecycleState::Running,
                        "wake_on_drive",
                    );
                    self.frontier.mark_dirty();
                } else {
                    continue;
                }
            }

            // 3. Refresh frontier when dirty (cheap at β-scale). The
            //    staleness pass (B3) consults object_history, so it
            //    runs alongside refresh whenever items are recomputed.
            //    The promotion pass (C0) also rides the same dirty
            //    gate; it inspects rset for already-promoted ids.
            if self.frontier.dirty {
                self.frontier.refresh_with_episodes(
                    &self.rset, self.tick, &self.memory.episodes,
                );
                self.frontier.refresh_stale_prune(
                    &self.memory.object_history,
                    self.tick,
                );
                // ADR 0082 — policy-driven theory maintenance.
                self.frontier.refresh_policy_targets(
                    &self.rset,
                    &self.memory.prediction_state,
                    &self.memory.episodes,
                    self.tick,
                );
                self.frontier.refresh_established_promotions(
                    &self.rset,
                    &self.memory.object_history,
                    self.tick,
                );
                self.frontier.refresh_shared_axiom_promotions(
                    &self.rset,
                    self.tick,
                );
                self.frontier.refresh_meta_meta_candidates(
                    &self.rset,
                    self.tick,
                );
                // ADR 0061 / Phase H1.2 — composite candidates
                // depend on BOTH the rset's named action-sequence
                // pairs AND the other frontier kinds present in this
                // refresh, so it must run last among the refresh
                // helpers.
                self.frontier.refresh_composite_candidates(
                    &self.rset,
                    self.tick,
                );
                // ADR 0068 / Phase B.5.1 — surface shape-family
                // discovery candidate when registered axioms have
                // a shared premise that's not yet a family. Cheap
                // structural check; ran after composite to keep
                // composite's freshness check unchanged.
                self.frontier.refresh_shape_family_candidates(
                    &self.rset,
                    self.tick,
                );
            }

            // 4. Scheduler decision. ADR 0063 / Phase H2.0 step 3b
            //    pre-computes normalized_drive_signal so the EP
            //    anti-stagnation gate can AND-combine it with the
            //    zero-streak gate. Computed before constructing the
            //    context to avoid borrow conflicts (drive_mix +
            //    drives are owned, not borrowed via the context).
            let normalized_drive_signal = self.normalized_drive_signal();
            // ADR 0079 (caching, 2026-05-11) — compute the
            // unexplained-R drive signal once per active tick,
            // pass it through SchedulerContext so stagnation
            // bypass + thrash bypass don't each recompute it.
            // Skip the compute when the maturity gate clearly
            // fails (no axioms / sparse rset) — drive bypass
            // wouldn't trigger anyway, and the gate fail keeps
            // small lifecycle-test fixtures fast.
            const DRIVE_CACHE_MATURE_FLOOR: usize = 100;
            let drive_for_tick: Option<crate::UnexplainedDriveSignal> =
                if !self.rset.axioms().is_empty()
                    && self.rset.iter().count() >= DRIVE_CACHE_MATURE_FLOOR
                {
                    Some(self.rset.unexplained_drive_signal())
                } else {
                    None
                };
            let decision = {
                let ctx = SchedulerContext {
                    rset: &self.rset,
                    memory: &self.memory,
                    frontier: &self.frontier,
                    mode: self.mode,
                    tick: self.tick,
                    normalized_drive_signal,
                    cached_drive: drive_for_tick.as_ref(),
                };
                self.scheduler.choose(&ctx)
            };

            // 5. Dispatch.
            match decision {
                SchedulerDecision::Execute(plan) => {
                    self.execute_and_record(plan);
                    self.frontier.mark_dirty();
                }
                SchedulerDecision::SwitchMode(m) => {
                    if m != self.mode {
                        let from = self.mode;
                        self.memory.record_mode_transition(ModeTransition {
                            tick: self.tick,
                            from,
                            to: m,
                            reason: "scheduler".to_string(),
                        });
                        // B0 / PolicyStats.
                        *self
                            .memory
                            .policy_stats
                            .mode_transition_counts
                            .entry((from, m))
                            .or_insert(0) += 1;
                        self.mode = m;
                    }
                }
                SchedulerDecision::Sleep => {
                    self.transition_lifecycle(
                        LifecycleState::Sleeping,
                        "scheduler_sleep",
                    );
                }
                SchedulerDecision::Stop => {
                    self.transition_lifecycle(
                        LifecycleState::Stopped,
                        "scheduler_stop",
                    );
                }
            }

            // 6. Snapshot predictions for verify-on-next-tick.
            //    ADR 0059 / Phase G1.3. Skipped while sleeping —
            //    nothing should change between sleep ticks, so the
            //    snapshot from before sleep stays valid until wake.
            if self.lifecycle == LifecycleState::Running {
                self.snapshot_predictions();
            }

            // 7. ADR 0063 / Phase H2.0 step 2 — advance the
            //    DriveMix A/B cycle. Shadow only: state advances
            //    based on episode count, but no caller yet
            //    consults `active_weights()` to gate behaviour.
            self.drive_mix.maybe_advance(&self.memory);
        }
    }

    /// Compute and store one prediction per named axiom, keyed by
    /// the axiom's id. Runs at the end of each Running tick. The
    /// stored set will be verified against rset state at the start
    /// of the next tick. ADR 0059 / Phase G1.3.
    fn snapshot_predictions(&mut self) {
        // ADR 0066 Addendum 4 perf fix (Option A): amortize
        // meta_ids + data_ids computation across all axioms.
        //
        // ADR 0066 Addendum 5+ perf fix (Option B / per-axiom
        // cache): cache forward_apply_axiom results keyed by
        // rset.version(). When rset is unchanged since last
        // snapshot, cache hits skip the expensive
        // forward_apply_recursive O(N^k) call. Cache invalidates
        // wholesale on any rset change.
        let rset_version = self.rset.version();
        let cache_valid = self
            .memory
            .prediction_state
            .forward_apply_cache_version
            == Some(rset_version);
        if !cache_valid {
            self.memory.prediction_state.forward_apply_cache.clear();
            self.memory
                .prediction_state
                .forward_apply_cache_version = Some(rset_version);
        }
        let meta = self.rset.collect_meta_ids();
        let data_ids = self.rset.compute_data_ids(&meta);
        let mut snapshot: HashMap<String, HashSet<R>> = HashMap::new();
        if data_ids.is_empty() {
            self.memory.prediction_state.last_predicted_at_tick =
                Some(self.tick);
            self.memory.prediction_state.last_predicted_per_axiom = snapshot;
            return;
        }
        for ax in self.rset.axioms() {
            let predicted = if let Some(cached) = self
                .memory
                .prediction_state
                .forward_apply_cache
                .get(ax)
            {
                cached.clone()
            } else {
                let p = self
                    .rset
                    .forward_apply_axiom_with_data_ids(ax, &data_ids);
                self.memory
                    .prediction_state
                    .forward_apply_cache
                    .insert(ax.to_string(), p.clone());
                p
            };
            if !predicted.is_empty() {
                snapshot.insert(ax.to_string(), predicted);
            }
        }
        self.memory.prediction_state.last_predicted_at_tick = Some(self.tick);
        self.memory.prediction_state.last_predicted_per_axiom = snapshot;
    }

    /// Compare the snapshotted prediction set (from the previous
    /// tick) against the rset's current data edges. Increment
    /// per-axiom total / verified counters. Skipped on the first
    /// tick after construction (no snapshot yet) and on the first
    /// tick after a sleep / wake (snapshot was taken pre-sleep but
    /// any wake event changes rset, so we still verify against the
    /// updated rset — that's correct). ADR 0059 / Phase G1.3.
    fn verify_predictions(&mut self) {
        if self.memory.prediction_state.last_predicted_at_tick.is_none() {
            return;
        }
        let meta = self.rset.collect_meta_ids();
        let data_edges: HashSet<R> = self
            .rset
            .iter()
            .filter(|r| !meta.contains(&r.x) && !meta.contains(&r.y))
            .cloned()
            .collect();
        let snapshot = std::mem::take(
            &mut self.memory.prediction_state.last_predicted_per_axiom,
        );
        for (axiom_id, predicted) in snapshot {
            let total = predicted.len() as u64;
            let verified = predicted.intersection(&data_edges).count() as u64;
            *self
                .memory
                .prediction_state
                .total_predictions_per_axiom
                .entry(axiom_id.clone())
                .or_insert(0) += total;
            *self
                .memory
                .prediction_state
                .verified_predictions_per_axiom
                .entry(axiom_id)
                .or_insert(0) += verified;
        }
        self.memory.prediction_state.last_predicted_at_tick = None;
    }

    fn apply_events(&mut self, events: Vec<Event>) {
        for ev in events {
            match ev {
                Event::AddEdge(r) => {
                    self.rset.add(r);
                }
                Event::RemoveEdge(r) => {
                    self.rset.remove(&r);
                }
                Event::Tick => {}
            }
        }
    }

    fn execute_and_record(&mut self, plan: ActionPlan) {
        let before = self.rset.abstraction_score();
        let patterns_before: HashSet<String> = self
            .rset
            .patterns()
            .iter()
            .map(|s| s.to_string())
            .collect();
        let theories_before: HashSet<String> = self
            .rset
            .theories()
            .iter()
            .map(|s| s.to_string())
            .collect();

        let delta_override = self.execute_action(&plan);

        let after = self.rset.abstraction_score();
        // ADR 0059 / G1.5: an action may produce a positive delta
        // without mutating rset (e.g. EvaluatePredictions reports
        // hit-rate improvement). When the action returns
        // Some(delta), use it instead of the abstraction-score
        // diff. Otherwise fall back to the standard before/after
        // arithmetic.
        let delta = delta_override.unwrap_or(after - before);
        let patterns_after: HashSet<String> = self
            .rset
            .patterns()
            .iter()
            .map(|s| s.to_string())
            .collect();
        let theories_after: HashSet<String> = self
            .rset
            .theories()
            .iter()
            .map(|s| s.to_string())
            .collect();

        self.episode_counter += 1;
        self.memory.record(Episode {
            id: self.episode_counter,
            tick: self.tick,
            mode: self.mode,
            action_kind: plan.action_kind,
            target: plan.target.clone(),
            score_before: before,
            score_after: after,
            delta,
        });

        // ── Phase B / B0: feed object history + policy stats. ──────
        let tick = self.tick;
        let store = &mut self.memory.object_history;

        for id in patterns_after.difference(&patterns_before) {
            store
                .patterns
                .entry(id.clone())
                .or_insert_with(|| ObjectHistory::new_at(tick));
        }
        for id in theories_after.difference(&theories_before) {
            store
                .theories
                .entry(id.clone())
                .or_insert_with(|| ObjectHistory::new_at(tick));
        }
        for id in patterns_before.difference(&patterns_after) {
            if let Some(h) = store.patterns.get_mut(id) {
                h.times_pruned += 1;
            }
        }
        for id in theories_before.difference(&theories_after) {
            if let Some(h) = store.theories.get_mut(id) {
                h.times_pruned += 1;
            }
        }
        for id in &patterns_after {
            if let Some(h) = store.patterns.get_mut(id) {
                h.last_seen_tick = tick;
                if delta > 0.0 {
                    h.last_improved_tick = Some(tick);
                    h.times_contributed_positive =
                        h.times_contributed_positive.saturating_add(1);
                }
            }
        }
        for id in &theories_after {
            if let Some(h) = store.theories.get_mut(id) {
                h.last_seen_tick = tick;
                if delta > 0.0 {
                    h.last_improved_tick = Some(tick);
                    h.times_contributed_positive =
                        h.times_contributed_positive.saturating_add(1);
                }
            }
        }
        match &plan.target {
            FrontierTarget::Pattern(id) => {
                if let Some(h) = store.patterns.get_mut(id) {
                    h.times_selected_as_focus += 1;
                }
            }
            FrontierTarget::Theory(id) => {
                if let Some(h) = store.theories.get_mut(id) {
                    h.times_selected_as_focus += 1;
                }
            }
            _ => {}
        }

        let stats = &mut self.memory.policy_stats;
        *stats.action_counts.entry(plan.action_kind).or_insert(0) += 1;
        if delta > 0.0 {
            *stats
                .action_positive_delta_counts
                .entry(plan.action_kind)
                .or_insert(0) += 1;
        }
        // ───────────────────────────────────────────────────────────

        if delta > 0.0 {
            self.steps_since_last_gain = 0;
        } else {
            self.steps_since_last_gain += 1;
        }
        self.current_score = after;

        // ADR 0061 / Phase H1.1 — auto-promote action-pair sequences
        // that cross the H1.1 promotion gate (count >= 5 AND mean
        // post-EP-delta > 0.05). Idempotent: rset.name_action_sequence_pair
        // returns the existing seq id if already named.
        self.maybe_promote_action_sequences();
        // ADR 0062 / Phase H1.3 — auto-demote pairs whose recent-
        // window mean has degraded below the retention floor.
        self.maybe_demote_action_sequences();
    }

    /// Auto-demotion sweep. Retracts named action-sequence pairs
    /// whose recent-window stats have degraded below the
    /// retention floor (recent count ≥ 3 AND recent mean < 0.02).
    /// ADR 0062 / Phase H1.3.
    ///
    /// Asymmetric vs. promotion (0.05 vs 0.02) for hysteresis —
    /// avoids the promote/demote oscillation that would happen if
    /// the same threshold gated both directions.
    pub(crate) fn maybe_demote_action_sequences(&mut self) {
        const MIN_RECENT_COUNT_FOR_DEMOTE: u64 = 3;
        const MIN_RECENT_MEAN_FOR_RETENTION: f64 = 0.02;
        let pairs = self.rset.action_sequence_pairs();
        let mut to_demote: Vec<(String, String)> = Vec::new();
        for (_seq_id, prefix_name, suffix_name) in pairs {
            let prefix_kind = match parse_action_kind(&prefix_name) {
                Ok(k) => k,
                Err(_) => continue,
            };
            let suffix_kind = match parse_action_kind(&suffix_name) {
                Ok(k) => k,
                Err(_) => continue,
            };
            let pair = (prefix_kind, suffix_kind);
            let recent_count = self
                .memory
                .sequence_stats
                .pair_recent_post_ep_count
                .get(&pair)
                .copied()
                .unwrap_or(0);
            if recent_count < MIN_RECENT_COUNT_FOR_DEMOTE {
                continue;
            }
            let recent_mean = match self
                .memory
                .sequence_stats
                .pair_recent_mean_post_ep_delta(pair)
            {
                Some(m) => m,
                None => continue,
            };
            if recent_mean < MIN_RECENT_MEAN_FOR_RETENTION {
                to_demote.push((prefix_name, suffix_name));
            }
        }
        for (prefix, suffix) in to_demote {
            self.rset.retract_action_sequence_pair(&prefix, &suffix);
        }

        // ADR 0062 retrospective #2 — triple demotion. Same
        // floor + minimum-recent-count gate as pairs, applied to
        // every named triple.
        let triples = self.rset.action_sequence_triples();
        let mut to_demote_t: Vec<(String, String, String)> = Vec::new();
        for (_seq_id, a_name, b_name, c_name) in triples {
            let a_kind = match parse_action_kind(&a_name) {
                Ok(k) => k,
                Err(_) => continue,
            };
            let b_kind = match parse_action_kind(&b_name) {
                Ok(k) => k,
                Err(_) => continue,
            };
            let c_kind = match parse_action_kind(&c_name) {
                Ok(k) => k,
                Err(_) => continue,
            };
            let triple = (a_kind, b_kind, c_kind);
            let recent_count = self
                .memory
                .sequence_stats
                .triple_recent_post_ep_count
                .get(&triple)
                .copied()
                .unwrap_or(0);
            if recent_count < MIN_RECENT_COUNT_FOR_DEMOTE {
                continue;
            }
            let recent_mean = match self
                .memory
                .sequence_stats
                .triple_recent_mean_post_ep_delta(triple)
            {
                Some(m) => m,
                None => continue,
            };
            if recent_mean < MIN_RECENT_MEAN_FOR_RETENTION {
                to_demote_t.push((a_name, b_name, c_name));
            }
        }
        for (a, b, c) in to_demote_t {
            self.rset.retract_action_sequence_triple(&a, &b, &c);
        }
    }

    /// Auto-promotion sweep over `Memory::sequence_stats`. Writes a
    /// meta-R chain for each pair that crosses the H1.1 thresholds.
    /// ADR 0061 / Phase H1.1.
    pub(crate) fn maybe_promote_action_sequences(&mut self) {
        const MIN_COUNT: u64 = 5;
        const MIN_MEAN_DELTA: f64 = 0.05;
        let mut to_promote: Vec<(String, String)> = Vec::new();
        for (pair, count) in
            self.memory.sequence_stats.pair_counts.iter()
        {
            if *count < MIN_COUNT {
                continue;
            }
            let mean = match self
                .memory
                .sequence_stats
                .pair_mean_post_ep_delta(*pair)
            {
                Some(m) => m,
                None => continue,
            };
            if mean <= MIN_MEAN_DELTA {
                continue;
            }
            let prefix_name = action_kind_to_str(pair.0).to_string();
            let suffix_name = action_kind_to_str(pair.1).to_string();
            if !self.rset.has_action_sequence_pair(
                &prefix_name,
                &suffix_name,
            ) {
                to_promote.push((prefix_name, suffix_name));
            }
        }
        for (prefix, suffix) in to_promote {
            let _ =
                self.rset.name_action_sequence_pair(&prefix, &suffix);
        }
        // ADR 0062 / Phase H1.4 — auto-promote triples too. Tighter
        // thresholds: count >= 3 (vs pair's 5) AND mean > 0.10 (vs
        // pair's 0.05). Triples accumulate slower but each
        // occurrence carries more signal.
        const MIN_TRIPLE_COUNT: u64 = 3;
        const MIN_TRIPLE_MEAN_DELTA: f64 = 0.10;
        let mut to_promote_t: Vec<(String, String, String)> = Vec::new();
        for (triple, count) in
            self.memory.sequence_stats.triple_counts.iter()
        {
            if *count < MIN_TRIPLE_COUNT {
                continue;
            }
            let mean = match self
                .memory
                .sequence_stats
                .triple_mean_post_ep_delta(*triple)
            {
                Some(m) => m,
                None => continue,
            };
            if mean <= MIN_TRIPLE_MEAN_DELTA {
                continue;
            }
            let a_name = action_kind_to_str(triple.0).to_string();
            let b_name = action_kind_to_str(triple.1).to_string();
            let c_name = action_kind_to_str(triple.2).to_string();
            if !self.rset.has_action_sequence_triple(
                &a_name, &b_name, &c_name,
            ) {
                to_promote_t.push((a_name, b_name, c_name));
            }
        }
        for (a, b, c) in to_promote_t {
            let _ = self
                .rset
                .name_action_sequence_triple(&a, &b, &c);
        }
    }

    /// Dispatch a single action. Returns `Some(delta)` if the action
    /// computes its own episode delta (e.g. ADR 0059 G1.5
    /// `EvaluatePredictions`); otherwise `None` and the caller uses
    /// the standard `abstraction_score` diff.
    pub fn execute_action(&mut self, plan: &ActionPlan) -> Option<f64> {
        match plan.action_kind {
            ActionKind::DiscoverTheory => {
                let cfg = AxiomDiscoveryConfig::default();
                let th = self.rset.discover_theory(&cfg);
                if !th.member_axiom_ids.is_empty() {
                    let ids: Vec<&str> = th
                        .member_axiom_ids
                        .iter()
                        .map(|s| s.as_str())
                        .collect();
                    let _ = self.rset.name_theory(&ids);
                }
            }
            ActionKind::DiscoverPatterns => {
                // ADR 0075 piece 2 (revisited 2026-05-06) — scheduler
                // integration with maturity-gated multi-size fallback.
                //
                // Changes from the original ADR 0018 dispatch:
                //
                //   1. rng_seed varies with episode_counter so
                //      successive DP dispatches sample different
                //      subgraphs.
                //
                //   2. sample_count raised from 200 to 400.
                //
                //   3. Explicit positive-delta override (counts
                //      newly-minted patterns rather than relying on
                //      abstraction_score diff which under-counts
                //      1-instance mints).
                //
                //   4. Maturity-gated multi-size fallback: when the
                //      requested size produces zero NewPattern AND
                //      the rset is "mature" (≥ 1 axiom AND
                //      ≥ MATURE_DATA_EDGE_FLOOR data edges), retry
                //      with sizes 4 / 5 / 3 / 2 in order until
                //      something mints. This addresses OQ#1-clade's
                //      issue where dense diamond posets reject
                //      small-size canonicals via `is_clean_subgraph`
                //      while sizes 4-5 wrap whole clusters
                //      successfully (kernel audit empirics).
                //
                //      The maturity gate preserves the
                //      lifecycle-test invariants:
                //      - `a3_resume_runs_full_run_to_completion`
                //        uses a 9-data-edge diamond_poset; gate
                //        blocks fallback so sleep timing matches
                //      - `a1_rule_based_runs_and_sleeps` uses the
                //        same fixture with no axioms initially;
                //        gate blocks fallback so TheoryCandidate
                //        gets dispatched first.
                let initial_size = match plan.target {
                    FrontierTarget::PatternSize(s) => s,
                    _ => 3, // fallback
                };
                let pattern_dispatch =
                    |rt: &mut Self, size: usize| -> usize {
                        let cfg = AutonomousConfig {
                            discovery: DiscoveryConfig {
                                target_size: size,
                                sample_count: 400,
                                top_m: 10,
                                rng_seed: 2024u64
                                    .wrapping_add(rt.episode_counter
                                        .wrapping_mul(0x9E3779B97F4A7C15))
                                    .wrapping_add(size as u64 * 0xCAFEBABE),
                                include_meta_in_discovery: false,
                            },
                            refinement: RefinementConfig {
                                max_tries: 200,
                                rng_seed: 999u64
                                    .wrapping_add(rt.episode_counter
                                        .wrapping_mul(0xDEADBEEFCAFEBABE))
                                    .wrapping_add(size as u64 * 0xFADE),
                            },
                            naming: NamingPolicy::default(),
                            instance_sampling: None,
                        };
                        let outcomes = rt.rset.autonomous_pass(&cfg);
                        outcomes
                            .iter()
                            .filter(|o| matches!(
                                o,
                                crate::AutonomousOutcome::NewPattern { .. }
                            ))
                            .count()
                    };
                let primary_new = pattern_dispatch(self, initial_size);

                // Maturity gate. Empirical floor of 100 data edges
                // matches the kernel-audit observation that mints
                // become reliable at this density.
                const MATURE_DATA_EDGE_FLOOR: usize = 100;
                let mature = self.rset.axioms().len() >= 1
                    && self.rset.iter().count() >= MATURE_DATA_EDGE_FLOOR;

                if primary_new == 0 && mature {
                    // Try fallback sizes in order: 4, 5, 3, 2 (skip
                    // initial). 4/5 first because they wrap whole
                    // clusters and have highest empirical mint rate
                    // on dense rsets.
                    let mut total_new = 0usize;
                    for &fallback in &[4usize, 5, 3, 2] {
                        if fallback == initial_size {
                            continue;
                        }
                        let new = pattern_dispatch(self, fallback);
                        total_new += new;
                        if new > 0 {
                            // Stop scanning once any size succeeds —
                            // bounded cost (≤ 4 sizes per dispatch
                            // total) prevents unbounded work.
                            break;
                        }
                    }
                    if total_new > 0 {
                        return Some(total_new as f64);
                    }
                }

                if primary_new > 0 {
                    return Some(primary_new as f64);
                }
            }
            ActionKind::UpdateTheoryRelations => {
                // Snapshot ids so we can mutate self.rset inside the loop.
                let theories: Vec<String> = self
                    .rset
                    .theories()
                    .into_iter()
                    .map(str::to_owned)
                    .collect();
                for i in 0..theories.len() {
                    for j in (i + 1)..theories.len() {
                        let a = theories[i].clone();
                        let b = theories[j].clone();
                        match self.rset.classify_theory_pair(&a, &b) {
                            Some(TheoryRelationKind::Extends) => {
                                let _ = self.rset.name_theory_extension(&a, &b);
                            }
                            Some(TheoryRelationKind::ExtendedBy) => {
                                let _ = self.rset.name_theory_extension(&b, &a);
                            }
                            Some(TheoryRelationKind::Independent) => {
                                let _ = self.rset.name_theory_independence(&a, &b);
                            }
                            Some(TheoryRelationKind::Parallel) => {
                                let _ = self.rset.name_theory_parallel(&a, &b);
                            }
                            _ => {}
                        }
                    }
                }
            }
            ActionKind::PruneLowValueObjects => {
                // Prune at the object pointed at by the plan, or all
                // negative-CV if `WholeRSet`.
                match &plan.target {
                    FrontierTarget::Pattern(id) => {
                        let _ = self.rset.retract_pattern(id);
                    }
                    FrontierTarget::Theory(id) => {
                        let _ = self.rset.retract_theory(id);
                    }
                    _ => {
                        // Prune all negative-CV named objects.
                        let to_prune: Vec<String> = self
                            .rset
                            .rank_by_counterfactual()
                            .into_iter()
                            .filter(|(_, v)| *v < 0.0)
                            .map(|(id, _)| id)
                            .collect();
                        for id in to_prune {
                            if self.rset.is_theory(&id) {
                                let _ = self.rset.retract_theory(&id);
                            } else if self
                                .rset
                                .patterns()
                                .iter()
                                .any(|p| *p == id.as_str())
                            {
                                let _ = self.rset.retract_pattern(&id);
                            } else if self
                                .rset
                                .extension_edges()
                                .iter()
                                .any(|e| *e == id.as_str())
                            {
                                let _ = self.rset.retract_extension(&id);
                            }
                        }
                    }
                }
            }
            ActionKind::Declarativize => {
                // ADR 0053 / Phases C0–C2. The frontier pass already
                // gated this; the marker is selected by target type:
                // patterns and theories carry ESTABLISHED ("experience-
                // with"); axioms carry SHARED_AXIOM ("structurally
                // referenced by ≥ 2 theories"). `rset.add` is
                // idempotent — duplicate edges return false silently.
                let edge = match &plan.target {
                    FrontierTarget::Pattern(id) => Some(R::new(
                        id.clone(),
                        ESTABLISHED_MARKER,
                    )),
                    FrontierTarget::Theory(id) => Some(R::new(
                        id.clone(),
                        ESTABLISHED_MARKER,
                    )),
                    FrontierTarget::Axiom(id) => Some(R::new(
                        id.clone(),
                        SHARED_AXIOM_MARKER,
                    )),
                    _ => None,
                };
                if let Some(e) = edge {
                    let _ = self.rset.add(e);
                }
            }
            ActionKind::DiscoverMetaMetaPatterns => {
                // ADR 0054 / Phase D0+. Probe the rset's M1 subgraph
                // and (loop closure) name the top novel candidate via
                // an Intensional pattern recording. Intensional means
                // we write the pattern's roles + Layer A structural
                // edges but skip Layer B instance bindings — keeps
                // marker nodes from being pinned as concrete
                // participants. The naming may fail if no clean
                // instance survives `is_clean_subgraph_with_meta_subset`,
                // in which case the action is effectively a no-op.
                let cfg = &self.frontier.meta_meta;
                let markers: Vec<&str> = cfg.markers.clone();
                let subset = self.rset.meta_meta_subset(&markers);
                let dconfig = DiscoveryConfig {
                    target_size: cfg.target_size,
                    sample_count: cfg.sample_count,
                    top_m: cfg.top_m,
                    rng_seed: cfg.rng_seed,
                    include_meta_in_discovery: false,
                };
                let candidates = self
                    .rset
                    .discover_motifs_with_meta_subset(&dconfig, &subset);
                // Walk the top-`top_m` candidates by frequency and
                // name the first novel one with at least one clean
                // instance under the meta-subset view. ADR 0055
                // sharpens canonical resolution, which means the
                // single highest-frequency candidate is now more
                // likely to encode a Y- or path-shape that crosses
                // markers and fails `is_clean_subgraph_with_meta_subset`.
                // The iteration is bounded by `top_m` so the action
                // stays predictable on its budget.
                for candidate in candidates.iter() {
                    if self
                        .rset
                        .find_pattern_matching(&candidate.canonical)
                        .is_some()
                    {
                        continue;
                    }
                    let instances = self
                        .rset
                        .find_instances_of_with_meta_subset(
                            &candidate.canonical,
                            &subset,
                        );
                    if instances.is_empty() {
                        continue;
                    }
                    let _ = self.rset.name_pattern_instances_with_policy(
                        &instances,
                        PatternRecordingPolicy::Intensional,
                    );
                    break;
                }
            }
            ActionKind::EvaluatePredictions => {
                // ADR 0059 / Phase G1.5. Use fresh forward-apply
                // against current rset (not the stored
                // total/verified counters which only update on
                // verify-against-snapshot). The instant rate
                // responds to environmental change — events
                // arriving during sleep are reflected immediately
                // at wake-time. Per-axiom hit-rate delta vs. the
                // previous EP snapshot becomes the episode delta;
                // pure observation, no rset mutation.
                // ADR 0066 Addendum 4 perf fix: amortize.
                let meta = self.rset.collect_meta_ids();
                let data_ids = self.rset.compute_data_ids(&meta);
                let data_edges: HashSet<R> = self
                    .rset
                    .iter()
                    .filter(|r| !meta.contains(&r.x) && !meta.contains(&r.y))
                    .cloned()
                    .collect();
                let mut delta_sum: f64 = 0.0;
                let axioms: Vec<String> = self
                    .rset
                    .axioms()
                    .into_iter()
                    .map(str::to_owned)
                    .collect();
                for ax in axioms {
                    let pred = self
                        .rset
                        .forward_apply_axiom_with_data_ids(&ax, &data_ids);
                    if pred.is_empty() {
                        continue;
                    }
                    let verified =
                        pred.intersection(&data_edges).count();
                    let now =
                        verified as f64 / pred.len() as f64;
                    let prev = self
                        .memory
                        .prediction_state
                        .last_reflect_hit_rate_per_axiom
                        .get(&ax)
                        .copied()
                        .unwrap_or(0.0);
                    delta_sum += now - prev;
                    self.memory
                        .prediction_state
                        .last_reflect_hit_rate_per_axiom
                        .insert(ax, now);
                }
                return Some(delta_sum);
            }
            ActionKind::ExecuteComposite => {
                // ADR 0061 / Phase H1.2 + ADR 0062 / Phase H1.4 —
                // run a promoted action sequence (length 2 or 3)
                // as a single dispatched unit. Look up the seq_id's
                // ordered ActionKinds in rset, find matching
                // frontier items for each step, execute in order.
                // Returns the abstraction-score delta over the
                // entire composite as the episode's delta.
                let seq_id = match &plan.target {
                    FrontierTarget::ActionSequence(id) => id.clone(),
                    _ => return Some(0.0),
                };
                // Collect the step kinds (length 2 or 3).
                let kinds: Vec<ActionKind> = {
                    let mut out: Vec<ActionKind> = Vec::new();
                    let pairs = self.rset.action_sequence_pairs();
                    if let Some((_, p, s)) = pairs
                        .iter()
                        .find(|(id, _, _)| id == &seq_id)
                    {
                        if let (Ok(pk), Ok(sk)) = (
                            parse_action_kind(p),
                            parse_action_kind(s),
                        ) {
                            out.push(pk);
                            out.push(sk);
                        }
                    } else {
                        let triples = self.rset.action_sequence_triples();
                        if let Some((_, a, b, c)) = triples
                            .iter()
                            .find(|(id, _, _, _)| id == &seq_id)
                        {
                            if let (Ok(ak), Ok(bk), Ok(ck)) = (
                                parse_action_kind(a),
                                parse_action_kind(b),
                                parse_action_kind(c),
                            ) {
                                out.push(ak);
                                out.push(bk);
                                out.push(ck);
                            }
                        }
                    }
                    out
                };
                if kinds.is_empty() {
                    return Some(0.0);
                }
                // Snapshot all targets by step kind upfront — frontier
                // could change shape between sub-actions if rset
                // mutates. ADR 0062 retrospective #3 — EP has no
                // FrontierKind; synthesize a `WholeRSet` target so
                // EP-containing composites can actually fire.
                let targets: Vec<Option<FrontierTarget>> = kinds
                    .iter()
                    .map(|k| {
                        if *k == ActionKind::EvaluatePredictions {
                            return Some(FrontierTarget::WholeRSet);
                        }
                        self.frontier
                            .items
                            .iter()
                            .find(|it| {
                                Self::execute_for_kind_static(it.kind)
                                    == *k
                            })
                            .map(|it| it.target.clone())
                    })
                    .collect();
                let before = self.rset.abstraction_score();
                for (kind, target_opt) in
                    kinds.into_iter().zip(targets.into_iter())
                {
                    if let Some(target) = target_opt {
                        let sub = ActionPlan {
                            action_kind: kind,
                            target,
                        };
                        let _ = self.execute_action(&sub);
                    }
                }
                let after = self.rset.abstraction_score();
                return Some(after - before);
            }
            ActionKind::DiscoverAxiomShapeFamilies => {
                // ADR 0068 / Phase Beta-1.5 (Direction B.5).
                // Mint shape families from registered axioms. Pure
                // structural derivation — predicate axioms ignored,
                // empty-premise families excluded, idempotent on
                // already-named families. Episode delta = count of
                // newly minted families.
                let minted = self.rset.discover_axiom_shape_families(2);
                return Some(minted.len() as f64);
            }
            ActionKind::RetractShapeFamily => {
                // ADR 0070 Step 2. Target carries the family id.
                // Episode delta is the count of axioms globally
                // retracted (L2) or member links removed (L3+).
                // Returns 0 (not None) when the family is unknown
                // or retraction errors out — the action consumed
                // a tick but produced no work.
                let family_id = match &plan.target {
                    FrontierTarget::ShapeFamily(id) => id.clone(),
                    _ => return Some(0.0),
                };
                match self.rset.retract_shape_family(&family_id) {
                    Ok(summary) => {
                        let delta = match summary.layer {
                            crate::FamilyLayer::L2 => {
                                summary.axioms_globally_retracted
                            }
                            crate::FamilyLayer::L3
                            | crate::FamilyLayer::L4 => {
                                summary.member_links_removed
                            }
                        };
                        return Some(delta as f64);
                    }
                    Err(_) => return Some(0.0),
                }
            }
            ActionKind::ApplyRecommendedIntervention => {
                // ADR 0082 — re-compute the recommendation at execute
                // time (state may have changed since proposal), then
                // route to the appropriate lib API.
                let theory_id = match &plan.target {
                    FrontierTarget::Theory(id) => id.clone(),
                    _ => return Some(0.0),
                };
                // Recompute primary_rates + reports + recommendation.
                const MIN_AXIOM_PREDICTIONS: u64 = 5;
                let mut primary_rates: std::collections::HashMap<String, f64>
                    = std::collections::HashMap::new();
                for ax in self.rset.axioms() {
                    if let Some(r) = self.memory.prediction_state
                        .hit_rate(ax, MIN_AXIOM_PREDICTIONS)
                    {
                        primary_rates.insert(ax.to_string(), r);
                    }
                }
                let substrates: Vec<RSet> = Vec::new();
                let reports = self.rset
                    .theory_quality_report_all(&substrates, &primary_rates);
                let focal = match reports
                    .iter().find(|r| r.theory_id == theory_id)
                {
                    Some(r) => r.clone(),
                    None => return Some(0.0),
                };
                let others: Vec<crate::TheoryQualityReport> = reports
                    .iter()
                    .filter(|r| r.theory_id != theory_id)
                    .cloned()
                    .collect();
                let rec = RSet::recommend_intervention(&focal, &others);
                // Track the mutation by comparing axiom + theory counts
                // (more reliable than abstraction_score for these
                // structural retractions which can DROP the score
                // even when successful).
                let axioms_before = self.rset.axioms().len();
                let theories_before = self.rset.theories().len();
                use crate::RecommendedIntervention as RI;
                match rec {
                    RI::FamilyDemote { family_id, .. } => {
                        let _ = self.rset.retract_shape_family(&family_id);
                    }
                    RI::AxiomRepair { axiom_ids } => {
                        for ax in &axiom_ids {
                            let _ = self.rset.retract_theory_member(
                                &theory_id, ax,
                            );
                        }
                    }
                    RI::TheoryDemote { .. } |
                    RI::DemoteSuperset { .. } => {
                        let _ = self.rset.retract_theory(&theory_id);
                    }
                    RI::Merge { partner_theory, .. } => {
                        let _ = self.rset.merge_theories(
                            &theory_id, &partner_theory,
                        );
                    }
                    RI::None | RI::ShadowMonitor { .. } | RI::Manual { .. } => {
                        // Recommendation flipped to no-op variant
                        // between propose and execute. Episode is
                        // recorded with delta=0.
                    }
                }
                let axioms_after = self.rset.axioms().len();
                let theories_after = self.rset.theories().len();
                let delta = ((axioms_before as i64) - (axioms_after as i64))
                    .abs() as f64
                    + ((theories_before as i64) - (theories_after as i64))
                    .abs() as f64;
                return Some(delta);
            }
        }
        None
    }

    /// Standalone version of `RuleBasedScheduler::execute_for_kind`
    /// callable from `AutonomousRuntime::execute_action` without a
    /// scheduler reference. Mirrors the trait-method body.
    fn execute_for_kind_static(kind: FrontierKind) -> ActionKind {
        RuleBasedScheduler::execute_for_kind(kind)
    }

    // ─── A3: checkpoint round-trip ─────────────────────────────────

    /// Serialize the runtime's mutable state into a hand-rolled
    /// section-based text format. Mirrors `RSet::to_text`'s TSV style
    /// (ADR 0038). Does NOT serialize scheduler / environment / frontier
    /// — those are behavior or rederivable. ADR 0052 / A3.
    ///
    /// Format (sections in fixed order, blank line between sections):
    ///
    /// ```text
    /// # v2 runtime checkpoint v1
    /// [meta]
    /// tick<TAB>N
    /// episode_counter<TAB>N
    /// steps_since_last_gain<TAB>N
    /// current_score<TAB>F
    /// lifecycle<TAB>Running|Sleeping|Stopped|Booting
    /// mode<TAB>Expand|Consolidate|Reflect
    /// max_episodes<TAB>N
    /// max_mode_transitions<TAB>N
    /// max_lifecycle_transitions<TAB>N
    /// actions_per_tick_cap<TAB>N
    ///
    /// [rset]
    /// <RSet::to_text() output>
    ///
    /// [episodes]
    /// id<TAB>tick<TAB>mode<TAB>action<TAB>tgt_kind<TAB>tgt_value<TAB>before<TAB>after<TAB>delta
    ///
    /// [mode_transitions]
    /// tick<TAB>from<TAB>to<TAB>reason
    ///
    /// [lifecycle_transitions]
    /// tick<TAB>from<TAB>to<TAB>reason
    /// ```
    pub fn checkpoint_text(&self) -> Result<String, String> {
        let mut out = String::new();
        out.push_str("# v2 runtime checkpoint v1\n");

        // [meta]
        out.push_str("[meta]\n");
        out.push_str(&format!("tick\t{}\n", self.tick));
        out.push_str(&format!("episode_counter\t{}\n", self.episode_counter));
        out.push_str(&format!(
            "steps_since_last_gain\t{}\n",
            self.steps_since_last_gain
        ));
        out.push_str(&format!("current_score\t{:?}\n", self.current_score));
        out.push_str(&format!(
            "lifecycle\t{}\n",
            lifecycle_to_str(self.lifecycle)
        ));
        out.push_str(&format!("mode\t{}\n", mode_to_str(self.mode)));
        out.push_str(&format!(
            "max_episodes\t{}\n",
            self.memory.max_episodes
        ));
        out.push_str(&format!(
            "max_mode_transitions\t{}\n",
            self.memory.max_mode_transitions
        ));
        out.push_str(&format!(
            "max_lifecycle_transitions\t{}\n",
            self.memory.max_lifecycle_transitions
        ));
        out.push_str(&format!(
            "actions_per_tick_cap\t{}\n",
            self.budget.actions_per_tick_cap
        ));
        out.push('\n');

        // [rset]
        out.push_str("[rset]\n");
        let rset_text = self
            .rset
            .to_text()
            .map_err(|e| format!("rset serialization failed: {:?}", e))?;
        out.push_str(&rset_text);
        if !rset_text.ends_with('\n') {
            out.push('\n');
        }
        out.push('\n');

        // [episodes]
        out.push_str("[episodes]\n");
        for ep in &self.memory.episodes {
            check_no_tab_or_newline(&ep.target, "episode target")?;
            let (tk, tv) = target_to_pair(&ep.target);
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{:?}\t{:?}\t{:?}\n",
                ep.id,
                ep.tick,
                mode_to_str(ep.mode),
                action_kind_to_str(ep.action_kind),
                tk,
                tv,
                ep.score_before,
                ep.score_after,
                ep.delta,
            ));
        }
        out.push('\n');

        // [mode_transitions]
        out.push_str("[mode_transitions]\n");
        for mt in &self.memory.mode_transitions {
            check_reason(&mt.reason, "mode_transition")?;
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\n",
                mt.tick,
                mode_to_str(mt.from),
                mode_to_str(mt.to),
                mt.reason,
            ));
        }
        out.push('\n');

        // [lifecycle_transitions]
        out.push_str("[lifecycle_transitions]\n");
        for lt in &self.memory.lifecycle_transitions {
            check_reason(&lt.reason, "lifecycle_transition")?;
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\n",
                lt.tick,
                lifecycle_to_str(lt.from),
                lifecycle_to_str(lt.to),
                lt.reason,
            ));
        }
        out.push('\n');

        // B2 — object_history sections (sorted by id for idempotency).
        write_history_section(
            &mut out,
            "[object_history_patterns]",
            &self.memory.object_history.patterns,
        )?;
        out.push('\n');
        write_history_section(
            &mut out,
            "[object_history_axioms]",
            &self.memory.object_history.axioms,
        )?;
        out.push('\n');
        write_history_section(
            &mut out,
            "[object_history_theories]",
            &self.memory.object_history.theories,
        )?;
        out.push('\n');

        // B2 — policy_stats sections.
        out.push_str("[policy_stats_action_counts]\n");
        let mut action_keys: Vec<&ActionKind> =
            self.memory.policy_stats.action_counts.keys().collect();
        // Also include keys present only in positive_delta_counts.
        for k in self.memory.policy_stats.action_positive_delta_counts.keys() {
            if !action_keys.contains(&k) {
                action_keys.push(k);
            }
        }
        action_keys.sort_by_key(|a| action_kind_to_str(**a));
        for k in action_keys {
            let total = self
                .memory
                .policy_stats
                .action_counts
                .get(k)
                .copied()
                .unwrap_or(0);
            let pos = self
                .memory
                .policy_stats
                .action_positive_delta_counts
                .get(k)
                .copied()
                .unwrap_or(0);
            out.push_str(&format!(
                "{}\t{}\t{}\n",
                action_kind_to_str(*k),
                total,
                pos
            ));
        }
        out.push('\n');

        out.push_str("[policy_stats_mode_transition_counts]\n");
        let mut mtc_keys: Vec<&(RuntimeMode, RuntimeMode)> = self
            .memory
            .policy_stats
            .mode_transition_counts
            .keys()
            .collect();
        mtc_keys.sort_by_key(|(f, t)| (mode_to_str(*f), mode_to_str(*t)));
        for k in mtc_keys {
            let n = self
                .memory
                .policy_stats
                .mode_transition_counts
                .get(k)
                .copied()
                .unwrap_or(0);
            out.push_str(&format!(
                "{}\t{}\t{}\n",
                mode_to_str(k.0),
                mode_to_str(k.1),
                n
            ));
        }
        out.push('\n');

        out.push_str("[policy_stats_lifecycle_counts]\n");
        out.push_str(&format!(
            "wake\t{}\n",
            self.memory.policy_stats.wake_count
        ));
        out.push_str(&format!(
            "sleep\t{}\n",
            self.memory.policy_stats.sleep_count
        ));
        out.push_str(&format!(
            "stop\t{}\n",
            self.memory.policy_stats.stop_count
        ));
        out.push('\n');

        // ADR 0059 / G1.3 — prediction-state cumulative counters.
        // Per-axiom rows: <axiom_id>\t<total>\t<verified>\t<last_reflect_hit_rate>.
        // The transient `last_predicted_per_axiom` snapshot is NOT
        // serialized — it regenerates on the first post-restore tick.
        out.push_str("[prediction_state]\n");
        let ps = &self.memory.prediction_state;
        let mut axiom_keys: std::collections::BTreeSet<&str> =
            std::collections::BTreeSet::new();
        for k in ps.total_predictions_per_axiom.keys() {
            axiom_keys.insert(k.as_str());
        }
        for k in ps.verified_predictions_per_axiom.keys() {
            axiom_keys.insert(k.as_str());
        }
        for k in ps.last_reflect_hit_rate_per_axiom.keys() {
            axiom_keys.insert(k.as_str());
        }
        for ax in axiom_keys {
            if ax.contains('\t') || ax.contains('\n') {
                return Err(format!(
                    "prediction_state axiom id '{}' contains tab or newline",
                    ax
                ));
            }
            let total = ps
                .total_predictions_per_axiom
                .get(ax)
                .copied()
                .unwrap_or(0);
            let verified = ps
                .verified_predictions_per_axiom
                .get(ax)
                .copied()
                .unwrap_or(0);
            let last_rate = ps
                .last_reflect_hit_rate_per_axiom
                .get(ax)
                .copied()
                .unwrap_or(0.0);
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\n",
                ax, total, verified, last_rate
            ));
        }
        out.push('\n');

        // ADR 0061 / H1.0 — sequence-stats accounting.
        // Per-pair rows: <a_kind>\t<b_kind>\t<count>\t<post_ep_count>\t<post_ep_delta_sum>.
        out.push_str("[sequence_stats]\n");
        let ss = &self.memory.sequence_stats;
        let mut pair_keys: HashSet<(ActionKind, ActionKind)> =
            HashSet::new();
        for k in ss.pair_counts.keys() {
            pair_keys.insert(*k);
        }
        for k in ss.pair_post_ep_count.keys() {
            pair_keys.insert(*k);
        }
        for k in ss.pair_post_ep_delta_sum.keys() {
            pair_keys.insert(*k);
        }
        let mut pair_list: Vec<(ActionKind, ActionKind)> =
            pair_keys.into_iter().collect();
        pair_list.sort_by_key(|(a, b)| {
            (action_kind_to_str(*a), action_kind_to_str(*b))
        });
        for (a, b) in pair_list {
            let count = ss.pair_counts.get(&(a, b)).copied().unwrap_or(0);
            let post_count = ss
                .pair_post_ep_count
                .get(&(a, b))
                .copied()
                .unwrap_or(0);
            let post_sum = ss
                .pair_post_ep_delta_sum
                .get(&(a, b))
                .copied()
                .unwrap_or(0.0);
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\n",
                action_kind_to_str(a),
                action_kind_to_str(b),
                count,
                post_count,
                post_sum,
            ));
        }
        out.push('\n');

        // ADR 0063 / Phase H2.0 step 2 — DriveMix A/B state.
        // Format: K/V lines (mirroring [meta]). Weight entries use
        // `<candidate>:<drive_id>\t<weight>` keys. State, window,
        // counters, and rng come as their own keys.
        out.push_str("[drive_mix]\n");
        let dm = &self.drive_mix;
        out.push_str(&format!(
            "state\t{}\n",
            match dm.state {
                DriveABState::TestingA => "TestingA",
                DriveABState::TestingB => "TestingB",
            }
        ));
        out.push_str(&format!("window_size\t{}\n", dm.window_size));
        out.push_str(&format!(
            "stage_start_episode_count\t{}\n",
            dm.stage_start_episode_count
        ));
        out.push_str(&format!(
            "last_completed_a_mean\t{}\n",
            match dm.last_completed_a_mean {
                Some(v) => format!("{:?}", v),
                None => "NONE".to_string(),
            }
        ));
        out.push_str(&format!("rng_state\t{}\n", dm.rng_state));
        let mut a_keys: Vec<&String> = dm.candidate_a.keys().collect();
        a_keys.sort();
        for k in a_keys {
            if k.contains('\t') || k.contains('\n') {
                return Err(format!(
                    "drive_mix.candidate_a key '{}' contains tab or newline",
                    k
                ));
            }
            let v = dm.candidate_a.get(k).copied().unwrap_or(0.0);
            out.push_str(&format!("candidate_a:{}\t{:?}\n", k, v));
        }
        let mut b_keys: Vec<&String> = dm.candidate_b.keys().collect();
        b_keys.sort();
        for k in b_keys {
            if k.contains('\t') || k.contains('\n') {
                return Err(format!(
                    "drive_mix.candidate_b key '{}' contains tab or newline",
                    k
                ));
            }
            let v = dm.candidate_b.get(k).copied().unwrap_or(0.0);
            out.push_str(&format!("candidate_b:{}\t{:?}\n", k, v));
        }

        Ok(out)
    }

    /// Reverse of `checkpoint_text`. Returns a runtime with default
    /// `StubScheduler` + `NoOpEnvironment`; caller swaps these in
    /// before calling `run_bounded`. Frontier starts dirty (empty
    /// items) and is rebuilt on the next tick. ADR 0052 / A3.
    pub fn from_checkpoint_text(text: &str) -> Result<Self, String> {
        let parsed = parse_checkpoint(text)?;

        // Rebuild rset from its dedicated section.
        let rset_blob = parsed.rset_lines.join("\n");
        let rset = RSet::from_text(&rset_blob)
            .map_err(|e| format!("rset parse failed: {:?}", e))?;

        // Pull required meta fields.
        let meta = &parsed.meta;
        let get = |k: &str| -> Result<&String, String> {
            meta.get(k).ok_or_else(|| format!("missing meta key '{}'", k))
        };
        let tick = parse_u64(get("tick")?, "tick")?;
        let episode_counter = parse_u64(get("episode_counter")?, "episode_counter")?;
        let steps_since_last_gain =
            parse_u64(get("steps_since_last_gain")?, "steps_since_last_gain")?;
        let current_score = parse_f64(get("current_score")?, "current_score")?;
        let lifecycle = parse_lifecycle(get("lifecycle")?)?;
        let mode = parse_mode(get("mode")?)?;
        let max_episodes =
            parse_usize(get("max_episodes")?, "max_episodes")?;
        let max_mode_transitions =
            parse_usize(get("max_mode_transitions")?, "max_mode_transitions")?;
        let max_lifecycle_transitions = parse_usize(
            get("max_lifecycle_transitions")?,
            "max_lifecycle_transitions",
        )?;
        let actions_per_tick_cap = parse_u32(
            get("actions_per_tick_cap")?,
            "actions_per_tick_cap",
        )?;

        // Episodes.
        let mut episodes: VecDeque<Episode> = VecDeque::new();
        for (idx, raw) in parsed.episode_lines.iter().enumerate() {
            let fields: Vec<&str> = raw.split('\t').collect();
            if fields.len() != 9 {
                return Err(format!(
                    "episode line {} has {} fields, expected 9",
                    idx + 1,
                    fields.len()
                ));
            }
            let target = pair_to_target(fields[4], fields[5])?;
            episodes.push_back(Episode {
                id: parse_u64(fields[0], "episode.id")?,
                tick: parse_u64(fields[1], "episode.tick")?,
                mode: parse_mode(fields[2])?,
                action_kind: parse_action_kind(fields[3])?,
                target,
                score_before: parse_f64(fields[6], "episode.score_before")?,
                score_after: parse_f64(fields[7], "episode.score_after")?,
                delta: parse_f64(fields[8], "episode.delta")?,
            });
        }

        // Mode transitions.
        let mut mode_transitions: VecDeque<ModeTransition> = VecDeque::new();
        for (idx, raw) in parsed.mode_transition_lines.iter().enumerate() {
            let fields: Vec<&str> = raw.splitn(4, '\t').collect();
            if fields.len() != 4 {
                return Err(format!(
                    "mode_transition line {} has {} fields, expected 4",
                    idx + 1,
                    fields.len()
                ));
            }
            mode_transitions.push_back(ModeTransition {
                tick: parse_u64(fields[0], "mode_transition.tick")?,
                from: parse_mode(fields[1])?,
                to: parse_mode(fields[2])?,
                reason: fields[3].to_string(),
            });
        }

        // Lifecycle transitions.
        let mut lifecycle_transitions: VecDeque<LifecycleTransition> =
            VecDeque::new();
        for (idx, raw) in parsed.lifecycle_transition_lines.iter().enumerate() {
            let fields: Vec<&str> = raw.splitn(4, '\t').collect();
            if fields.len() != 4 {
                return Err(format!(
                    "lifecycle_transition line {} has {} fields, expected 4",
                    idx + 1,
                    fields.len()
                ));
            }
            lifecycle_transitions.push_back(LifecycleTransition {
                tick: parse_u64(fields[0], "lifecycle_transition.tick")?,
                from: parse_lifecycle(fields[1])?,
                to: parse_lifecycle(fields[2])?,
                reason: fields[3].to_string(),
            });
        }

        // B2 — object history.
        let object_history = ObjectHistoryStore {
            patterns: parse_history_lines(
                &parsed.history_patterns_lines,
                "object_history_patterns",
            )?,
            axioms: parse_history_lines(
                &parsed.history_axioms_lines,
                "object_history_axioms",
            )?,
            theories: parse_history_lines(
                &parsed.history_theories_lines,
                "object_history_theories",
            )?,
        };

        // B2 — policy stats.
        let mut policy_stats = PolicyStats::default();
        for (idx, raw) in parsed.action_count_lines.iter().enumerate() {
            let fields: Vec<&str> = raw.splitn(3, '\t').collect();
            if fields.len() != 3 {
                return Err(format!(
                    "action_count line {} has {} fields, expected 3",
                    idx + 1,
                    fields.len()
                ));
            }
            let kind = parse_action_kind(fields[0])?;
            let total = parse_u64(fields[1], "action_count.total")?;
            let pos = parse_u64(fields[2], "action_count.positive")?;
            if total > 0 {
                policy_stats.action_counts.insert(kind, total);
            }
            if pos > 0 {
                policy_stats.action_positive_delta_counts.insert(kind, pos);
            }
        }
        for (idx, raw) in parsed.mode_transition_count_lines.iter().enumerate() {
            let fields: Vec<&str> = raw.splitn(3, '\t').collect();
            if fields.len() != 3 {
                return Err(format!(
                    "mode_transition_count line {} has {} fields, expected 3",
                    idx + 1,
                    fields.len()
                ));
            }
            let from = parse_mode(fields[0])?;
            let to = parse_mode(fields[1])?;
            let n = parse_u64(fields[2], "mode_transition_count.n")?;
            if n > 0 {
                policy_stats
                    .mode_transition_counts
                    .insert((from, to), n);
            }
        }
        for (idx, raw) in parsed.lifecycle_count_lines.iter().enumerate() {
            let (k, v) = raw.split_once('\t').ok_or_else(|| {
                format!(
                    "lifecycle_count line {} not key<TAB>value: '{}'",
                    idx + 1,
                    raw
                )
            })?;
            let n = parse_u64(v, "lifecycle_count.value")?;
            match k {
                "wake" => policy_stats.wake_count = n,
                "sleep" => policy_stats.sleep_count = n,
                "stop" => policy_stats.stop_count = n,
                other => {
                    return Err(format!(
                        "unknown lifecycle_count key '{}' (line {})",
                        other,
                        idx + 1
                    ))
                }
            }
        }

        // ADR 0059 / G1.3 — restore prediction-state cumulative
        // counters. last_predicted_per_axiom intentionally stays
        // empty; it regenerates on the first post-restore Running
        // tick.
        let mut prediction_state = PredictionState::default();
        for (idx, raw) in parsed.prediction_state_lines.iter().enumerate() {
            let fields: Vec<&str> = raw.split('\t').collect();
            if fields.len() != 4 {
                return Err(format!(
                    "prediction_state line {} has {} fields, expected 4",
                    idx + 1,
                    fields.len()
                ));
            }
            let ax = fields[0].to_string();
            let total = parse_u64(fields[1], "prediction_state.total")?;
            let verified =
                parse_u64(fields[2], "prediction_state.verified")?;
            let last_rate = fields[3]
                .parse::<f64>()
                .map_err(|e| format!(
                    "prediction_state.last_rate parse '{}' failed: {}",
                    fields[3], e
                ))?;
            if total > 0 {
                prediction_state
                    .total_predictions_per_axiom
                    .insert(ax.clone(), total);
            }
            if verified > 0 {
                prediction_state
                    .verified_predictions_per_axiom
                    .insert(ax.clone(), verified);
            }
            if last_rate.abs() > f64::EPSILON {
                prediction_state
                    .last_reflect_hit_rate_per_axiom
                    .insert(ax, last_rate);
            }
        }

        // ADR 0061 / H1.0 — restore sequence-stats from checkpoint.
        let mut sequence_stats = SequenceStats::default();
        for (idx, raw) in parsed.sequence_stats_lines.iter().enumerate() {
            let fields: Vec<&str> = raw.split('\t').collect();
            if fields.len() != 5 {
                return Err(format!(
                    "sequence_stats line {} has {} fields, expected 5",
                    idx + 1,
                    fields.len()
                ));
            }
            let a = parse_action_kind(fields[0])?;
            let b = parse_action_kind(fields[1])?;
            let count = parse_u64(fields[2], "sequence_stats.count")?;
            let post_count =
                parse_u64(fields[3], "sequence_stats.post_count")?;
            let post_sum = fields[4]
                .parse::<f64>()
                .map_err(|e| format!(
                    "sequence_stats.post_sum parse '{}' failed: {}",
                    fields[4], e
                ))?;
            if count > 0 {
                sequence_stats.pair_counts.insert((a, b), count);
            }
            if post_count > 0 {
                sequence_stats
                    .pair_post_ep_count
                    .insert((a, b), post_count);
            }
            if post_sum.abs() > f64::EPSILON {
                sequence_stats
                    .pair_post_ep_delta_sum
                    .insert((a, b), post_sum);
            }
        }

        let memory = Memory {
            episodes,
            mode_transitions,
            lifecycle_transitions,
            max_episodes,
            max_mode_transitions,
            max_lifecycle_transitions,
            object_history,
            policy_stats,
            prediction_state,
            sequence_stats,
        };

        // ADR 0063 / H2.0 step 2 — DriveMix round-trip.
        // Parses the K/V lines emitted by checkpoint_text. Missing
        // section → DriveMix::default() (graceful for older
        // checkpoints).
        let drive_mix = if parsed.drive_mix_lines.is_empty() {
            DriveMix::default()
        } else {
            let mut state = DriveABState::TestingA;
            let mut window_size: u64 = 50;
            let mut stage_start: u64 = 0;
            let mut last_a: Option<f64> = None;
            let mut rng_state: u64 = 0xc0ffee_dead_beef_u64;
            let mut cand_a: HashMap<String, f64> = HashMap::new();
            let mut cand_b: HashMap<String, f64> = HashMap::new();
            for raw in &parsed.drive_mix_lines {
                let (k, v) = raw.split_once('\t').ok_or_else(|| {
                    format!("drive_mix line not key<TAB>value: '{}'", raw)
                })?;
                if let Some(drive_id) = k.strip_prefix("candidate_a:") {
                    let parsed_v = v.parse::<f64>().map_err(|e| {
                        format!(
                            "drive_mix candidate_a:{} value parse '{}' failed: {}",
                            drive_id, v, e
                        )
                    })?;
                    cand_a.insert(drive_id.to_string(), parsed_v);
                } else if let Some(drive_id) = k.strip_prefix("candidate_b:") {
                    let parsed_v = v.parse::<f64>().map_err(|e| {
                        format!(
                            "drive_mix candidate_b:{} value parse '{}' failed: {}",
                            drive_id, v, e
                        )
                    })?;
                    cand_b.insert(drive_id.to_string(), parsed_v);
                } else {
                    match k {
                        "state" => {
                            state = match v {
                                "TestingA" => DriveABState::TestingA,
                                "TestingB" => DriveABState::TestingB,
                                other => {
                                    return Err(format!(
                                        "drive_mix.state unknown: '{}'",
                                        other
                                    ))
                                }
                            };
                        }
                        "window_size" => {
                            window_size =
                                parse_u64(v, "drive_mix.window_size")?;
                        }
                        "stage_start_episode_count" => {
                            stage_start = parse_u64(
                                v,
                                "drive_mix.stage_start_episode_count",
                            )?;
                        }
                        "last_completed_a_mean" => {
                            last_a = if v == "NONE" {
                                None
                            } else {
                                Some(v.parse::<f64>().map_err(|e| {
                                    format!(
                                        "drive_mix.last_completed_a_mean parse '{}' failed: {}",
                                        v, e
                                    )
                                })?)
                            };
                        }
                        "rng_state" => {
                            rng_state =
                                parse_u64(v, "drive_mix.rng_state")?;
                        }
                        other => {
                            return Err(format!(
                                "drive_mix unknown key '{}'",
                                other
                            ))
                        }
                    }
                }
            }
            DriveMix {
                candidate_a: cand_a,
                candidate_b: cand_b,
                state,
                window_size,
                stage_start_episode_count: stage_start,
                last_completed_a_mean: last_a,
                rng_state,
            }
        };

        let drives: Vec<Box<dyn Drive>> = vec![
            Box::new(CompressionDrive),
            Box::new(PredictionErrorDrive),
            Box::new(ModeThrashPenalty),
        ];
        let mut rt = Self {
            rset,
            lifecycle,
            mode,
            memory,
            scheduler: Box::new(StubScheduler),
            environment: Box::new(NoOpEnvironment),
            frontier: Frontier::default(),
            tick,
            episode_counter,
            steps_since_last_gain,
            budget: BudgetState::new(actions_per_tick_cap),
            current_score,
            last_checkpoint: None,
            drive_mix,
            drives,
        };
        // ADR 0064 / Phase H2.1.0 — re-register drives in rset.
        // Idempotent: edges already present from the checkpoint
        // restore will be no-op'd by RSet::add.
        rt.register_drives_in_rset();
        Ok(rt)
    }
}

