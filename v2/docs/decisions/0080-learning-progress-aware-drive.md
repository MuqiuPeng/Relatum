# 0080: Learning-progress-aware drive

Status: Proposed
Date: 2026-05-11

Parents:
- [0078 — Pattern-aware drive metric](0078-pattern-aware-drive-metric.md)
- [0079 — Drive-driven frontier candidate](0079-drive-driven-frontier-candidate.md)
- [0079.1 — Drive-aware thrash bypass](0079-1-drive-aware-thrash-bypass.md)

Reference inspiration:
- Oudeyer & Kaplan (2007), Schmidhuber (2006) — Learning
  Progress framework
- Curiosity as Information Gain (CIG 2026) — three-component
  decomposition (Novelty / Learnability / Competence)
- 2026-05-08 retrospective — identified the v2 gap

## Context

ADR 0079 + 0079.1 wired drive metric to scheduler with three
coordinated gate bypasses. v2 reached sustained drive-driven
cognition on OQ#2 — but the empirical observation
(`phase_emergence_oq2_equilibrium`) reveals a quality issue
beneath the quantity success:

- 800-tick run produces DP_count=21 / DP_pos=8 (38% hit rate)
- Pattern count 7 is the structural ceiling — OQ#2's mature
  rset has exactly 7 distinct canonical forms
- After 7 mints, dispatches keep firing on the same canonicals,
  outcome shifts from `NewPattern` to `Existing` or `Skipped`,
  drive briefly shrinks then refills as new R arrives
- Long-horizon (3000+ tick) runs become impractical: each wake
  triggers a dispatch that runs autonomous_pass (multi-size
  fallback, ~1600 samples), most of which produce no new mints

The runtime is **busy but not learning**. v2's drive is
purely novelty-based; it doesn't distinguish "this canonical is
a fresh structure" from "this canonical is the same one we've
minted seven times already." Both look like "unexplained R" to
the metric.

The 2026-05-08 retrospective's world-model-research review
identified the gap precisely: v2 implements *Novelty
Sensitivity* (CIG component 1) but not *Learnability
Filtering* (component 2) or *Competence-Weighted Priority*
(component 3). Adding the missing components is this ADR's
work.

## Decision

Add a **learning-progress factor** to the drive-driven
PatternCandidate priority. The factor downweights buckets whose
recent dispatches did not produce new mints (low learning
progress) and upweights buckets whose dispatches recently
*did* mint (high learning progress). Buckets in zero-progress
plateau get arbitrarily low priority and effectively stop
triggering dispatches.

### Mechanism

The frontier already proposes drive-driven `PatternCandidate`s
based on modal canonical (ADR 0079). This ADR layers a
*progress-weighted priority* on top:

```text
For each drive bucket B identified in unexplained_drive_signal():
  recent_dispatches_at_B = episodes where (ActionKind=DiscoverPatterns
                          AND PatternSize matches B.canonical.len())
                          in last K episodes
  net_mint_delta_at_B = (NewPattern outcomes in those episodes) -
                       (drives bypass triggered without mint outcomes)
  learning_progress = max(0, net_mint_delta_at_B) / max(1, recent_dispatches_at_B)

  bucket_priority = bucket_count × learning_progress_factor
  where learning_progress_factor =
      1.0                 if recent_dispatches_at_B == 0  (no history → try)
      learning_progress    if recent_dispatches_at_B > 0  (downweight if no progress)
```

The "1.0 on no history" clause is important: a brand-new
canonical that's never been dispatched gets full priority. Only
buckets with dispatch history but zero net mint get downweighted.

### Implementation surface

Single addition to `Frontier::refresh` drive-driven branch
(ADR 0079 location). The code currently:

```rust
let priority = drive.modal_count() as f64 * 5.0;
```

Becomes:

```rust
let progress_factor = compute_learning_progress(
    memory.episodes(),
    canonical.len(),
    LP_WINDOW,  // e.g., last 30 episodes
);
let priority = drive.modal_count() as f64 * 5.0 * progress_factor;
```

Where `compute_learning_progress` is a pure function over
`Memory::episodes`:

```rust
pub fn compute_learning_progress(
    episodes: &VecDeque<Episode>,
    target_size: usize,
    window: usize,
) -> f64 {
    let n_recent = episodes.len().min(window);
    let recent = episodes.iter().rev().take(n_recent);
    let mut dp_attempts = 0;
    let mut dp_positive_delta = 0;
    for e in recent {
        if let (ActionKind::DiscoverPatterns,
                FrontierTarget::PatternSize(sz)) = (e.action_kind, &e.target)
            if *sz == target_size
        {
            dp_attempts += 1;
            if e.delta > 0.0 {
                dp_positive_delta += 1;
            }
        }
    }
    if dp_attempts == 0 {
        return 1.0;  // no history, full attention
    }
    dp_positive_delta as f64 / dp_attempts as f64
}
```

### What gets weighted

The mechanism downweights drive-driven candidates whose history
shows no mints. It does NOT:

- Affect organic PatternCandidate priority (`Frontier::refresh`
  computes them via the existing `value / (size+1)` formula —
  unchanged)
- Affect TheoryCandidate / other frontier kinds
- Bypass cooldown / thrash gates (those still apply)
- Modify dispatch logic (autonomous_pass / multi-size fallback
  unchanged)

This is intentionally a *minimal additive change*: drive's
priority becomes progress-aware; nothing else changes.

### Maturity gate preserved

The existing maturity gate from ADR 0079
(`axioms ≥ 1 AND data_edges ≥ 100`) stays. Lifecycle test
fixtures never trigger drive-driven candidates, so they never
trigger progress weighting.

## Why this matches CIG / Oudeyer

