# ADR 0067 — Source-tree refactor: split monolithic lib.rs and runtime/mod.rs (2026-04-28)

## Status

Accepted; landed.

## Context

By 2026-04-28 the v2 codebase had grown to two monolithic Rust files:

- `src/lib.rs` — **10,691 lines** (production code + tests interleaved)
- `src/runtime/mod.rs` — **10,972 lines** (production code + tests interleaved)

Total: 21,663 lines in 2 files. While the code itself was correct (524 lib tests passing, all examples building), the file size made navigation, code review, and incremental ADR traceability painful. Adding a new feature meant scrolling through 10k+ line files to find the right insertion point. ADRs referencing "runtime scheduler" and "runtime drive" all pointed at the same file.

The user requested: 目前的代码文件过长了，而且放在一起没有逻辑，将其进行一次重构.

## Decision

Refactor in 4 phases, each independently verifiable via `cargo test --lib` (524 tests must remain green) and `cargo build --release --examples`.

### Phase 1 — Extract test modules

`#[cfg(test)] mod tests { ... }` blocks moved from inline to sibling files via `mod tests;`:

- `src/lib.rs` lines 5864–10691 → `src/tests.rs` (4,825 lines)
- `src/runtime/mod.rs` lines 4978–10972 → `src/runtime/tests.rs` (5,992 lines)

Submodule semantics preserved — tests still see all crate-private items via `super::*`.

### Phase 3 — Split `runtime/mod.rs` into 12 submodule files

