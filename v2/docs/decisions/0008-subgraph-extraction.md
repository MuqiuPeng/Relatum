# 0008: Subgraph representation and connected-component extraction

Status: Accepted
Date: 2026-04-23

## Context

ADR 0007's probe showed that compound classes over-merge in a known
way: chain-middle edges and cycle edges share the same compound
fingerprint at 1-hop. Naming the raw biggest class as a pattern would
lump chains and cycles together. The probe's log recommended subgraph
coherence (filter **b**) over a 2-hop tie-breaker (filter **a**), and
the user selected **b** plus β Option 2 (subgraph β).

This is the first β-layer ADR. Its scope is deliberately narrow: make
subgraphs into a first-class object in the code, and provide a
connected-component extraction so the compound-class false merges
automatically split. Canonicalization (isomorphism), naming (meta-R
encoding), and drive (γ) are separate, later ADRs.

## Decision

Introduce `Subgraph` — a set of R instances representing a connected
(or single-edge) chunk of the R graph:

```rust
pub struct Subgraph {
    edges: HashSet<R>,
}
```

Provide a static constructor for partitioning a set of edges into
connected components:

```rust
Subgraph::connected_components_of(iter: impl IntoIterator<Item = R>) -> Vec<Subgraph>
```

Two edges are considered **connected** iff they share at least one
identifier (in any of the four position combinations: x↔x, x↔y, y↔x,
y↔y). Connectivity is transitive; components are maximal connected
sets.

Provide a convenience on `RSet` that applies extraction per compound
class:

```rust
RSet::compound_class_subgraphs(&self) -> HashMap<EdgeFingerprint, Vec<Subgraph>>
```

Each compound class's members are split into their connected components
within the RSet. The result is one vector of subgraph instances per
fingerprint.

## Alternatives considered

- **Borrowed `Subgraph<'a>` holding `&R`.** Rejected: ownership keeps
  the type self-contained, simplifies tests, and avoids lifetime noise
  when subgraphs are stored in maps.
- **Union-find instead of BFS.** Deferred. O(n²) BFS is comfortably
  fast at experimental scale (tens to low-thousands of edges). Swap in
  union-find only if a benchmark makes it necessary.
- **Cache identifiers inside `Subgraph`.** Rejected: identifiers are
  derivable from edges; avoid redundancy that can drift out of sync.
- **Hash / Ord for `Subgraph` now.** Deferred. Equality at this ADR is
  trivial set-of-edges equality; the meaningful notion (isomorphism)
  is ADR 0009's job. Introducing structural Hash now would either
  duplicate or pre-empt that work.
- **Skip `compound_class_subgraphs` and leave it to the example
  binary.** Rejected: the "compound class → subgraphs" lift is going
  to be a common operation; having it on `RSet` lets later mechanisms
  (and tests) call it directly.

## Consequences

Predicted behavior on ADR 0007's mixed graph:

| Compound class | |members| | Subgraphs produced |
|---|---|---|
| chain-middle + cycle (predicted false merge) | 5 | **2** subgraphs: 2-edge chain fragment, 3-edge cycle |
| star spokes | 3 | **1** subgraph: 3-edge star (connected through hub) |
| chain-tail + tree-leaf (terminal descent) | 2 | **2** subgraphs: one single-edge each (disjoint components) |
| chain-head (singleton) | 1 | **1** subgraph: one edge |
| tree branches (each singleton) | 1 each | **1** subgraph each |
| isolated edge | 1 | **1** subgraph: one edge |

Key outcomes:
- The false-merge class cleanly splits into two differently-shaped
  subgraphs. Filter (b) is operational.
- "Terminal descent" surfaces as two isolated single-edge subgraphs
  with the same fingerprint — a legitimate same-pattern candidate
  across components. (Whether they *are* the same pattern is an
  isomorphism question for ADR 0009.)
- Star spokes form one subgraph, not three — the star is a unified
  structural unit, not a repetition of independent edges.

Complexity:
- `connected_components_of(E)` is O(|E|²) in the worst case with the
  plain BFS implementation. Acceptable at current scale.
- `compound_class_subgraphs` is one `connected_components_of` call per
  fingerprint; the total work is O(|R|²) in the worst case.

What this ADR does **not** yet do (deferred to later ADRs):
- Decide when two subgraphs represent the same pattern (isomorphism
  canonical form) → ADR 0009.
- Create a name for a pattern and encode it as R instances
  (commitment 3) → ADR 0010.
- Choose which candidates actually get named → ADR 0011 (γ's first
  real job).

## Implementation

- Source: `v2/src/lib.rs` — `Subgraph`, `Subgraph::connected_components_of`,
  `RSet::compound_class_subgraphs`, and tests.
- Example: `v2/examples/subgraph_extraction.rs` — applies the
  extraction to the ADR 0007 mixed graph and prints subgraph sizes
  per class.
- Experiment log: `v2/logs/2026-04-23_subgraph_extraction.log` with
  the raw output and observations against the oracle above.
