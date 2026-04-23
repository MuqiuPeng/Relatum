# v2 Progress Log

Chronological record. Append-only (except typo fixes). Each entry dated;
entries link to the ADR that governs the step.

---

## 2026-04-23

### Project restructure
v1 archived under `v1/` at tag `v1.0`. v2 scaffolded at `v2/` with an empty
Rust package: only the `R(x, y)` struct and three invariant tests
(construction, directionality, token-based identity).

Decision: [0001-project-restructure](decisions/0001-project-restructure.md).
Commits: `b8aaa84`, `79541ea`.

### RSet harness
Added `RSet` — a deduplicated R-instance container with observation methods
(`identifiers`, `left_of`, `right_of`). Zero interpretation at this layer.

Decision: [0002-rset-harness](decisions/0002-rset-harness.md).
Commit: `5abdb73`.

### IdentifierProfile (first pass)
Added `profile(id)` returning `(degree_out, degree_in, slots)`. First-pass
answer to the "object emergence" question: structurally salient identifiers
can be found by comparing profiles, but the profile itself makes no salience
judgment. Richer granularity (neighbor sets, self-loop flag, co-occurrence,
multi-hop) documented in the ADR as deferred candidates.

Decision: [0003-identifier-profile](decisions/0003-identifier-profile.md).
Commit: `8886d53`.

### Traceability infrastructure
Added `docs/practices.md`, `docs/decisions/` (with index), `docs/progress.md`,
and `logs/` directory. Backfilled ADRs 0001–0003 to cover the work above.

Commit: `6a1f4f9`.

### Structural signature — first pass
`signature(id)` aliased to `profile(id)`; added `equivalence_classes()` on
`RSet`. Two identifiers are structurally equivalent iff their 0-hop profiles
are equal. 6 new unit tests cover chain / cycle / star / disjoint-union
collapse behavior.

Ran the first v2 experiment (`examples/structural_equivalence.rs`) over
six canonical small graphs. Findings:
- Role classification works as intended — head/middle/tail in chains,
  pivot-vs-leaves in stars, full collapse in cycles.
- Disjoint unions merge equivalent roles across components without extra
  machinery.
- 0-hop is **not** sufficient for naming compound patterns (e.g., "this
  is a chain of three"); that is a separate, later mechanism.

0-hop signatures are adopted as the role-classification layer; no immediate
upgrade to 1-hop needed. Open questions for the next layer (pattern
detection) are listed at the end of the experiment log.

Decision: [0004-signature-is-profile](decisions/0004-signature-is-profile.md).
Log: [logs/2026-04-23_structural_equivalence.log](../logs/2026-04-23_structural_equivalence.log).

Commit: `1437569`.

### R-instance signature — edge-level (first pass)
Lifted the signature machinery one level: `RSignature = (Signature, Signature)`
(ordered endpoint profiles), with `r_signature(&R)` and
`r_equivalence_classes()` on `RSet`. 6 new unit tests plus an
`edge_equivalence.rs` example covering the same six canonical graphs as
the identifier-level demo.

All six ADR-0005 predictions verified. Key findings:
- **First "repetition inside a single graph."** The 5-chain's middle-middle
  edges `R(a2,a3)` and `R(a3,a4)` merge into one class — the first
  single-graph-derived multi-member class not caused by pure symmetry.
  This is the signal a later pattern-mining layer can mine.
- **Direction is preserved.** Bidirectional chain produces three distinct
  classes (out-from-end, in-to-end, middle-middle) because pair order matters.
- **Stars reduce to "one edge type repeated."** Both out-star and in-star
  collapse their spokes; shape of the compound-pattern definition starts
  to come into view.
- **Cycles and stars collide at this layer** — both go to a single class.
  Distinguishing them requires a locality / co-occurrence signal, which
  is the motivation for the next ADR.

Decision: [0005-r-instance-signature](decisions/0005-r-instance-signature.md).
Log: [logs/2026-04-23_edge_equivalence.log](../logs/2026-04-23_edge_equivalence.log).
