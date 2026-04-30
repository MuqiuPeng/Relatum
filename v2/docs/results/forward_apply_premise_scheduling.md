# Forward-apply premise scheduling

**Status**: ✓ done (2026-05-01)
**Pre-baseline log**: [`logs/2026-05-01_phase_premise_scheduling_baseline_pre.log`](../../logs/2026-05-01_phase_premise_scheduling_baseline_pre.log)
**Post-baseline log**: [`logs/2026-05-01_phase_premise_scheduling_baseline_post.log`](../../logs/2026-05-01_phase_premise_scheduling_baseline_post.log)
**Companion to**: [A.1 premise reorder](A.1_premise_reorder.md) (verified-deferred)

## Goal

Per the user's strategic critique (2026-04-30) item #6:
> Alpha-2 / cognitive MCTS 的关键问题是：value-guided search 的成本是否低到能跑足够多 rollout？
> ...
> 真正该做的是继续把 forward-apply 从 enumerate-all-bindings 推向
> join-style evaluation / premise-driven binding propagation.

A.1 (2026-04-29) verified-deferred premise reordering by selectivity
because Alpha-6's earlier optimization had already moved the
bottleneck elsewhere. This slice attempts ONE more low-cost
forward-apply improvement: **premise scheduling** — eliminating the
redundant leaf check in `forward_apply_recursive_indexed`.

## What was redundant

The pre-existing leaf check (depth == num_vars):

```rust
if depth == binding.len() {
    for e in &template.premise {
        if !rs.instances.contains(&R::new(...)) {
            return;  // re-verify EVERY premise
        }
    }
    out.insert(R::new(cx, cy));
    return;
}
```

This re-verified all premises at every leaf. But:

1. **Cross-variable premises** (e.g., `R(0, 1)` in a 3-var template):
   the candidate filter at depth `max(e.x_var, e.y_var)` already
   constrains the binding via neighbor-set intersection. By the
   time we reach the leaf, the premise is structurally satisfied.
2. **Self-loop premises** (e.g., `R(0, 0)` — both endpoints same var):
   the candidate filter cannot express these via neighbor-set
   intersection. They DO need explicit verification — but only
   once per binding, at the depth where their var is bound, not
   at the leaf.

## The fix

**Premise scheduling**: precompute, for each depth d, the list of
self-loop premises `R(d, d)` to verify when binding[d] is set.
At the leaf, no premise verification is needed.

```rust
// Precompute once per template
let mut self_loop_at_depth: Vec<Vec<usize>> = vec![Vec::new(); n];
for (idx, e) in template.premise.iter().enumerate() {
    if e.x_var == e.y_var && e.x_var < n {
        self_loop_at_depth[e.x_var].push(idx);
    }
}

// At each depth d, verify self-loop premises before recursing
for &prem_idx in self_loop_at_depth[depth] {
    let id = ids[binding[depth]];
    if !rs.contains(R::new(id, id)) { return; }
}

// At the leaf, just emit the conclusion (no verification)
```

This:
- Eliminates the per-leaf O(P) verification loop
- Keeps cross-variable premise enforcement via the existing
  candidate filter (unchanged)
- Adds a single contains() check per binding at the var depth
  for axioms with self-loop premises (e.g., the noise family
  `shape_premise_p0-0_p1-2`)

## Empirical result on OQ#1 baseline (2000 ticks)

| metric | pre | post | delta |
|---|---|---|---|
| Total wall-clock | 279.66s | 272.95s | **−6.71s (−2.4%)** |
| Avg ms/tick | 139.8 | 136.5 | −3.3 (−2.4%) |
| Final chunk (tick 1900-2000) | 496.8ms | 488.6ms | −8.2 (−1.7%) |

Improvement is small but real and consistent across chunks. Early
chunks (1-10) are noise-bound; later chunks show the steady ~2%
improvement.

## Why the improvement is modest

A.1's verdict already explained this: forward_apply is no longer
the dominant cost. The bottleneck has moved to:
- snapshot construction (HashMap operations per axiom)
- prediction state tracking (per-axiom hit-rate accumulation)
- mode-transition + sequence-stats accounting

Premise scheduling is a clean structural improvement to
forward_apply, but its impact is bounded by forward_apply's
share of total runtime cost. ~2.4% improvement matches the
shape of "small but real" expected.

## What was correctness-preserving

The optimization is **provably equivalent** for any axiom shape:

- For non-self-loop premises: the candidate filter at depth
  `max(e.x, e.y)` constrains the binding to satisfy the premise.
  The leaf re-check was always redundant.
- For self-loop premises: the new per-depth verification fires
  before recursing, so any binding that fails the self-loop
  is pruned at the same point as before (just earlier in the
  call chain). The set of generated bindings is identical.

Verified by:
- 593 lib tests pass post-change (587 pre + 6 expansion path
  exercise this code).
- The unit tests for forward_apply_axiom (e.g.,
  `forward_apply_axiom_returns_predictions`,
  `forward_apply_axiom_with_self_loop_premise`) cover both
  cross-variable and self-loop premise shapes.

## What this slice does NOT do

- **Does not implement candidate-set allocation reduction**.
  Each candidate set is still allocated as HashSet<usize> at
  intermediate depths. Replacing with a BitSet (for small N)
  or leapfrog-triejoin (for large N) is a bigger refactor;
  not motivated by current empirical pressure.
- **Does not reorder premises by selectivity**. A.1's
  verified-deferred verdict still applies — selectivity
  computation costs about as much as the work it would save.
- **Does not address the snapshot construction bottleneck**.
  That's a different code path (per-tick snapshot in
  `update_prediction_state`).

## Conclusion

Premise scheduling is the LAST low-cost improvement available to
the forward_apply path before either:
- Moving to bigger refactors (BitSet candidate sets, leapfrog
  triejoin)
- Or accepting that further forward_apply optimization is
  diminishing-returns territory

Per the empirical data, the second option is honest. The slice
ships the small improvement but explicitly marks the wider
"forward-apply join optimizer" direction as **largely complete**:
- Alpha-6 indexed-join enumerator: shipped, 25% gain
- Alpha-6 Option D early premise termination: shipped, 40-47% gain
- Premise scheduling (this slice): shipped, 2.4% gain
- A.1 selectivity reordering: verified-deferred (catch-22)

Total cumulative improvement from forward_apply optimizations:
~50% over the naive enumeration baseline. Bottleneck is now
elsewhere; chasing further forward_apply gains has poor ROI.

## Future-direction sign-off

Item #9 of the user's punch list (2026-04-30 strategic critique:
"forward-apply join optimizer") is **substantially complete**. The
remaining "join-style evaluation / premise-driven binding
propagation" framing was the design intent of the Alpha-6 work +
this slice's premise scheduling. Going further (BitSet, leapfrog,
incremental maintenance) would require a substrate that exercises
forward_apply more aggressively than current OQ#1 / long5k. No
such substrate exists today.

If MCTS-style cognitive search lands later, it will create the
substrate that motivates further optimization. Until then,
forward_apply is "good enough by 50%" — which is the project
state A.1 documented and this slice empirically reaffirms.
