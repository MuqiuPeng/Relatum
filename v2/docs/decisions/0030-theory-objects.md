# 0030: Theory objects (conjunctive concept naming)

Status: Accepted
Date: 2026-04-24

## Context

After ADR 0028, `discover_axioms_minimal` returns a minimal set of
axioms that hold on an RSet. The user's diagnostic:

> The system can do axiom mining, but not conjunctive concept naming.
> On an equivalence relation it finds symmetry, transitivity, and
> reflexivity (via the separate check) — but "equivalence relation"
> itself is never constructed as a single object.

This is the gap between "fragment rules" and "theoretical objects."
Patterns (ADR 0010/0029) gave us a story for **structural** types
(the type is a meta-R object defined by its intension). Axioms (ADR
0027/0028) gave us **individual rules**. Missing: a meta-R object
that bundles multiple axioms under one identity.

ADR 0030 adds that bundling as a first-class mechanism, without
injecting any external concept library — theories arise from the
axioms that happen to hold on the current RSet, not from a
pre-compiled taxonomy.

## Decision

### Reserved markers

```rust
pub const AXIOM_MARKER:  &str = "__axiom__";
pub const THEORY_MARKER: &str = "__theory__";
pub const AX_REFLEXIVITY:  &str = "ax_reflexivity";
pub const AX_ANTISYMMETRY: &str = "ax_antisymmetry";
```

### Axiom ids

Every axiom has a stable, deterministic id:

- **Template-based axioms** derive their id from the canonicalized
  `AxiomTemplate`:
  `ax_tpl_v{num_vars}_p{x}-{y}_..._c{cx}-{cy}`.
  Examples:
    transitivity `[R(0,1), R(1,2)] ⇒ R(0,2)`
      → `ax_tpl_v3_p0-1_p1-2_c0-2`
    symmetry `[R(0,1)] ⇒ R(1,0)`
      → `ax_tpl_v2_p0-1_c1-0`
- **Predicate axioms** (not expressible as single-edge conclusions)
  use fixed strings: `ax_reflexivity`, `ax_antisymmetry`.

Template-based ids are reversible via `axiom_id_to_template(&str) ->
Option<AxiomTemplate>` so `name_theory` can re-verify that a
claimed member still holds.

### Storage

```
R(AXIOM_MARKER, ax_X)         — axiom is registered in this RSet
R(THEORY_MARKER, t_N)         — theory exists
R(t_N, ax_i)                  — theory t_N contains axiom ax_i
```

No axiom *intension* is stored here — that's deferred to B (ADR
0031 or later). The A-phase registry is just a name.

### API on RSet

```rust
impl RSet {
    pub fn discover_theory(&self, config: &AxiomDiscoveryConfig) -> Theory;
    pub fn name_theory(&mut self, axiom_ids: &[&str]) -> Result<String, TheoryError>;
    pub fn retract_theory(&mut self, theory_id: &str) -> Result<usize, TheoryError>;

    pub fn axioms(&self) -> Vec<&str>;
    pub fn is_axiom(&self, id: &str) -> bool;
    pub fn theories(&self) -> Vec<&str>;
    pub fn is_theory(&self, id: &str) -> bool;
    pub fn theory_axioms(&self, theory: &str) -> Vec<&str>;
    pub fn theories_containing(&self, axiom_id: &str) -> Vec<&str>;
}

pub fn axiom_template_id(template: &AxiomTemplate) -> String;
pub fn axiom_id_to_template(id: &str) -> Option<AxiomTemplate>;
```

### `discover_theory` semantics

1. Run `discover_axioms_minimal(config)` → collect the minimal
   template axioms.
2. Call `check_reflexivity`; if rate == 1.0 and identifier count > 0,
   include `AX_REFLEXIVITY`.
3. Call `check_antisymmetry`; if `holds && directed_pairs_checked >
   0`, include `AX_ANTISYMMETRY`.
4. Return `Theory { id: "", member_axiom_ids, template_members }`
   (id empty until named).

### `name_theory` semantics

1. Reject empty input → `EmptyMemberList`.
2. Verify each member actually holds. Template ids go through
   `axiom_id_to_template` → `evaluate_axiom_template`; predicate
   ids dispatch to the predicate checks. Any failure →
   `UnsatisfiedMember(id)`. Unparseable id → `UnparseableAxiomId`.
