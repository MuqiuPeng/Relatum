# 0058: Axiom forward-application semantics

Status: Proposed
Date: 2026-04-26

## Context

ADR 0057 / Phase G0 confirmed empirically what the
architectural analysis predicted: anomaly pressure alone is not
enough to overcome the runtime's mode-thrash bound. The
sleep-suppression hook fires but the thrash gate forces Sleep
within a few oscillations. The runtime needs a **finer success
signal** — one where individual ticks can have positive-delta
episodes *without* naming a new pattern (so the activity isn't
tied to mode transitions the thrash gate punishes).

Phase G1 (prediction-error drive) is that signal. But before
G1's drive can be designed, the system needs a precise notion
of **prediction**: given the rset's current state and its named
axioms, what R-instances do those axioms claim should also
hold?

This ADR scopes that: how to take a named axiom and
**forward-apply** it to the rset to produce a set of predicted
conclusion edges. No drive logic here — just the prediction
mechanism. ADR 0059 (TBD) will use this output to define
prediction error and wire it into the scheduler.

The mechanism is also independently useful: even without the G1
drive, forward application is the right "what does this axiom
say?" operator. Existing v2 code doesn't have it — axioms today
are evaluated only post-hoc against existing rset state
(`evaluate_axiom_template`).

## Decision

### Definition

A named axiom in v2 corresponds to a stored `AxiomTemplate`
(ADR 0027, extended by ADRs 0044 / 0047). The template is a
tuple:

```text
AxiomTemplate {
  num_vars: usize,
  premise: Vec<EdgeTemplate>,
  conclusion: EdgeTemplate,
}
```

(For ADR 0044's extensions: equality constraints and
disjunctive premise alternatives; addressed in
the Phase G1 sub-slices below.)

**Forward application** of an axiom against an rset is the set
of `R(x, y)` instances that result from instantiating the
conclusion edge under every valid premise binding:

```text
forward_apply(axiom, rset) =
  { R(σ(c.x_var), σ(c.y_var))
    | σ : 0..num_vars → identifiers(rset)
    , for every p in axiom.premise:
        R(σ(p.x_var), σ(p.y_var)) ∈ rset }
```

Two semantic choices to commit to:

1. **Identifier domain**: `σ` ranges over **data identifiers
   only** — the same set used by `data_edges_sorted` (i.e.,
   nodes not in `collect_meta_ids`). Reason: axioms encode
   facts about user-level relations; predicting meta-R edges
   would conflate the layers and break commitment 3 (types
   are meta-R instances, not subject to axiomatic prediction).
2. **Output filter**: the predicted set returned by
   `forward_apply` is the *raw* set as defined above. The
   caller decides whether to subtract `rset.instances ∩
   data_edges` (i.e., "predictions that are already facts").
   Both interpretations are useful; the prediction-error drive
   (G1) wants to compare predicted-vs-observed including
   "facts that an axiom would have predicted anyway" because
   those count as evidence the axiom is right.

### `RSet::forward_apply_axiom(axiom_id) -> HashSet<R>` (G1.0 slice)

Public API. Returns the predicted conclusion-edge set, raw
form. Errors:

- `axiom_id` not a named axiom → return empty set (no panic).
- Template parse failure (shouldn't happen — ADR 0027
  guarantees stored templates are valid) → return empty.

The implementation iterates `σ` over the data-id space using
a generic recursive enumerator analogous to
`evaluate_template_recursive`:

```text
recurse(var_idx, partial_σ):
    if var_idx == num_vars:
        if every premise edge holds under partial_σ:
            insert R(partial_σ(c.x), partial_σ(c.y)) into result
        return
    for id in data_ids:
        partial_σ[var_idx] = id
        recurse(var_idx + 1, partial_σ)
```

Pruning: if a premise edge mentions only variables ≤ var_idx
already bound, check it inside the loop and short-circuit on
failure. This is the same optimization
`evaluate_template_recursive` uses today. ADR 0027 found it
sufficient at v2 scale.

