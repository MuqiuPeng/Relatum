# 0084: Direction housekeeping — freeze / close / hand off dormant branches

Status: Accepted
Date: 2026-07-06

Trigger: 2026-07-05 full-project retrospective (v1/v2/v3 multi-agent review).
One of its findings: the direction menu has grown monotonically across
forward-directions snapshots, and several branches have been in a
half-open state for 6+ weeks — consuming attention without a decision.
This ADR closes the accounting. It ships no code.

## Context

The project's own history shows that an unbounded direction menu is a
real cost (2026-05-01: user critique of the 40-direction heap phase).
It also shows that premature closure is a real risk: B.8 declared a
structural limit (L5 = 0) that B.8.1 overturned with one vocabulary
extension (L5 = 8). The rule adopted here follows that lesson:

> **Every freeze or closure must carry a written reopen trigger.
> Unconditional closure is not allowed.**

Distinction used below:

- **frozen** — direction is sound but gated on evidence that does not
  exist yet; reopen trigger states what evidence.
- **closed** — the direction's premise was tested and failed, or its
  content has been folded into another item; reopen trigger states
  what would falsify the closure.

## Decision

### 1. H2.2 drive synthesis (E.3 / forward-directions T2) — frozen

- **Evidence**: ADR 0063's H2.2 has been Proposed since 2026-04-27 with
  no implementation. E.3's own sketch (results/E.3_drive_synthesis_sketch.md)
  self-assesses "H2.2 is the most speculative direction in v2" and
  recommends "ship H2.1 fully first" — and H2.1.1/H2.1.2 never shipped.
  Meanwhile the drive
  concept's operationalization has been replaced repeatedly
  (0031 compression → 0057 anomaly-coverage → 0059 prediction-error →
  0078/0080 canonical-bucket + learning progress); synthesizing new
  drives from primitives is premature while the primitive drive layer
  itself has no stable falsifiable success criterion.
- **Reopen trigger**: v3's intrinsic-drive line produces empirical
  validation (the M4 "self-curriculum" milestone cell filled by a
  shipped, verified mechanism — not a scheduler stub), OR H2.1.1/H2.1.2
  ship in v2 with E.1/E.2-grade verification.

### 2. Alpha-2 primitive-layer MCTS + self-play candidates (b)/(c) — closed

- **Evidence**: ADR 0065 Addendum 1 — UCB1 ≡ greedy, byte-identical at
  every checkpoint over 2000 ticks, because composite branching factor
  is ≤ 1 ("the runtime fires exactly 1 composite over the entire run").
  The precondition for any tree search (multiple competing candidates)
  has not appeared on any substrate across ADRs 0066–0083. Cost
  asymmetry (rollouts = expensive cognitive ops) was independently
  confirmed as the second obstacle. Self-play candidate (a) internal
  theory competition *did* ship — ADR 0066 tournament — and is not
  affected; candidates (b) cross-substrate clones and (c) mutual
  prediction are closed with Alpha-2.
- **Reopen trigger**: a substrate demonstrably sustains ≥ 5
  simultaneously-eligible composites at a decision point (the condition
  0065's addendum names for any future rerun), or a v3-side scheduler
  evolves genuine multi-candidate contention worth porting back.

### 3. C.3a–d integer detection series (+ forward-directions T1, O3) — frozen in v2, handed to v3

- **Evidence**: C.3_prep.md's own gating — C.3a needs a chain-rich
  substrate ("current OQ#1, long5k, OQ#2 don't qualify") and none has
  appeared; C.3b–d each gate on the prior stage. The terminal question
  C.3d is a primitive-layer hole in v2 by C.3_prep's own analysis: "the
  SEMANTICS of 'chain extends without limit' can't be FULLY expressed
  in finite R facts." Unbounded extension is an episode-time phenomenon;
  v3's `state(node,t)` / `transition` primitive is the layer designed
  to attack it. Keeping C.3 on the v2 menu manufactures false
  incompleteness.
- **Hand-off**: the detection question (recognize chain / successor /
  unbounded-extension structure from data) transfers to v3 as a
  candidate mechanism-recovery target. The construction half (G-series,
  ADR 0069) is shipped and stays v2.
- **Reopen trigger (v2 side)**: a chain-rich substrate arrives through
  the M5 bridge or external data (N1), at which point C.3a runs as
  specified in C.3_prep.md.

### 4. vibe-proving bridge (proposal 2026-05-11 / ADR 0081) — closed, folded into N1

- **Evidence**: the proposal's Phase 0 ran under ADR 0081 with a
  synthetic stand-in ("synthetic Lean dep", renamed after ARIS Round 1
  flagged the label). The headline substrate-sensitivity claim was
  retracted through the review arc (8 rounds per retrospective-2026-05-19;
  counted as 9 in forward-directions-2026-05-19 — the two records
  disagree, itself a §8-class accounting note); the surviving positive
  is classical subgraph census. Phase 1 with
  real data never started. The proposal's remaining scientific content
  — what does v2 say about naturally-occurring math graphs — is exactly
  N1 (Phase 1.E real Mathlib) in forward-directions-2026-05-19, and the
  proposal itself already recommended the Mathlib path that requires no
  vibe-proving integration at all (Option A, direct extraction).
- **Closure**: the vibe-proving-specific ETL bridge is closed. ADR 0081's
  Status is updated to reflect this (permitted status-line mutation per
  decisions/README.md rules). The proposal doc gets a closure header.
- **Reopen trigger**: explicit user commitment to N1's multi-week scope.
  N1 then starts from the proposal's Phase 0 protocol (pre-registered
  go/no-go/abort criteria, including the tokenization-leak abort test),
  with real Mathlib from day one.

