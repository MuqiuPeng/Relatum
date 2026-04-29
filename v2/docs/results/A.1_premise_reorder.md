# A.1 — ILP premise reordering by selectivity (verified deferred)

**Status**: ✓ verified deferred (2026-04-29) — no code change

## Goal

ILP / Datalog optimization: reorder premise edges by selectivity so the candidate filter intersects smaller neighbor sets first, producing earlier prunes.

## Audit

Looked at `forward_apply_recursive_indexed` (Alpha-6). Three relevant points:

1. **Leaf-check already short-circuits**: when all variables bound, the loop checks each premise and returns on first miss. Already optimal at the leaf.

2. **Iteration step iterates all premises**: at each depth, the function iterates `for e in &template.premise` and builds candidate sets for premises that involve `depth` + a bound var. Order DOES affect intersection cost when multiple premises constrain the same depth.

3. **Catch-22 for selectivity-aware reordering**: to know which premise has the smallest neighbor set, you'd need to compute the neighbor set size — which IS the work itself.

## Why Alpha-6 result still holds

Alpha-6 measured ~25% per-tick speedup from indexed join over Option D, but **theory predicted 100×+**. The bottleneck has moved elsewhere (snapshot construction, prediction state HashMap operations, etc.). Premise reordering would yield additional gains in the ~5% range at best, given:
- It only affects the rare case of multiple premises constraining the same depth
- HashSet intersection is already O(min(|a|,|b|)), so reorder doesn't change asymptotic complexity
- The bottleneck isn't here anymore

## Verdict

**Verified deferred — no code change**. The optimization mechanism is well-understood; the empirical case for implementing it is weak per Alpha-6. Documented as deferred with rationale; not blocking any future work.

## What this slice produced

- Audit confirming Alpha-6's forward_apply diminishing-returns finding
- Methodological note: when a candidate optimization has a known catch-22 (computing the heuristic costs as much as the work it would optimize), and prior work shows the function is no longer dominant, deferring is the correct call
- Future-direction sign-off: no remaining "obvious" forward_apply optimizations on the books

## Future implications

- Real future perf work needs profile data first (per Alpha-6's verdict)
- Could revisit A.1 if a profiler shows premise iteration is a hot spot — currently no such evidence
- Closing this last item from the original scout-framework backlog
