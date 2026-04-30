# C.3 — Integer construction prep (design + minimal C.3a test)

**Status**: ✓ done as PREP slice (2026-05-01); C.3a-d remain research direction
**Companion log**: [`logs/2026-05-01_phase_c3_prep_chain_predicate.log`](../../logs/2026-05-01_phase_c3_prep_chain_predicate.log)
**Companion example**: [`examples/phase_c3_prep_chain_predicate.rs`](../../examples/phase_c3_prep_chain_predicate.rs)

## Goal

Per the user's strategic critique (2026-04-30) item #5:

> Integer construction / C.3 是最大理论坑，但现在还没准备好。
> 我建议暂时把 C.3 拆成 prep，不要直接冲 integer：
> ```
> C.3a: chain-family detection
> C.3b: successor-like axiom family
> C.3c: extension prediction on held-out longer chain
> C.3d: compression/generalization beyond fixed finite motif
> ```
> 只有这些都 positive，再谈 integer-like concept.

This slice **does not implement integer construction**. It produces:
1. A design doc framing C.3a-d as a sequenced research path
2. The smallest viable empirical test for C.3a (chain-pattern
   recognition) using G-series minted output as input

## Two halves of the integer story

The full integer story has **two complementary halves**:

| half | direction | how v2 does it |
|---|---|---|
| **construction** | recipe → identifiers | G-series (G.1-G.7, ADR 0069) — IMPLEMENTED |
| **detection** | data → recognized chain pattern | C.3a-d — RESEARCH |

G-series produces chains (mint successor + materialize R edges).
C.3 series asks the inverse: given a substrate already containing
chain-like structure, can the system **detect** it as a chain
pattern, abstract it (chain-of-N → chain), predict beyond it
(extension to N+1), and generalize it (independence from any
fixed N)?

The user's framing in design-notes.md anticipated both:

> Possible emergent path:
> 1. Identify stable identifiers (a1, a2, ...) as objects.
> 2. Notice role patterns (each ai appears as both left and right).
> 3. Detect chain patterns (connected linear sequences).
> 4. Abstract chain to ordering.
> 5. Abstract local adjacency to successor.
> 6. Recognize bidirectional chain pairing.
> 7. Recognize unbounded extension.
> 8. Name the whole structure as type_1 (integer-like).
> 9. Explain new chain inputs by matching type_1.

G-series produces (1)-(5) constructively. C.3 attacks (3), (4),
(7) detectively.

## C.3a — Chain-family detection

### Definition

A subgraph is a **chain** of length N (N ≥ 1) iff:
- N+1 distinct identifiers (nodes)
- N edges, all directed
- Exactly one source node (in-degree 0, out-degree 1)
- Exactly one sink node (in-degree 1, out-degree 0)
- All other nodes have in-degree 1 AND out-degree 1
- Connected (single component)
- Acyclic (no cycle in the underlying graph)

### Existing v2 facilities relevant

| facility | role |
|---|---|
| `Subgraph` (ADR 0008) | the data structure |
| `connected_components_of` | extracts components |
| `Subgraph::canonicalize` (ADR 0009) | WL-1 refinement → canonical form |
| `discover_motifs` (ADR 0016) | sample-score-select motif discovery |
| `name_pattern_instances` (ADR 0010) | promotes a recurring subgraph to a named pattern |
| `pattern_structure(pattern_id)` (ADR 0013) | get canonical form of a named pattern |

### What's missing (for C.3a)

- **A `is_chain_subgraph(sg) -> Option<usize>` predicate** that returns
  `Some(N)` when the subgraph is a chain of length N, `None` otherwise.
- **A `is_chain_pattern(pattern_id) -> Option<usize>` wrapper** that
  applies the predicate to a named pattern's canonical structure.

These are mechanical given the definition above. Implementation in
this prep slice's example (~30 lines).

### Empirical question for C.3a

