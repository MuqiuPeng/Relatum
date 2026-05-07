# 0078: Pattern-aware drive metric (constitution-compliant)

Status: Proposed
Date: 2026-05-07

Parents:
- [Reflection 0001 — meaning emerges with concept](../reflections/0001-meaning-emerges-with-concept.md)
- [Constitution amendment — strict reading](../constitution.md#strict-reading-differentiation-requires-registration)
- [0059 — unexplained data edges metric](0059-unexplained-data-edges.md)
- [0075 — Emergence kernel audit](0075-emergence-kernel-audit-and-runtime-integration.md)
- [0076 — Micro-agent reframing](0076-micro-agent-reframing.md)

## Context

The 2026-05-06 long-horizon observation
(`docs/results/phase_emergence_long_horizon_observation.md`)
established that v2's mint-and-trim cycle is **single-shot**:
all discovery happens within ~250 ticks of substrate maturity,
after which the runtime sleeps permanently. Episodes in the
second half of every long run = 0 across all observed
substrates.

The runtime is **reactive** (needs stream events to wake) not
**proactive** (no internal driver pushes it to keep working
when the stream is silent).

A **drive metric** could fix this — give the scheduler a "there
is unexplained / under-explored structure to attend to" signal
that doesn't depend on new stream events. But the original
form, drafted on 2026-05-06 as the first ADR 0075 attempt
(withdrawn before commit), used `EdgeFingerprint = (RSignature,
LocalityProfile)` as its bucket key — per-token derived
signatures, forbidden by the constitution's heavy reading.

This ADR specifies a **constitution-compliant** drive metric.

## Decision

The drive signal is computed from **unexplained R organized by
connected-component canonical form** — not by per-token
signature.

```rust
pub struct DriveCanonicalBucket {
    pub canonical: CanonicalForm,    // subgraph-level (ADR 0009)
    pub component_count: usize,      // # connected components
    pub edge_count: usize,           // total edges across them
    pub example_edges: Vec<R>,       // ≤ 5 representatives
}

pub struct UnexplainedDriveSignal {
    pub total_data_edges: usize,
    pub unexplained_count: usize,
    pub unexplained_ratio: f64,
    pub canonical_buckets: Vec<DriveCanonicalBucket>,  // sorted desc
    pub modal_canonical: Option<CanonicalForm>,
    pub distinct_canonicals: usize,
}

impl RSet {
    pub fn unexplained_drive_signal(&self) -> UnexplainedDriveSignal;
}
```

### Algorithm

```text
1. unexplained = self.unexplained_data_edges()  // ADR 0059
2. components = Subgraph::connected_components_of(unexplained)
3. for each component:
     canonical = component.canonicalize()       // ADR 0009 — subgraph-level
     bucket[canonical].component_count += 1
     bucket[canonical].edge_count += component.len()
     bucket[canonical].example_edges += first 5 component edges
4. sort buckets by component_count desc
5. modal = first bucket's canonical (if any)
```

Each bucket is keyed by **the canonical form of a connected
subgraph of unexplained edges**. The key is a property of the
subgraph (a `Vec<(u64, u64)>` derived from WL refinement on
the subgraph's own edges) — **never** of any single token.
This is the same canonical form the existing emergence kernel
(ADR 0010 / 0029) uses for pattern naming. Reusing it ensures
the drive metric speaks the same structural language as the
mint mechanism it's intended to drive.

### Why this is constitution-compliant

The reflection 0001 / heavy reading rule:

> Two tokens are distinguishable iff some explicitly-registered
> concept names the distinction.

The drive metric does not distinguish tokens. It distinguishes
**subgraph canonical forms** — structural properties of edge
collections. A token participates in some canonical's
buckets, but its identity within those buckets is not used as
a classification feature. Tokens with high vs. low degree in
the unexplained set produce the same canonical bucket if they
sit in subgraphs of the same shape.

Compare to the withdrawn first form, which used
`EdgeFingerprint = (RSignature, LocalityProfile)` per edge:
that approach assigned each edge a key derived from its
endpoints' positions in the broader RSet — implicitly typing
tokens. The new approach takes the unexplained edges as
a whole, partitions them by structural connectivity, and asks
"what shape does each connected piece of unexplained R take?"
— pure subgraph-level information.

### What this slice ships

- The `UnexplainedDriveSignal` struct and `unexplained_drive_signal`
  method on `RSet`
- 4-5 unit tests covering edge cases (empty rset, all-explained,
  single component, multiple distinct shapes)
- An example program running the metric on each canonical
  substrate at maturity, before/after `autonomous_pass` to show
  drive shrinking as patterns mint

### What this slice does NOT ship

- **Scheduler integration**. Computing the drive signal does
  not change runtime behaviour. The scheduler does not
  consume it. Wiring it into `RuleBasedScheduler` to keep the
  runtime awake while drive > threshold is a separate concern,
  intended for a follow-up ADR (0078.1 or later) once the
  metric has been observed for empirical sanity.

- **Threshold calibration**. The metric is a structured
  report, not a gate. A threshold for "drive is high enough
  that the scheduler should not sleep" is a decision for the
  integration ADR.

- **Drive-targeted dispatch**. The modal canonical could in
  principle direct `DiscoverPatterns` to use that target size
  / structure as a starting point for sampling. That requires
  scheduler / dispatch wiring; deferred.

## Alternatives considered

**Alt A — Resurrect the withdrawn `EdgeFingerprint` form**.
Rejected: violates heavy reading.

**Alt B — Bucket by integer-only shape descriptors (vertex
count, edge count, density) without canonicalize**. Coarser
than canonical-form bucketing but legal. Rejected: too coarse
to direct attention; `n=3, m=2` covers chain, fork, and merge
all under one bucket. Canonical-form preserves the
substantive structural distinction between motifs.

**Alt C — Per-axiom unexplained ratio (no buckets)**. Reports
which axioms have lowest predict-rate on the rset. Already
exists in `prediction_state.hit_rate`. Rejected: this is an
aggregated scalar, not a drive *pointer*. The whole point of
drive is to direct mining at *something specific* — a
canonical form that mining can target — not just to report a
problem.

**Alt D — Skip the metric; rely on existing
`PruneLowValueObjects` to keep the runtime busy**. Rejected:
the long-horizon observation showed `PruneLowValueObjects`
also stops dispatching after the initialization phase. Both
discovery and pruning fall idle simultaneously when the
scheduler runs out of frontier items. The drive metric is
upstream of the frontier.

## Consequences

**Now possible:**
- Read each substrate's drive signature: which structural
  shapes of R are most under-explored?
- Cross-substrate drive comparison: do OQ#1 / OQ#2 leave
  different shapes unexplained?
- Empirical input for the scheduler-integration follow-up:
  what threshold would meaningfully change runtime behaviour?

**Now harder:**
- (Nothing immediately — the metric is observation-only this
  slice.)

**Now newly easy:**
- Diagnostic visualization: `format_canonical_shape` (ADR 0075
  piece b) renders canonicals as readable shapes; modal
  canonical's shape becomes a human-readable description of
  "what should the system attend to next?"

## Implementation sketch

New types in `lib.rs`, parallel placement to ADR 0077's
pattern-quality types:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct DriveCanonicalBucket {
    pub canonical: CanonicalForm,
    pub component_count: usize,
    pub edge_count: usize,
    pub example_edges: Vec<R>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnexplainedDriveSignal {
    pub total_data_edges: usize,
    pub unexplained_count: usize,
    pub unexplained_ratio: f64,
    pub canonical_buckets: Vec<DriveCanonicalBucket>,
    pub modal_canonical: Option<CanonicalForm>,
    pub distinct_canonicals: usize,
}

impl UnexplainedDriveSignal {
    pub fn has_signal(&self) -> bool { self.unexplained_count > 0 }
    pub fn modal_count(&self) -> usize {
        self.canonical_buckets.first().map(|b| b.component_count).unwrap_or(0)
    }
}

impl RSet {
    pub fn unexplained_drive_signal(&self) -> UnexplainedDriveSignal {
        let unexplained = self.unexplained_data_edges();
        // ... (group by canonical, sort, return)
    }
}
```

Test coverage:
- empty rset → drive signal with 0 / 0 / 0.0 / no buckets / no modal
- all-explained rset → ratio 0, empty buckets
- one connected component of unexplained → 1 bucket, modal = its canonical
- multiple components of same canonical → 1 bucket with N components
- multiple distinct canonicals → N buckets, sorted desc

Example program: `phase_emergence_drive_signal.rs`
Runs each canonical substrate to maturity. Prints:
- baseline drive signal (after Phase 0)
- drive signal after manual `autonomous_pass` for sizes 2-5
  (should shrink as patterns mint)
- modal canonical's readable shape via
  `format_canonical_shape` (helper TBD or ad-hoc)

Lib tests target: 637 → ~642. 0 regressions.

## Open questions

- **Bucket equivalence under WL collisions**. ADR 0009 notes
  WL-1 has rare false-merges. For drive purposes, treating
  WL-equivalent components as the same bucket is
  conservative-correct: it slightly under-counts diversity
  but does not invent it. Acceptable.
- **Should buckets exclude components that already have a
  matching pattern in the rset?** Currently no: an
  unexplained component whose canonical matches a registered
  pattern is suspect — by definition, registered patterns
  cover their canonical's instances. But cleanness rules in
  `is_clean_subgraph` may exclude some instances from
  pattern coverage. Future refinement.
- **Memory cost on large unexplained sets**. The bucket map
  caps at one entry per distinct canonical, which on
  realistic substrates is bounded by the variety of structural
  shapes. Should not balloon. If empirics surface a problem,
  add a `max_buckets` config.

## Implementation

Pending. Initial implementation in next commit.
