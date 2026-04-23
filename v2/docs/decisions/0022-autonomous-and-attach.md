# 0022: Autonomous pass + attach composition

Status: Accepted
Date: 2026-04-23

## Context

`autonomous_pass` (ADR 0018) discovers novel canonicals via sampling
and records all their clean instances.
`run_naming_pass` with `attach_only = true` (ADR 0015) extends
*already-named* patterns with new instances found via subgraph
matching.

On a fresh RSet, autonomous already uses `find_instances_of`
exhaustively per discovered canonical, so attach adds nothing.
On an **incremental** RSet — new data added since prior naming —
the two mechanisms cover disjoint cases:

- autonomous catches **novel canonicals** in the new data.
- attach catches **new instances of already-named canonicals**.

Callers can compose them manually; this ADR adds a one-call wrapper
so the incremental workflow doesn't depend on remembering to chain
the two.

## Decision

```rust
#[derive(Debug, Clone)]
pub struct AutonomousAndAttachSummary {
    pub autonomous: Vec<AutonomousOutcome>,
    pub attach: Vec<(CanonicalForm, NamingDecision)>,
}

impl RSet {
    /// Run `autonomous_pass` then `run_naming_pass` with
    /// `attach_only = true`. The combination is the natural
    /// incremental-data workflow: autonomous names novel canonicals;
    /// attach picks up new instances of existing canonicals. ADR 0022.
    pub fn autonomous_and_attach(
        &mut self,
        config: &AutonomousConfig,
    ) -> AutonomousAndAttachSummary;
}
```

Implementation: invoke `autonomous_pass(config)`, clone
`config.naming` with `attach_only = true`, invoke
`run_naming_pass(&attach_policy)`, bundle both outputs into the
summary.

Autonomous runs first because it may mint new patterns that attach
will then iterate. Doing attach first would miss the newly-named
ones; running autonomous first is strictly inclusive.

## Alternatives considered

- **Keep them as separate calls.** Rejected. One-liner wrappers
  that bake in the correct order are worth the surface-area cost —
  they prevent misuse (e.g., running attach before autonomous and
  wondering why new patterns aren't being attached).
- **Run attach first, then autonomous.** Rejected. Attach would
  miss any canonicals autonomous is about to mint. Autonomous →
  attach is strictly inclusive.
- **Accept a separate attach policy rather than deriving from
  `config.naming`.** Rejected. Making the two phases share policy
  thresholds (min_edges, min_instances, MDL) is the common case;
  a caller who wants divergent policies can compose manually.

## Consequences

- Incremental workflow becomes a single call. Useful when data is
  added to the RSet over time.
- On a fresh RSet, the attach phase is a no-op. No harm done; the
  summary reflects it (attach entries all `AlreadyKnown`).
- Composes cleanly with `autonomous_sweep` (ADR 0021): callers who
  want to sweep sizes AND attach can loop `autonomous_and_attach`
  over sizes (or sweep first, then attach once — both work).

## Implementation

- Source: `v2/src/lib.rs` — `AutonomousAndAttachSummary`,
  `RSet::autonomous_and_attach`.
- Tests: 3 new — on a fresh RSet the attach phase finds only
  existing/skipped; after adding new data, the attach phase picks
  up new instances of a pre-existing pattern; idempotence.
- Example: `v2/examples/autonomous_and_attach.rs` showing the
  incremental workflow.
- Experiment log: `v2/logs/2026-04-23_autonomous_and_attach.log`.
