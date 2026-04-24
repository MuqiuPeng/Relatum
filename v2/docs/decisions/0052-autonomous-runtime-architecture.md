# 0052: Autonomous runtime architecture

Status: Accepted (design; Phase A0 implemented)
Date: 2026-04-24

## Context

v2 reached 51 ADRs and 282 tests with a clear shape: 11 layers of
capability (observation → abstraction → type naming → rule
discovery → subsumption → theory objects → theory relations →
drive → adaptive config), zero external dependencies, and all
five ontological commitments preserved. The honest-clinic list of
what v2 *can't* do, repeated across several round-summary
reports, reduces to a single architectural gap:

- **No memory** — each `intrinsic_drive` call starts from zero.
- **No meta-drive** — the system doesn't decide *when* to run.
- **Drive calls are independent** — no policy accumulates.
- **Can't decide whether to wake** — external code triggers
  everything.

All four symptoms are the same: v2 is **a library of functions**,
not **an agent**. Fixing any one of them in isolation creates
friction (a drive that remembers but doesn't sleep; a sleep
condition with no memory to judge against). They have to land
together.

This ADR proposes that landing: a **runtime layer** that holds
long-running state, decides what to do, records what happened,
sleeps when idle, and wakes on events. Not a new Relatum, not a
new discovery algorithm — a control loop that uses every existing
v2 mechanism as an execution primitive.

## Decision

### Option chosen: **v2 + runtime module**, not v3

Add a `runtime` module to v2. The existing library API (RSet,
discover_axioms, autonomous_pass, intrinsic_drive, ...) stays
intact and usable as before. Runtime wraps the library in a
perpetual control loop without replacing it.

**Rejected alternative**: start a new v3 crate. Rationale:

- v2's semantic layer is mature; runtime is *glue*, not a new
  universe. Moving the semantic layer to a new crate would
  duplicate code for no gain.
- Caller-level comparison ("with runtime vs. without") is much
  easier if both modes coexist in one crate.
- "v3" as a label is easy to over-commit to; a new module is
  honest about the actual scope.
- ADR numbering stays continuous (0052+), preserving the
  traceable history.

If the runtime layer stabilizes and becomes the dominant usage
mode, a later ADR can rename the crate to v3 at no semantic cost.

### Five modules

Edges drawn deliberately; each knows as little as it can.

1. **`runtime`** — holds lifecycle, mode, tick, budget. Drives
   the main loop. Knows nothing about discovery algorithms.
2. **`scheduler`** — given current state, produces an
   `ActionPlan` or a `ModeChange`. Policy is pluggable.
3. **`memory`** — episodic memory, object histories, policy
   stats, plus a cache. Pure Rust structs in the M0 layer.
4. **`environment`** — source of external events and ticks.
   Starts with two trivial implementations: `NoOp` and
   `SyntheticStream`.
5. **`evaluation`** (cross-cutting) — delta summaries, runtime
   score snapshots, object-level counterfactual reads. Wraps
   existing `abstraction_score` + `counterfactual_value` + Wilson
   / null-baseline into one evaluation façade consumed by
   scheduler and runtime.

Interfaces:

```
World knowledge:      RSet (existing, unchanged)
Operational memory:   Memory
Action selection:     Scheduler → ActionPlan | ModeChange
Lifecycle:            AutonomousRuntime
External stimulus:    Environment (trait)
Evaluation façade:    Evaluator (consumes above, produces scalars)
```

## Module boundaries

**runtime** can call: scheduler, memory (read+write), evaluator,
environment (poll), RSet mutators via ActionExecutor (not
directly).

**scheduler** can call: evaluator (read), memory (read),
RSet (read, never write). Never pokes RSet; never mutates memory
outside its own policy-stats updates.

**memory** is self-contained. Exposes query APIs. Writes come
only from runtime after an episode finalizes.

**environment** is strictly upstream — polled for events,
never called into from inside the loop's body.

**evaluator** is stateless; reads RSet + memory + a `DeltaSummary`
in flight.

Violations (e.g., scheduler mutating RSet) should be caught at
review; Rust's borrow rules will help enforce some of it.

## Data shapes (concept-level)

### AutonomousRuntime

