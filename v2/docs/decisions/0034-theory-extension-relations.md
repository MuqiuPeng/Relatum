# 0034: Theory extension relations

Status: Accepted
Date: 2026-04-24

## Context

After ADR 0030 the system can recognize that two RSets satisfy the
same theory via structural fingerprint equality. What it could NOT
yet say is that one theory is a *proper extension* of another — the
system could not observe, for example, that "full poset" has
strictly more axioms than "strict partial order" and name that
relation explicitly.

Task 1 of the approved five-step extension (after A→C→B→D) adds
theory-to-theory extension as a first-class meta-R object.

## Decision

### Marker and encoding

```rust
pub const EXTENDS_MARKER: &str = "__extends__";
```

A "T_sub extends T_super" relation is encoded as a three-edge chain,
same direction-as-role convention used for axiom premise/conclusion
edges:

```text
R(__extends__, ext_N)       — registry
R(T_sub,       ext_N)       — source-side chain link
R(ext_N,       T_super)     — target-side chain link
```

One extension relation costs three edges. The chain's direction
encodes "which is sub" vs. "which is super" without needing
separate markers.

### API

```rust
impl RSet {
    pub fn name_theory_extension(&mut self, sub: &str, super_: &str) -> Result<String, TheoryError>;
    pub fn extension_edges(&self) -> Vec<&str>;
    pub fn extension_endpoints(&self, ext_id: &str) -> Option<(String, String)>;
    pub fn theory_extends(&self, theory: &str) -> Vec<&str>;
    pub fn theory_extended_by(&self, theory: &str) -> Vec<&str>;
    pub fn discover_theory_extensions(&self) -> Vec<(String, String)>;
}
```

`name_theory_extension` verifies:
1. Both theories exist.
2. They're distinct.
3. `members(T_sub) ⊇ members(T_super)` (strict superset recommended;
   equal-member case reuses `t_N` via ADR 0030 already, so doesn't
   need an extension edge).

On success, reuses an existing ext id if the same (sub, super) pair
already has one, else mints `ext_N`.

`discover_theory_extensions` scans all pairs of named theories and
returns the strict-superset pairs. Doesn't write; callers combine
with `name_theory_extension` to persist selected relations.

## Alternatives considered

- **Direct edge `R(T_sub, T_super)`**. Rejected — no way to
  disambiguate this from any future T-to-T relation (equivalence,
  refinement, similarity). The marker + chain pattern keeps the
  relation kind explicit and leaves room for future relation kinds
  without retagging.
- **Auto-name every discovered extension**. Rejected — ADR 0030's
  pattern is "discover is read-only, name is explicit." Preserved
  here. Callers opt in.
- **Transitive-closure storage**. When A extends B and B extends
  C, we could auto-insert A extends C. Rejected — stored only
  direct extensions; transitive reasoning is a query-time operation.

## Consequences

- **First higher-order relation in v2.** All prior meta-R linked
  objects to their definitions; 0034 is the first to link objects
  to *each other* structurally.
- **Query cost.** `theory_extends(T)` is O(|extensions| · cost of
  decode). Linear scan; fine for β-scale.
- **Commitment 5.** Extension is purely structural — `members(T)`
  is a derived structural set. No external labels. ✓

## Verification

- 176 → 182 tests pass (6 new covering: valid extension, non-subset
  rejection, self-loop rejection, discover_theory_extensions pair
  scan, id reuse, meta-id inclusion).
- Doctest in the extension-chain docstring now uses `text` fence
  (initial attempt failed; fixed in same commit).

## Implementation

- `v2/src/lib.rs` — `EXTENDS_MARKER`, six new methods, extended
  `collect_meta_ids`.
- `v2/docs/decisions/0034-theory-extension-relations.md` — this ADR.