Selected before Phase 2 because runtime had cleaner subsystem boundaries (trait-based scheduler/drive/environment design vs. RSet's monolithic `impl` block).

| New file | Contents | Lines |
|---|---|---|
| `runtime/lifecycle.rs` | `LifecycleState`, `RuntimeMode`, `BudgetState` | 42 |
| `runtime/action.rs` | `ActionKind`, `FrontierTarget`, `ActionPlan`, `SchedulerDecision` | 75 |
| `runtime/scheduler.rs` | `Scheduler` trait + `SchedulerContext` + `StubScheduler` | 61 |
| `runtime/scheduler_rule.rs` | `RuleBasedScheduler` (ADR 0052 / A1+A2) | 593 |
| `runtime/scheduler_meta.rs` | `MetaScheduler` (ADR 0060 / Phase H0) | 172 |
| `runtime/scheduler_ucb.rs` | `UcbCompositeScheduler` (ADR 0065 / Phase Alpha-1, negative) | 163 |
| `runtime/drive.rs` | `Drive` trait + 3 baseline impls + `DriveMix` (ADR 0063) | 334 |
| `runtime/environment.rs` | `Environment` trait + impls + `Event` + `should_wake` | 81 |
| `runtime/memory.rs` | `Episode`, transitions, `ObjectHistory`, `PredictionState`, `Memory` | 505 |
| `runtime/frontier.rs` | `FrontierKind/Status/Item`, configs, `Frontier` | 690 |
| `runtime/autonomous.rs` | `AutonomousRuntime` + main tick loop | 1,932 |
| `runtime/persistence.rs` | A3 serialization helpers + `parse_checkpoint` | 400 |

`runtime/mod.rs` reduced to **68 lines** — module declarations + `pub use` re-exports + `mod tests;`.

Backward compatibility: all public items remain accessible at `relatum_v2::runtime::*` via `pub use`. Some private helpers became `pub(crate)` to allow cross-submodule access (`would_thrash`, `pattern_cooldown_active`, `meta_meta_cooldown_active`, `any_axiom_has_hit_rate`, `h1_1_bonus_kinds`, `pick_top_biased`, `MetaScheduler::maybe_advance`, `MetaScheduler::mutate`, `DriveMix::mutate`, `composite_stats`, `ucb1_score`, `theory_pair_has_relation`, `execute_action`, `register_drives_in_rset`, `maybe_promote_action_sequences`, `maybe_demote_action_sequences`, `parse_checkpoint`, the persistence helpers, `ParsedCheckpoint` fields).

### Phase 2 — Pull standalone types/functions out of `lib.rs`

Conservative approach: extract isolated standalone definitions; **do not** split the giant `impl RSet { ... }` block. The RSet impl stays in lib.rs to avoid the multi-impl-block split-across-files complexity for now.

| New file | Contents | Lines |
|---|---|---|
| `markers.rs` | All 18 reserved meta-R registry markers + interleaved adjacent types (`TheoryRelationKind`, `TheoryNeighborhood`, `PersistenceError`) | 185 |
| `stats.rs` | `wilson_score_95`, `null_baseline_probability` (ADR 0045) | 57 |
| `axiom_ids.rs` | `axiom_template_id`, `axiom_id_to_template`, `equality_*`, `disjunctive_*` (ADR 0030, 0047) | 151 |
| `types_axiom_drive.rs` | `DiscoveryConfig`, `MotifCandidate`, `EdgeTemplate`, `AxiomTemplate` family, `ExtendedAxiomEvidence`, `AxiomEvidence`, `AxiomDiscoveryConfig`, `Reflexivity/Antisymmetry/TotalityEvidence`, `PosetCheck`, `DriveAction`, `DriveActionResult`, `DriveStep`, `DriveTrace`, `DriveConfig` (ADR 0016, 0027, 0030, 0031, 0044, 0045, 0047) | 344 |
| `types_runtime.rs` | `SamplingMatchConfig`, `RefinementConfig`, `AutonomousConfig`, `AutonomousSkip`, `RetractionError`, `RetractionSummary`, `AutonomousAndAttachSummary`, `AutonomousOutcome` (ADR 0017, 0020, 0021, 0022, 0024) | 93 |

`lib.rs` reduced to **5,086 lines** (52% reduction from 10,691).

The remaining 5k lines are dominated by the `impl RSet { ... }` block (~3,900 lines) plus `R` struct, `RSet` struct, `Subgraph` impl, motif sampling helpers, canonicalization helpers, and a few other types not extracted. A future refactor could split RSet impl across multiple files using Rust's "extension methods" pattern (multiple `impl RSet` blocks across files); deferred.

### Phase 4 — This ADR + commit

## Results

| metric | before | after | Δ |
|---|---|---|---|
| files in `src/` | 2 (.rs) | 21 (.rs) | +19 |
| `src/lib.rs` lines | 10,691 | 5,086 | −52% |
| `src/runtime/mod.rs` lines | 10,972 | 68 | −99% |
| largest production file | 10,972 | 1,932 (autonomous.rs) | −82% |
| `cargo test --lib` | 524 pass | 524 pass | — |
| examples build | ✓ | ✓ | — |

## Constitution check

- **C1 (R singular)**: ✓ Refactor is mechanical — no new primitive introduced.
- **C2 (R binary)**: ✓ N/A.
- **C3 (types as meta-R)**: ✓ Markers stay in `markers.rs` as `pub const &str`; their meta-R semantics are unchanged.
- **C4 (token identity)**: ✓ N/A.
- **C5 (structural similarity)**: ✓ N/A.

No constitutional surface touched.

## What this slice does NOT do

- **Does not split `impl RSet { ... }`.** The 3,900-line RSet impl remains in lib.rs. Splitting it across files (using multi-impl-block pattern) is a higher-risk follow-up if file size becomes painful again.
- **Does not introduce new public API.** Re-exports keep all existing paths (`relatum_v2::RSet`, `relatum_v2::AutonomousRuntime`, etc.) working.
- **Does not change test semantics.** Tests are still `#[cfg(test)]` submodules; the extraction is purely textual.

## Visibility changes (audit trail)

For cross-submodule access in `runtime/`, these were made `pub(crate)`:
- `BudgetState::reset_per_tick`
- `RuleBasedScheduler::{would_thrash, pattern_cooldown_active, meta_meta_cooldown_active, any_axiom_has_hit_rate, h1_1_bonus_kinds, pick_top_biased}`
- `MetaScheduler::{maybe_advance, mutate}`
- `DriveMix::mutate`
- `UcbCompositeScheduler::{composite_stats, ucb1_score}`
- `AutonomousRuntime::{execute_action, register_drives_in_rset, maybe_promote_action_sequences, maybe_demote_action_sequences}`
- `theory_pair_has_relation`
- `persistence::*` (all helper functions and `ParsedCheckpoint` + its fields)

In `lib.rs`:
- `canonicalize_template` (used by `axiom_ids.rs`)
- `DriveConfig::candidate_actions`

These changes increase visibility within the crate but do not export new symbols publicly.

## Future work

If `lib.rs` grows uncomfortable again, the next refactor candidate is splitting `impl RSet { ... }` into multiple files using Rust's extension-method pattern:

- `src/rset/core.rs` — base CRUD + invariants
- `src/rset/pattern.rs` — pattern naming/discovery methods
- `src/rset/axiom.rs` — axiom registration/forward-apply methods
- `src/rset/theory.rs` — theory naming/retraction methods
- `src/rset/persistence.rs` — to_text / from_text

Each file would have `impl RSet { ... }` blocks merged at compile time. Deferred until concrete pain emerges.

Subsumption helpers (`subsume_by_*`, `template_derivable_from`, plus their private helpers `template_subsumes`, `extend_and_check`, `premise_contained`, `evaluate_template_recursive`, `forward_chain_apply`, `forward_apply_recursive`) were considered for extraction but the dependency graph is tangled with private helpers used by the RSet impl block. Deferred.

`Subgraph` and its 370-line impl block were also considered but they sit between motif-discovery code and the RSet impl, making clean extraction non-trivial. Deferred.