```rust
pub struct AutonomousRuntime {
    rset: RSet,
    lifecycle: LifecycleState,   // Booting | Running | Sleeping | Stopped
    mode: RuntimeMode,           // Expand | Consolidate | Reflect

    scheduler: Box<dyn Scheduler>,
    memory: Memory,
    frontier: Frontier,
    environment: Box<dyn Environment>,
    evaluator: Evaluator,

    tick: u64,
    episode_counter: u64,
    steps_since_last_gain: u64,
    budget: BudgetState,

    current_score: RuntimeScoreSnapshot,
    recent_deltas: VecDeque<DeltaSummary>,   // bounded, e.g. last 20

    last_checkpoint_tick: u64,
    config: RuntimeConfig,
}
```

`LifecycleState` is the **macro state** (alive/asleep/dead).
`RuntimeMode` is the **micro state** (what kind of work right
now). Sleep is a lifecycle state; Reflect is a mode. Keeping
them separate avoids the trap of "Sleep mode but still running".

### BudgetState

Two-dimensional budget by default; nothing fancy.

```rust
pub struct BudgetState {
    pub ticks_remaining: Option<u64>,          // None = unbounded
    pub actions_remaining_this_tick: u32,      // decremented per action
    pub actions_per_tick_cap: u32,             // policy reset
}
```

No wall-clock — unreliable across machines. Step counts are
deterministic and test-friendly. A future ADR could add
wall-clock as an optional second gate.

### Memory (M0 layer)

```rust
pub struct Memory {
    pub episodic: EpisodicMemory,
    pub object_history: ObjectHistoryStore,
    pub policy_stats: PolicyStats,
    pub cache: RuntimeCache,
}

pub struct EpisodicMemory {
    pub recent: VecDeque<Episode>,              // bounded, e.g. 1000
    pub aggregate: AggregateEpisodeStats,
}

pub struct ObjectHistoryStore {
    pub patterns: HashMap<String, ObjectHistory>,
    pub axioms:   HashMap<String, ObjectHistory>,
    pub theories: HashMap<String, ObjectHistory>,
}

pub struct ObjectHistory {
    pub first_seen_tick: u64,
    pub last_seen_tick: u64,
    pub last_improved_tick: Option<u64>,
    pub times_selected_as_focus: u32,
    pub times_retained: u32,
    pub times_pruned: u32,
    pub stability_estimate: f64,   // rolling; 0 on first observation
    pub last_counterfactual_value: Option<f64>,
}

pub struct PolicyStats {
    pub action_stats_by_regime: HashMap<RegimeKey, ActionStats>,
    pub mode_transition_stats: ModeTransitionStats,
    pub wake_sleep_stats: WakeSleepStats,
}
```

### Frontier

```rust
pub struct Frontier {
    pub items: Vec<FrontierItem>,
    pub last_full_refresh_tick: u64,
    pub dirty_regions: HashSet<RegionKey>,  // for incremental refresh
}

pub struct FrontierItem {
    pub id: FrontierItemId,
    pub kind: FrontierKind,
    pub target: FrontierTarget,

    pub priority: f64,
    pub estimated_value: f64,
    pub estimated_cost: f64,
    pub novelty_score: f64,

    pub first_seen_tick: u64,
    pub last_visited_tick: Option<u64>,
    pub revisit_count: u32,
    pub cooldown_until_tick: Option<u64>,

    pub status: FrontierStatus,   // Fresh | Active | Cooling | Saturated | Blocked
}

pub enum FrontierKind {
    LocalRegion,
    PatternCandidate,
    TheoryCandidate,
    AxiomCluster,
    LowValueObjectForPrune,
    RecentlyChangedNeighborhood,
}
```

### Episode

```rust
pub struct Episode {
    pub id: u64,
    pub tick: u64,
    pub mode: RuntimeMode,
    pub selected_action: ActionPlan,
    pub frontier_snapshot: FrontierSummary,
    pub pre_score: RuntimeScoreSnapshot,
    pub outcome: EpisodeOutcome,
    pub post_score: RuntimeScoreSnapshot,
    pub delta: DeltaSummary,
    pub cost: CostReport,
}

pub struct DeltaSummary {
    pub score_delta: f64,
    pub new_objects: u32,
    pub pruned_objects: u32,
    pub theory_relation_changes: u32,
    pub novelty_delta: f64,
    pub stability_change: f64,
}
```

### Scheduler output

Scheduler returns **either** an action-to-execute or a
mode-change instruction; not all actions are RSet mutations.

