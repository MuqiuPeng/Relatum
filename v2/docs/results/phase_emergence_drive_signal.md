# ADR 0078 — Pattern-aware drive signal audit

**Status**: ✓ shipped (2026-05-07); confirms drive metric reveals untapped substrate
**Log**: [`logs/2026-05-07_phase_emergence_drive_signal.log`](../../logs/2026-05-07_phase_emergence_drive_signal.log)
**Example**: [`examples/phase_emergence_drive_signal.rs`](../../examples/phase_emergence_drive_signal.rs)
**ADR**: [0078 — Pattern-aware drive metric](../decisions/0078-pattern-aware-drive-metric.md)

## Goal

The 2026-05-06 long-horizon observation showed v2's runtime
sleeps permanently after the ~250-tick initialization phase.
ADR 0078 specified a constitution-compliant drive metric
(unexplained R organized by connected-component canonical
form) to measure whether unexplored structure remains.

This is the metric's first audit on the canonical substrates.

## What shipped

### Library (`src/lib.rs`)

- `DriveCanonicalBucket` struct — one canonical-form bucket
  with component count, total edge count, and ≤ 5 example
  edges
- `UnexplainedDriveSignal` struct — full drive signal with
  total/unexplained counts, ratio, sorted buckets, modal
  canonical, distinct count
- `RSet::unexplained_drive_signal()` — pure read-only
  computation

The bucket key is the **subgraph canonical form** (ADR 0009)
of each connected component of unexplained R. No per-token
signatures; no IdentifierProfile / LocalityProfile lookups.
Constitution-heavy-reading-compliant.

### Tests

5 new ADR-0078 tests in `src/tests.rs`:

- `adr0078_drive_signal_empty_rset_zero_signal`
- `adr0078_drive_signal_all_unexplained_one_bucket`
- `adr0078_drive_signal_isomorphic_components_merge_buckets`
- `adr0078_drive_signal_distinct_shapes_separate_buckets`
- `adr0078_drive_signal_example_edges_capped_at_5`

Lib tests: 637 → **642**, 0 regressions.

### Example

`phase_emergence_drive_signal.rs` runs each canonical substrate
to maturity and prints the drive signal before/after manually
invoking `autonomous_pass(sizes 2-5)`.

## Result

### OQ#1 (1000 ticks to maturity)

```
Phase 0 state: 11 axioms, 3 theories, 1 patterns

Drive at maturity (post Phase 0)
  total data edges: 75, unexplained: 30 (40.0%)
  distinct canonicals: 1, modal-bucket count: 5
  #1: 5 components × 30 edges, canonical size 6
      e.g. R(bL9, bR15), R(bL10, bR14), R(bL9, bR13)
```

