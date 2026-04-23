# 0029: Intension vs extension for pattern naming

Status: Accepted (partially supersedes ADR 0010)
Date: 2026-04-24

## Context

Reviewing v2's progress against the "property relations, not fact
relations" stance turned up a structural mismatch inside ADR 0010.
The three-shape encoding from 0010 is:

```
R(__pattern__, p_N)            × 1         ← one "type exists" edge
R(p_N, p_N_i_M)                × N         ← N instance registrations
R(p_N_i_M, participant)        × N·k       ← N·k participant bindings
```

Of these, only the first is genuinely the **intension** of the type
(its structure as an abstract object). The remaining ≈ N·(k+1) edges
are the **extension** (which specific witnesses and which specific
tokens appeared in them). Even the canonical form — the thing that
actually defines the type — is not stored; ADR 0010 recovers it by
taking the first instance, restricting RSet edges to its
participants, and re-canonicalizing. That has two consequences:

1. **Commitment 3 is not fully expressed.** "Types are meta-R
   instances" should mean the type's structure lives in meta-R. Under
   0010, only the type's *name* lives in meta-R; its structure is
   implicit, conditional on a surviving first instance.
2. **Fact layer dominates pattern meta-R.** At scale, N · (k+1)
   dwarfs the property-layer information. This collides with the
   "property not fact" stance: v2 is a system that abstracts over
   facts, not one that accumulates them.

This ADR introduces an explicit **intension / extension split** in
meta-R, so that pattern naming writes the type's structure directly
into R, and extension recording becomes a configurable choice rather
than an invariant.

## Decision

### Layer A — pattern intension (property, always written on first mint)

```
R(__pattern__, p_N)                × 1             registry
R(__role__, p_N_role_i)            × k             role registry
R(p_N, p_N_role_i)                 × k             pattern owns roles
R(p_N_role_i, p_N_role_j)          × e             structural edges
```

Where `k` is the number of distinct participants in the first
instance (sorted alphabetically, assigned indices 0..k-1) and `e` is
the number of edges in that instance. Role ids are `p_N_role_i`.

Role ids are **instance-dependent in their labeling but
isomorphism-invariant in the structure they encode**. The first
instance's edges, expressed over role indices rather than token ids,
have the same canonical form as any other instance of the same type.
So comparison via `Subgraph::canonicalize` on the role-subgraph
yields the same answer for any first instance that could have been
chosen.

`ROLE_MARKER = "__role__"` is a new reserved marker, on the same
footing as `PATTERN_MARKER`.

### Layer B — pattern extension (facts, configurable)

```rust
pub enum PatternRecordingPolicy {
    Intensional,     // no instance records at all
    InstancesOnly,   // R(p_N, p_N_i_M) per instance, no participants
    FullBindings,    // R(p_N, p_N_i_M) + R(p_N_i_M, participant) (ADR 0010 legacy)
}

impl Default for PatternRecordingPolicy {
    fn default() -> Self { PatternRecordingPolicy::FullBindings }
}
```

### API

```rust
impl RSet {
    // Existing. Backward-compatible: calls the policy version with FullBindings.
    pub fn name_pattern_instances(
        &mut self,
        instances: &[Subgraph],
    ) -> Result<String, PatternError>;

    // New.
    pub fn name_pattern_instances_with_policy(
        &mut self,
        instances: &[Subgraph],
        policy: PatternRecordingPolicy,
    ) -> Result<String, PatternError>;

    // New — Layer A queries.
    pub fn roles(&self) -> Vec<&str>;
    pub fn pattern_roles(&self, pattern: &str) -> Vec<&str>;
    pub fn pattern_structure(&self, pattern: &str) -> Option<CanonicalForm>;
    pub fn is_role(&self, id: &str) -> bool;
}
```

### `find_pattern_matching` upgrade

Primary path reads Layer A (`pattern_structure`) and compares
canonical forms directly. Fallback path — for patterns created
before ADR 0029 — is the original ADR 0010 first-instance recovery.
Once a pattern has Layer A, the fallback never fires.

### `retract_pattern` upgrade

Removes the full stack: participant edges, ownership edges,
Layer A structural edges, `R(pattern, role)` edges, `R(__role__,
role)` entries, and finally `R(__pattern__, pattern)`. Order is
chosen so that mid-retraction state remains self-consistent
(instances removed before ownership, ownership before registry).

### Knock-on fixes

- `instances_of(p)` now filters out role ids (both share the
  `R(p, *)` shape). Without this filter, Layer A roles would appear
  as "instances".
- `memberships_of(id)` skips role ids when resolving "is the parent
  of this edge an instance?" — same reason.
- `collect_meta_ids` picks up `ROLE_MARKER` and every role id so
  that `run_naming_pass`'s meta-subgraph skip continues to work.

## Alternatives considered

