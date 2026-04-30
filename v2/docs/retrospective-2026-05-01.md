# v2 retrospective — 2026-05-01

Three days after the 2026-04-27-late retrospective. Two
qualitatively distinct phases happened in those three days:

- **2026-04-28 → 2026-04-30 — the heap phase**: 4 rounds of
  directions-sweep workflow, 40 directions executed, structural
  vocabulary extended (Beta-1, B.6, B.7, B.8.1) and identifier
  space opened (G.1-G.7).
- **2026-04-30 → 2026-05-01 — the consolidation phase**: user's
  strategic critique reframed work mode from "add" to "consolidate"
  — three ADRs (0070 / 0071 / 0072) formalized scattered
  capabilities into a layered system. Migration atlas + multi-
  substrate diagnostic verified the consolidation end-to-end.

This retrospective documents both phases, their relationship, and
what's left.

## Recap from 2026-04-27-late

The previous retrospective closed with H2.0 step 3a (combined-
signal observability) shipped. H2.0 step 3b was unimplemented;
H2.1 sketched but unbuilt. The runtime had a self-tuning
evaluation loop in shadow mode but not yet load-bearing.

Three days later: H2.0 step 3b α landed (commit `e739b7d` upstream),
H2.1.0 + H2.1.0+ shipped (drives as meta-R; query rewire), and
all of the Alpha series (Alpha-1 through Alpha-9, theory tournament
+ dream phase + cross-precision) landed. F0 verdict on
stream_diamond stayed STILL GROWING throughout.

## Phase 1 — The heap (Round 1-4 directions sweep)

The user established the directions-sweep workflow on 2026-04-29:

> 创建一个 md 用于记录可行方向，依次执行，执行过程中发现新方向就添加入这个文档，每完成 10 个方向停下汇报

This produced a steady cadence: 10 directions per round, each
landing as one or more positive empirical findings. Cross-cutting
results across the four rounds:

### Round 1-2 (2026-04-29) — structure layer extension

| direction | what landed |
|---|---|
| B.2 | family-level demote (Beta-1's families gain decision power) |
| B.3 | conclusion families |
| B.4 | family-aware enumeration |
| B.5 / B.5.1 | scheduler integration of family discovery |
| B.6 | nested families (L3) |
| B.7 | super-meta families (L4) |
| C.1 / C.2 / C.2.1 | substrate cleanup + cross-substrate validation |
| D.2-D.5 | predicate enforcement, composite signals, dream loop, signal disagreement |
| E.1 / E.2 | drive meta-R verification |
| F.1-F.4 | merge selectors + family quality |

### Round 3 (2026-04-29-30) — identifier-layer extension

The G-series broke a year-old constraint: previously v2 could only
DESCRIBE identifiers handed to it. After G.1-G.4, it could MINT
new identifiers via deterministic recipes and reason over them
through the existing axiom processor.

| direction | what landed |
|---|---|
| G.1 | identifier minting POC (5-step Peano chain) |
| G.2 | minted output integrates with forward_apply_axiom — no lib change |
| G.3 | ADR 0069 — generative-axiom contract (4 properties) |
| G.4 | predicate-compliance metric (cross-precision analog for generative) |

### Round 4 (2026-04-30) — integer scaffold + merge safety

| direction | what landed |
|---|---|
| G.6 | addition recipe (multi-arity generative) |
| G.7 | integer arithmetic embedding scaffold (Peano "<" via transitivity) |
| G.5 / G.8 | drive + ActionKind sketches for autonomy bridge |
| F.5 | empirical merge safety: F.4's pick verified lossless |
| B.8.1 | new L3 kind lifted L5 ceiling 0 → 8 |
| I.1 / I.2 | cross-substrate transfer (strong on same regime, partial on hostile) |

### What the heap produced

By 2026-04-30 evening:
- **6 abstraction layers**: L0 data → L1 axioms → L1.5 theories →
  L2/L3/L4 shape families
- **40 result documents** in `docs/results/`
- **~50 examples** in `examples/`
- **~70 ADRs** (final number 73 by 2026-05-01)
- 568 lib tests passing, all green

### The risk the heap created

Each direction was a positive slice in isolation, but cumulatively
they were drifting toward an experiment heap rather than a system.
Specifically:

- **9 examples** (Alpha-3, Alpha-3+, Alpha-3++++, Alpha-5, Beta-2,
  F.2, F.2.1, F.4, F.5) each rolled their own theory-quality
  classification logic. Total: ~2700 lines of inline code
  reimplementing the same primitives.
- **Shape-family layer** (Beta-1 + B.2 + B.4 + B.5 + B.6 + B.7 +
  B.8.1 + F.1.1) had grown into an obvious cognitive layer but no
  ADR documented it as such.
- **Cross-precision** (Alpha-7 + F.1) was an excellent diagnostic
  signal but unclear whether it was diagnostic-only or a decision
  signal.
- **Theory intervention** had at least 6 distinct mechanisms
  (theory demote, repair, naive merge, smart merge, family demote,
  subset-aware demote) with no unifying policy.

The user named this directly in their 2026-04-30 strategic
critique:

> 之后的关键问题不在"还能加什么机制"，而在 **哪些机制应该升级成 v2 的正式认知层**。
> ... 继续加方向会让系统变成实验堆；但如果能把这些结果收束成几条主干，
> Relatum 会从"实验原型"变成"有理论结构的系统"。

## Phase 2 — Consolidation

The user's critique enumerated 6 numbered concerns and ordered
them by priority. The highest-priority three became ADRs.

### ADR 0070 — Shape-family abstraction layer (3 steps)

Promoted scattered family code (Beta-1 + 7 follow-ups) into a
formal cognitive layer with:
- `FamilyLayer { L2, L3, L4 }` enum
- `family_layer / family_members / family_kind / family_quality`
  unified query API
- `retract_shape_family` lifted from B.2 inline → lib + ActionKind
- `discover_nested_shape_families_by_member_overlap` lifted from
  B.8.1 inline → lib
- 5 KIND constants emitting kind-tag edges during discovery

3 commits (3e5f5e4 / 3011514 / 2544036). 14 ADR-0070-specific
unit tests. 0 behavior regressions.

### ADR 0071 — Unified theory-quality report

Aggregated 7 scattered theory-quality signals into `TheoryQualityReport`:
- primary hit rate (mean / min / qualifying count)
- cross-precision (mean / min / max / qualifying count)
- family memberships with per-family quality classification
- noise / signal axiom counts
- structural neighborhood (subset / parallel / etc.)
- 4-class summary: Signal / Mixed / Noise / **Indeterminate**

The Indeterminate class proved load-bearing on OQ#2 (no template
axioms, all primary stats absent). Without it, missing data would
default to Noise → wrong TheoryDemote recommendations.

