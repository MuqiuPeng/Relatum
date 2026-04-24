# 0042: Theory independence relations

Status: Accepted
Date: 2026-04-24

## Context

ADR 0034 added `extends` — a directed structural relation between
theories (T_sub has strictly more members than T_super). Task 4 of
the 1'→4' extension adds a second, symmetric relation:
**independence** — two theories share no axioms.

Independence is interesting precisely because it lets the system
say "these two theoretical objects have no structural overlap" in
meta-R. On the current 8-case rigorous battery, tolerance and
antisymmetry are independent; equivalence and strict-partial-order
are not (both contain transitivity). This becomes a first-class
observation rather than something the caller has to derive
externally.

## Decision

### Marker

```rust
pub const INDEPENDENT_MARKER: &str = "__independent__";
```

### Encoding

```text
R(__independent__, ind_N)    — registry
R(T_lo, ind_N)               — canonical source (lex-smaller id)
R(ind_N, T_hi)               — canonical target
```

Independence is symmetric, so the chain is stored only in canonical
order: the lex-smaller theory id goes on the source side. This
means `name_theory_independence(a, b)` and
`name_theory_independence(b, a)` both return the same `ind_N` id
without writing a second edge.

### API

```rust
impl RSet {
    pub fn name_theory_independence(&mut self, a: &str, b: &str) -> Result<String, TheoryError>;
    pub fn independence_edges(&self) -> Vec<&str>;
    pub fn independence_endpoints(&self, ind_id: &str) -> Option<(String, String)>;
    pub fn theories_independent_from(&self, theory: &str) -> Vec<String>;
    pub fn discover_theory_independences(&self) -> Vec<(String, String)>;
    pub fn retract_independence(&mut self, ind_id: &str) -> Result<usize, TheoryError>;
}
```

`name_theory_independence` verifies:
1. Both theories exist.
2. They're distinct (self-independence is disallowed).
3. Their member sets are disjoint.

`theories_independent_from(T)` returns every theory independent of
`T`, accepting the chain in either canonical direction.

`discover_theory_independences` scans all pairs of named theories
for disjoint member sets (both non-empty). Read-only; callers pair
with `name_theory_independence` to persist.

## Alternatives considered

- **Store both directions `R(T_a, ind_N)` + `R(ind_N, T_a)` etc**.
  Rejected — doubles meta-R for a symmetric relation. Canonical
  direction with `lo < hi` costs 3 edges always.
- **Unify `extends` and `independent` as "theory relation" with a
  kind flag**. Rejected — extensions and independences have
  different arities of symmetry and different validation rules;
  unifying them would complicate both APIs. Two markers read
  cleaner.
- **Also add "equivalence" (same member set)**. Considered. Already
  partly covered by `name_theory`'s reuse-on-match semantics: two
  theories with the same member set produce the same id. Explicit
  equivalence relation not needed.
- **Also add "conflict" (T_a has axiom A, T_b has axiom ¬A)**.
  Rejected for this ADR — requires a notion of axiom negation
  that doesn't exist in the current template form. Future work
  if disjunctive / negative conclusions land.

## Consequences

### Theory-space has two structural relations

After 0034 + 0042, named theories form a partial structure with:
- **Extension** (directed): A extends B iff members(A) ⊋ members(B).
- **Independence** (symmetric): A ⊥ B iff members(A) ∩ members(B) = ∅.

Together they give the system basic "theory-space geography."

### Commitment 5 (structural similarity)

Independence, like extension, is a purely structural relation —
derived only from the member sets of named theories. No external
labels or semantic judgments. ✓

### Interaction with auto-prune (ADR 0040)

Independence relations are not rewarded by the current score term
(which only rewards extensions). Each independence edge costs 3
meta-R (overhead tax −0.3) with no positive contribution, giving
negative counterfactual value.

This is deliberate for now: extensions carry genuine hierarchical
information (A-knows-about-B); independences mostly characterize
the theory-space, not each theory individually. If auto-prune
removes them, the system has not lost much — they're
re-discoverable by `discover_theory_independences`. A future ADR
could reward them symmetrically if use cases justify it.

### Limits

1. **No transitive closure.** Stored only direct pairs; "A ⊥ B and
   B ⊥ C does NOT imply A ⊥ C" (A and C could share axioms
   independently). Query cost is linear per call.
2. **No conflict relation.** See alternatives.
3. **Meta-R cost linear in |theories|²** in the worst case.
   `discover_theory_independences` enumerates pairs; for a graph
   with hundreds of theories this becomes expensive. β-scale only.

## Verification

- 222 → 230 tests pass (8 new: valid independence, rejects
  overlap, rejects self, canonical ordering, symmetric query,
  discover pairs, retract, meta-id inclusion).

## Implementation

- `v2/src/lib.rs` — `INDEPENDENT_MARKER`, six methods for
  independence relations, `retract_independence`, extended
  `collect_meta_ids`.
- `v2/docs/decisions/0042-theory-independence.md` — this ADR.
