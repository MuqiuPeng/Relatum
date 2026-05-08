# 0079: Drive-driven frontier candidate (sustained cognition)

Status: Proposed
Date: 2026-05-08

Parents:
- [0078 — Pattern-aware drive metric](0078-pattern-aware-drive-metric.md)
- [0075 — Emergence kernel audit](0075-emergence-kernel-audit-and-runtime-integration.md)

## Context

ADR 0078 ships the drive metric — `unexplained_drive_signal()`
returns canonical-form-bucketed unexplained R as an attention
pointer. The metric works (extinguishes to 0 when
`autonomous_pass` is invoked manually) but is not consumed by
the scheduler. Two empirical artifacts establish the gap:

1. **Long-horizon observation (2026-05-06)**: v2's mint-and-trim
   cycle is single-shot. After ~250 ticks of activity, runtime
   sleeps permanently even when stream still feeds events.

2. **Generative-stream experiment (2026-05-08)**: An
   indefinitely-emitting `GenerativeDiamondEnvironment` (fresh
   diamond poset every 100 polls) saturates the runtime
   *worse* than finite OQ#1 — only 3 axioms, 1 theory, **zero
   patterns** over 5000 ticks. The runtime does not stay active
   even with unbounded input.

The root cause is **triggering, not input**. With drive metric
in hand, the runtime can know there is unexplored structure;
without it, the scheduler exhausts its frontier and sleeps.

This ADR closes that gap with the smallest possible change.

## Decision

Add a **drive-driven `PatternCandidate`** to the frontier
refresh. When the drive signal is non-empty AND the rset is
mature, frontier proposes one extra `PatternCandidate` whose
`PatternSize(N)` size matches the modal canonical's edge count
(clamped to [2, 5]) and whose priority is high enough to win
against other PatternCandidates but not against TheoryCandidate
in early rset states.

```rust
// In Frontier::refresh, after the existing PatternCandidate loop:
let drive = rset.unexplained_drive_signal();
let mature = rset.axioms().len() >= 1
    && rset.iter().count() >= MATURE_DATA_EDGE_FLOOR;
if drive.has_signal() && mature {
    if let Some(canonical) = &drive.modal_canonical {
        let size = canonical.len().clamp(2, 5);
        items.push(FrontierItem {
            id: format!("drive_pattern_size_{}_{}", size, tick),
            kind: FrontierKind::PatternCandidate,
            target: FrontierTarget::PatternSize(size),
            // Priority slightly above the highest organic
            // PatternCandidate priority, so it gets selected
            // first among pattern proposals; but lower than
            // typical TheoryCandidate values so theory
            // discovery still wins on fresh rsets.
            priority: drive.modal_count() as f64 * 5.0,
            ...
        });
    }
}
```

The maturity gate (`axioms ≥ 1 AND data_edges ≥ 100`) is the
same one ADR 0075 piece 2 (revisited) used for multi-size
fallback. It preserves lifecycle-test invariants: small
fixtures (`diamond_poset` with 9 edges and 0 axioms initially)
fail the gate, so drive-driven candidates never appear there.

### What this changes (and what it doesn't)

**Changes:**
- `Frontier::refresh` reads drive signal each refresh
- One extra `FrontierItem` when drive has signal + rset mature
- The dispatch path is unchanged — uses existing
  `DiscoverPatterns` action with `PatternSize(N)` target

**Doesn't change:**
- No new `ActionKind`
- No new `FrontierKind` (reuses `PatternCandidate`)
- No new ontology entities, no new meta-R markers
- Cooldown logic unchanged
- Lifecycle / sleep semantics unchanged at the `should_wake` /
  `should_sleep` decision level

### Why "PatternCandidate" not new kind

Could introduce `FrontierKind::DriveTargeted` for clarity. But:
- The dispatch is identical (calls `autonomous_pass` with
  `PatternSize`)
- Adding a kind requires updating `execute_for_kind`, cooldown
  bookkeeping, and persistence layer
- The "drive-driven" provenance is already encoded in the
  item's id prefix (`drive_pattern_size_*`) for diagnostic
  purposes

If empirics show drive-driven candidates need different
cooldown / priority semantics from organic ones, a separate
kind becomes justified. Until then, reuse.

### Why size = canonical_size, clamped to [2, 5]

The drive's modal canonical is a structurally precise pointer
("OQ#2 has unexplained 9-edge star-hub subgraphs"). Telling
dispatch to use size = 9 directly would have `find_instances_of`
explore size-9 subgraph space, which is very expensive. The
[2, 5] clamp matches existing frontier proposal range and the
empirical mint sweet spot. The dispatch's multi-size fallback
(ADR 0075 piece 2 revisited) handles size mismatch by trying
4-5 first when the requested size fails.

Future refinement: when canonical_size > 5, fragmenting it
into multiple smaller-size candidates could be a follow-up.

## Alternatives considered

