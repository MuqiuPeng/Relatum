# 0048: Confidence thresholds in AxiomDiscoveryConfig

Status: Accepted
Date: 2026-04-24

## Context

ADR 0045 added Wilson score CI and null-baseline probability to
every `AxiomEvidence` but stopped short of applying them as
filters: they were exposed for the caller to consult and nothing
more. Task 2 of the 1'''→5''' round finishes that work — wire
them into `AxiomDiscoveryConfig` so `discover_axioms` can filter
using them directly.

## Decision

### Two new config fields

```rust
pub struct AxiomDiscoveryConfig {
    // ... existing fields
    pub min_posterior_lower: f64,    // new, default 0.0
    pub max_null_baseline: f64,      // new, default 1.0
}
```

Default values make the filters no-ops — every existing caller is
unaffected. Callers who want stricter acceptance:

- `min_posterior_lower = 0.8` → "only axioms with ≥ 80% posterior
  lower-95 CI" → rejects small-support / low-confidence findings.
- `max_null_baseline = 0.01` → "only axioms that would be < 1%
  likely under iid Bernoulli edges" → rejects dense-random
  accidents.

### Integration

`discover_axioms` extends its filter expression:

```rust
if ev.premise_bindings >= config.min_evidence
    && ev.rate >= config.min_rate
    && ev.posterior_lower_95 >= config.min_posterior_lower
    && ev.null_baseline_prob <= config.max_null_baseline
{
    results.push(ev);
}
```

`discover_axioms_minimal`, `discover_axioms_minimal_compositional`,
and `discover_extended_axioms` all call through `discover_axioms`,
so the filters compose with subsumption automatically.

`discover_theory` still uses predicate forms for reflexivity /
antisymmetry / totality — it does not consult the confidence
thresholds. Those are checked separately and have their own
rate-1.0 gate.

## Alternatives considered

- **Make the filters active by default** (e.g. `min_posterior_lower = 0.5`).
  Rejected — would silently change existing test expectations. Keep
  opt-in.
- **Filter in `discover_axioms_minimal` separately** from
  `discover_axioms`. Rejected — a uniform gate in `discover_axioms`
  is simpler; every downstream path benefits.
- **Warn when filters drop many axioms but leave results the same**.
  Rejected — `AxiomEvidence` already exposes both values;
  callers can inspect pre-filter output by setting defaults and
  comparing to filtered output.

## Consequences

### Concrete usage

Three representative configurations:

```rust
// Strict mode (default): rate = 1.0, no confidence filter
let default_cfg = AxiomDiscoveryConfig::default();

// Statistically honest: reject small-sample and accidental findings
let honest = AxiomDiscoveryConfig {
    min_posterior_lower: 0.5,
    max_null_baseline: 0.1,
    ..AxiomDiscoveryConfig::default()
};

// Exploratory: defeasible + loose confidence
let exploratory = AxiomDiscoveryConfig {
    min_rate: 0.7,
    min_posterior_lower: 0.3,
    ..AxiomDiscoveryConfig::default()
};
```

### On ADR 0041's dense-random case

A complete graph on 4 identifiers has `p_edge = 1.0`, so every
axiom's `null_baseline_prob = 1.0`. Under `max_null_baseline =
0.5`, all axioms drop. Verified by
`adr0048_low_null_threshold_drops_dense_accidents`.

### On small-support cases

Diamond poset's transitivity holds at rate 1.0 on 2 bindings.
Wilson CI lower at n=2 s=2 is ~0.34. Under
`min_posterior_lower = 0.7`, transitivity drops. Under
`min_posterior_lower = 0.9`, every diamond-poset axiom drops.

### Caveat: extended families

The threshold filter applies only to edge-family axioms via
`discover_axioms`. `discover_extended_axioms` returns
`ExtendedAxiomEvidence::Equality` / `Disjunctive` variants which
do not carry posterior / null-baseline fields. The config is
applied through their `rate` check (ADR 0044 path) but confidence
thresholds are silently ignored for them. Documented limit.

## Verification

- 265 → 270 tests pass (5 new: default no-op, high posterior
  drops small-support, low null drops dense accidents, additive
  composition, high posterior preserved by large support).

## Implementation

- `v2/src/lib.rs` — two new config fields, filter extension in
  `discover_axioms`.
- `v2/docs/decisions/0048-confidence-filters.md` — this ADR.
