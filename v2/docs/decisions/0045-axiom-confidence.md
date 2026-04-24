# 0045: Axiom confidence — Wilson score + null-baseline probability

Status: Accepted
Date: 2026-04-24

## Context

ADR 0041's scale benchmark surfaced an honesty problem: on a dense
random graph (400 edges / 20 identifiers), 31 axioms came out at
rate = 1.0, 9 after minimization. Most are accidents: with edge
density `p ≈ 1.0`, any template that reaches the minimum evidence
threshold also happens to always hold. The raw `rate` scalar gives
no way to distinguish structural truth from coincidence.

Task 3+4 of the 1''→5'' round combines two complementary
confidence measures into one ADR:

1. **Wilson score 95% CI** (Bayesian-ish posterior): replaces
   raw `rate = s/n` with an interval estimate that penalizes
   small support.
2. **Null-baseline probability**: the chance this rate would be
   observed under iid Bernoulli edges with density
   `p = |edges| / |ids|²`. Small value = surprising = more likely
   a real axiom than a coincidence.

## Decision

### `AxiomEvidence` extensions

```rust
pub struct AxiomEvidence {
    // ... existing fields
    pub posterior_lower_95: f64,   // new
    pub posterior_upper_95: f64,   // new
    pub null_baseline_prob: f64,   // new
}
```

Computed at `evaluate_axiom_template` time, so every returned
evidence carries these without caller action.

### Wilson score helper

```rust
pub fn wilson_score_95(successes: usize, n: usize) -> (f64, f64);
```

Standard Wilson score interval at `z = 1.96`:

```text
p_hat = s / n
denom = 1 + z² / n
center = (p_hat + z² / (2n)) / denom
halfwidth = z · sqrt(p_hat(1 − p_hat) / n + z² / (4n²)) / denom
(lower, upper) = (max(0, center − halfwidth), min(1, center + halfwidth))
```

- `n = 0`: returns `(0.0, 1.0)` — no information.
- Clamps to `[0, 1]` to prevent floating-point overshoot.

### Null-baseline helper

```rust
pub fn null_baseline_probability(
    bindings: usize,
    satisfied: usize,
    p_edge: f64,
) -> f64;
```

Assumes edges are iid Bernoulli(`p_edge`). Under that null, the
probability that `satisfied == bindings` by chance is `p_edge^bindings`.
Returns 1.0 (no discount) when:
- `p_edge` is 0 (no edges) or 1 (all edges) — either extreme
  gives no discriminatory information.
- `satisfied < bindings` — the claim isn't rate=1.0 to begin with.
- `bindings == 0` — vacuous.

### Uniform application

Both fields populate every `AxiomEvidence` from
`evaluate_axiom_template`, which means:
- `discover_axioms`: all output has the fields.
- `discover_axioms_minimal`: same (it post-filters).
- `discover_axioms_minimal_compositional`: same.
- `discover_extended_axioms`: edge-family evidence carries them;
  equality/disjunction variants have their own rate but currently
  no posterior CI fields (out of scope for ADR 0045).

### No default filter applied

ADR 0045 **adds fields** but does not automatically filter by
them. Callers who want stricter acceptance use:
- `axiom.posterior_lower_95 > 0.8` → "I want at least 80%
  posterior confidence."
- `axiom.null_baseline_prob < 0.01` → "I want statistical
  significance at α = 0.01."

## Alternatives considered

- **Use raw Beta posterior (Beta(1+s, 1+f))** for CI. Requires
  implementing inverse Beta CDF; non-trivial without a math
  library. Rejected — Wilson score is cheaper, close to Beta CI
  in practice, and v2 stays zero-dep.
- **Compute null as Fisher exact test**. More statistically sound
  but requires hypergeometric distribution. Rejected for the same
  no-external-deps reason; `p^N` under iid is the simple
  conservative analog.
- **Auto-filter by significance in `discover_axioms_minimal`**.
  Rejected — would silently drop axioms that ADR 0027/0028 tests
  expect to see (they use diamond-poset-type small N). Filter
  opt-in only.
- **Apply to equality / disjunction templates too**. Deferred —
  the null-baseline computation is different for those families
  (e.g., disjunctive conclusion's "accidental" probability is
  `1 − (1 − p)^k` per binding, not `p^N`). Clean scope for a
  future ADR.

## Consequences

### Small-N caveat is now visible

Diamond poset's transitivity holds at rate 1.0 on 2 bindings. With
ADR 0045, the same evidence now reports `posterior_lower_95 < 0.5`.
A caller who wants high confidence would reject it; a caller who
just wants "rate=1.0 seen" keeps the ADR 0027 behavior via raw
`rate`.

### Dense random graphs can be filtered

On the 400-edge complete-ish graph from ADR 0041, all axioms report
`null_baseline_prob ≈ 1.0` because `p_edge ≈ 1.0`. A caller adding
`axiom.null_baseline_prob < 0.01` to their filter would drop all
31 of them as likely accidents — which is exactly the intent.

### Field additions are non-breaking at the contract level but
**are breaking at the struct-literal level**. Every test that
constructed `AxiomEvidence { … }` needed updating. Three call sites
in the test suite were patched with `posterior_lower_95: 0.0,
posterior_upper_95: 1.0, null_baseline_prob: 1.0` defaults.

### Metric unchanged

`abstraction_score` still uses raw `rate` signals indirectly via
pattern reuse and theory membership. It does not consult
posterior or null-baseline. Future ADR could integrate them into
the drive metric; out of scope here.

## Verification

- 243 → 249 tests pass (6 new: Wilson edge cases, null-baseline
  edge cases, small null-baseline on dense synthetic, every
  `AxiomEvidence` carries the fields, dense random has high null,
  small support gives wide CI).
- No pattern / theory / drive tests changed.

## Implementation

- `v2/src/lib.rs` — `wilson_score_95`, `null_baseline_probability`,
  three new `AxiomEvidence` fields, population in
  `evaluate_axiom_template`.
- `v2/docs/decisions/0045-axiom-confidence.md` — this ADR.
