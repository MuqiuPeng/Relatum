# ADR 0070 — Shape-family abstraction layer (formalization) (2026-04-30)

## Status

**Accepted.** All three implementation steps shipped:
- Step 1 (commit `3e5f5e4`) — schema unification + types + query
  methods + kind-tag edge emission. 6 unit tests, 560 lib tests
  pass.
- Step 2 (commit `3011514`) — operation lift: `retract_shape_family`,
  `discover_nested_shape_families_by_member_overlap`,
  `ActionKind::RetractShapeFamily`, `FrontierTarget::ShapeFamily`,
  full persistence round-trip. B.2 + B.8.1 examples migrated. 7
  more unit tests, 567 lib tests pass.
- Step 3 (this commit) — convenience `discover_shape_family_layer`
  + `ShapeFamilyDiscoverySummary` + doc-comment cross-references.
  1 more unit test, 568 lib tests pass.

Supersedes ADR 0068's narrower scope. The layer is now a
documented, queryable, intervenable cognitive abstraction.
Future ADRs (0071 quality report, 0072 intervention classifier)
treat ADR 0070 as a stable platform.

## Context

ADR 0068 (Phase Beta-1) introduced `SHAPE_FAMILY_MARKER` and
`discover_axiom_shape_families`, framed as "the first runtime
extension of structural vocabulary since H1". It was an
empirical breakthrough but a narrow ADR — it specified ONE
discovery kind (shared canonicalized premise) and left the
rest as "future deferred slices".

Eight follow-up slices then landed, each adding a piece without
unifying ADR coverage:

| slice | what it added | where it lives now |
|---|---|---|
| Beta-1 (0068) | premise families + L2 marker | `discover_axiom_shape_families`, `SHAPE_FAMILY_MARKER` |
| B.2 | family-level demote | example only (not in lib) |
| B.3 | conclusion families | merged into `discover_axiom_shape_families` |
| B.4 | family-aware enumeration | `enumerate_axiom_templates_filtered`, `shape_premise_key` |
| B.5 / B.5.1 | runtime ActionKind + scheduler integration | `ActionKind::DiscoverAxiomShapeFamilies`, `FrontierKind::ShapeFamilyDiscoveryCandidate` |
| B.6 | nested families (L3) | `discover_nested_shape_families`, `META_SHAPE_FAMILY_MARKER` |
| B.7 | super-meta families (L4) | `discover_super_meta_shape_families`, `SUPER_META_SHAPE_FAMILY_MARKER` |
| B.8 | L5 audit | example only (no L5 mints on OQ#1) |
| B.8.1 | new L3 kind (member-overlap) | example only (lifts L5 ceiling 0 → 8) |
| F.1 / F.1.1 | per-axiom + per-family quality | `axiom_cross_precision`; F.1.1 inlined in example |

What's now in the codebase amounts to a working **5-layer
cognitive layer** (L0 → L1 → L2 → L3 → L4) with documented
extension to L5 — but no document defines it as a layer. The
layer's existence is provable empirically (all slices positive,
including cross-substrate generalization in C.2) but its
ontology, schema, and operations are scattered.

The user's strategic critique (2026-04-30):

> Shape-family 已经有足够多的 positive slice。下一步危险的不是
> 不够；是它会停留在"实验 grouping"而非成为正式的 cognitive
> layer。需要一个 ADR 把 intension / 命名 / 操作 / 质量维度
> 全部固化下来。

ADR 0070 is that consolidation.

## Decision

Promote shape-family from a discovery mechanism to a **first-class
cognitive abstraction layer** in v2's ontology. The layer
sits between L1 axioms / L1.5 theories and the higher-order
nested abstractions, mirroring the constitutional shape of
existing layers (PATTERN, AXIOM, THEORY).

### 1. Layer definition

The shape-family layer comprises **a recursively self-similar
abstraction stack**:

