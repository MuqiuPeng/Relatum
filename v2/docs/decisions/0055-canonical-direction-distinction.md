# 0055: Direction-distinguishing canonical form

Status: Proposed
Date: 2026-04-26

## Context

The Phase D0+ end-to-end demo
(`logs/2026-04-26_phase_d_demo.log`) ran the meta-meta discovery
loop and named a pattern with 4 roles and 3 structural edges:

```
intension structural edges: 3 [
  ("p_5_role_0", "p_5_role_1"),
  ("p_5_role_0", "p_5_role_2"),
  ("p_5_role_0", "p_5_role_3"),
]
```

This is a **fan-OUT star** — one source with three outgoing edges
to distinct targets. The seed had only fan-in shapes
(`R(p_a, ESTABLISHED)`, `R(p_b, ESTABLISHED)`, …) and fan-out
shapes (`R(PATTERN_MARKER, p_a)`, `R(PATTERN_MARKER, p_b)`, …).
The named pattern's canonical matches **both**: WL-1 + the
current `rank_labels` step canonicalises fan-in and fan-out
identically at this size.

ADR 0009 already noted WL-1 as heuristic ("Rare graph pairs can
produce identical canonical forms while being non-isomorphic").
Phase D0+ is the first place this matters in practice. The
runtime cannot tell its two interesting M1 hypotheses apart:

- Fan-IN to ESTABLISHED — *"these patterns are all established"*
- Fan-OUT from PATTERN_MARKER — *"these are all named patterns"*

Treating these as the same pattern conflates a meaningful
discovery (M1-anchored) with a structural triviality (registry-
anchored).

## Decision

### What's wrong (precisely)

The bug is not in the WL refinement — it's in the projection
to canonical labels:

1. `Subgraph::canonicalize` runs WL-1 on the directed graph,
   producing per-node signatures `(label, sorted_out_labels,
   sorted_in_labels)`. These signatures **already distinguish**
   fan-in source `(1, [0], [])` from fan-out target
   `(0, [], [1])`.
2. After convergence, `rank_labels(&signatures)` reduces each
   signature to its **local rank** within the current
   subgraph's signature list. Both fan-in and fan-out happen to
   have exactly two distinct signatures, so both get labels
   `{0, 1}` — the rich signature content is discarded.
3. The output `CanonicalForm = Vec<(u32, u32)>` records edges in
   terms of these local ranks, yielding identical
   `[(1, 0), (1, 0), (1, 0)]` for both shapes.

The signatures carry the direction information; the projection
step throws it away.

### Phase E0 — minimal surgical fix

Replace `rank_labels` with a **stable global hash** of the
converged signature, computed once after WL converges:

```text
canonical_label(node) = hash(converged_signature(node))
                      // u64, deterministic, direction-preserving
```

Concretely:

- Add `fn signature_hash(sig: &(u32, Vec<u32>, Vec<u32>)) -> u64`
  using `std::hash::Hasher` with a fixed seed (`SipHash` or the
  default `DefaultHasher` is fine — we just need determinism
  within a process; cross-process stability isn't a v2 invariant
  yet).
- After the existing WL fixed-point loop, replace the final
  edge-tuple construction:

  ```rust
  // before (current — local ranks)
  let canonical: Vec<(u32, u32)> = edges
      .map(|r| (labels[idx_x], labels[idx_y]))
      .collect();

  // after (hash-of-converged-signature)
  let final_sigs: Vec<(u32, Vec<u32>, Vec<u32>)> = ...; // last sigs from loop
  let hashes: Vec<u64> = final_sigs.iter().map(signature_hash).collect();
  let canonical: Vec<(u64, u64)> = edges
      .map(|r| (hashes[idx_x], hashes[idx_y]))
      .collect();
  ```

- `CanonicalForm` changes from `Vec<(u32, u32)>` to
  `Vec<(u64, u64)>`. This is a wire-format break for any code
  that serialises canonicals — currently none in tree
  (canonical forms live only in memory; `find_pattern_matching`
  and `is_isomorphic_to` compare them in-process).

The fixed-point loop itself stays unchanged. WL-1 continues to
produce the converged signature; only the *projection to a
storable canonical* is upgraded.

### What this does NOT fix

- **Strongly regular graphs** and other classical WL-1
  counterexamples remain indistinguishable. The signature is
  the same, so the hash is the same. WL-1 limit unchanged.
- **The hypothetical case** where two subgraphs have different
  signature multisets but identical sorted multisets after
  rank-collapse, AND those multisets canonicalize identically
  under further refinement: also unchanged.

E0 fixes exactly the failure observed in Phase D0+. Larger
WL-1 limitations are out of scope.

### Phase E1 (sketch, deferred)

If meaningful new failure modes surface, upgrade to **WL-2** or
**individualisation-refinement** (nauty / bliss-style). Both are
cubic in node count vs. quadratic for WL-1; deferring until
there's evidence beyond the fan-in/fan-out case.

## Alternatives considered

- **Pre-compute and store signature multisets**, compare those
  instead of edge lists. Equivalent in distinguishing power to
  E0 (signature is the same input either way) but requires
  changing `is_isomorphic_to` from edge-list equality to
  signature-multiset equality. More invasive without added
  benefit.
- **Tag neighbour entries with their direction inside the
  signature.** The signature is already tuple `(label, outs,
  ins)`, so direction *is* preserved. The bug isn't here.
- **Add a "canonical_format_version" field to discovery
  results.** Future-proofing but no current consumers; YAGNI.
- **Run WL-2 just for canonical projection and keep WL-1 for
  the iteration loop.** Conceptually clean but mixes algorithms;
  E0's hash-of-signature gets the same distinguishing power
  for less code.

## Non-goals

- A canonical form that handles all graph isomorphism
  edge-cases. v2's experimental scale never reaches strongly
  regular graphs; if it ever does, ADR 0055-followup.
- Changing the public API of `Subgraph::canonicalize` callers.
  Returns a `CanonicalForm` either way; the inner type widens
  from `u32` to `u64` per pair, which Rust's type-checker will
  catch at every callsite. Migrate all in one commit.
- Wire-format / persistence stability for canonicals. Currently
  not persisted; no compat constraint.

## Verification plan

For Phase E0:

1. **Existing 396 tests pass after the type change.** All
   current canonical-form usage compares pairs in-process; only
   the inner integer width changes.
2. **New regression test**:
   `subgraph_canonicalize_distinguishes_fan_in_from_fan_out`.
   Build the two 4-node, 3-edge subgraphs (fan-in / fan-out)
   directly; assert their canonicals are *not equal*. This is
   the precise failure case from the demo.
3. **D0+ demo re-run**: re-capture
   `logs/2026-04-26_phase_d_demo.log` (or a date-bumped successor)
   and verify the named meta-meta-pattern's intension reflects
   the actual seeded shape (fan-in → roles oriented `(role_X,
   role_0)`, with role_0 acting as the shared right-endpoint).
4. **No mass invalidation of existing canonicals**: existing
   tests that pin a specific canonical literal will need
   updates because the integer width changed and hashes look
   different. Pin to *equality between two computed canonicals*,
   never to literal `(0, 1)` tuples — and audit existing tests
   for the latter pattern before landing.

## Open questions (for the implementation, not blocking acceptance)

1. **Hasher choice.** `std::collections::hash_map::DefaultHasher`
   is process-randomised on some platforms (it isn't on current
   Rust, but the spec leaves room). Use `SipHasher13` with a
   fixed seed for cross-run determinism, or accept process-
   determinism only? Suggest the former — costs nothing extra,
   keeps dump-and-diff workflows working.
2. **Conflict probability.** With u64 hashes and ≤ 10⁴ nodes per
   subgraph at v2 scale, birthday-paradox collisions are
   astronomically unlikely. Document the assumption; revisit if
   the scale ever moves.
3. **Migration of existing literal canonical references in
   tests.** Audit before landing. If any test pins
   `vec![(0, 1), (0, 1), …]`, replace with "compute then
   compare" form.
4. **D0+ behavioural change.** Once fan-in and fan-out are
   distinguished, the meta-meta discovery may name *two*
   patterns where it previously named one. Hit-rate cooldown
   (B1+) and the existing pattern-cooldown gates handle this
   automatically, but the demo's captured log will need
   re-recording. Worth noting in the commit message that
   2026-04-26's log captured pre-E0 behaviour.

## Touched ADRs

- **ADR 0009** introduced WL-1 canonicalisation; this ADR
  refines its projection step.
- **ADR 0054** Phase D0+'s observed limitation is the
  motivation; the open-question #1 ("Strict vs lax matching for
  meta-meta") is partially addressed — same-marker
  distinguishability is now structural, not policy.
- **ADR 0029** pattern naming uses canonical equality to mint
  ids; the type widening is the only API touch.

## Summary

WL-1 already produces direction-distinguishing signatures for
the fan-in/fan-out case; the rank-collapse step in
`Subgraph::canonicalize` discards that information when
projecting to a stable label space. Phase E0 swaps the
projection from local rank to a global hash of the converged
signature — five-line fix in `lib.rs`, type-width change at
every canonical-comparing callsite (caught by the compiler),
no algorithmic upgrade.

Phase E1 (full WL-2 / individualisation-refinement) is sketched
and deferred until the next concrete failure surfaces.

Status: **Proposed**.
