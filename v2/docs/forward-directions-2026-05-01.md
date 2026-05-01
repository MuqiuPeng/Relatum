# Forward directions — observation + structured roadmap (2026-05-01)

After the consolidation cycle closed (12/12 punch list, retrospective
written, 593 lib tests, 73 ADRs), the user requested "观察并搭建未来
方向" — observe and scaffold what comes next. This document does
both halves.

It is **not an action plan**. It's a structured menu. Items are
organized so that the next "what do we do?" question has a
dependency-aware, empirically-grounded answer ready.

## Part 1 — Observation

### Re-run of the multi-substrate diagnostic

The diagnostic shipped 2026-05-01 morning produced ONE distribution.
After the same day's ADR 0072 Addenda 1+2, re-running produced a
DIFFERENT distribution:

| substrate | pre-addenda recommendations | post-addenda recommendations |
|---|---|---|
| OQ#1 | 2×None + 1×FamilyDemote + 1×Manual | 1×FamilyDemote + **3×Merge** |
| long5k | same as OQ#1 | same as OQ#1 |
| OQ#2 | 2×ShadowMonitor | 2×ShadowMonitor |

Three Merge recommendations now appear on each Signal-rich substrate:
- t_1 → Merge(t_2, Complementary) — Addendum 2 catches near-disjoint
- t_2 → Merge(t_3, HighQualityBoth) — Addendum 1 catches Signal-Signal
- t_3 → Merge(t_2, HighQualityBoth) — mirror of above

**Observation 1**: the empirical distribution of recommendations
shifted DRAMATICALLY across one design slice. Yet the runtime
behavior is byte-identical (no merges executed; recommendations
read-only). The system can now *describe* a much richer
maintenance plan without changing what it actually *does*.

**Observation 2**: the diagnostic's hardcoded sanity verdict (was:
"t_2/t_3 → None") is now stale. Sanity check fails 1/3 on OQ#1
and long5k; OQ#2 still passes via graceful degradation. This is
NOT a regression — it's an artifact of the post-addendum
expectations not being reflected in the test logic. **Cleanup
candidate.**

### Inventory of result docs (46 total)

| category | count | examples |
|---|---|---|
| Implementation-shipped | 30 | most B/D/F/G slices |
| Design-only / sketch | 8 | A.1, C.3 prep, E.2, E.3, G.5, G.8, H2.2, consolidation atlas |
| Diagnostic / verification | 6 | OQ#1 diagnostic, multi-substrate, atlas, etc. |
| Hybrid (impl + design notes) | 2 | F.5 verification, B.8.1 (lib promotion) |

**Observation 3**: 17% of results are design-only. This includes
the deepest research directions (C.3d unboundedness, H2.2 drive
synthesis, Alpha-2 MCTS). Sketches preserve the trajectory; they
don't ship runtime capability.

### Latent vs active mechanisms

What's IN the lib that ALSO FIRES at runtime:

| mechanism | status | how it fires |
|---|---|---|
| forward_apply_axiom | active | per-tick snapshot construction |
| family discovery (Beta-1) | active | scheduler dispatches DiscoverAxiomShapeFamilies |
| nested family discovery (B.6) | active (limited) | one-shot during scheduler integration |
| theory-quality scoring (Alpha-3+) | active | tournament logic per phase |
| DriveMix mutation | active | per-window A/B |
| H1.x composite dispatch | active | post-promotion |
| pattern naming | active | autonomous pass |
| meta-meta pattern discovery | active (occasional) | when pattern catalog matures |

What's IN the lib but **never fires** at runtime (only via examples):

| mechanism | status | who uses it |
|---|---|---|
| `recommend_intervention` (ADR 0072) | latent | examples + atlas only |
| `retract_shape_family` (ADR 0070) | latent | examples + B.2 only |
| `discover_nested_shape_families_by_member_overlap` | latent | B.8.1 example only |
| `theory_quality_report_all` (ADR 0071) | latent | diagnostics only |
| `merge_theories` | latent | F.5 example only |
| `family_quality` | latent | F.1.1 + diagnostics only |
| **(any G-series minting)** | **latent** | **G.1-G.7 examples only; no drive triggers** |

**Observation 4 (the big one)**: the consolidation triad (0070 +
0071 + 0072) produced a complete THEORY-MAINTENANCE LANGUAGE,
but the runtime doesn't speak it. Recommendations are read-only;
G-series minting is example-driven; merges happen only when an
example explicitly calls `merge_theories`. **The runtime has a
diagnostic vocabulary it can't act on.**

This is the dominant gap.

### What this observation reveals about the rhythm

The retrospective offered a research rhythm hypothesis:

```
accrete → consolidate → verify → cleanup → retrospective → next accretion
```

The empirical pattern from observation suggests an additional cycle:

```
accrete → consolidate → verify → cleanup → retrospective →
                                                ↓
                                       OPERATIONALIZE  ← the missing step
                                                ↓
                                          next accretion
```

"Operationalize" = take consolidated APIs and wire them into the
runtime loop so the runtime CONSUMES what was built. This is the
missing rung between consolidation and accretion.

