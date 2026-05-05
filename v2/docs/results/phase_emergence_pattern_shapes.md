# Phase Emergence — Pattern shape visualization

**Status**: ✓ done (2026-05-06); makes the kernel's output legible
**Log**: [`logs/2026-05-06_phase_emergence_pattern_shapes.log`](../../logs/2026-05-06_phase_emergence_pattern_shapes.log)
**Example**: [`examples/phase_emergence_pattern_shapes.rs`](../../examples/phase_emergence_pattern_shapes.rs)
**Predecessor**: [`phase_emergence_canonical_form_diversity.md`](phase_emergence_canonical_form_diversity.md)
**ADR**: [0075 — Emergence kernel audit and runtime integration](../decisions/0075-emergence-kernel-audit-and-runtime-integration.md)

## Goal

The canonical-form-diversity slice produced 12 distinct
canonical-form hashes across the 4 substrates but each hash was
opaque (`can_<hex>`). To check whether the emergent patterns are
**semantically meaningful** — i.e. that they correspond to
recognisable substructures in each substrate's stream regimes —
they need to be rendered as readable shapes.

This slice adds `RSet::format_pattern_shape` (in `lib.rs`) and
runs it on every minted canonical, grouping by substrate
membership.

## What shipped

### Library

- `RSet::format_pattern_shape(pattern_id) -> String` — renders a
  pattern's intension as readable text:
  - role count + edge count + coarse shape classifier
  - sorted role-role edge list with roles renamed `r0..rN`
- `classify_pattern_shape(n_roles, edges)` — internal coarse
  classifier recognising small motifs: self-loop, isolated,
  directed edge, bidirectional pair, fork, merge, chain, 3-cycle,
  3-edge triple, star (hub of degree N), self-loop combinations,
  generic fallback.

Pure read-only helper. No rset mutation.

### Tests

4 new tests in `src/tests.rs`:
- `adr0075_format_pattern_shape_renders_chain`
- `adr0075_format_pattern_shape_renders_self_loop`
- `adr0075_format_pattern_shape_handles_unknown_pattern`
- `adr0075_format_pattern_shape_renders_3_cycle`

Lib tests: 613 → **617**, 0 regressions.

### Example

`phase_emergence_pattern_shapes.rs` — runs autonomous_pass on
each substrate, renders every minted pattern, groups by
substrate-membership.

## Result

### Universal canonical (1)

```
can_1dcd38f6674f1878  [OQ#1, long5k, narrow_a, OQ#2; max instances=50]
  p_2 (2 roles, 3 edges, shape: 2 self-loops + 1 cross-edge)
    r0 → r0
    r0 → r1
    r1 → r1
```

Two reflexive nodes connected by one directed edge. Every
substrate that has reflexive identity + at least one directed
relation produces this. The most basic structural fact in v2's
vocabulary.

### OQ#1-clade only canonicals (5)

All correspond to subgraphs of the diamond-poset regime that
OQ#1 / long5k / narrow_a share:

```
can_cb21943a71a9eb8e  [OQ#1, long5k; max instances=10]
  star (hub of degree 3)
    r0 → r1, r0 → r2, r0 → r3
  ← diamond top: top covers two middles + bottom

can_703239bca97249ae  [OQ#1, long5k; max instances=15]
  merge (two sources, one target)
    r0 → r2, r1 → r2
  ← diamond bottom: two middles cover bottom

can_3d2da53b81a90ad7  [OQ#1, long5k; max instances=15]
  4-edge graph on 4 nodes (bipartite 2×2)
    r0 → r2, r0 → r3, r1 → r2, r1 → r3
  ← diamond middle: two upper × two lower

can_bf91273eb195d4e9  [OQ#1, long5k, narrow_a; max instances=10]
  3 self-loops + 2 cross-edges
    r0→r0, r0→r2, r1→r1, r1→r2, r2→r2
  ← reflexive chain: each node self-loops + chains forward

can_ed6c192b563a80e3  [OQ#1, long5k, narrow_a; max instances=10]
  3 self-loops + 2 cross-edges
    r0→r0, r0→r1, r0→r2, r1→r1, r2→r2
  ← reflexive fork
```

### OQ#2-only canonicals (5)

All correspond to tournament/lattice/star regimes — non-poset
structures the OQ#1 clade does not produce:

