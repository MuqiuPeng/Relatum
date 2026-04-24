# 0033: Defeasible axioms (rate < 1.0 with support threshold)

Status: Accepted
Date: 2026-04-24

## Context

ADR 0027's `discover_axioms` accepted only axioms holding at rate
`== 1.0`. That is the right default for correctness — one violating
binding disqualifies the rule — but it makes the system silent on
inputs where a rule "nearly" holds. Real data often has that shape:
a binary directed relation that is transitive on 95% of its triples
is structurally significant, but ADR 0027 reports "no axioms found."

Task D of the A → C → B → D plan: admit rate < 1.0 axioms with
explicit rate and support reporting, while preserving strict mode
as the default.

## Decision

### Config addition

```rust
pub struct AxiomDiscoveryConfig {
    pub max_premise_edges: usize,
    pub max_vars: usize,
    pub min_evidence: usize,
    pub min_rate: f64,        // new, default 1.0
}
```

`min_rate = 1.0` preserves every ADR 0027/0028 behavior. Lowering
it (e.g. `0.8`) admits defeasible rules.

### Discovery rule change

In `discover_axioms`:
- before: `ev.premise_bindings >= min_evidence && ev.rate == 1.0`
- after:  `ev.premise_bindings >= min_evidence && ev.rate >= min_rate`

Support (premise bindings) threshold is unchanged. Every returned
`AxiomEvidence` already carries rate and bindings, so callers can
sort and filter further without any new API.

### `discover_axioms_minimal` guard

Subsumption (ADR 0028) assumes strict soundness: if axiom A holds
universally and A's premise subsumes B's, then B is redundant.
Under defeasible semantics this reasoning breaks — an axiom at
rate 0.9 could cover different bindings than another axiom that
looks "stronger." To preserve soundness, `discover_axioms_minimal`
skips subsumption entirely when `config.min_rate < 1.0`:

```rust
if config.min_rate < 1.0 {
    return raw;   // defeasible: no subsumption applied
}
```

Strict behavior (min_rate = 1.0) is identical to before.

### Intentional non-changes

- **`discover_theory`** still uses strict semantics. A defeasible
  theory is a larger question (what does "this RSet satisfies this
  bundle with confidence c" mean for c < 1? composition of
  uncertainties? out of scope here).
- **`intrinsic_drive`** scores strict axioms only. Adding a
  defeasible reward term would require choosing how to weight
  confidence; deferred until a use case surfaces.
- **`subsume_by_*` free functions** remain unchanged. They're
  called only by `discover_axioms_minimal`, which now gates them.

## Alternatives considered

- **Two separate methods** (`discover_axioms_strict` +
  `discover_axioms_defeasible`). Rejected — duplication, and the
  difference is literally one float. Single method with a config
  knob is cleaner.
- **Auto-detect "strict mode" from config**. Already effectively
  done: `min_rate = 1.0` is strict. No separate API needed.
- **Rate-weighted subsumption** (drop B only if A subsumes B AND
  A.rate ≥ B.rate + ε). More aggressive, but soundness proof is
  nontrivial. Deferred — the current "skip in defeasible" is
  conservative and simple.
- **Support threshold beyond min_evidence**. We could add
  `min_support_ratio` (support / |identifiers|^vars) to prune
  low-coverage rules. Not added — `min_evidence` already provides
  an absolute lower bound; a relative threshold is tunable if
  needed later.

## Consequences

### "Almost-transitive" is no longer silent

The case-4 input from the rigorous battery (4-chain transitive
closure minus one closure edge) previously returned zero axioms.
With `min_rate = 0.5`, transitivity surfaces at rate 0.667, support
2/3:

```
[R(0,1) ∧ R(1,2)] ⇒ R(0,2)    rate=0.667  support=2/3
```

Now the system can say "transitivity holds 2 of 3 times on this
input" rather than staying silent.

### Backward compatibility

All 170 existing tests pass unchanged. The default config value
continues to strict rate 1.0; every caller not explicitly setting
`min_rate` gets the ADR 0027/0028 behavior.

### Cost

None. Evaluation cost per template is already `|ids|^num_vars`
regardless of rate threshold; the threshold only affects which
results are retained.

### Limits

1. **Subsumption off in defeasible**. A fully rigorous
   "rate-aware subsumption" is possible but not implemented.
   Consequence: defeasible output can be noisy in the same way
   ADR 0027's output was before 0028. Callers can still sort by
   rate and filter.
2. **No Bayesian prior**. Rate is a raw frequency, not a
   posterior. A rule satisfied 1 of 1 times has rate 1.0 — same
   as one satisfied 100 of 100. `min_evidence` mitigates this,
   but a prior-adjusted rate would be more honest. Deferred.
3. **Theory discovery strict**. See "Intentional non-changes."

## Verification

- `cd v2 && cargo test` → 170 → 176 (6 new D tests).
- `cd v2 && cargo run --example defeasible_axioms` — shows per-
  threshold output on the case-4 input.

## Implementation

- `v2/src/lib.rs` — `AxiomDiscoveryConfig::min_rate`,
  relaxed rate check in `discover_axioms`, strict gate in
  `discover_axioms_minimal`.
- `v2/examples/defeasible_axioms.rs` — threshold sweep demo.
- `v2/docs/decisions/0033-defeasible-axioms.md` — this ADR.
- `v2/docs/progress.md`, `v2/README.md`, decisions index.