## Part 2 — Forward-direction menu

Items organized by category. Each has:
- **ID**: stable handle (O = operationalize, E = empirical, T = theoretical, C = cleanup)
- **Maturity**: cleanup → impl-ready → design-ready → research
- **Precondition**: what would justify pursuing it
- **Scope**: XS (< 50 lines) / S (50-200) / M (200-500) / L (500+)
- **Dependency**: what needs to land first

### A. Operationalization (turning sketches into runtime behavior)

#### O1. Recommendation execution loop
- **Maturity**: design-ready (ADR 0072 §6 references "potential ADR 0073")
- **Precondition**: empirical demand for autonomous theory maintenance
- **Scope**: M (200-500 lines)
- **Dependency**: none beyond consolidation triad
- **Sketch**: scheduler, when in Reflect/Consolidate mode, calls
  `theory_quality_report_all` + `recommend_intervention` for each
  theory; dispatches the recommended action via existing APIs
  (`retract_shape_family`, `merge_theories`, etc.). New
  `ActionKind::ApplyRecommendedIntervention` or routing through
  existing kinds.
- **Risk**: closes the loop on automated theory rewriting; could
  thrash without empirical guards
- **Why this first if Operationalization**: the highest-leverage
  latent mechanism. Without it, the entire policy layer is
  diagnostic-only.

#### O2. G-series autonomy bridge
- **Maturity**: design-ready (G.5 + G.8 sketches ship full design)
- **Precondition**: substrate where mint-pressure naturally arises
  (e.g., a drive that demands new ids when prediction-error in
  a specific axiom family stays high)
- **Scope**: M-L (~300-700 lines, spread across drive + ActionKind +
  scheduler + tests)
- **Dependency**: ideally O1 first (so generative recommendations
  can be tested against same substrate)
- **Risk**: G-series minting is constrained-but-unbounded; without
  drive saturation logic, runaway minting possible

#### O3. C.3a empirical chain detection
- **Maturity**: design-ready (C.3 prep + chain predicate)
- **Precondition**: a chain-rich substrate (none today)
- **Scope**: S (engineer substrate) + S (run motif discovery + apply
  predicate)
- **Dependency**: substrate engineering (orthogonal slice)
- **Risk**: motif discovery may not naturally surface chains;
  could yield NULL result like B.8.1 originally did before
  the new L3 kind was added

#### O4. Multi-substrate diagnostic sanity-verdict update
- **Maturity**: cleanup
- **Precondition**: this ADR 0072 addenda are accepted as
  permanent (they are)
- **Scope**: XS (~30 lines)
- **Dependency**: none
- **Quick fix**: rewrite the diagnostic's hardcoded
  `t_2/t_3 → None` expectation to recognize the new
  Merge(HighQualityBoth) recommendations as correct.

### B. Empirical observation (using existing facilities)

#### E1. Long-run with policy execution active
- **Maturity**: implementation-ready ONCE O1 ships
- **Precondition**: O1 done
- **Scope**: S (just configure long-horizon run with policy on)
- **Dependency**: O1
- **Question to answer**: does the system reach a stable
  state or oscillate? After family demote, does new family
  discovery surface different families? After merge, does
  the merged theory get re-merged?

#### E2. Engineered substrates probing edge cases
- **Maturity**: implementation-ready
- **Precondition**: a hypothesis worth testing
- **Scope**: S each (one substrate + diagnostic)
- **Examples**:
  - Substrate where `noise_family_axiom_count` exactly equals
    `axiom_count / 2` (boundary case for Step 3 vs Step 6)
  - Substrate with subset+noise pattern (DemoteSuperset path)
  - Substrate where complementarity is borderline (Jaccard ≈ 0.50)

#### E3. Cross-version regression battery
- **Maturity**: cleanup tooling
- **Precondition**: 2+ ADR addenda land that change recommendation
  output (already true)
- **Scope**: M (~300 lines for systematic battery + comparison
  framework)
- **Dependency**: none
- **Idea**: capture each ADR/addendum's "expected output diff"
  on a battery of substrates; CI-check that future changes
  produce only intended diffs. Subsumes O4.

#### E4. Long-run divergence study (post-addendum)
- **Maturity**: implementation-ready
- **Precondition**: curiosity about whether DriveMix mutation
  is now stable
- **Scope**: S (existing long-run example + recompare to baseline)
- **Question**: have ADR 0072 Addenda 1+2 changed long-run
  metrics on F0 / OQ#1? They shouldn't (no execution path
  changed) but worth checking.

### C. Theoretical research (genuinely open questions)

#### T1. C.3d — unbounded type expression
- **Maturity**: research (theoretical hole)
- **Precondition**: a use case demands "unbounded" semantics
  (no use case today)
- **Scope**: ADR + new primitive (potentially `__unbounded__`
  marker) + constitutional review
- **Dependency**: C.3a + C.3b + C.3c all positive
- **Risk**: deepest theoretical question in v2. Adding a
  primitive that can't be FULLY witnessed by finite R facts
  is the kind of move that needs ADR-grade attention.