| CIG component | v2 implementation |
|---|---|
| Novelty Sensitivity | `bucket_count` (current ADR 0079) — high count = high novelty |
| Learnability Filtering | `learning_progress_factor` — low if dispatches don't mint |
| Competence-Weighted Priority | implicit via DP success rate (which IS competence at that size) |

The combination produces the Oudeyer/Schmidhuber Learning
Progress signal: priority proportional to *rate of learning*,
not absolute uncertainty.

## Expected empirical effect

On OQ#2 long-horizon:

**Pre-ADR 0080 (current):**
- DP fires every drive-driven wake
- After 7 patterns minted, subsequent DP attempts mostly
  Existing / Skipped → 0 positive delta
- Drive bypass keeps firing → keep dispatching → expensive
- 800-tick: 21 dispatches, 8 positive

**Post-ADR 0080 (predicted):**
- First few dispatches: high priority, mint successfully (~5-7
  initial mints)
- Subsequent dispatches: priority drops (positive_delta / attempts
  ratio falls below 1.0)
- At equilibrium: priority ≈ 0 for fully-mined buckets, full for
  any new bucket from incoming stream events
- 800-tick: ~7-10 dispatches (the productive ones), 7 positive
- 3000-tick should now complete in similar time to 800-tick
  (proportional only to mint count, not tick count)

This makes long-horizon observation tractable. Combined with
caching from 0079 perf step 1, total speedup may be 5-10× on
realistic OQ#2 runs.

## Alternatives considered

**Alt A: time-based cooldown on drive candidates.** Instead of
learning progress, cool down drive after N consecutive
zero-mint attempts. Rejected: less principled (arbitrary N),
doesn't reward partial progress, can't re-engage a previously
mined bucket if new R changes the canonical.

**Alt B: per-bucket exact tracking instead of per-size.**
Track learning progress per specific canonical form (not just
per size). More precise but requires storing canonical forms
across episodes (more state). Defer until per-size proves
insufficient.

**Alt C: full JEPA-style latent prediction error tracking.**
Latent spaces violate constitution heavy reading. Rejected on
principle.

**Alt D: defer ADR 0080 until long-horizon observation is
empirically necessary.** Tempting but the retrospective + 800-
tick equilibrium observation already establish the need. Ship
0080 to unblock long-horizon analysis.

## Consequences

**Now possible:**

- 3000+ tick OQ#2 observation tractable (per-dispatch budget
  bounded by mint events, not by ticks)
- Empirical comparison: pre-0080 vs post-0080 dispatch counts
  on identical horizon → quantifies the perf win

**Now harder:**

- Edge case: if the stream introduces *new* canonicals after
  full mining, learning progress at that size is 0 (averaged
  over recent window), so the new canonical doesn't get high
  priority. Mitigation: 1.0 fallback for "no history" applies
  only if no DP dispatched at that size recently. If history
  exists but is all zero, the new canonical struggles. May
  need a "novelty bonus" to fix; deferred until observed.

**Newly easy:**

- Long-horizon observation runs that test mint dynamics over
  thousands of ticks
- Comparison probes that vary drive sensitivity (e.g., does
  raising LP_WINDOW change pattern stability?)
- The 2026-05-08 retrospective's full ADR 0080 hypothesis
  becomes empirically testable

## Implementation plan

1. Add `compute_learning_progress` to `src/runtime/agent_view.rs`
   (it's a query over episode log, fits the ADR 0076 pattern).
2. Modify `Frontier::refresh`'s drive-driven candidate priority
   to multiply by `compute_learning_progress(...)`.
3. Add ~4 unit tests:
   - LP = 1.0 on no history
   - LP = 0.0 when all attempts produced zero delta
   - LP between 0.0 and 1.0 on mixed history
   - Drive-driven priority changes with LP
4. Re-run `phase_emergence_oq2_equilibrium` at 3000 ticks
   (was hung pre-0080), verify completion + measure post-0080
   dispatch count.
5. Result doc comparing pre-0080 vs post-0080 numbers.

Estimated cost: ~half day. Self-contained change to one method
in frontier + one helper in agent_view.

## Constitution check

- C1 (R is singular): no R changes — ✓
- C3 (types are meta-R): no new types registered as meta-R; LP
  is a pure derivation from episode log — ✓
- C4 (identity is token-based): no token differentiation — ✓
- C5 (similarity is structural): LP is computed from per-episode
  delta + ActionKind matching, not from any structural similarity
  function — ✓
- Heavy reading (differentiation requires registration): no
  bucketing introduced; LP is a scalar weight on existing
  priority. No phantom typing — ✓

All clean.

## Open questions

- **What's the right LP_WINDOW?** 30 episodes is a guess. Too
  small → noisy (single bad luck dispatches kill priority);
  too large → slow to recover when canonical changes. Defer to
  empirical tuning.
- **Should `delta > 0.0` count, or specifically `NewPattern`
  outcome?** The dispatch path returns explicit `Some(new_patterns
  as f64)` when NewPattern was minted (per ADR 0075 piece 2
  revisited). delta > 0 conflates this with abstraction_score
  positive diffs from other paths. Probably want explicit
  NewPattern check; defer to implementation.
- **How does this interact with the multi-size fallback in
  dispatch (ADR 0075 piece 2 revisited)?** Fallback can mint at
  a different size than initial_size — does LP for size=2
  include or exclude these fallback-minted patterns? Probably
  exclude (use the target size from the FrontierTarget, not the
  actual mint size). Defer to implementation.

## Verdict

ADR 0080 is the natural next mechanism — directly inspired by
mature world-model intrinsic-motivation literature, implementable
as one priority-formula change, and unblocks empirical work
that's been infeasible.

This is the world-model research investment from the 2026-05-08
retrospective showing up as concrete v2 mechanism.