### 5. N3 merge_patterns / N5 BA-scaling — remain frozen, triggers formalized

- **N3 (true structural pattern merge)**: frozen since
  forward-directions-2026-05-19 ("deferred until operational evidence
  justifies"). Reopen trigger: `PatternMergeWith` recommendations fire
  on a live run at a rate where skip-as-noop (ADR 0083's current
  handling) measurably costs coverage.
- **N5 (BA/hub-graph scaling)**: same freeze. Reopen trigger: a
  power-law substrate becomes a committed target. **Note recorded**:
  real Mathlib dependency graphs are hub-rich; if N1 is committed, N5
  likely becomes a prerequisite rather than an option (Phase 1.D
  Round 5: 38–72 min `autonomous_pass` on hub-rich graphs at n=80).

### 6. v3 scope boundary: additive-noise causal pairs declared out of scope

- **Evidence**: Tübingen benchmark 108 pairs, iid_directionality = 52%
  (chance). v3's CE/PE/VE fingerprint family is built for regime-shift /
  constraint-class mechanisms (state-space compression, latency
  propagation, synchronization, suppression), not additive functional
  dependence in iid samples. The 52% result is a domain boundary, not a
  bug — this ADR makes the boundary official so it cannot silently
  motivate estimator creep.
- **Implementation**: scope-boundary section added to
  `v3/docs/design-notes.md`.
- **Reopen trigger**: a dedicated ADR-grade decision that argues why
  additive-noise pairs belong in v3's target class — not an incremental
  estimator addition.

### 7. cognitive-game-framing.md errata — two overdue corrections applied

- **(a)** The search-vs-construction table and the "roughly 70%
  construction" characterization (2026-04-28) predate the constitution's
  2026-05-06 strict-reading amendment. Under reflection 0001's four-way
  classification, most rows labeled "construction" are curation /
  explicit naming / implicit conceptualization. An erratum note is added;
  the original text stays (continuous-editing doc, error kept visible).
- **(b)** ADR 0065's addendum requested the framing doc flag **low
  branching factor** as a second MCTS obstacle alongside cost asymmetry
  ("update deferred to a later editing pass" — 9 weeks ago). Applied.

### 8. Documentation-debt repairs (same pass)

- `decisions/README.md` index backfilled: 0053–0062 and 0080–0083 were
  missing; 0084 added.
- `directions.md`: duplicate D.4 entry resolved (was simultaneously
  `pending` in Phase D and `✓ done` in Round 2); C.3 and E.3 statuses
  updated to point here.

## Alternatives considered

- **Close nothing; let the menu stand.** Rejected: the 2026-07-05
  retrospective found the menu's growth is monotonic and the half-open
  items (vibe-proving especially) impose recurring re-evaluation cost
  at every planning point.
- **Delete closed items from the docs.** Rejected: violates append-only
  spirit and the project's own record-keeping standard (errors stay
  visible, corrections are additive).
- **Freeze everything, close nothing.** Rejected: Alpha-2's premise was
  empirically tested and failed (0065); pretending it is merely gated
  misstates the evidence.

## Consequences

- The v2 menu shrinks to: **O2 G-series autonomy bridge** (largest
  shipping item), **N1 real Mathlib** (largest empirical item, needs
  explicit commitment), **N2 substrate generation in runtime** — plus
  the frozen items above with explicit wake conditions.
- v3's active surface is unchanged; it gains one scope boundary (§6)
  and one candidate future target (§3 hand-off).
- Nothing in this ADR is irreversible: every item carries its reopen
  trigger in writing.

## Implementation

Docs-only change, one commit, referencing this ADR:

- `v2/docs/decisions/0084-direction-housekeeping.md` (this file)
- `v2/docs/decisions/README.md` — index backfill
- `v2/docs/decisions/0081-vibe-proving-bridge.md` — status line update
- `v2/docs/directions.md` — C.3 / E.3 / D.4 status updates
- `v2/docs/proposal-vibe-proving-bridge-2026-05-11.md` — closure header
- `v2/docs/cognitive-game-framing.md` — errata section
- `v3/docs/design-notes.md` — scope-boundaries section