| layer id | name | members are | marker |
|---|---|---|---|
| L2 | shape family | axioms (L1 instances) | `SHAPE_FAMILY_MARKER` |
| L3 | nested shape family | shape families (L2 instances) | `META_SHAPE_FAMILY_MARKER` |
| L4 | super-meta shape family | nested families (L3 instances) | `SUPER_META_SHAPE_FAMILY_MARKER` |
| L5+ | (recursive) | super-meta families | (deferred — see B.8 audit) |

All layers share the same constitutional shape:
`R(<layer_marker>, <family_id>)` declares membership in the
layer; `R(<family_id>, <member_id>)` declares each member of
the family. This is constitutionally identical to PATTERN /
THEORY meta-R conventions.

**Status of L5+**: B.8 confirmed L5 = 0 on OQ#1 with current L3
discovery kinds; B.8.1 confirmed L5 = 8 with one additional L3
kind. The recursive schema is open-ended; it terminates when no
further structural similarity exists.

### 2. Family kind taxonomy

A "family kind" is a structural similarity predicate over
members at the layer below. Each kind has a deterministic id
naming convention. ADR 0070 inventories kinds discovered to
date and treats this list as **extension-ready** — adding a kind
is a follow-up ADR, not a constitutional change.

| layer | kind id | similarity predicate | id format | introduced |
|---|---|---|---|---|
| L2 | `premise_shared` | byte-identical canonicalized premise edge set | `shape_premise_<canon>` | Beta-1 |
| L2 | `conclusion_shared` | identical canonicalized conclusion edge | `shape_conclusion_c<x>-<y>` | B.3 |
| L3 | `premise_edge_shared` | L2 families sharing a single premise edge | `meta_premise_p<x>-<y>` | B.6 |
| L3 | `member_overlap` | L2 families sharing a member axiom (cross-cutting) | `meta_via_<axiom_id>` | B.8.1 |
| L4 | `member_l2_shared` | L3 families sharing a member L2 family | `super_<sf_id>` | B.7 |

(L3's `member_overlap` is currently inlined in the B.8.1 example
but is the most consequential discovery — it's the single change
that unblocked L5 = 0 → 8. ADR 0070 promotes it to lib status.)

### 3. Schema (intension / extension split)

Per ADR 0029's intension / extension distinction, the family
layer's intension is the set of edges that declare the family's
existence and membership:

```text
# Layer marker — declares the family
R(<layer_marker>, <family_id>)

# Membership — declares each member
R(<family_id>, <member_id>) for each member

# Kind tag (NEW — not currently written)
R(<family_id>, <family_kind_id>)
```

The "kind tag" edge is a small extension introduced by ADR 0070
to make the family kind queryable from rset rather than parsed
from the id string. Existing examples that derive kind from
prefix matching (`shape_premise_*` etc.) continue to work; the
explicit edge is additive.

### 4. Operations (lift to first-class)

The following operations are defined as the layer's API. Some
exist; others are inlined in examples and need promotion.

#### 4.1 Discovery (already in lib)

```rust
pub fn discover_axiom_shape_families(&mut self, min_members: usize) -> Vec<String>;
pub fn discover_nested_shape_families(&mut self, min_member_families: usize) -> Vec<String>;
pub fn discover_super_meta_shape_families(&mut self, min_member_metas: usize) -> Vec<String>;
```

ADR 0070 adds:

```rust
/// Discover all kinds at all layers in one call. Convenience.
pub fn discover_shape_family_layer(&mut self, min_members: usize) -> ShapeFamilyDiscoverySummary;

/// L3 member-overlap kind (B.8.1 promotion).
pub fn discover_nested_shape_families_by_member_overlap(&mut self, min_overlap: usize) -> Vec<String>;
```

#### 4.2 Query (mostly in lib)

```rust
pub fn axiom_shape_families(&self) -> Vec<&str>;
pub fn nested_shape_families(&self) -> Vec<&str>;
pub fn super_meta_shape_families(&self) -> Vec<&str>;
pub fn shape_family_members(&self, id: &str) -> Vec<&str>;
pub fn nested_shape_family_members(&self, id: &str) -> Vec<&str>;
pub fn super_meta_shape_family_members(&self, id: &str) -> Vec<&str>;
```