**Alt A — Gate `should_sleep` on drive directly**: instead of
proposing a frontier item, modify the scheduler to refuse to
sleep while drive > 0. Rejected: this skips the dispatch
mechanism. The runtime would stay awake but have no work item;
result is a busy-loop with no progress, or scheduler returning
`Sleep` again on the next tick anyway. The frontier-item
approach is a real work proposal.

**Alt B — New `ActionKind::DiscoverPatternsTargeted`** that
consumes the modal canonical directly. Rejected for this
slice: introduces ActionKind sprawl. The existing
`DiscoverPatterns + PatternSize(N)` already provides the size
target; canonical-form targeting is a finer signal that may be
worth a future ADR but is not required to make drive useful.

**Alt C — Auto-execute drive without scheduler involvement**:
have the runtime call `autonomous_pass` directly when drive
is non-empty, bypassing the scheduler. Rejected: breaks the
scheduler/agent abstraction; episodes would not record the
drive-triggered work; cooldown would not apply.

**Alt D — Defer until manual-execution data justifies it**:
ship drive metric only, wait for users to consume it. Rejected
because the long-horizon observation already shows the gap is
costing v2 measurable cognitive activity. Auto-integration is
the natural completion of ADR 0078.

## Consequences

**Now possible:**

- Runtime stays active past Phase 0 initialization while drive
  signals unexplored structure. Single-shot cognition becomes
  sustained-mint-and-trim cycle.
- Generative streams genuinely sustain activity (drive keeps
  refilling as fresh-token diamonds arrive)
- `phase_emergence_long_horizon_observation` should now show
  ongoing pattern minting, not flat-line idle past tick 250

**Now harder:**

- Tuning the priority constant. Setting it too high crowds out
  TheoryCandidate at substrate maturity; too low and drive
  doesn't get attention. Initial value `modal_count * 5` is a
  guess; the long-horizon re-run will reveal whether it's
  reasonable.
- Termination conditions. With drive-driven candidates,
  runtime may stay awake indefinitely on substrates where
  drive never extinguishes. Acceptable for this slice (the
  mint-and-trim cycle should reduce drive over time even if
  not to zero); but a future "graceful idle" mechanism may be
  warranted if drive levels stabilize at non-zero values
  forever.

**Newly easy:**

- Cross-substrate proactive cognition: with drive→scheduler
  wired, OQ#2's 91% unexplained at maturity will trigger
  ongoing pattern mints across the lattice / star / tournament
  regimes. Long-horizon runs should reveal v2 *learning OQ#2*
  for the first time in v2's history.

## Implementation

Single-file change to `src/runtime/frontier.rs`, plus 1-2 unit
tests verifying:
- Drive-driven candidate appears when drive non-empty + mature
- Does not appear on small / fresh rsets (maturity gate works)
- Does not duplicate organic PatternCandidates of the same size

Then re-run:
- Lib test suite (642 tests, no regressions expected — the
  maturity gate ensures lifecycle fixtures unchanged)
- `phase_emergence_long_horizon_observation` to verify
  sustained activity
- `generative_stream_experiment` to verify generative streams
  now produce ongoing patterns
- `phase_emergence_capability_demo` to confirm 9-section demo
  still works end-to-end

Result doc records empirical "before/after" — the
single-shot vs sustained transition is the key data point.

## Open questions

- **Should drive-driven candidates have their own cooldown?**
  Currently they share `min_pattern_attempts_before_cooldown`
  with organic ones. If drive-driven dispatches keep
  succeeding (extinguishing drive), cooldown shouldn't trip.
  If they fail, cooldown protects the runtime from
  drive-amplified pattern-mint thrashing. Acceptable shared
  cooldown for this slice.
- **What if multiple canonicals tie for modal?** Current code
  takes `canonical_buckets[0]` regardless. Stable iteration
  in `unexplained_drive_signal` ensures determinism. Future
  refinement: rotate through top-K canonicals across
  successive dispatches.
- **What if `drive.modal_count()` is huge (drive overwhelming
  TheoryCandidate priority)?** The `* 5.0` priority scaling
  combined with modal_count gives priorities like 25-100 for
  realistic drive signals — well above PatternCandidate (~10)
  but well below TheoryCandidate (~200 for axiom_count*100).
  Initial empirics expected to confirm this balance; the
  long-horizon re-run will show if any item starves another.

## Implementation note (load-bearing)

This ADR makes one architectural commitment that's worth
isolating:

> **The runtime no longer enters permanent sleep solely because
> its frontier is empty.** It can be awakened by drive even
> without external stream events. Drive becomes the
> *intrinsic* signal that complements stream's *extrinsic*
> input.

Before this ADR, v2's cognitive substrate was strictly
reactive — every dispatch traceable to either an external
event or a frontier proposal made during initial discovery.
After this ADR, the runtime can be self-directing: drive
proposes work that the runtime then executes, with no
external cause needed.

This is a small change in code (one block in
`Frontier::refresh`) but a substantive change in v2's
character: it crosses the reactive→proactive boundary.

## Implementation

Pending. Initial implementation in next commit.