```
can_61c89385fc0342a0  [OQ#2; max instances=84] ← largest pattern
  3-cycle (transitive triple)
    r0 → r1, r0 → r2, r1 → r2
  ← tournament regime: a < b < c plus a < c

can_d22cf47563c30091  [OQ#2; max instances=30]
  1 self-loop + 4 cross-edges  (star hub bidirectional)
    r0→r0, r0→r1, r0→r2, r1→r0, r2→r0
  ← star regime: hub with bidirectional edges to leaves

can_a24624ecca927e67  [OQ#2; max instances=20]
  1 self-loop + 2 cross-edges
    r0→r0, r0→r1, r1→r0
  ← star core or lattice top: reflexive node + bidirectional pair

can_356b87478aee25fa  [OQ#2; max instances=1]
  bidirectional pair
    r0 → r1, r1 → r0

can_ff8b08ea746bf094  [OQ#2; max instances=3]
  chain (length 2)
    r0 → r1, r1 → r2
  ← tournament/lattice transitive step
```

### Mixed-membership (1)

```
can_faefcad1cdc772d7  [OQ#1, long5k, OQ#2; max instances=30]
  fork (one source, two targets)
    r0 → r1, r0 → r2
```

Appears in 3 substrates but not in narrow_a. narrow_a's
diamond-only stream produces forks, but they're embedded inside
larger diamond patterns that get minted at a higher size, never
as a standalone fork.

## Verdict

**Each canonical has a clear semantic correspondence to its
substrate's stream regimes:**

- OQ#1 / long5k / narrow_a stream diamond posets → minted
  patterns are diamond subgraphs (star, merge, bipartite,
  reflexive chain/fork)
- OQ#2 streams tournament + lattice + star regimes → minted
  patterns are 3-cycles, hub stars, bidirectional pairs,
  chains
- The universal pattern is the most trivial "two reflexive
  nodes + one edge" present in every substrate
- The largest emergent pattern across the entire study is
  OQ#2's 3-cycle (84 instances) — exactly what tournament
  regimes ought to produce as a transitive triple

This is **strong evidence the emergence kernel produces
semantically faithful structural abstractions**. The kernel is
not just running and minting random subgraphs; what it mints
corresponds to the actual recurring R-substructures in the
input stream. The 12 canonicals across 4 substrates partition
into a clean "who produces what" table that maps directly to
the stream-regime designs.

## What this changes

- **Interpretability is now possible** for any future kernel
  output. New patterns minted by autonomous_pass can be
  rendered as readable shape descriptors via
  `format_pattern_shape`. No more opaque hashes during
  diagnostic work.
- **The kernel-audit + canonical-form-diversity narrative
  closes**: v2's pattern path produces substrate-distinct,
  semantically faithful structural abstractions. The
  diagnosis "v2 cannot create new concepts" is fully
  retracted; the remaining diagnosis is "v2 cannot mint new
  axiom shapes" (still true) but concept emergence at the
  structural-abstraction level works as designed.
- **Scheduler integration becomes the next concrete step**
  (ADR 0075 piece 2). The kernel produces meaningful output
  but is currently dormant during normal stream processing;
  promoting `DiscoverPatterns` to high priority in
  `RuleBasedScheduler` would let v2 build this vocabulary
  autonomously during normal Phase 0 progression.

## What was not addressed

- **Pattern quality / cross-precision**. The patterns are
  structurally faithful but no quality metric assesses their
  predictive value yet. ADR 0072's quality framework was
  designed for axioms / theories; extending it to patterns is
  a future ADR.
- **Pattern subsumption**. Several minted patterns on
  OQ#1-clade are subgraphs of one another (e.g., the 4-edge
  bipartite contains the 2-edge fork as a sub-shape).
  Currently each is minted independently; a future ADR may
  identify and dedup these via subsumption rules analogous
  to ADR 0028 (axiom subsumption).
- **Coarse-classifier robustness**. `classify_pattern_shape`
  recognises a small set of motifs and falls back to a
  generic descriptor for everything else. The fallback is
  correct but not insightful; richer classification (e.g.,
  motif catalog with names like "transitive closure",
  "bipartite cover", "anti-chain") is a future helper.

## Files

- `src/lib.rs` — `format_pattern_shape` + `classify_pattern_shape` + `degree_map`
- `src/tests.rs` — 4 new ADR-0075 tests
- `examples/phase_emergence_pattern_shapes.rs`
- `logs/2026-05-06_phase_emergence_pattern_shapes.log`
- This result: `docs/results/phase_emergence_pattern_shapes.md`

Lib tests: 613 → 617.

## Next step

Per ADR 0075's three pieces:
- Piece 1 (kernel audit): ✓ shipped
- Piece 3 (canonical-form diversity): ✓ shipped
- Piece (b): visualization: ✓ shipped (this slice)
- **Piece 2 (scheduler integration)**: pending. Promote
  `DiscoverPatterns` priority in `RuleBasedScheduler` so the
  runtime calls `autonomous_pass` periodically during normal
  Phase 0 stream processing. With visualization in place, the
  effects of integration are now legible — we'll see exactly
  what patterns the runtime mints autonomously.
