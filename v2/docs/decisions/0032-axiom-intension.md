# 0032: Axiom intension as meta-R

Status: Accepted (extends ADR 0030)
Date: 2026-04-24

## Context

ADR 0030 established theory objects but stopped at axiom **names** in
meta-R. The axiom id `ax_tpl_v3_p0-1_p1-2_c0-2` was a deterministic
serialization of the template's canonical form, but its premise /
conclusion / variable structure existed only implicitly (decodable
from the id string, but never materialized as R edges). Commitment 3
was therefore satisfied for theories (membership in meta-R) but not
for axioms themselves.

ADR 0032 (task B of the A → C → B → D plan) closes that gap:
template axioms now carry their full structural intension as meta-R,
analogous to what ADR 0029 did for pattern types.

## Decision

### Reserved markers

```rust
pub const AXIOMVAR_MARKER:   &str = "__axiomvar__";
pub const PREMISE_MARKER:    &str = "__premise__";
pub const CONCLUSION_MARKER: &str = "__conclusion__";
```

### Intension encoding — chain form

For a template axiom `ax_X` with `num_vars = n`, `|premise| = m`:

```
R(AXIOM_MARKER, ax_X)                        × 1       (registry — from 0030)

R(AXIOMVAR_MARKER, ax_X_var_i)               × n       (variable registry)
R(ax_X, ax_X_var_i)                          × n       (axiom owns variable)

R(PREMISE_MARKER, ax_X_prem_j)               × m       (premise-edge registry)
R(ax_X, ax_X_prem_j)                         × m       (axiom owns premise edge)
R(ax_X_var_{x_j}, ax_X_prem_j)               × m       (premise j's source)
R(ax_X_prem_j, ax_X_var_{y_j})               × m       (premise j's target)

R(CONCLUSION_MARKER, ax_X_concl)             × 1       (conclusion-edge registry)
R(ax_X, ax_X_concl)                          × 1       (axiom owns conclusion)
R(ax_X_var_{cx}, ax_X_concl)                 × 1       (conclusion's source)
R(ax_X_concl, ax_X_var_{cy})                 × 1       (conclusion's target)
```

Every premise / conclusion edge becomes a **3-node chain** through
the edge's identifier node:

```
var_x  →  edge_node  →  var_y
```

Direction of R itself encodes source vs. target, no extra markers
needed per edge. Total: `2n + 4m + 4` edges per template axiom.

- Transitivity (n=3, m=2): 18 edges
- Symmetry (n=2, m=1):     12 edges

Predicate axioms (`ax_reflexivity`, `ax_antisymmetry`) get only
the registry edge. Their semantics live in the predicate checkers;
the current template language cannot express them. This asymmetry
is documented but unresolved; a future ADR could extend the
template form to admit disjunctive / equality-conclusion axioms.

### API

Additions on `RSet`:

```rust
pub fn register_axiom_with_intension(&mut self, id: &str) -> bool;

pub fn axiom_variables(&self, axiom_id: &str) -> Vec<&str>;
pub fn axiom_premise_edges(&self, axiom_id: &str) -> Vec<&str>;
pub fn axiom_conclusion(&self, axiom_id: &str) -> Option<String>;

pub fn reconstruct_axiom_template(&self, axiom_id: &str) -> Option<AxiomTemplate>;
pub fn retract_axiom(&mut self, axiom_id: &str) -> Result<usize, TheoryError>;
```

`register_axiom_with_intension` replaces the ADR 0030 bare
`R(AXIOM_MARKER, id)` write inside `name_theory`: now every
new template axiom gets its intension written at theory-naming
time. Predicate axioms fall through to the registry-only path.

`reconstruct_axiom_template` reads the stored chain structure and
returns the equivalent `AxiomTemplate`. It's the inverse of
`register_axiom_with_intension` for template axioms; for predicate
axioms it returns `None` (by design).

`retract_axiom` removes the full intension (variables, premise and
conclusion chains, registry). Refuses if any theory still
references the axiom — theories must be retracted first.

### `collect_meta_ids` extension

Now includes:
- The three new markers
- Every variable / premise / conclusion id for every registered
  axiom

This keeps `run_naming_pass`'s meta-subgraph filter honest: axiom
intension ids are meta, not data.

## Alternatives considered