```rust
pub trait Scheduler {
    fn choose(
        &mut self,
        ctx: &SchedulerContext<'_>,
    ) -> SchedulerDecision;
}

pub enum SchedulerDecision {
    Execute(ActionPlan),
    SwitchMode(RuntimeMode),
    Sleep,
    Stop,
}

pub struct ActionPlan {
    pub action_kind: ActionKind,
    pub target: FrontierTarget,
    pub expected_value: f64,
    pub expected_cost: f64,
    pub rationale: String,
}

pub enum ActionKind {
    DiscoverPatterns,
    DiscoverTheory,
    MinimizeAxioms,
    UpdateTheoryRelations,
    PruneLowValueObjects,
    RefreshFrontier,
}
```

Sleep and Reflect are lifted from "actions" to
`SchedulerDecision` variants: they change the loop's shape, not
the knowledge state.

## Main loop

```text
fn boot():
    rset      = load_or_new()
    memory    = load_or_new()
    frontier  = Frontier::refresh_from(rset, memory)
    current   = evaluator.snapshot(rset, memory)
    mode      = RuntimeMode::Expand
    lifecycle = Running

fn main_loop():
    while lifecycle != Stopped:
        tick += 1
        budget.reset_per_tick()

        // 1. Ingest
        events = environment.poll()
        if events.nonempty():
            apply_events(rset, events)
            frontier.mark_dirty(regions_of(events))
            maybe_wake(lifecycle, events)

        // 2. Sleeping short-circuit
        if lifecycle == Sleeping:
            if should_wake(frontier, memory, events, recent_deltas):
                lifecycle = Running
                mode = choose_wake_mode(frontier, memory)
            else:
                maybe_checkpoint(); continue

        // 3. Frontier maintenance
        if should_refresh_frontier(tick, frontier, recent_deltas):
            frontier.refresh(rset, memory, events_since_last)

        // 4. Inner action budget
        while budget.actions_remaining_this_tick > 0:
            decision = scheduler.choose(ctx(rset, frontier, memory,
                                            mode, budget, current))
            match decision:
                Execute(plan):
                    episode = begin_episode(tick, mode, plan, frontier, current)
                    outcome = execute(plan, &mut rset, &mut frontier)
                    post    = evaluator.snapshot(rset, memory)
                    delta   = summarize(current, post, outcome)
                    finalize(episode, outcome, post, delta)
                    memory.record(episode)
                    frontier.update_after(episode)
                    current = post
                    budget.consume(1)
                    if delta.is_positive(): steps_since_last_gain = 0
                    else: steps_since_last_gain += 1
                SwitchMode(m):
                    mode = m
                    break   // start fresh budget next tick
                Sleep:
                    lifecycle = Sleeping
                    break
                Stop:
                    lifecycle = Stopped
                    break

        // 5. Outer control
        if should_enter_consolidate(mode, delta, memory): mode = Consolidate
        if should_enter_reflect(recent_deltas, memory):   mode = Reflect
        if should_sleep(frontier, memory, recent_deltas): lifecycle = Sleeping

        maybe_checkpoint()
```

Scheduler gets multiple shots per tick (up to `actions_per_tick_cap`)
so Expand mode can actually expand within a tick. Mode changes and
Sleep short-circuit out of the inner loop to re-evaluate from the
top next tick.

## Memory policy

### Durable M0 (this ADR)

- Memory is a pure Rust struct.
- Serialized to disk on checkpoint. Format: one JSON-ish file per
  runtime instance. Reuses the `to_text` TSV pattern idea from
  ADR 0038 but for structured records — likely a minimal
  hand-rolled schema (no serde) or a text file with one
  `episode_N: {...}` per line.
- Checkpoint triggers: on sleep entry, on clean shutdown, every N
  ticks (default N = 50).

### Deferred M1 (declarativized memory)

