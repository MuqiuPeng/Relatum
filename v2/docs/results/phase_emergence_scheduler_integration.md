# Phase Emergence — Scheduler integration of pattern discovery (partial)

**Status**: ⚠ partial (2026-05-06); honest documentation of progress + remaining gap
**Logs**:
- [`logs/2026-05-06_phase_emergence_scheduler_diagnostic.log`](../../logs/2026-05-06_phase_emergence_scheduler_diagnostic.log) — pre-fix baseline
- [`logs/2026-05-06_phase_emergence_scheduler_diagnostic_post_fix.log`](../../logs/2026-05-06_phase_emergence_scheduler_diagnostic_post_fix.log) — post-fix
**Example**: [`examples/phase_emergence_scheduler_diagnostic.rs`](../../examples/phase_emergence_scheduler_diagnostic.rs)
**ADR**: [0075 — Emergence kernel audit and runtime integration](../decisions/0075-emergence-kernel-audit-and-runtime-integration.md), piece 2

## Goal

ADR 0075's piece 2: bring the pattern-naming kernel from the
manually-invoked state (kernel audit / canonical-form-diversity /
pattern-shapes slices) into the runtime's autonomous Phase 0
behaviour — let `RuleBasedScheduler` dispatch `DiscoverPatterns`
periodically without requiring an explicit experiment script.

## What was already in place

The infrastructure was largely complete before this slice:
- `RuleBasedScheduler` already supports `PatternCandidate` frontier
  items (mapped to `DiscoverPatterns`)
- Frontier already proposed `PatternCandidate` per pattern size
- Cooldown gate already existed (`min_pattern_hit_rate`,
  `min_pattern_attempts_before_cooldown`)
- ADR 0018's `autonomous_pass` was already wired to
  `ActionKind::DiscoverPatterns` dispatch

The pre-fix diagnostic showed 5 DP dispatches per substrate,
0 patterns minted on OQ#1 / long5k / narrow_a, 2 minted on OQ#2.

## Diagnosis (pre-fix)

Three independent issues compounded:

1. **Fixed dispatch RNG seed.** The dispatch hard-coded
   `rng_seed = 2024` regardless of which DP call. Successive
   calls sampled identical subgraphs, every call after the
   first rediscovered the same canonicals, and 0 of the 5
   calls produced new mints.

2. **Stale `abstraction_score` delta**. When DP minted a fresh
   pattern at 1 instance, `abstraction_score` (a) credits only
   patterns with ≥ 2 instances and (b) subtracts 0.1 per new
   meta-R edge. The score *decreased* on a 1-instance mint, so
   the cooldown gate counted these as unproductive, inflating
   the cool-down counter.

3. **Conservative cooldown threshold.** `min_pattern_attempts_
   before_cooldown = 5` meant 5 attempts at any time were
   sufficient to gate DP for the rest of the run. Combined
   with #1 + #2, DP self-locked early at tick ~30 on dense
   substrates.

## Changes shipped in this slice

### `src/runtime/autonomous.rs` — DP dispatch path

- `rng_seed` now varies with `episode_counter`:
  `2024 + episode_counter * 0x9E37...`. Successive DP dispatches
  sample different subgraphs.
- `sample_count` raised 200 → 400 to match the kernel audit's
  empirically validated budget (size 2-5 mint reliably at 400
  samples).
- Explicit positive-delta override: when at least one
  `AutonomousOutcome::NewPattern` is in the outcomes, return
  `Some(new_patterns_count as f64)` so cooldown-gate input
  reflects mints rather than score arithmetic.

### `src/runtime/frontier.rs` — PatternCandidate sizes

- Sizes proposed extended from `[2, 3]` to `[2, 3, 4, 5]`.
  Matches the kernel audit's range. Note: the priority
  formula `value / (size + 1)` still favours size 2/3 within
  the PatternCandidate cohort, so sizes 4/5 are present in the
  frontier but rarely picked at the moment. Future scheduler
  changes can leverage the wider range.

### `src/runtime/scheduler_rule.rs` — cooldown threshold

- `min_pattern_attempts_before_cooldown`: 5 → 30. Gives DP
  enough early attempts that initial-rset failures (when the
  rset is too small to produce mint-worthy samples) don't
  permanently lock out DP.

### `src/runtime/tests.rs` — fixture updates

Four cooldown tests updated to use the new threshold-30 default:
- `b1plus_pattern_cooldown_activates_on_low_hit_rate`
- `b1plus_cooled_pattern_falls_back_to_theory_candidate`
- `b1plus_cooled_pattern_with_no_theory_falls_back_to_consolidate`
- `meta_meta_cooldown_independent_of_pattern_cooldown`