ADR 0070 adds:

```rust
/// Layer-agnostic member query (works for L2, L3, L4 ids).
pub fn family_members(&self, family_id: &str) -> Vec<&str>;

/// Which layer does this id belong to? Returns L2/L3/L4 or None.
pub fn family_layer(&self, family_id: &str) -> Option<FamilyLayer>;

/// Which kind? Returns the kind id (parsed from id prefix or
/// from the explicit kind-tag edge).
pub fn family_kind(&self, family_id: &str) -> Option<&str>;
```

#### 4.3 Quality (currently in F.1 + inlined in F.1.1)

F.1's `axiom_cross_precision` is in lib. F.1.1's per-family
aggregate is inlined in example. ADR 0070 promotes:

```rust
/// Per-family cross-precision summary: mean, std, min, max.
pub fn family_quality(&self, family_id: &str, substrates: &[RSet]) -> Option<FamilyQuality>;

pub struct FamilyQuality {
    pub mean: f64,
    pub std: f64,
    pub min: f64,
    pub max: f64,
    pub n_members: usize,
}

/// Three-class structural classification (signal/noise/uniform).
pub fn family_quality_class(q: &FamilyQuality) -> FamilyQualityClass;

pub enum FamilyQualityClass {
    Signal,    // mean ≥ 0.80
    Noise,     // mean < 0.50
    Uniform,   // std < 0.05 (orthogonal to signal/noise)
    Mixed,     // anything else
}
```

(Thresholds 0.80/0.50/0.05 are from B.2 + F.1.1 empirics on OQ#1
+ long5k. They are constants in the decision class but reviewable.)

#### 4.4 Intervention (currently in B.2 example only)

ADR 0070 promotes family-level demote to a lib operation and an
ActionKind:

```rust
/// Retract every member of the family. For L2 families: detach
/// each axiom from every theory + globally retract the axiom
/// registration. For L3+: detach each member family without
/// retracting the underlying L2 (deeper retraction would cascade).
pub fn retract_shape_family(&mut self, family_id: &str) -> Result<RetractionSummary, RetractionError>;
```

```rust
// in src/runtime/action.rs
pub enum ActionKind {
    // ... existing variants
    /// ADR 0070 — retract a family wholesale based on
    /// quality-class threshold. Episode delta = number of
    /// underlying axioms retracted.
    RetractShapeFamily { family_id: String },
}
```

#### 4.5 Family-aware enumeration (already in lib via B.4)

`enumerate_axiom_templates_filtered(config, blocked_premise_keys)`
exists. ADR 0070 generalizes the gate to accept any family kind:

```rust
/// Block templates whose canonical structure matches any
/// blocked family kind+key tuple.
pub fn enumerate_axiom_templates_with_family_block(
    config: &AxiomDiscoveryConfig,
    blocks: &[FamilyBlock],
) -> Vec<AxiomTemplate>;

pub struct FamilyBlock {
    pub kind: FamilyKindId,
    pub key: FamilyKey,
}
```

(Existing API stays as a wrapper over the new one, for
back-compat.)

#### 4.6 Persistence

Family registration already round-trips through `to_text` /
`from_text` because all data is stored as ordinary R edges. The
ADR 0070-introduced kind-tag edge is included automatically.