- **Use WL-labels as role indices.** Rejected: WL collapses symmetric
  nodes to a single label. A 3-star's canonical form `[(0,1), (0,1),
  (0,1)]` would give only two roles, and the three distinct edges
  would deduplicate in meta-R's set semantics — multiplicity lost.
  Using first-instance sorted indices keeps structure exact.
- **Edge-as-object reification.** Each canonical edge would get its
  own `p_N_edge_j` identifier with `R(p_N_edge_j, source_role)`,
  `R(p_N_edge_j, target_role)`. Preserves multiplicity but triples
  the edge count per structural edge, and introduces yet another
  reserved marker. The sorted-index approach costs one edge per
  structural edge — same density as the canonical form itself.
- **Supersede 0010 fully and break old RSets.** Rejected. Old
  persistent data should still be queryable. The legacy fallback in
  `find_pattern_matching` is a single `if stored.is_none()` branch
  and costs nothing new.
- **Bump `PatternRecordingPolicy::default()` to `InstancesOnly` or
  `Intensional`.** Considered. Rejected for this ADR — changing the
  default would silently alter every existing caller. The knob is
  available; choosing the right default is a separate decision that
  can be made after the shape of real usage becomes visible.

## Consequences

### Commitment 3 now lands

Before 0029, `R(__pattern__, p_N)` was the only meta-R that
genuinely expressed the type's existence, and even that required
reading first-instance edges to recover what the type actually was.
After 0029, the four intension edge families above fully express the
type. Any query that wants to know "what is p_N?" can read Layer A
and stop — no instance required.

### Property vs fact is now a dial, not an invariant

Callers can pick Intensional (no facts), InstancesOnly (instance
pointers only, no token bindings), or FullBindings (ADR 0010
behavior). The dial makes the stance from the property-not-fact
feedback memory concrete. Default stays on FullBindings so existing
workflows are unchanged; callers that want the cleanest property
layer just pass `Intensional`.

### Layer A cost is constant per pattern

For a pattern with k participants and e edges (in first instance):
Layer A = 1 + k + k + e = 2k + e + 1 edges. Independent of N
(instance count). On a 3-chain: 9. On a 3-cycle: 10. On a 3-star:
8. This is a one-time, per-type cost — not per-instance.

### Edge-count impact on the canonical example

`cargo run --example pattern_naming` on the ADR 0007 mixed graph:

| | ADR 0010 only | ADR 0029 (FullBindings default) |
|---|---:|---:|
| meta-R edges added | 31 (historical) | 68 |
| of which Layer A | — | 33 |
| of which Layer B | 31 | 35 |

Under `Intensional` mode on the same graph, the 35 Layer B edges
disappear, and total meta-R added is 33 — already below the old
31-edge baseline would be a wash, and on larger graphs (N ≫ k) the
Intensional mode wins decisively.

### Interaction with other mechanisms

- **compound_class_subgraphs, discover_motifs, find_instances_of,
  autonomous_pass**: all read *data edges* (via
  `data_edges_sorted`). Layer A lives in meta-R and is excluded by
  the same filter that already excluded 0010 meta-R. No change.
- **cross-graph pattern transfer (ADR 0023)**: transfers canonical
  forms plus fresh instances. Will now also re-emit Layer A on the
  destination. Net: richer intension carried with the canonical, no
  API change needed.
- **axiom discovery (ADR 0027/0028)**: operates on
  `data_edges_sorted` and filters out `collect_meta_ids`. Layer A
  roles are now in meta-R by construction, so axiom discovery never
  sees them. No change.

### Limits kept open

- **Automorphism quotienting.** A 3-star has 4 roles under this
  encoding (center + 3 leaves indexed separately), even though three
  of them are automorphism-equivalent. A future ADR could fold
  automorphic roles into equivalence classes, which would give a
  more compact intension at the cost of losing per-position
  distinctness. Not done here; the current encoding is structurally
  faithful, which is the more conservative choice.
- **Role-filler bindings.** Layer B currently records
  `R(instance, participant)` without saying *which role the
  participant fills*. A future ADR can reify that via an extra
  marker (`__filler__`) if needed. Not done; no current consumer
  requires it.
- **Composed/nested types.** Intension is currently a flat graph of
  role edges. If a type's definition refers to another type (e.g.,
  "a chain of 3-chains"), Layer A has no vocabulary for that yet.
  Out of scope.

## Verification

- `cd v2 && cargo test` → 134 → 143 tests pass (9 new tests for
  Layer A writes, each policy mode, legacy fallback, retraction,
  meta-id collection).
- `cd v2 && cargo run --example pattern_naming` → unchanged
  user-visible structure; meta-R count bumped from 31 to 68 due to
  new Layer A edges under default FullBindings policy.
- `cd v2 && cargo run --example axiom_rigorous_test` → identical
  outputs to the 0028 run (axiom discovery operates over data
  edges, never touches Layer A).

## Implementation

- `v2/src/lib.rs` — `ROLE_MARKER`, `PatternRecordingPolicy`,
  updated `name_pattern_instances*`, new `roles`, `pattern_roles`,
  `pattern_structure`, `is_role`, upgraded `find_pattern_matching`,
  upgraded `retract_pattern`, fixes to `instances_of` and
  `memberships_of`, extended `collect_meta_ids`.
- `v2/docs/decisions/0029-intension-extension-split.md` — this ADR.
- `v2/docs/constitution.md` — footnote on commitment 3 clarifying
  intension vs extension (does not amend the commitment).
- `v2/docs/progress.md` — entry.
- `v2/README.md` — status bump.
- `v2/logs/2026-04-24_intension_extension.log` — before/after meta-R
  counts, per-example check of three policies.