**Not in this ADR.** The rule for when a memory item graduates to
meta-R: it must have been (a) stable for ≥ K passes, (b)
referenced by ≥ 2 distinct theories / patterns, and (c) have a
clear knowledge semantics (e.g. "theory X persistently uses axiom
Y"). Until a real use-case surfaces, M1 stays speculative.

### Why split durability from declarativeness

Two different needs:
- **Durability**: surviving a process restart. Every
  operational-memory record needs it. Disk + JSON-ish dump.
- **Declarativeness**: being part of world knowledge usable by
  downstream abstraction. Only the stable, knowledge-flavored
  subset. meta-R.

Conflating them was the trap I almost walked into.

## Touched v2 ADRs

Runtime wraps / calls-into (but does not replace):

- ADR 0018 `autonomous_pass` — invoked by `ActionKind::DiscoverPatterns`
- ADR 0028 / 0037 `discover_axioms_minimal[_compositional]` — via
  `DiscoverTheory` and `MinimizeAxioms`
- ADR 0030 `name_theory` — via `DiscoverTheory`
- ADR 0031 `abstraction_score`, `intrinsic_drive` — `intrinsic_drive`
  becomes "one-shot runtime facade"; still callable
- ADR 0035 `counterfactual_value`, `rank_by_counterfactual` — core
  evaluator inputs
- ADR 0038 persistence — reused for RSet side of checkpoint
- ADR 0040 `DriveAction::Prune` — becomes
  `ActionKind::PruneLowValueObjects`
- ADR 0043 sampling-path — runtime inherits the flag through
  `ActionPlan`'s context
- ADR 0051 `adaptive_drive_config` — runtime calls once at boot
  + again at Reflect

Indirectly touched (relation APIs consumed by scheduler):

- 0034 / 0042 / 0046 theory relations
- 0049 classifier

## Backward compatibility

Nothing in v2's existing public API changes. In particular:

- `RSet::intrinsic_drive(&mut self, &cfg)` continues to exist.
  New code should generally prefer
  `AutonomousRuntime::new(...).run_until(cond)`, but single-shot
  drive remains legitimate and tested.
- All existing `name_*`, `discover_*`, `check_*`, `retract_*`
  methods unchanged.
- Existing examples (`axiom_rigorous_test`, `intrinsic_drive`,
  `theory_discovery`, etc.) run unchanged.

## Scheduler policy (initial version)

First `Scheduler` implementation is **rule-based**, not learned:

- In Expand: prefer frontier items with `novelty_score ×
  estimated_value / (estimated_cost + 1)` at top.
- In Consolidate: prefer `MinimizeAxioms` if
  `discover_axioms_minimal` would materially reduce axiom count
  (heuristic: any axiom with ≥ 2 post-subsumption equivalents
  triggers); then `PruneLowValueObjects` for objects with CV < 0;
  then `UpdateTheoryRelations` if any newly-named theory has no
  extension/independent/parallel edges logged yet.
- In Reflect: aggregate `recent_deltas`; retune
  `adaptive_drive_config`; decide next wake threshold; return
  `SwitchMode(Expand)` or `Sleep`.

Only one scheduler type in Phase A. The `Scheduler` trait exists
so a learned policy can replace it later without runtime changes.

## Non-goals (explicit)

This ADR does not include:

- **Learned policy**: no RL, no NN, no statistical
  meta-learning on action outcomes. Scheduler stays rule-based.
- **Cross-process / distributed runtime**: single process.
- **M1 declarativized memory**: stays as a future ADR.
- **Wall-clock budgets**: step counts only.
- **Real-world environments**: `Environment` trait defined, but
  only `NoOpEnvironment` and `SyntheticStreamEnvironment`
  implemented in this ADR.
- **Hot-reloading scheduler / memory** during a run.
- **Multi-runtime orchestration** (e.g. one runtime per RSet
  shard).
- **Formal termination proofs**: we test termination behavior,
  we don't prove it.

## Testing strategy

Three families:

1. **Bounded-tick tests**: run with
   `BudgetState { ticks_remaining: Some(N), .. }`, assert final
   state (mode, lifecycle, score, memory contents, RSet named
   objects). Primary correctness surface.
2. **NoOp-environment tests**: `NoOpEnvironment` never produces
   events. Runtime must enter `Sleeping` in finite ticks and
   stay there. Protects against runaway-loop bugs.
3. **Deterministic-trace tests**: fixed seed + fixed
   `SyntheticStream` input → episode trace (ordered vector of
   `Episode` summaries) is byte-identical across runs. Allows
   snapshot-style testing of scheduler decisions.

Plus unit tests for:
- Individual `Memory` submodules (episode append, object-history
  update, policy-stat aggregation).
- Frontier: dirty-region marking, cooldown expiry, status
  transitions.
- Evaluator: delta summary arithmetic; confirm
  `score_delta = post.total - pre.total`.

## Phased delivery

### Phase A0 — "spin loop"

- `AutonomousRuntime` skeleton (rset + tick + lifecycle +
  single mode = Expand).
- `Scheduler` stub (always returns
  `Execute(DiscoverTheory { target: whole-rset })`).
- `Memory` stub (empty stores, append-only on episode).
- `NoOpEnvironment`.
- Bounded-tick test: runs N ticks on a diamond poset, logs N
  episodes, does not crash.

### Phase A1 — "working frontier"

- Real `Frontier` with mark_dirty / refresh / cooldown.
- Rule-based `Scheduler` for Expand mode.
- Deterministic-trace test.

### Phase A2 — "mode machine"

- Consolidate and Reflect modes wired.
- `should_switch_mode` logic (the `should_enter_consolidate /
  _reflect` predicates).
- Mode-transition stats recorded.

### Phase A3 — "sleep/wake"

- `should_sleep` / `should_wake` predicates.
- NoOp-environment test: runtime reaches Sleeping in finite
  ticks.
- Checkpoint on sleep entry; resume from checkpoint works.

### Phase B — "history kicks in"

- `ObjectHistory` populated and queried by scheduler.
- `PolicyStats` accumulated.
- `SyntheticStreamEnvironment`: inject edges over time, observe
  system respond.

### Phase C — "selective declarativize"

- Deferred; future ADR.

## Verification plan

At the end of Phase A3, the following should all be true:

1. All 282 prior tests pass unchanged.
2. ≥ 30 new tests exercise runtime behavior across the three
   test families.
3. A bounded-tick run on the 8-case rigorous battery (from ADR
   0027) produces sensible trace: each case reaches a stable
   state within N ticks, final theory fingerprint matches the
   pre-runtime `discover_theory` output.
4. NoOp test confirms termination.
5. A synthetic-stream scenario: start empty, drip-feed a diamond
   poset over 20 ticks, runtime ends up with `is_poset = true`
   and a named theory.

## Implementation

- `v2/src/runtime/mod.rs` — new module, exposes
  `AutonomousRuntime`, `Memory`, `Frontier`, `Scheduler` trait.
- `v2/src/runtime/main_loop.rs` — the loop implementation.
- `v2/src/runtime/memory.rs` — Memory stores, persistence.
- `v2/src/runtime/scheduler.rs` — default rule-based scheduler.
- `v2/src/runtime/environment.rs` — Environment trait, NoOp and
  SyntheticStream.
- `v2/src/runtime/evaluator.rs` — Evaluator façade.
- `v2/tests/runtime/*.rs` — bounded-tick, noop, deterministic-
  trace tests (pulled out of `lib.rs` because of size).
- `v2/examples/runtime_diamond_poset.rs` — end-to-end demo.

## Open questions (for the implementation, not blocking acceptance)

1. **Episode-log format**: JSON-ish hand-rolled vs. TSV vs. a
   `Vec<String>` one-line-per-episode. TSV-style probably wins
   on simplicity and matches ADR 0038's aesthetic.
2. **Frontier item cap**: Unbounded frontier could grow.
   Suggest per-kind caps with LRU eviction for cold items; exact
   numbers deferred to implementation.
3. **Stability estimate decay**: decay rate? Simple EMA with
   fixed α is probably enough for Phase B.
4. **"Clean shutdown" signal**: stick with Ctrl-C in tests and a
   `stop()` method for callers; no signal handling.

These are to be decided during implementation; not blocking this
ADR.

## Summary

v2 has the semantic layer. Runtime is the missing control layer.
It adds durability to what v2 already computes, and adds the
temporal structure (Expand/Consolidate/Reflect/Sleep) that
`intrinsic_drive`'s single pass never had.

Landing order: Phase A0 (spin loop) → A1 (real frontier) → A2
(mode machine) → A3 (sleep/wake) → B (history kicks in).

Phase A0 is ~300 lines of scaffolding; each subsequent phase is
~500–1000 lines with tests. Total estimate: ~3500 lines across
the runtime module for Phases A–B.

No rename to v3 proposed. If runtime-mode usage overtakes
library-mode usage over time, a future ADR can rename the crate;
for now, v2 gains a runtime module.