All increased their DP attempt counts from 10/20 to 40 to
exceed the new threshold, preserving the original test intent
("when attempts above threshold AND hit rate below floor, DP
cools down").

Lib tests: 617 passing, 0 regressions.

## Result

```
substrate    ticks  episodes   DP_count   DP_pos   final_patterns
OQ#1          1000      106         18        0                0
long5k        1500      159         30        0                0
narrow_a       500       76         18        0                0
OQ#2          4500       10          5        2                2
```

DP dispatches went from 5 per substrate (pre-fix) to 18-30 on
OQ#1-clade substrates (post-fix). OQ#2 is unchanged at 5 because
its total episode count is small (10) — it sleeps quickly given
its sparse axiom path.

**But final patterns minted by runtime is still 0 on OQ#1 /
long5k / narrow_a.** The dispatch fixes restored the cool-down
counter to truth (DP fires more, but each fire still produces 0
mints). The remaining issue is structural and not part of this
slice.

## Why mints still fail on dense substrates

The dispatch path attempts whatever `target_size` the frontier
target carries. Frontier currently almost always promotes
size = 2 (priority formula `value / (size + 1)` is decreasing
in size). On dense substrates like OQ#1's diamond posets, size-2
samples almost universally fail the `is_clean_subgraph` check:
the participants' neighbourhoods induce more data edges than the
sample contains, so the canonical form found by sampling has no
matching clean instance in the rset.

The kernel audit was able to mint patterns by manually trying
sizes 2-5 with `sample_count=400`. **Sizes 4 and 5 wrap whole
connected clusters whose induced edges match the canonical**,
giving clean instances. The runtime currently tries only the
size proposed by the highest-priority frontier item.

Two attempted fixes that broke other tests:

- **Multi-size dispatch loop**: each DP call tries size 2-5 in
  sequence, falling through on no-mint. This works on OQ#1
  (would mint patterns) but changes runtime tick timing in
  ways that break `a3_resume_runs_full_run_to_completion`
  (lifecycle test that depends on specific sleep timing).

- **Reverse-priority formula** (`priority = value * size`):
  PatternCandidate(size=5) wins over TheoryCandidate at
  diamond_poset density, breaking `a1_rule_based_runs_and_sleeps`
  (the standard test fixture expects TheoryCandidate to be
  picked first on a 9-edge rset).

Both deferred. They demonstrate that the issue is not a single
parameter but a multi-component coupling: priority formula,
dispatch path, frontier proposal, and cooldown semantics all
need to align before scheduler integration produces autonomous
mints on dense substrates.

## Honest verdict

**Piece 2 is partial:**

✓ DP fires more frequently (5 → 18-30 on OQ#1-clade) and
  cooldown is no longer a self-locking trap
✓ Dispatch parameter changes match the kernel audit's
  empirically validated budget
✓ Existing test invariants preserved
✗ DP still produces 0 mints on dense substrates because the
  scheduler's priority formula picks size-2 PatternCandidate
  which is structurally doomed on dense rsets
✗ The "kernel runs autonomously during normal stream
  processing" goal of piece 2 is unreached for diamond-poset
  substrates; OQ#2's 2 mints are unchanged

What's left for a future "piece 2.1":

- Multi-size dispatch path that doesn't break lifecycle test
  timing — perhaps gated to specific runtime-mode contexts so
  expanded-attempt cost is only paid when relevant
- Per-size cooldown tracking — current cooldown is per
  ActionKind (lumps all sizes), so a size 2 lockout also locks
  size 4/5
- Or a different priority semantics that avoids starving
  TheoryCandidate at low rset density while still letting
  size 4/5 PatternCandidate get picked once the rset is dense

## Files

- `src/runtime/autonomous.rs` — DP dispatch updates
- `src/runtime/frontier.rs` — sizes [2, 3] → [2, 3, 4, 5]
- `src/runtime/scheduler_rule.rs` — cooldown threshold 5 → 30
- `src/runtime/tests.rs` — 4 fixture updates
- `examples/phase_emergence_scheduler_diagnostic.rs`
- `logs/2026-05-06_phase_emergence_scheduler_diagnostic.log`
- `logs/2026-05-06_phase_emergence_scheduler_diagnostic_post_fix.log`
- This result doc

Lib tests: 617 passing, 0 regressions.

## What this slice did NOT achieve

The original framing of ADR 0075 piece 2 was:

> Promote `DiscoverPatterns` priority in `RuleBasedScheduler`
> so the runtime calls `autonomous_pass` periodically during
> normal Phase 0 stream processing.

This slice ships the *infrastructure* improvements (dispatch
parameters, cooldown threshold, frontier size range) needed
for that goal but does not yet achieve the goal itself: the
runtime still does not mint patterns autonomously on
diamond-poset substrates. Future work must address the
priority / multi-size coupling without breaking lifecycle test
invariants.

The result is a clean handoff: the dispatch path is corrected,
cooldown no longer self-traps, frontier proposes the necessary
size range, but the priority-based scheduler choice still picks
a doomed size first. The next attempt has a clear constraint
list.