1 commit (6619d9a). 8 ADR-0071-specific unit tests.

### ADR 0072 — Intervention policy classifier (+ 2 addenda)

The policy layer. Consumes a `TheoryQualityReport` plus other
theories' reports; produces a `RecommendedIntervention` enum
covering 8 outcomes (None / ShadowMonitor / FamilyDemote /
AxiomRepair / TheoryDemote / DemoteSuperset / Merge / Manual).

7-step decision tree implementing the user's literal pseudocode
from the strategic critique:

```
if focal extends Signal subset:    DemoteSuperset
if has noise family:                FamilyDemote
if Mixed + few weak axioms:         AxiomRepair
if Mixed + Signal complementary:    Merge(Complementary)
if Noise:                            TheoryDemote
else:                                Manual
```

Addendum 1 (HighQualityBoth merge) and Addendum 2 (near-disjoint
Jaccard) followed after the migration atlas surfaced 4 conservative-
by-design divergences. F.5's empirical safety result was the
green-light for the addendum.

3 commits (3e34c22 / df52f59 / part of df52f59). 15 ADR-0072
unit tests including priority-order verification.

### Diagnostic + verification slices

- **OQ#1 single-substrate diagnostic** (e739b7d): end-to-end
  validation; classifier produces all expected recommendations.
- **Multi-substrate diagnostic** (6dacec5): same expected
  recommendations on long5k (same regime types), graceful
  degradation on OQ#2 (0 template axioms → ShadowMonitor only).
- **Migration atlas** (0e6e211): 9 historical examples' decisions
  reproduced via the modern API. After Addenda 1+2, **9/9**
  positive (8 reproductions + 1 correctly-falsified).

### Adjacent slices

- **H2.1.1 shadow cleanup** (3f025d0): lifecycle-survival fix
  for `register_drives_in_rset`. Manual penalty-marker
  retractions now survive checkpoint round-trip — unblocks
  future H2.1.2 demotion work.