- **Edge reification with explicit src / tgt markers.** Would
  have required `__src__` / `__tgt__` markers plus two edges per
  endpoint. Rejected — adds 4 edges per premise/conclusion edge
  with no information the direction already encodes.
- **One edge per template edge directly: `R(var_x, var_y)`.**
  Rejected — conflates premise and conclusion; duplicate
  endpoints would collapse (set semantics); no way to recover
  which are premise and which are conclusion.
- **Hash-only intension.** Store just a content hash of the
  template and compute bindings at matching time. Rejected — the
  whole point of ADR 0032 is to land commitment 3 for axioms.
  A hash is not intension.
- **Embed the entire template as a serialized string in a single
  R instance** (`R(ax_X, "ax_tpl_...")`). Rejected — that's what
  the id string itself is. We need the structure to be
  R-inspectable.

## Consequences

### Commitment 3 fully lands for axioms

After ADR 0029, pattern types had intension in meta-R. After ADR
0030, theories had their *membership* in meta-R. After ADR 0032,
template axioms also carry their intension. Every named meta-R
object in v2 now has its definition (not just its name) in R.

The holdouts are predicate axioms — but this is an expressive
limit of the current template language, not a commitment-3
failure. A predicate axiom's "intension" would need a richer
template form (empty premise for reflexivity, equality conclusion
for antisymmetry). That's a future ADR if the need arises.

### Cost growth

For a theory on a diamond poset with 3 members (transitivity,
reflexivity, antisymmetry):
- Transitivity intension: 18 edges
- Reflexivity: 1 (registry only)
- Antisymmetry: 1 (registry only)
- Theory membership: 3
- Theory registry: 1
- **Total: 24 edges per theory-with-intension** (was 5 in 0030).

For the equivalence case with 6 member axioms (4 template variants
+ symmetry + reflexivity): each template is n=2/3, m=1/2. Roughly
12–18 edges per template axiom + 1 for reflexivity + 6 membership
+ 1 registry ≈ 80 edges.

This is acceptable. Theories are few per RSet, and the intension
is constant per axiom (independent of how many RSets reference
it, since an axiom's intension is fixed by its template).

### reconstruct roundtrip

For both transitivity and symmetry, `reconstruct_axiom_template`
returns exactly the original template. Verified by
`adr0032_reconstruct_roundtrip_transitivity` and
`adr0032_reconstruct_roundtrip_symmetry`. This is the
"name means what meta-R says" integrity property.

### Interaction with discover / minimal / drive

`discover_axioms`, `discover_axioms_minimal`, `check_reflexivity`,
`check_antisymmetry`, `discover_theory`, `intrinsic_drive` all
read only data edges (via `collect_meta_ids` exclusion). Axiom
intension lives entirely in meta-R and is invisible to these
operations. Verified by
`adr0032_axioms_do_not_pollute_data_discovery`.

### `intrinsic_drive` score effect

The drive metric's overhead-tax term `−0.1 × meta-R` now reflects
axiom intension size. On the poset case, final score dropped from
~14 (0030 intension absent) to comparable numbers after the larger
meta-R footprint. The drive still converges and picks the same
action orders — intension overhead is linear and small.

### Implementation note

The premise/conclusion recovery uses `right_of` and `left_of`
carefully:
- `right_of(edge_id)` returns edges with edge_id on the right side
  (target). So `R(var_x, edge_id)` shows up there; `r.x` is var_x
  (the source).
- `left_of(edge_id)` returns edges with edge_id on the left side
  (source). So `R(edge_id, var_y)` shows up there; `r.y` is var_y
  (the target).

A first implementation had this inverted and failed the
reconstruction tests. Fixed in the commit.

## Verification

- `cd v2 && cargo test` → 162 → 170 (8 new).
- Reconstruction roundtrip passes for both template axioms in the
  rigorous battery.
- `adr0032_axioms_do_not_pollute_data_discovery` confirms axiom
  intension doesn't leak into data-space discovery.

## Implementation

- `v2/src/lib.rs` — three new markers, `register_axiom_with_intension`,
  `axiom_variables`, `axiom_premise_edges`, `axiom_conclusion`,
  `reconstruct_axiom_template`, `retract_axiom`, extended
  `collect_meta_ids`.
- `v2/docs/decisions/0032-axiom-intension.md` — this ADR.
- `v2/docs/progress.md`, `v2/README.md`, decisions index.