3. Look for an existing theory whose member set equals the input
   (`HashSet` equality, ignores ordering). If found, reuse its id.
4. Otherwise mint `t_N` and write registry + axiom registrations
   (for previously-unseen ax ids) + membership edges.

### `retract_theory` semantics

Removes the theory registry and its membership edges. Does **not**
remove axiom registrations — they may be shared with other
theories.

### Known-collisions with existing mechanisms

- `memberships_of`, `pattern_of` operate on pattern ids; they
  inspect `pattern_set`, which contains only `R(__pattern__, *).y`.
  Theory ids and axiom ids never appear in `pattern_set`, so no
  false positives.
- `instances_of` / `pattern_roles` filter by role ids; theory and
  axiom ids aren't roles, so those queries ignore them.
- `collect_meta_ids` extended to include `AXIOM_MARKER`,
  `THEORY_MARKER`, all registered axiom ids, and all theory ids.
  Ensures `run_naming_pass`'s meta-subgraph skip continues to work.

## Alternatives considered

- **Compile a concept library** (equivalence = {sym, trans, refl},
  poset = {refl, antisym, trans}, etc.) and match against it.
  **Rejected.** That injects external labels; violates the stance
  behind commitment 5 (similarity is structural — no hand-coded
  labels). The current design lets an "equivalence-shaped" theory
  get named `t_N`; humans can call it equivalence, but the system
  identifies it purely by membership-set equality.
- **Cross-RSet concept discovery** (observe that multiple RSets
  satisfy the same theory fingerprint and name the fingerprint).
  **Rejected for now.** Needs a notion of "multiple R worlds,"
  which v2 doesn't have. Could revisit after C.
- **Embed axiom intension at name time.** Would merge this ADR
  with task B. **Rejected.** Cleaner to separate: A builds the
  conjunctive layer; B enriches each axiom with its structural
  meta-R. Current axiom id is a deterministic string, so B can
  attach intension under the same id without renaming.

## Consequences

### Theory as structural identity

Two RSets that both support `{ax_sym, ax_trans, ax_refl}` produce
the same fingerprint and, if both called `name_theory`, would share
a single `t_N` id under member-set lookup. Identity is structural,
consistent with commitment 5.

### Works on top of existing minimization

`discover_theory` calls `discover_axioms_minimal` (ADR 0028), so
theories are automatically phrased in terms of the reduced axiom
set, not the raw discovery output. Same reason equivalence gives
six members rather than forty-five.

### Commitment 3 stretches one more step

Pattern types (ADR 0029) had their intension materialized in
meta-R. Theory types here have their *membership* materialized.
The full "theory has intension too" story needs the axioms
themselves to carry intension, which is B's job. A-phase is
faithful to commitment 3 in that theories' identity is structural
meta-R (`R(t_N, ax_i)` tells you what t_N is); it just defers the
deeper question of *what a single axiom is* to B.

### Fingerprint quality depends on the axiom space

On cases 4, 5, 8 — broken transitive, random sparse, complete
bipartite — the theory fingerprint reduces to `{ax_antisymmetry}`.
All three genuinely have antisymmetry holding at rate 1.0 over
their directed pairs (no mutual edge exists anywhere). The
fingerprint is correct for what it claims, but it doesn't
discriminate among these inputs. Real differences among them live
at the pattern layer (different motif distributions), which ADR
0030 does not pretend to address. Theory identity is coarser than
pattern identity, by design.

### Size

Meta-R added by a theory write is `1 + (new axioms) + |members|`.
On the rigorous battery this is 3–13 edges per case.

## Verification

- `cd v2 && cargo test` → 143 → 155 tests pass (12 new).
- `cd v2 && cargo run --example theory_discovery` → prints the
  per-case theory fingerprints. See
  `logs/2026-04-24_theory_discovery.log` for the full capture.

## Implementation

- `v2/src/lib.rs` — constants, `AxiomTemplate` id codec, `Theory`
  / `TheoryError`, 10 new RSet methods, extended
  `collect_meta_ids`.
- `v2/examples/theory_discovery.rs` — 8-case demo.
- `v2/logs/2026-04-24_theory_discovery.log` — experiment log.
- `v2/docs/decisions/0030-theory-objects.md` — this ADR.
- `v2/docs/progress.md` — entry.
- `v2/README.md` — status bump.
