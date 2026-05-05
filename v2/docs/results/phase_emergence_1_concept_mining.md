# Phase Emergence-1 — Shape co-occurrence concept mining

**Status**: ✓ shipped (2026-05-06); first concept minted, cross-substrate identity confirmed
**Log**: [`logs/2026-05-06_phase_emergence_1_concept_mining.log`](../../logs/2026-05-06_phase_emergence_1_concept_mining.log)
**Example**: [`examples/phase_emergence_1_concept_mining.rs`](../../examples/phase_emergence_1_concept_mining.rs)
**ADR**: [0074 — Phase Emergence-1: shape co-occurrence mining](../decisions/0074-phase-emergence-1-shape-co-occurrence-mining.md)

## Goal

ADR 0073 named the curation/creation boundary: the system can
integrate, refine, and prune existing concepts (Phase 0070-0072)
but cannot mint new ones. ADR 0074 specified shape co-occurrence
mining as the first creation step. This slice is its
implementation and end-to-end validation.

The success criterion is concrete: **mint at least one concept
on OQ#1 such that the same concept id mints on long5k**, since
both substrates are known to converge to isomorphic RSets (Phase
0072-A finding). Identity portability proves the mining is
structural, not stream-dependent.

## What shipped

### Library (commit pending)

- `src/markers.rs`: 4 new constants — `CONCEPT_MARKER`,
  `HAS_CONSTITUENT_SHAPE`, `ATTESTED_IN_THEORY`,
  `CROSS_PRECISION_AT_MINT`
- `src/types_concept.rs`: new module with `ConceptCandidate`,
  `ConceptStatus` (Live / Stale / Validated / Falsified),
  `ConceptMiningConfig`, `ConceptRegistrationError`,
  `concept_id_from_constituents`, `concept_alias_from_constituents`
- `src/lib.rs`: 9 new methods on `RSet`:
  - `propose_concept_candidates(&config, &reports)`
  - `validate_concept(&candidate, &substrates, floor)`
  - `register_concept(&candidate)`
  - `retract_concept(&id)`
  - `is_concept(&id)`, `concepts()`,
    `concept_constituent_shapes(&id)`,
    `concept_attested_theories(&id)`,
    `concept_cross_precision_at_mint(&id)`,
    `concept_status(&id)`
- `src/lib.rs`: helper `combinations(items, k)` for subset
  enumeration in propose
- `src/lib.rs::collect_meta_ids`: extended with concept markers
  and registered concept ids

### Tests

13 new tests in `src/tests.rs` (all passing):
- `adr0074_concept_id_is_deterministic`
- `adr0074_concept_alias_is_human_readable`
- `adr0074_propose_surfaces_pair_for_cooccurrence`
- `adr0074_propose_skips_below_min_theories`
- `adr0074_propose_filters_noise_theories_when_signal_only`
- `adr0074_propose_admits_all_classes_when_signal_only_disabled`
- `adr0074_register_creates_meta_r_chains`
- `adr0074_register_rejects_unvalidated`
- `adr0074_register_rejects_degenerate_constituents`
- `adr0074_register_rejects_unknown_constituent`
- `adr0074_retract_removes_concept`
- `adr0074_concept_status_live_when_all_constituents_present`
- `adr0074_concept_status_stale_after_constituent_retracted`

Lib tests: 600 → **613**, 0 regressions.

### Example

`phase_emergence_1_concept_mining.rs` runs the full
propose-validate-register loop on OQ#1 (1000 ticks) and long5k
(1500 ticks), then compares concept ids across substrates.

## Result

### OQ#1 @ 1000 ticks

```
Phase 0: 4 theories ([t_0, t_1, t_2, t_3]); 13 axioms; 6 L2 families

Theory quality summary:
   t_0       Mixed     p=0.3759   c=0.6835
   t_1       Mixed     p=0.5863   c=0.8354
   t_2      Signal     p=1.0000   c=1.0000
   t_3      Signal     p=0.9144   c=1.0000

Proposed candidates: 1
   id=concept_4c2d2fde3b2d8360
   constituents: [shape_conclusion_c0-2, shape_premise_p0-1_p1-2]
   attested in:  [t_2, t_3]

Validation: cross_precision_mean = 1.0000 → PASS (floor 0.80)

Registered: 1 concept, status = Live
```

### long5k @ 1500 ticks

```
Phase 0: 4 theories ([t_0, t_1, t_2, t_3]); 13 axioms; 6 L2 families

Theory quality summary:
   t_0       Mixed     p=0.3640   c=0.6835
   t_1       Mixed     p=0.6017   c=0.8354
   t_2      Signal     p=1.0000   c=1.0000
   t_3      Signal     p=0.9673   c=1.0000

Proposed candidates: 1
   id=concept_4c2d2fde3b2d8360       ← same id
   constituents: [shape_conclusion_c0-2, shape_premise_p0-1_p1-2]
   attested in:  [t_2, t_3]

Validation: cross_precision_mean = 1.0000 → PASS

Registered: 1 concept, status = Live
```

