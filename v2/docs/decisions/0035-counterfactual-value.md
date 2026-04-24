# 0035: Counterfactual value / meta-metric

Status: Accepted
Date: 2026-04-24

## Context

ADR 0031's intrinsic drive has a first-order metric
(`abstraction_score`) and picks actions that raise it. That is a
*selection* mechanism — it doesn't tell us *why* the system chose
what it did, or which named objects are actually carrying weight
versus sitting idle. The natural follow-up is a **second-order**
signal: for each named object, what would it cost me if I didn't
have it?

Task 2 of the 1→5 extension adds that signal.

## Decision

### Counterfactual value

```rust
impl RSet {
    pub fn counterfactual_value(&self, id: &str) -> Option<f64>;
    pub fn rank_by_counterfactual(&self) -> Vec<(String, f64)>;
    pub fn retract_extension(&mut self, ext_id: &str) -> Result<usize, TheoryError>;
}
```

`counterfactual_value(id)` clones self, retracts the object, and
returns `before_score − after_score`. Positive values mean the
object is load-bearing; near-zero means it contributes nothing;
negative means removing it would raise the score (a net cost,
possible when overhead tax dominates the reward).

Supported object types: **patterns**, **theories**,
**extensions**, and **axioms not yet bound to any theory**. For
axioms currently referenced by a theory, `retract_axiom` fails and
`counterfactual_value` returns `None` — caller must retract the
owning theory first.

`rank_by_counterfactual()` produces a global ranking by value
descending, with id as tiebreaker for determinism.

### `retract_extension` added

Symmetric with `retract_theory` / `retract_pattern` / `retract_axiom`.
Removes the `__extends__` registry edge and the two chain edges
(3 total).

## Alternatives considered

- **Cache precomputed values**. Could speed up repeated ranking.
  Rejected — clone-and-retract is simple and correct on β-scale
  graphs; caching has invalidation pitfalls.
- **Approximate via marginal edge-count delta**. Cheaper but
  loses the reuse-savings and theory-reward signals that the
  actual score captures. Rejected — the point is to reflect the
  real metric.
- **Report per-action counterfactual** (drive's own trace, but
  redone). Not done here; `DriveStep` already has `delta`. Ranking
  named objects is a complementary signal, not a replacement.

## Consequences

### Closes the first/second-order gap

The drive told the system *what to do next*. Counterfactual value
tells it *how well it was doing it*. Two different signals, both
useful:
- Drive: "What action improves score now?"
- Counterfactual: "Which of my named objects are earning their
  keep?"

A future mechanism can use counterfactual ranking to prune
low-value objects (retract anything with value ≤ ε), or to
prioritize exploration (dig deeper into regions whose objects are
all high-value).

### Value signs explain the scoring formula

The metric from ADR 0031 is `reuse_savings + 2.0 × theory_members −
0.1 × meta-R`. Counterfactual values are the contribution each
object makes to this scalar:

- A pattern with N=6, k=2 contributes `(6−1)·2 = +10` reuse, minus
  roughly `0.1 × (Layer A + Layer B)` overhead. Net counterfactual
  value is that difference.
- A theory with 3 members contributes `+6` reward minus
  `0.1 × (1 registry + 3 membership + induced axiom intension)`.
  For the diamond poset this nets to ~+4 in tests.
- An extension edge contributes 0 reward (extensions don't appear
  in the metric) minus `0.1 × 3` overhead. Counterfactual is
  slightly *negative* — retracting the extension actually *raises*
  the score. This is the metric telling us it doesn't care about
  extensions; a future ADR could reward them.

### Truth invariant

`adr0035_counterfactual_respects_actual_retract_behavior`
verifies: predicted drop ≡ (before − after-retract-score), to
within floating-point epsilon. Counterfactual is not an estimate;
it's the real delta computed by actually running the retraction on
a clone.

### Commitment check

- 1–2: all writes / retractions stay in R. ✓
- 3: no new named-object kind introduced; only a read-only signal
  computed from existing meta-R. ✓
- 4, 5: unaffected.

## Limits

- **Clone cost.** `counterfactual_value` clones the whole RSet
  per call. On β-scale graphs this is negligible; on large graphs
  `rank_by_counterfactual` becomes O(|objects| × |edges|). A
  future ADR can do cheap-path approximations.
- **Metric dependency.** All values are relative to the ADR 0031
  metric. If the metric changes, every counterfactual value
  shifts. Acceptable — the value *is* relative to the metric by
  definition.
- **Nothing auto-prunes.** This ADR adds only the *signal*. Using
  it to drive retraction decisions is a separate step; left to
  a future mechanism when a use case surfaces.

## Verification

- 182 → 188 tests pass (6 new: theory value positive, unknown id
  returns None, axiom-in-theory blocked, rank descending, predicted
  ≡ actual, retract_extension removes 3 edges).

## Implementation

- `v2/src/lib.rs` — `retract_extension`, `counterfactual_value`,
  `rank_by_counterfactual`.
- `v2/docs/decisions/0035-counterfactual-value.md` — this ADR.