40% of OQ#1's data edges are unexplained at maturity. They form
**5 isomorphic 6-edge components** all carrying the same
canonical: bipartite-regime subgraphs (`bL` / `bR` token
prefixes from OQ#1's stream). The runtime had axioms for the
diamond-poset regime but none for bipartite — those edges sit
uncovered.

After `autonomous_pass(sizes 2-5)`, drive drops to **0**. The
emergence kernel can extinguish the drive completely, given
the chance.

### narrow_a (500 ticks to maturity)

```
Phase 0 state: 11 axioms, 3 theories, 1 patterns
Drive at maturity:  unexplained: 0 (0.0%) — drive is silent
```

narrow_a's stream is pure diamond posets, fully covered by the
axioms discovered during Phase 0. Drive is silent — no
structural pressure for further mining.

### OQ#2 (4500 ticks to maturity) — the headline result

```
Phase 0 state: 2 axioms, 2 theories, 2 patterns

Drive at maturity (post Phase 0)
  total data edges: 164, unexplained: 149 (90.9%)
  distinct canonicals: 5, modal-bucket count: 5
  #1: 5 components × 45 edges, canonical size 9
      e.g. R(s2_l3, s2_hub), R(s2_l0, s2_hub), R(s2_l2, s2_hub)
      ← star-regime (hub of degree N)
  #2: 5 components × 45 edges, canonical size 9
      e.g. R(l2_b, l2_m1), R(l2_m1, l2_t), R(l2_b, l2_m2)
      ← lattice-regime (bot/mid/top elements)
  #3: 2 components × 28 edges, canonical size 14
      e.g. R(t4_1, t4_3), R(t4_3, t4_4)  ← tournament-regime
  #4: 1 component × 16 edges, canonical size 16
  #5: 1 component × 15 edges, canonical size 15
```

**91% of OQ#2's data edges are unexplained at maturity.** The
drive signal organizes this into 5 distinct canonical shapes,
each corresponding precisely to one of OQ#2's stream regimes:

| bucket | shape | regime |
|---|---|---|
| #1 | hub of degree N | star |
| #2 | bot → m1/m2 → top | lattice |
| #3-5 | larger tournament motifs | tournament |

The runtime currently leaves all of this unexamined and goes
to sleep. After `autonomous_pass(sizes 2-5)`, drive drops to
**0** — every one of the 149 unexplained edges gets absorbed
by minted patterns or refined coverage.

## What this confirms

1. **The drive metric is informative.** It correctly reveals
   that OQ#2 has substantial untapped structural content
   that the runtime is not addressing.
2. **The metric's modal pointer is actionable.** The example
   edges in each bucket name specific token instances that
   downstream mechanisms (`autonomous_pass` with the modal
   canonical's size, or a new drive-targeted dispatch) could
   focus on.
3. **`autonomous_pass` already has the capability** to handle
   what drive surfaces — it extinguishes drive to 0 in all
   measured cases. The gap is in *triggering*, not in
   capability.

## What this exposes about v2's current behaviour

The long-horizon observation found OQ#2 stayed at 2 axioms / 2
theories / 2 patterns for 6000 ticks, with episodes count
flat at 10 throughout. This audit explains why precisely:

- The runtime exhausts its frontier proposals during Phase 0
  initialization (axioms named, basic patterns minted)
- After that, `RuleBasedScheduler::has_expand_work` reports
  no further work
- But 91% of substrate edges remain structurally unaccounted
- The emergence kernel could mint patterns for these (proven
  by the post-pass measurement), but no scheduler logic
  proposes them

The drive signal makes this gap quantitative. **Without drive
integration, the runtime is provably under-using its own
capabilities on OQ#2.**

## Constitution compliance check

- Bucket key: canonical form of each connected component of
  unexplained R. Subgraph-level (depends on ADR 0009 WL
  refinement applied to component-internal edges only).
- No `IdentifierProfile`, `LocalityProfile`, or
  `EdgeFingerprint` lookups.
- Tokens appear in `example_edges` for human readability but
  are not used as classification keys — two components with
  different tokens but the same canonical merge into one
  bucket, as the OQ#1 audit shows (5 components, same
  canonical).
- Pure read-only computation; rset is never mutated.

The withdrawn 2026-05-06 first-form drive (which used per-edge
fingerprints) is replaced with a strictly stronger signal that
satisfies the constitution heavy reading.

## What this slice does NOT ship

- **Scheduler integration**. Computing drive does not change
  runtime behaviour. The runtime still sleeps when the
  frontier is empty, regardless of the drive signal. A
  follow-up ADR will specify how to consume drive — likely
  options: gate `should_sleep` on drive, propose a new
  `FrontierKind::DriveTargeted` item, or feed the modal
  canonical into a future `DiscoverPatternsTargeted` action.
- **Threshold calibration**. The metric is a structured
  report; no scalar threshold for "drive high enough to keep
  awake." The integration ADR decides this.
- **Drive-targeted dispatch**. The modal canonical gives a
  pointer; using it as a target requires extending the
  dispatch path. Future work.

## Files

- `src/lib.rs` — types + 1 RSet method
- `src/tests.rs` — 5 new tests (637 → 642)
- `examples/phase_emergence_drive_signal.rs`
- `logs/2026-05-07_phase_emergence_drive_signal.log`
- `docs/decisions/0078-pattern-aware-drive-metric.md`
- This result doc

## Verdict

**ADR 0078 ships a constitution-compliant drive metric that
reveals OQ#2 has 91% unexplained structural content the
runtime currently never examines.** The metric organizes the
unexplained edges into 5 canonical buckets corresponding
exactly to OQ#2's stream regimes (star, lattice, tournament),
giving the next-step scheduler integration a precise pointer
to consume.

Capability is not the gap — `autonomous_pass` extinguishes
drive to 0 when invoked. The gap is *triggering*: the runtime
needs to know, while sleeping with empty frontier, that drive
is non-zero and that work remains.

Closing the gap is the next step (scheduler integration). With
the metric in place, that step has a concrete signal to act
on rather than a hand-tuned threshold scan.