### Cross-substrate identity verdict

**✓ POSITIVE — concept id `concept_4c2d2fde3b2d8360` is portable
across OQ#1 and long5k.**

The constituent shape ids (`shape_conclusion_c0-2`,
`shape_premise_p0-1_p1-2`) hash to the same concept id on both
substrates because both substrates discover the same shape
families and the id is a deterministic hash of the sorted
constituent set. Per Phase 0072-A's finding that OQ#1 @ 1000
ticks and long5k @ 1500 ticks converge to isomorphic RSets, this
is the expected outcome and validates the loop end-to-end.

## What this concept means

`concept_4c2d2fde3b2d8360` (alias:
`concept_alias_conclusion-c0-2__premise-p0-1-p1-2`) captures the
joint pattern:

- A canonical conclusion shape with conclusion edge (0, 2)
- A canonical premise shape with premise edges {(0, 1), (1, 2)}

That conclusion shape captures totality-flavoured rules ("when
both directions of an edge exist, conclude something pointing
out"); that premise shape captures transitive-style premises.
Their joint occurrence in Signal theories t_2 and t_3 is what
"a high-quality theory looks like" on both OQ#1 and long5k.

The concept is now a queryable noun in the system:
`rset.concepts()`, `rset.concept_constituent_shapes(id)`,
`rset.concept_attested_theories(id)`, etc. This is genuinely new
vocabulary — neither a theory (instance-bound) nor a shape
family (single canonical shape) was capable of expressing this
joint pattern before.

## What changed in the system's capabilities

| capability | before E1 | after E1 |
|---|---|---|
| name a single shape | ✓ (ADR 0068/0070) | ✓ |
| name a theory (axiom collection) | ✓ (ADR 0030) | ✓ |
| **name a shape-shape co-occurrence** | **✗** | **✓ (this slice)** |
| query concept constituents | — | ✓ `concept_constituent_shapes` |
| query concept attestations | — | ✓ `concept_attested_theories` |
| validate concept survival under constituent retract | — | ✓ `concept_status` (Live → Stale) |
| concept identity portable across substrates | — | ✓ deterministic hash |

This is the smallest possible cross of the curation/creation
boundary. The system now has *one* genuinely new noun in its
vocabulary — and that noun was discovered, not declared.

## Open items / next slices

1. **Concept transfer to structurally distinct substrate.** OQ#1
   and long5k converge to isomorphic RSets so identity transfer
   is "free." Real generalization test: does this concept (or a
   variant) appear on narrow_a or OQ#2 once they mature into
   theory-bearing states? Or does the concept fail to mint
   because the underlying structure is different? This is the
   real falsifiability test; current evidence is consistent with
   the concept being substrate-structural but does not
   distinguish "universal" from "OQ#1-isomorphic".

2. **Concept retraction cascade policy.** When a constituent
   shape family is retracted via `retract_shape_family`, the
   concept enters Stale status but its meta-R remains. Should
   it auto-retract? Phase Emergence-1 leaves this opt-in (the
   user calls `retract_concept` explicitly); the right policy
   may emerge from the next slice.

3. **AxiomComplete intervention** (Phase Emergence-1.5). If a
   Mixed theory contains some-but-not-all of a registered
   concept's constituent shapes, recommend "look for the
   missing shape." This requires extending ADR 0072's
   `RecommendedIntervention` enum with a new variant — the next
   ADR's job.

4. **Re-validation cadence**. Concepts can become Falsified
   after enough stream evolution. The current implementation
   exposes `concept_status` (Live / Stale) but does not
   actively re-validate. Adding `revalidate_concepts(&mut self)`
   that re-runs validate against current substrates and updates
   status is a small follow-up; deferred until at least one
   concept lives long enough to potentially fail.

5. **Drive integration (E3)**. With concepts registered, the
   scheduler can prioritize "the stream is firing one
   constituent of a known concept; attend to find its
   partners." Pair priority with E1 per ADR 0073.

6. **Larger concepts (size ≥ 3)**. Currently propose enumerates
   subsets up to `max_candidate_size = 4`, but the OQ#1 /
   long5k Signal theories only co-occur in 2-shape patterns
   given the 6-family universe. More family-rich substrates
   would let triples and beyond surface; needs new substrates.

## Verdict

**Phase Emergence-1 ships the first system capability for minting
new abstractions outside its hard-coded vocabulary.** A concrete
concept (`concept_4c2d2fde3b2d8360`) was discovered from data,
validated against cross-precision, registered as meta-R, and
shown to be substrate-portable. The propose-validate-register
loop is now production infrastructure parallel to (and reusing)
ADR 0070's family layer + ADR 0071's quality reports.

The diagnosis from ADR 0073 ("the system cannot mint new
concepts") no longer holds without qualification. The system can
mint *second-order shape-shape concepts*; it still cannot mint
new axiom shapes or emerge candidate objects (E2, E3 still open).
The curation/creation boundary has been crossed — by the
narrowest possible margin, but crossed.