- **Forward-apply premise scheduling** (ebbd966): 2.4% wall-clock
  improvement; cumulative forward_apply optimization tally now
  ~50% over naive baseline.
- **C.3 integer construction prep** (44dd447): design doc framing
  C.3a-d as sequenced research path; chain-pattern predicate
  ships positive on G-series output.

## Distance to original goal

The v2 goal:
> Under intrinsic drive, construct from R instances new relations
> that explain new phenomena.

Today's status, dimension by dimension:

| dimension | status |
|---|---|
| Construct first-order relations from R | yes (patterns / axioms / theories) |
| Construct meta-relations | yes (theory extension / parallel / family memberships) |
| Construct meta-meta-meta-relations | yes (L4 super-meta families on OQ#1) |
| Sustain construction past compression equilibrium | yes (G1.5 outward drive + STILL GROWING) |
| Self-tune scheduler parameters | yes (H0 MetaScheduler) |
| Self-tune drive weights | yes (H2.0 DriveMix) |
| Mint new operational mechanisms | yes (H1 composite ActionKinds) |
| **Mint new identifiers** | **yes (G-series, post-2026-04-29)** |
| **Detect chain structure in data** | partial (predicate works; motif discovery integration deferred to C.3a) |
| Recommend interventions over a unified policy | **yes (ADR 0072 + addenda; 9/9 historical agreement)** |
| Express genuinely-unbounded types | NO (C.3d theoretical hole) |
| Self-modify drive functions (synthesis) | NO (H2.2 deferred research) |

The unbounded type expression (C.3d) and drive synthesis (H2.2)
are the two genuinely-unsolved theoretical questions. Everything
else has either landed or has a clear path.

## What surprised me along the way

### The user's strategic critique was the load-bearing event

Up to 2026-04-30, the natural mode was "more directions". The
critique's reframing — "consolidate, don't add" — converted 4
rounds of disparate work into a layered system in 5 days. **The
critique itself was a higher-leverage contribution than any
individual slice.** Without it, the consolidation wouldn't have
happened, and the experiment heap would have continued growing.

This is worth noting because it's not a code or design contribution
— it's a research-direction contribution. The lesson: the
"continue producing positive slices" default is not always
correct; sometimes the right move is to stop and consolidate.

### Indeterminate is a real category

The 4-class summary (Signal / Mixed / Noise / Indeterminate) was
introduced in ADR 0071 as a defensive design choice. I expected
Indeterminate to rarely fire in practice. Instead, OQ#2's
hostile substrate ENTIRELY occupied the Indeterminate class —
every theory there had no qualifying primary or cross data. The
distinction between "no signal" and "signal saying no" became
empirically load-bearing.

A 3-class scheme (Signal / Mixed / Noise) would have produced
TheoryDemote recommendations on OQ#2's perfectly-honest "no
data" situation. Indeterminate prevents that.

### The migration atlas's 5/9 → 9/9 progression

When the migration atlas first ran (post-Step 2 of 0072), it
showed 5/9 agreement with 4 open divergences. All 4 were Signal-
Signal merge cases. F.5's empirical data (lossless verified
merge) was already in the codebase; it was a small step to add
the Addenda 1+2 that closed the 4 divergences.

Lesson: **historical empirical data accumulates as latent
signal**. Once the consolidation made the recommendation logic
explicit and queryable, the divergences became findable.
Without the consolidation, F.5's data was just one example
among many; with it, F.5 was the precedent that justified a
specific follow-up rule.

### Forward-apply optimization plateaued

A.1 had verified-deferred premise reordering on 2026-04-29. The
2026-05-01 premise-scheduling slice yielded a real but modest
2.4% improvement. Cumulative forward_apply work now totals
~50% over naive baseline. **Performance is no longer the
binding constraint** — the constraint has moved to snapshot
construction + prediction-state HashMap operations. Further
forward-apply optimization is diminishing-returns territory
until a workload (MCTS, large rollouts) demands it.

This is honest news: the system is "good enough by 50%" on its
load-bearing kernel. Effort should now go into other layers.

## What still doesn't exist

- **Genuinely-unbounded type expression** (C.3d). Finite R facts
  cannot express "this chain extends without limit". An
  `__unbounded__` primitive would need constitutional review;
  even then the SEMANTICS of "extends without limit" can't be
  fully witnessed in finite R.