**Complexity**: `O(N^num_vars)` worst case where N =
data-id count. For the axiom templates v2 actually mints,
`num_vars ≤ 3`, so N=300 → ≤ 2.7e7 candidate substitutions.
Acceptable for a once-per-tick prediction snapshot at v2
scale.

### `RSet::forward_apply_all() -> HashSet<R>` (G1.0 slice)

Convenience — union of `forward_apply_axiom` over every named
axiom. This is the predicted set the prediction-error drive
will compare against observation.

**Determinism**: the result is a `HashSet<R>` — order-
insensitive. Iteration order over `axioms()` doesn't matter.
Determinism between runs depends only on the rset's content,
which is what we want.

### Phase G1.1 (sketch, deferred) — equality constraints

ADR 0044 added equality constraints (e.g., axiom premise
includes "x_var == y_var"). The forward enumerator skips
σ's that violate any equality constraint.

```text
recurse(var_idx, partial_σ):
    ...
    for each equality constraint (a, b):
        if both a, b are < var_idx and partial_σ[a] != partial_σ[b]:
            return  // early pruning
    ...
```

### Phase G1.2 (sketch, deferred) — disjunctive premises

ADR 0044 also added disjunctive premise (any one of N
alternatives must hold). The forward enumerator branches on
disjunction:

```text
all_premises_hold(σ) :=
  for each premise (or premise-disjunction):
    if disjunctive: any alternative holds under σ
    else:           the standard premise edge holds under σ
```

### What this does NOT do

- Does not mutate the rset. Pure read.
- Does not produce *probabilities*. The output is a discrete
  set; an axiom either says R(x,y) or it doesn't. Probability
  weighting would be a separate ADR.
- Does not recursively expand. If axiom A's conclusion
  triggers axiom B's premise, this implementation does NOT
  forward-apply B. One-shot, one-axiom-at-a-time. (Recursive
  closure would be useful but is much more expensive and a
  separate decision.)
- Does not predict R involving identifiers that don't yet
  exist in the rset. The id domain is bounded by current
  rset content; new identifiers come from environment events,
  not from axiom forward-apply.
- Does not handle ADR 0046 / 0049 theory-relation axioms.
  Those operate on theory ids, not data ids; forward-apply on
  data domain returns empty for them. Acceptable.

## Phase G1.X (deferred) — interactions with other v2 mechanisms

Once forward-apply is in place, several Phase D/E mechanisms
can be re-considered against it:

- **Layer B coverage (ADR 0057)**: G0 defined "uncovered" by
  Layer B participation. With forward-apply, "uncovered" can
  also mean "data edges not predicted by any axiom's forward
  application". Stronger signal — closer to the
  prediction-error drive's natural definition.
- **counterfactual_value (ADR 0035)**: an axiom's
  counterfactual value should arguably weight by how much its
  forward-apply predictions reduce prediction error, not just
  by abstraction_score delta on retraction.
- **theory_independence (ADR 0042)**: independence currently
  measured by member-axiom set disjointness. With forward-
  apply, two theories could be "predictively independent"
  iff their forward-apply outputs don't overlap.

These are improvements, not requirements. Don't conflate G1.0
with redesigning Phase D/E.

## Alternatives considered

- **Materialize forward-apply outputs as new R edges in the
  rset.** That conflates "predicted" with "observed", losing
  the prediction-error signal. Stay pure-read.
- **Cache forward-apply results.** Stale-cache problem under
  rset mutation. Defer until profiling shows it's needed.
- **Generic axiom-forward beyond AxiomTemplate.** Some
  axioms have richer forms (negation, quantifiers). ADR 0027
  templates only cover universal-conditional shape; that's
  what v2 mints today. Don't over-engineer.
