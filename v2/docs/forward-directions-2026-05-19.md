# Forward directions — 2026-05-19 snapshot (supersedes 2026-05-01)

Updates the 2026-05-01 forward-directions menu with status changes from the May 11-19 sustained autonomous session (21 commits, 4 major outcomes). The earlier document remains accurate for its own date; this one captures the new state.

## What changed since 2026-05-01

**Empirical thread — Phase 1.D substrate-sensitivity arc** (commits `aa21ded` → `c4c8ac9`):
- Original "v2 substrate-distinct emergent abstraction" claim **retracted** via 9-round ARIS auto-review-loop + post-loop scans.
- Surviving narrow positive: v2 distinguishes structurally-constrained classes (BIPARTITE, pure tree) from random graphs at sizes 3-4. Classical subgraph census, not emergent abstraction.
- Phase 1.E real Mathlib remains the only test that could change the picture.

**Runtime thread — Phase Emergence operationalization** (commits `48c007d` → `a8d0c8e`):
- ADR 0080 LP-threshold tuning + prune-loop fix: 3k OQ#2 from hangs → 1.9 min. ADR 0079.1 baseline preserved.
- ADR 0082 (theory policy loop): designed + implemented + empirically verified. Runtime autonomously demoted t_0 on OQ#1 at tick=511. v2 milestone — runtime maintains its own theory layer end-to-end.
- ADR 0083 (pattern policy loop, mirror of 0082): designed + implemented + targeted verification. Both PatternRetract (Anomalous) and PatternMergeWith (Redundant) executable.
- 650 lib tests pass; 0 warnings.

## Updated direction menu (now-vs-then)

### A. Operationalization (from 2026-05-01)

#### O1. Recommendation execution loop
- **2026-05-01 status**: design-ready.
- **2026-05-19 status**: ✓ **DONE** — ADR 0082 implementation shipped in commit `b8f5954`; long-horizon stability verified in `f946b72`.

#### O2. G-series autonomy bridge
- **2026-05-01 status**: design-ready (G.5 + G.8 sketches).
- **2026-05-19 status**: still design-ready. **Largest remaining bounded work in current architecture.** ADR 0082's mechanism (PolicyTarget + execute_action arm + cooldown filter) provides a template.

#### O3. C.3a empirical chain detection
- Unchanged. Still pending; requires a chain-rich substrate.

#### O4. Multi-substrate diagnostic sanity-verdict update
- Already done (per Round 2 directions update).

### B. Empirical observation

#### E1. Long-run with policy execution active
- **2026-05-01 status**: pending O1.
- **2026-05-19 status**: ✓ DONE — `logs/2026-05-11_adr0082_oq1_6k_stability.log` shows 4 theories → 3 (post-t_0-demote) → 6 (with new discovery), stable through tick 2400. No thrash; no further policy fires after the initial t_0 retraction.

#### E2. Engineered substrates probing edge cases
- Unchanged. Targeted Anomalous test (commit `c4c8ac9`) is one instance of this.

### C. Theoretical research

#### T1. C.3d unbounded type expression — unchanged.
#### T2. H2.2 drive synthesis — unchanged.
#### T3. Alpha-2 cognitive MCTS — unchanged.

## New direction menu items (added 2026-05-19)

### N1. Phase 1.E — real Mathlib ingestion

- **Maturity**: research (external data + ETL needed).
- **Precondition**: substrate-sensitivity claim needs natural-data evidence to revive.
- **Scope**: L (1-3 weeks: download, parse, ETL, runtime, analysis).
- **Dependency**: none beyond standard tooling.
- **Risk**: Mathlib's dep structure may fall into the "tree-like / sparse" zone where v2 cannot distinguish from random (Round 6 TREE × DAG = 0.78). Negative result is still publishable.
- **Why important**: the ONLY remaining experiment that could revive the substrate-sensitivity claim post-9-round retraction.

### N2. Substrate generation in runtime (cross-precision-aware policy)

- **Maturity**: design-ready.
- **Precondition**: ADR 0082/0083 are shipped but operate with empty substrates → cross-precision degrades to None → FamilyDemote path in 0082 is unreachable; Anomalous detection in 0083 narrows to instance_count=1.
- **Scope**: M (200-500 lines: generate substrates on a longer cadence, cache, pass to recommend_intervention).
- **Dependency**: substrate generation has known performance cost (BA-scaling explosion at n=80 size=4).
- **Risk**: per-refresh generation likely too expensive; cadence tuning required.

### N3. merge_patterns lib API (true structural merge)

- **Maturity**: design-needed.
- **Precondition**: ADR 0083 PatternMergeWith currently maps to "retract self." A true structural merge (e.g., move self's unique instances to partner) would preserve coverage.
- **Scope**: S-M (50-200 lines).
- **Dependency**: none.
- **Risk**: pattern merge semantics need careful design — different canonicals can't structurally merge, only their instance sets can.

### N4. v1 vs v2 benchmark battery

- **Maturity**: implementation-ready.
- **Precondition**: v1 phase 1-9 has 935 models / 10 axiom classes. v2 hasn't been compared.
- **Scope**: M-L (2-4 weeks: port v1 tasks, run on v2, compare).
- **Dependency**: none beyond v1 task descriptions.
- **Why important**: establishes v2's relative position vs v1's closure-engine architecture. Benchmark paper candidate.

### N5. BA-scaling fix for v2 discovery

- **Maturity**: design-needed.
- **Precondition**: Phase 1.D Round 5 documented v2's `autonomous_pass` taking 38 min on BA-style hub-rich graph at n=80 size=3, and 72 min at n=80 size=4 on synth-DAG. Power-law / large-dense graphs are computationally intractable.
- **Scope**: ADR-grade investigation (smarter subgraph sampling on hub structures).
- **Dependency**: only matters if power-law-graph substrates become targets (real-world data often has hub structure).
- **Risk**: deep algorithmic work; may require fundamental change to sampling strategy.

## Recommended sequencing

If a focused session resumes:

1. **N1 (Phase 1.E real Mathlib)** — highest scientific value remaining. Requires explicit user commitment to multi-week investment.
2. **O2 (G-series autonomy bridge)** — second-highest leverage among shipping work. Designed; needs implementation.
3. **N2 (substrate generation in runtime)** — bounded improvement to ADR 0082/0083 engagement quality. Medium scope.
4. **N4 (v1 vs v2 benchmark)** — produces a benchmark paper candidate. Medium-large scope.

Items O3 (C.3a chain detection), N3 (merge_patterns), N5 (BA scaling), T1-T3 (theoretical) are all **deferred until operational evidence justifies them**.

## What this document is NOT

- Not a commitment — the user retains direction.
- Not a sprint plan — each item is independent and pickable in any order.
- Not exhaustive — implementation of any item may surface new directions not enumerated here.

## Closing

The 2026-05-01 menu's biggest item (O1 recommendation execution loop) is now DONE in two parts (ADR 0082 theory; ADR 0083 pattern). The current biggest remaining shipping item is O2 (G-series autonomy bridge). The current biggest remaining empirical item is N1 (Phase 1.E real Mathlib).

v2's runtime now maintains its own consolidation layer end-to-end on both knowledge types (theory + pattern). The next-level capability requires either external data (N1) or constructive cognition (O2). Both need explicit user direction.