`collect_meta_ids` already includes the three layer markers
(verified in F.1's bonus latent-bug fix). No additional work
needed.

### 5. Constitutional review

| commitment | status | argument |
|---|---|---|
| C1 R singular | PASS | No new R class; family edges are plain R(x,y) |
| C2 R binary | PASS | All edges 2-arity; no n-ary primitives |
| C3 types as meta-R | PASS — strongest realization to date | Family kinds discovered structurally; family ids are tokens registered under existing markers; structurally identical to PATTERN/THEORY |
| C4 token identity | PASS | Family ids are deterministic from canonical structure (id format protocols above); same input → same id across processes |
| C5 similarity is structural | PASS | Each kind's similarity predicate is purely structural; no external metric |

**C3 is where the layer pays the most.** Pre-Beta-1 v2 had a
strict separation between "compile-time declared types" (markers)
and "data-derived instances" (named patterns, theories). Beta-1
collapsed that for shape families: the marker is declared, but
the **kinds and keys** are also data-derived. ADR 0070 makes
this collapse load-bearing across L2-L4 uniformly.

### 6. What ADR 0070 explicitly does NOT do

- **Does not introduce new behavior.** Every operation listed
  exists either in lib or in an example. ADR 0070 promotes,
  unifies, and documents.
- **Does not change discovery behavior.** Existing call sites
  return identical results. New convenience methods are
  additive.
- **Does not auto-trigger family demote.** B.2's manual demote
  becomes available as a runtime ActionKind, but no policy
  yet decides when to fire it. That's ADR 0072's job
  (intervention classifier).
- **Does not standardize family quality consumption.** F.1.1's
  classification is exposed; how callers should USE it (e.g.,
  in tournament selection, merge-pair scoring) is left to
  ADR 0071 (unified theory-quality report).
- **Does not commit to L5+.** B.8.1 demonstrated L5 is
  reachable; the lib promotion of B.8.1's kind is included,
  but the recursive layer-N discovery API is not formalized
  beyond L4. Adding new layers is a follow-up ADR per layer.
- **Does not remove any existing API.** Backward compatible
  for all existing examples and tests.

### 7. Layer hierarchy as constitutional progress

The full v2 abstraction lattice as of ADR 0070:

```
L0:   data            R(x, y) instances
L1:   axioms          R(__axiom__, ax_id) + intension
L1.5: theories        R(__theory__, t_id) + members
L2:   shape families  R(__shape_family__, fam_id) + members ← formalized here
L3:   nested families R(__meta_shape_family__, meta_id) + members ← formalized here
L4:   super-meta      R(__super_meta_shape_family__, super_id) + members ← formalized here
L5+:  (recursive)     same shape, deferred to per-layer ADR
```

This lattice is the structural backbone of v2's cognitive
ontology. Higher layers derive from lower; each layer's
instances are data-derived and registered under a declared
marker. The layer markers themselves are compile-time (commitment
3 admits this — what's data-derived is INSTANCE population,
not category declaration).

## Alternatives considered

### A. Keep families as a discovery mechanism only

**Rejected.** This is the status quo. The strategic problem
(experiment heap vs. system) doesn't get solved without
formalization.

### B. Collapse all layers into one "family" type

**Rejected.** Layers L2/L3/L4 differ in MEMBER TYPES (axioms vs
families vs nested-families). Collapsing them obscures the
recursive structure. The layered marker scheme matches
constitutional convention.

### C. Formalize each layer separately (L2, L3, L4 as 3 ADRs)

**Rejected.** The whole point of consolidation is to recognize
the layers SHARE schema and ops. Three ADRs would re-fragment
what should be one document. If a future layer L5 needs unique
operations, it gets its own ADR — but L2-L4 are uniform.

### D. Skip ADR; just refactor lib

**Rejected.** ADR-less refactor leaves the strategic
"experiments vs. system" question open. The user's framing was
explicit: write the ADR FIRST so the layer's constitutional
position is documented, THEN refactor against it.

## Consequences

### What becomes easy

- Family layer is now a documentable, queryable, intervenable
  cognitive layer — referenceable from future ADRs (0071
  quality report, 0072 intervention classifier)
- New L2/L3 kinds can be added as small slices, each one
  promoting to lib via the schema in §4
- Family-level demote becomes a real ActionKind, available to
  the scheduler via the same mechanism as
  `DiscoverAxiomShapeFamilies` (analog of B.5.1's wiring)
- The user's reframing ("Relatum is a system with theory
  structure, not an experiment heap") becomes literally true
  for this region of code

### What becomes harder

- New L2/L3/L4 kinds are now ADR-gated. Adding `shape_arity`
  or `shape_symmetry` requires a follow-up ADR explaining the
  similarity predicate, naming convention, and empirical
  evidence. This is the right friction.
- Future reorganizations of the layer markers (e.g.,
  unifying them under a single LAYER_MARKER chain) would
  break ADR 0070's schema. Such changes need a superseding
  ADR with migration plan.

### Deferred

- **L5 lib API.** B.8.1 promoted to lib; B.8 audit
  (per-substrate prerequisite check) deferred. Add when L5
  actually mints on a real substrate.
- **Family quality DRIVE.** Could a future drive consume
  family quality directly? Open. Not in 0070.
- **Cross-layer queries.** "Which L2 families are in any L3?"
  — ergonomic but absent. Add when needed.
- **Family lifecycle (ESTABLISHED-promotion analog).**
  Patterns have ESTABLISHED markers; families could too. Not
  in 0070; conditional on demand.

## Implementation

This ADR specifies the consolidation; implementation lands in
phased steps:

### Step 1 — schema unification (no behavior change)

- Add `family_members(id)`, `family_layer(id)`, `family_kind(id)`
- Add `FamilyQuality` struct + `family_quality()` method
- Add explicit kind-tag edge writing during discovery
- All existing tests pass; existing examples produce identical
  output

**Verification**: 554 lib tests pass; 7 family-related examples
(`phase_beta_1_*`, `phase_b51_*`, `phase_beta_6_*`, `phase_b7_*`,
`phase_b81_*`, `phase_f11_*`) produce byte-identical logs.

### Step 2 — operation lift

- `retract_shape_family(id)` lifted from B.2 example
- `ActionKind::RetractShapeFamily` in action.rs +
  persistence.rs + autonomous.rs `execute_action` arm
- `discover_nested_shape_families_by_member_overlap` lifted
  from B.8.1 example

**Verification**: B.2 + B.8.1 examples re-implemented using
new lib API; outputs unchanged. New unit test:
`shape_family_layer_retract_clears_axiom_count`.

### Step 3 — convenience and docs

- `discover_shape_family_layer(min)` convenience method
  (chains L2 + L3 + L4)
- doc-comments on all public items reference ADR 0070
- README.md decision index updated

**Verification**: All examples build; convenience method
produces same family set as manual chaining.

Each step is independently committable. Step 1 is the riskiest
(adds an edge type to discovery output); Steps 2/3 are
mechanical.

## Touched ADRs

- **ADR 0029** (intension/extension split) — family layer
  follows the schema convention
- **ADR 0030** (theory objects) — family layer is structurally
  parallel to theories (registration + members)
- **ADR 0034** (theory extension relations) — future ADR could
  add `family_extends`/`family_disjoint` analogs (out of scope)
- **ADR 0053** (selective declarativization / ESTABLISHED) —
  family layer ESTABLISHED-promotion is deferred but
  constitutionally compatible
- **ADR 0064** (drives as meta-R) — family layer follows the
  same marker-then-instances convention introduced for drives
- **ADR 0068** (axiom shape families, Beta-1) — narrow original
  introduction; ADR 0070 supersedes its scope (not its content)
  by formalizing the layer

## Future ADRs gated on this one

- **ADR 0071** — Unified theory-quality report (consumes family
  quality + cross-precision + primary hit rate)
- **ADR 0072** — Intervention policy classifier (uses family
  quality classification to choose demote/repair/merge)
- (deferred) **ADR 0073** — L5 lib API + new L3 kinds beyond
  member-overlap

## Verification

Step-by-step verification plan (per Implementation phasing
above). The standing test suite (554 lib tests as of 2026-04-30
+ runtime integration tests) is the safety net; new tests are
added per step. No empirical regressions allowed; behavior is
strictly additive.

## Status

Proposed. Awaiting Step 1 implementation.

---

*Author's note: ADR 0070 is the first explicit consolidation
ADR in v2. It does not introduce a new mechanism — it states
that the existing eight slices ARE one mechanism, and gives
that mechanism a name, a schema, and a place in the
constitutional layer hierarchy. Subsequent ADRs (0071, 0072)
will treat ADR 0070 as a stable platform.*