> Given a substrate ALREADY containing chain-like structure (e.g.,
> the output of G.1's mint), does the existing motif-discovery /
> pattern-naming machinery surface chains as named patterns?

Answering this empirically requires:
1. Mint a chain (G.1 logic)
2. Run pattern discovery (autonomous_pass)
3. Classify each discovered pattern via `is_chain_pattern`
4. Count chain-class patterns

If answer is "yes, ≥ 1 chain pattern surfaces": existing machinery
suffices for chain detection — C.3a is empirically unblocked.
If answer is "no, chains aren't detected": custom chain-aware
discovery is needed — C.3a is its own implementation work.

This prep slice ships only step 1 + step 3 (predicate + minted
chain test). Steps 2-4 require additional plumbing; deferred to
the C.3a slice itself.

## C.3b — Successor-like axiom family

### Definition

A successor-like axiom family is a structurally-coherent group of
axioms whose templates encode "if `R(a, b)` then `R(succ(b), c)`"
or similar parameterized recurrences. The family shape is
**recursive**: applying the axiom to its own output produces
another instance of the same axiom shape.

### Existing v2 facilities relevant

- ADR 0070 shape-family abstraction layer (premise / conclusion
  / nested / member-overlap kinds)
- G.1-G.7 generative recipe contract (ADR 0069)

### Gap

ADR 0070's family kinds are STATIC — they group axioms by
structural similarity at a single layer. They don't capture
"this axiom's CONCLUSION matches this axiom's PREMISE shape" —
the recursive coupling that makes a successor schema.

A new family kind would be needed:
- **`KIND_RECURSIVE_PREMISE_CONCLUSION`** — axioms whose
  conclusion structure matches their premise structure (so
  output of one application can feed another).

This is a non-trivial extension. Not in scope for prep.

## C.3c — Extension prediction on held-out longer chains

### Definition

Given a runtime trained on chain inputs of length ≤ K, can the
named "chain pattern" predict instances of length K+1? K+5? K+100?

### Existing v2 facilities relevant

- ADR 0023 cross-graph pattern transfer
- I.1 cross-substrate theory transfer (already empirically tested
  for general theories)

### Gap

A chain-as-pattern may bind specific identifiers (a1, a2, ...,
aK). Predicting length K+1 requires the pattern to be PARAMETRIC
over chain length. The current pattern-naming layer (ADR 0010)
binds identifier roles; it doesn't natively support "the chain is
unbounded".

A successor schema (C.3b) would be the parametric form. Without
C.3b, C.3c is testing whether two FIXED chain patterns
(length-K and length-K+1) match — a degenerate version of the
real question.

## C.3d — Compression / generalization beyond fixed finite motif

### Definition

The integer concept's defining feature: **unbounded extension**.
A 5-element chain isn't an integer concept; the abstraction
"chain that can keep going" is. Can v2 ever express that?

### Existing v2 facilities relevant

- The "type" layer (commitment 3): types are meta-R instances
- ADR 0019 MDL gain scoring
- ADR 0035 counterfactual value

### Gap

A genuinely-unbounded type would be expressed as something like:
`R(__type__, "chain_type") AND R("chain_type", __unbounded__)`
where `__unbounded__` is itself a meta-R primitive. v2 has no
such primitive. Adding one would require constitutional review —
does an `unbounded` marker preserve commitments 1-5?

(Likely yes if it's just another R fact, but the SEMANTICS of
"chain extends without limit" can't be FULLY expressed in finite
R facts. The system would have a TYPE saying "chain is unbounded"
without any finite extension witness — a conceptual hole.)

This is the deepest theoretical question of the C.3 chain. Worth
its own ADR when the time comes.

## What this prep slice produces

### Design contributions

1. The four-stage breakdown C.3a → C.3d, framed as a SEQUENCED
   research path (each stage gates the next)
2. Map of each stage onto existing v2 capabilities + identified gaps
3. Explicit non-overlap with G-series: C.3 is **detection**, G is
   **construction**
4. Risk assessment: C.3d's theoretical hole (unbounded type
   expression) flagged early

### Code contributions

1. `is_chain_subgraph(sg) -> Option<usize>` predicate (in example)
2. Empirical test: G.1's minted chain IS recognized as a chain
   subgraph (POSITIVE — predicate works)
3. Negative control: a non-chain subgraph (3-node star) is
   correctly rejected (POSITIVE — predicate isn't trivial)

The predicate is small and well-defined; the empirical test
confirms G-series output is COMPATIBLE with C.3a's framing.
What's NOT yet proven: that motif discovery on a chain-rich
substrate naturally surfaces chains as patterns. That's the
C.3a slice itself.

## Recommendations

### When to attempt C.3a as its own slice

When at least one of the following is true:
- A substrate emerges (engineered or otherwise) that's
  chain-rich enough to trigger motif discovery for chain
  patterns (current OQ#1, long5k, OQ#2 don't qualify)
- A specific use case demands chain detection (none today)
- Further C.3 work (b, c, d) is gated on C.3a being shipped

### When to attempt C.3b-d

Each should follow only after the prior stage delivers a positive
empirical finding. C.3d (the unbounded type) requires a constitutional
ADR before any code.

### What this slice DOES NOT recommend

- Implementing C.3a-d sequentially in the next sweep. The user's
  punch list is finite; integer construction is one direction
  among several. This slice prepares the ground; doesn't claim
  to walk it.
- Building chain-rich substrates for empirical testing. That's
  a substrate-engineering slice, parallel to this one.
- Pre-supposing successor schemas as C.3b's solution. Other
  parametric forms (e.g., addition / multiplication / induction
  templates) might be equally valid.

## Verdict

**Prep complete.** The four-stage breakdown is now in writing;
each stage's relationship to existing v2 capabilities is mapped;
the smallest empirical test (chain predicate on G-series output)
ships positive.

The integer construction direction is the **largest single open
research question** in v2's punch list. This slice does not
shrink that question. It frames it.

When the user (or a future research push) decides to prosecute
C.3a, this prep document is the starting point.