- **Sampling-based forward-apply** (parallel of ADR 0043).
  At β-scale or larger, exhaustive σ enumeration is too slow.
  Sampling yields an under-approximation of the predicted
  set. Suggest sampling as Phase G1.X follow-up; G1.0 stays
  exhaustive.

## Non-goals

- A general theorem prover. Forward-apply is grounded in the
  current rset's identifier domain; there is no symbolic
  reasoning about "all possible identifiers".
- A new ActionKind or FrontierKind. ADR 0058 only adds
  passive observation methods to RSet.
- Integration with the runtime scheduler. That's Phase G1's
  job, scoped in a separate ADR (0059, TBD).
- Recursive closure / fixpoint computation. One-shot only.

## Verification plan

For Phase G1.0 (when implemented):

1. **Existing 408 tests pass.** Pure additive change.
2. **Empty cases**: no axioms named → forward_apply_all
   returns empty. Axiom with `num_vars = 0` (none currently
   minted) → returns conclusion as-is if premise holds.
3. **Round-trip with discover_axioms_minimal**: discover an
   axiom, forward-apply it, verify the output ⊇ the rset's
   data edges that match the conclusion shape. Sanity check.
4. **Reflexivity axiom**: `R(x, x)` for all x in data — given
   a named reflexivity axiom and an rset where every data id
   has a self-loop, forward-apply returns the same self-loop
   set.
5. **Antisymmetry axiom**: `R(x, y) ∧ R(y, x) ⇒ x = y` — this
   needs equality constraint support; defer to G1.1 test.
6. **Performance smoke**: 100 data ids, axiom with num_vars=3
   → forward_apply completes in < 1s on dev machine. Below
   that threshold, exhaustive enumeration is fine; above
   that, defer to G1.X sampling.

## Open questions

1. **Should the result subtract existing rset edges?** The
   ADR proposes "raw set, caller decides". Phase G1's drive
   might prefer one or the other. Defer to G1's design.
2. **Substitution domain — include theory/axiom ids?** No —
   commitment 3 says types are meta-R; axioms shouldn't
   predict facts about types. But there's a corner case:
   ADR 0046's theory-parallel relation axioms have variables
   that bind to theory ids. If we ever forward-apply those,
   the domain restriction breaks. Suggest carving out a
   special case via axiom kind discrimination, not by
   broadening the default domain. Decide when an actual
   theory-relation axiom is forward-applied (TBD).
3. **Performance at β scale**. ADR 0050 showed sampling-mode
   pushed the drive to 1000+ edges. Forward-apply with
   `num_vars=3` and 1000 ids = 1e9 substitutions. Definitely
   needs sampling. Defer to G1.X.
4. **Confidence-weighted forward-apply.** ADR 0045 attached
   Wilson-score confidence to axioms. Should low-confidence
   axioms have their forward-apply outputs *weighted* in
   prediction-error attribution? Probably yes — but that's a
   drive-design question (ADR 0059), not a mechanism question.

## Touched ADRs

- **ADR 0027** axiom templates — the input shape for
  forward-apply.
- **ADR 0044** template-language extensions (equality,
  disjunctive) — Phase G1.1 / G1.2 follow.
- **ADR 0045** axiom confidence — informs G1.X weighting.
- **ADR 0050** sampling-scale benchmark — the cost analysis
  forward-apply will eventually rely on.
- **ADR 0057** Phase G0 anomaly drive — provides the
  empirical case for needing forward-apply.

## Summary

ADR 0058 specifies one new pure-read RSet operator: given a
named axiom, return the set of conclusion-edge instances the
axiom predicts under every valid premise binding over data
identifiers. Phase G1.0 is the standard-premise version;
equality (G1.1) and disjunctive premises (G1.2) are sub-slices
that follow.

This ADR is the **prerequisite** for ADR 0059 (prediction-error
drive) — the drive can't compute "expected vs observed"
without a defined "expected." Forward-application is also
independently useful as a debugging operator: "what does this
axiom actually claim?"

Status: **Proposed**. No code yet.