- **Why deferred**: no operational pressure; integer concept
  is approximated by finite chains, which are sufficient
  for current substrates.

#### T2. H2.2 — drive synthesis
- **Maturity**: research (E.3 sketch + detailed design)
- **Precondition**: existing 3 baseline drives prove insufficient
  on some substrate
- **Scope**: L (full grammar + synthesizer + lifecycle integration)
- **Dependency**: H2.1.2 (drive ESTABLISHED-promotion) for clean
  catalog management
- **Risk**: synthesis explosion; coevolution with MCTS if both
  active

#### T3. Alpha-2 — cognitive MCTS
- **Maturity**: research (H2.2_alpha2_design_sketch.md)
- **Precondition**: rule-based scheduler proven suboptimal on
  measurable axes
- **Scope**: M (Path 2 only) → L (Path 1)
- **Dependency**: stable value function (i.e., DriveMix not
  actively being mutated by H2.2)
- **Risk**: cost asymmetry; state-snapshot cloning at scale

### D. Cleanup (small structural tasks)

#### C1. Update multi-substrate diagnostic's sanity-verdict logic
- **Maturity**: cleanup
- **Scope**: XS
- **Same as O4 above**.

#### C2. Promote `is_chain_subgraph` to lib
- **Maturity**: cleanup (conditional)
- **Precondition**: C.3a empirical positive
- **Scope**: XS (move + 1 unit test)
- **Dependency**: O3 (or its substitute)

#### C3. ADR status audit
- **Maturity**: cleanup
- **Scope**: S (~3-4 hours of diff-style status review)
- **Idea**: many ADRs say "Proposed" in their headline but ship
  via addenda. Audit each ADR's actual implementation status
  and update headlines to "Accepted (via Addendum N)" for
  clarity.

#### C4. Migrate the 9 historical examples to the modern API
- **Maturity**: cleanup (post-migration-atlas)
- **Scope**: M (each example becomes ~50 lines instead of
  ~300; total ~500 lines net deletion)
- **Why deferred from migration atlas**: the atlas demonstrated
  feasibility; actual migration is mechanical work, low priority.
- **Dependency**: none (atlas already proves correctness)

### E. Mode-shift triggers

The retrospective offered "stop and observe". Concretely, the
following empirical observations would justify resuming the
**accretion cycle** (as opposed to operationalization):

1. **The runtime gets stuck in a ShadowMonitor / Manual loop**
   on a non-trivial substrate. Means: data dimensions are
   genuinely missing; new measurement is needed.
2. **A new substrate produces theories that don't fit any
   intervention pattern**. Means: the 0072 decision tree is
   incomplete for that substrate's structure.
3. **The recommendation distribution becomes degenerate** (all
   None, or all Manual). Means: the classifier's thresholds
   are wrong for the new setting; threshold tuning ADR
   needed.
4. **A use case demands unbounded type expression**. Means:
   C.3d goes from theoretical-hole to load-bearing.
5. **Drive signal saturates and nothing fires**. Means:
   existing drives don't capture the relevant pressure;
   H2.2 becomes empirically motivated.

Until ONE of these triggers, the right work is in
**Operationalization** (A) and **Cleanup** (D).

## Part 3 — Recommended sequencing

If the user resumes work tomorrow, the natural order is:

1. **C1 / O4** (sanity-verdict cleanup) — XS, makes future
   diagnostic runs self-validating again.
2. **O1** (recommendation execution loop) — M, the highest-
   leverage latent capability. Closes the loop ADR 0072
   intentionally left open. After O1, the runtime can
   maintain its own theory layer.
3. **E1** (long-run with policy execution) — S, observes O1's
   real behavior. Either confirms stability or surfaces
   thrash.
4. **E2** (engineered edge-case substrates) — S each, builds
   coverage for the policy layer's boundary cases.

Items O2 (G-series autonomy), T2 (H2.2), T3 (Alpha-2) are all
**deferred until operational evidence justifies them**. The
"stop and observe" period is precisely the time when this
evidence accumulates.

## Part 4 — What this document is NOT

- **Not a commitment**. The user retains full authority over
  which (if any) of these directions to pursue.
- **Not a sprint plan**. Items are independent; pick any subset
  in any order.
- **Not exhaustive**. Future observation may surface directions
  not enumerated here. The roadmap is a snapshot, not the
  whole map.
- **Not a deadline structure**. v2 is a research project;
  cadence is set by what's interesting, not by a calendar.

## Closing

The consolidation cycle closed cleanly. The forward menu has
17 items across 4 categories, each with maturity / precondition /
scope / dependency annotated. 5 mode-shift triggers identified.

The biggest empirical observation: **the runtime has a complete
theory-maintenance language but doesn't yet speak it to itself.**
Operationalization (the O cluster) is the natural next phase.

Whether to enter that phase, or stay in observation, is the
user's call. This document is the menu.

---

*Author's note: Producing this document IS itself the
"observation phase" the retrospective recommended. Looking at
what was built — what fires vs sits — is what generates
empirically-grounded direction. The 6 hours between the
retrospective and this document is exactly the kind of pause
that the rhythm was missing.*
