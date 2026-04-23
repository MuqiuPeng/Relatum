# 0028: Axiom subsumption (structural canonicalization + redundancy filters)

Status: Accepted
Date: 2026-04-24

## Context

ADR 0027's rigorous blind-test log recorded the semantic correctness
of `discover_axioms` across 8 cases, but with a loud caveat: the
equivalence relation, tolerance, and total-order cases produced 45,
37, 25 axioms respectively, where only a handful were genuinely
independent. The rest were consequences of universal reflexivity
("any conclusion of the form `R(v, v)` is trivially true"), or of
variable relabeling ("transitivity appeared twice under different
position-to-variable assignments"), or of strictly-weaker-premise
redundancy ("an axiom with 2-edge premise was dominated by a 1-edge
axiom with the same conclusion").

The noise was not a cosmetic UI problem. As the user pointed out:
any upper-layer work that needs to pick canonical axioms for theory
synthesis, relation typing, or automated naming must start from a
filtered set. Subsumption is therefore a precondition for further
progress, not an optional polish.

## Decision

Add two redundancy-removal mechanisms plus a stronger canonical form.
Keep raw `discover_axioms` unchanged; expose the filtered view as a
separate composable pipeline.

### 1. Structural template canonicalization

ADR 0027 canonicalized templates by first-use variable renumbering
plus premise-edge sort. That was invariant under variable *renaming*
but not under variable *permutation*. Result: transitivity leaked as
two templates in the output (`[R(0,1) ∧ R(1,2)] ⇒ R(0,2)` and
`[R(0,1) ∧ R(2,0)] ⇒ R(2,1)`).

ADR 0028 replaces the canonicalizer with: *minimum over all
permutations of variable labels, of the first-use-normalized form*.
For `max_vars ≤ 4` this is at most 24 permutations per template, and
the enumeration itself hits only a bounded template space, so the
total cost is negligible.

```rust
fn canonicalize_template(tpl: AxiomTemplate) -> AxiomTemplate {
    let base = canonicalize_template_first_use(tpl);
    // For each permutation of 0..base.num_vars, apply, re-first-use,
    // keep the lex-smallest by (num_vars, premise_edges, conclusion).
}
```

### 2. Subsumption by universal reflexivity

When `check_reflexivity().rate == 1.0` on the RSet, any axiom whose
*conclusion* has `x_var == y_var` is entailed by reflexivity alone
— the premise contributes nothing to the truth of the conclusion.
Drop those axioms.

```rust
pub fn subsume_by_reflexivity(axioms: Vec<AxiomEvidence>) -> Vec<AxiomEvidence>;
```

Free function; caller (specifically `discover_axioms_minimal`) is
responsible for checking reflexivity before invoking.

### 3. Subsumption by premise weakening

If axiom A has a premise that is a subset of axiom B's premise (under
some variable mapping σ that maps A's conclusion onto B's conclusion),
then A is strictly stronger than B: every model satisfying A's
premise also satisfies A's conclusion = B's conclusion, so B never
adds information. Drop B.

```rust
pub fn subsume_by_premise_weakening(axioms: Vec<AxiomEvidence>) -> Vec<AxiomEvidence>;
```

Algorithm: for each ordered pair (i, j), enumerate variable mappings
σ from `a_i.template`'s vars to `a_j.template`'s vars, seeded with
the conclusion-endpoint constraints. For each σ, check whether
`σ(premise_A) ⊆ premise_B` as a set of edge templates. If any σ
works and the mapping isn't symmetric in both directions, drop j.
If both directions subsume (equivalent under relabeling), keep the
lex-smaller template.

Variable-count bound is `max_vars ≤ 4`, so mapping enumeration is at
most `4^4 = 256` per pair — cheap.

### 4. `RSet::discover_axioms_minimal`

Composes the three pieces:

```rust
impl RSet {
    pub fn discover_axioms_minimal(&self, config) -> Vec<AxiomEvidence> {
        let raw = self.discover_axioms(config);
        let after_refl = if universal_reflexivity_holds {
            subsume_by_reflexivity(raw)
        } else { raw };
        subsume_by_premise_weakening(after_refl)
    }
}
```

Raw enumeration is unchanged; users who want the unfiltered output
still have it.

## Consequences

### Effect on the rigorous battery (8 cases)

Raw counts vs. minimal counts:

| Case | Raw (0027) | After canonicalization | Minimal (0028) |
|------|-----------:|-----------------------:|---------------:|
| 1. transitive closure chain | 2  | 1  | 1  |
| 2. equivalence relation     | 45 | 26 | 5  |
| 3. strict partial order     | 2  | 1  | 1  |
| 4. broken transitivity      | 0  | 0  | 0  |
| 5. random sparse            | 0  | 0  | 0  |
| 6. tolerance                | 37 | 22 | 1  |
| 7. total order              | 25 | 15 | 1  |
| 8. bipartite                | 0  | 0  | 0  |

Cases 1, 3, 6, 7 reduce to exactly one minimal axiom each — the
"canonical" rule humans recognize (transitivity or symmetry). Case 2
reduces to five: symmetry plus four genuinely-independent transitivity
variants that arise because an equivalence relation admits multiple
composed chains. See "Limits" below.

### Effect on upper-layer feasibility

Before ADR 0028 the discovery output was a flood of consequences of
universal reflexivity plus variable-relabeling duplicates. Any
would-be consumer ("name this theory," "pick representative rules,"
"compose axioms into concepts") would have had to filter redundancy
first. ADR 0028 makes that filtering the default. Minimal output is
now the sensible starting point for theory-level work.

### Commitments

- **Commitment 1 (only R):** Unchanged; axioms still live as Rust
  values, not as R instances. ADR 0028 is pure postprocessing.
- **Commitments 2, 3, 4, 5:** Unaffected.

### Limits

Three kinds of redundancy are *not* removed:

1. **Compositional derivability.** On the equivalence relation, four
   transitivity variants survive (e.g., `R(x,y) ∧ R(y,z) ⇒ R(x,z)`
   and `R(x,y) ∧ R(x,z) ⇒ R(y,z)`). All four are consequences of
   {symmetry, transitivity}, but the check requires reasoning about
   axiom composition — essentially a propositional theorem prover,
   which is out of scope here. Acceptable: five minimal axioms on an
   equivalence relation is a factor of nine reduction from 45 and
   is tractable for upper layers.
2. **Premise-side self-loop in the absence of universal reflexivity.**
   If reflexivity does NOT hold universally, an axiom with a self-loop
   in its premise is not in general equivalent to the self-loop-free
   version. Premise-weakening still catches the specific case where
   both variants are in the output; other cases are genuinely
   different rules.
3. **Vacuous universal truths.** The `min_evidence ≥ 1` filter is
   still what suppresses vacuous truths (premises with no binding at
   all). ADR 0028 does not re-examine this.

## Verification

- All 128 previous tests still pass; 6 new tests added (134 total).
- Rigorous blind battery re-run via
  `cargo run --example axiom_rigorous_test`. See
  `v2/logs/2026-04-24_axiom_subsumption.log` for the full before/after.

## Out of scope for this ADR

- Encoding axioms as meta-R (still open from ADR 0027).
- Compositional subsumption via theorem-prover-style derivation.
- Disjunctive conclusions (totality).
- Defeasible axioms (rate < 1.0).