- **Drive synthesis** (H2.2). The runtime can mutate drive
  weights (H2.0) and lift drives to meta-R (H2.1), but cannot
  synthesize new drive function bodies from primitive metrics.
  Research direction; not blocked by anything but design.
- **Recommendation execution loop**. ADR 0072 recommends
  interventions; nothing executes them automatically. A future
  ADR 0073 (potential) could close the loop: tick → identify
  struggling theories → fetch reports → recommend → execute →
  verify.
- **Substrate engineering for chain-rich data**. C.3a's
  empirical question requires a substrate that motif discovery
  would naturally surface as chains. None of OQ#1 / long5k /
  OQ#2 qualify.

## Open architectural questions

1. **When does C.3d's unboundedness become a constraint?** The
   integer concept's defining feature is unbounded extension.
   v2 has no unbounded primitive. A future ADR might:
   - Add `__unbounded__` marker (constitutional review needed)
   - Or leave unboundedness implicit (if `R(__type__, T) AND
     R(T, __chain__)` is enough, no new primitive needed)
   - Or accept that integer-as-type is a finite approximation
     ("chain of length N for some unspecified large N")
2. **Is ADR 0072's Manual recommendation diagnostically
   informative or a punt?** On OQ#1, t_1 → Manual because
   primary 0.59 < 0.60 repair gate. The threshold is one
   tunable choice among defensible alternatives. A future
   ADR could codify "explain the Manual reason in
   policy-aware language" — beyond the current generic
   "no specific intervention pattern matched".
3. **Does H2.1.2 (drive ESTABLISHED-promotion) need to land
   before H2.2 (synthesis)?** The current H2.1 is registration +
   query routing. H2.1.2 would tie drive weight to ESTABLISHED
   status, gating active mix membership on empirical
   contribution. H2.2 wants to synthesize new drive bodies —
   they need a lifecycle home. H2.1.2 is the natural home.

## Distance covered, in one sentence

5 days ago: "v2 has 9 alpha-phase positive slices on theory
maintenance; the system can score, demote, repair, and merge
theories ad hoc."

Today: "v2 has a 3-layer consolidated theory-maintenance system
(structural / observation / policy) with empirical 9/9 historical
consistency, multi-substrate validation, identifier minting (G-
series) for the constructive half, and explicit prep for the
detective half (C.3a-d)."

The transition was the user's 2026-04-30 strategic critique.
That single message had higher leverage than any of the code
slices it triggered.

## Next directions

In rough priority:

1. **Stop and observe**. The consolidation just landed. Real
   empirical work on the new APIs (running them across novel
   substrates, checking for bugs the test suite missed) before
   adding more direction. The previous retrospectives never
   built in pause time; this one explicitly recommends it.
2. **Recommendation execution loop** (potential ADR 0073) —
   when there's empirical demand. Not before.
3. **C.3a empirical** — engineer a chain-rich substrate, run
   motif discovery + name_pattern_instances, classify outputs
   via the predicate. Real test of whether existing machinery
   surfaces chains.
4. **H2.1.2 drive lifecycle** — small slice if H2.2 territory
   becomes pressing.
5. **H2.2 drive synthesis** — research direction, low priority.

## Author's note

This is the first retrospective written AFTER a self-imposed
mode shift. Previous retros documented work that just happened;
this one documents work that happened BECAUSE of a mode shift.

The mode shift was:
- before: "what direction to add next?"
- after: "what mechanism to consolidate next?"

Both modes are productive. The wrong default is to stay in one
forever. Round 1-4 were correct mode at the time. The 2026-04-30
critique was correct timing for the shift.

The work since 2026-04-23 (v2 birth) totals ~75 ADRs, ~600 lib
tests, ~60 examples, ~50 result documents. The architectural
spine — the (S, A, T, V, D, C) cognitive game framing in
`cognitive-game-framing.md` — has stayed stable across the
phases. Code accreted; the framing didn't drift.

That's the property worth preserving: keep the framing stable,
let the implementation accrete + consolidate in cycles. This
retrospective marks the close of the second consolidation cycle
(the first was the H1 → H2 transition documented in the prior
retros).

A natural rhythm seems to be emerging:
- accrete (sweep / experiment) → consolidate (ADRs) → verify
  (diagnostics) → consolidate-cleanup (atlas / shadow) →
  retrospective → next accretion phase

If that rhythm holds, the next phase will be accretion again
— but only when the consolidated system has been observed long
enough to surface the NEXT round of "what's missing".

Stop and observe is the right immediate next step.
