# 0073: v2 phase pivot — from concept curation to concept emergence

Status: Accepted
Date: 2026-05-05

## Context

Phase 0070-0072 closed the "intervention layer":

- **ADR 0070** consolidated B.2-B.8.1 + F.1.1's structural
  abstraction into the **shape-family layer**
- **ADR 0071** consolidated primary-rate + cross-precision +
  family + neighborhood signals into the **unified theory-quality
  report**
- **ADR 0072** consolidated 6 scattered intervention mechanisms
  into a single **`recommend_intervention` policy classifier**,
  with three Addenda (HighQualityBoth merge, near-disjoint
  Jaccard, focal quality floor) honed through Phase 0072-A
  intervention ablation and Phase 0072-B threshold scanning

The triad is verified end-to-end:
- Migration atlas: 7 AGREE + 2 correct falsifications + 0 open
  on historical examples
- OQ#1 + long5k ablation: identical -0.0907 cross_min regression
  pattern; Addendum 3's quality floor of 0.70 within
  empirically defensible band [0.5863, 1.0)
- 600 lib tests, 73 ADRs, 49 result docs, 88 examples

This is **terminal work for the intervention axis**. Further
threshold tuning, more substrate replications, or sensitivity
scans on Jaccard would all stay within the same loop: "given a
shape library and discovered theories, choose how to demote /
merge / repair them."

The user named the strategic question that ends this loop:
**"现在的问题是这个系统无法去进行新的概念的创造吧"**.

The diagnosis is correct. The system can integrate, refine, and
prune existing concepts cleanly — but it cannot mint new ones.
A precise inventory of "what counts as creation":

| layer | can the system create here? | mechanism |
|---|---|---|
| new R(x, y) instance | ✓ | `forward_apply_axiom` |
| new axiom instance | ✓ | shape-library instantiation from stream |
| new theory (axiom subset) | ✓ | discover / merge / demote |
| new shape family | ✓ (first-order) | `discover_axiom_shape_families` clusters existing axioms |
| **new axiom shape** | **✗** | shape library is hard-coded (antisymmetry, transitivity, totality, equivalence, modus ponens, ...) |
| **new second-order abstraction** | **✗** | no mechanism to compress co-occurring shapes into a named "axiom of axioms" (e.g. discover that antisymmetry+totality co-occur and mint "linear-order") |
| **object/type emergence** | **✗** | tokens come from the stream as strings; the network topology never lifts an equivalence class or a stable subgraph into a named candidate object |
| **intrinsic drive** | **✗** | scheduler is rule-driven; there is no "what phenomenon is currently unexplained" signal |

The shape library is the hard ceiling. Everything Phase 0070-0072
shipped is curation **inside** that fixed library.

This ADR records the pivot: v2's next phase is **concept
emergence**, not concept curation.

## Decision

**Phase 0070-0072 is closed.** Subsequent threshold tuning,
ablation expansion, or migration-atlas re-validation are
explicitly deprioritized. Open items (3a "deeper ablation on
structurally-distinct substrate", Jaccard 0.50 sensitivity scan)
are deferred indefinitely; if they ever matter again, they
return as a separate ADR.

**v2's next phase has three primary entry points**, in priority
order:

### E1 — Shape mining (highest priority)

Discover **new axiom shapes** from co-occurrence and structural
relations between existing axioms — i.e. let the system extend
its own shape library.

Concrete shape: an axiom $A$ in shape $S_A$ and an axiom $B$ in
shape $S_B$ that consistently fire together on the same
substrate fragments are candidates for a composite shape $S_{AB}$
that captures their joint applicability. Candidate shapes are
validated by the same cross-precision machinery already shipped
in ADR 0071: a new shape is real iff axioms instantiated from it
predict R correctly on imagined substrates.

This is a true first — for the first time the system extends
the *vocabulary* it reasons in, not just the *combinations*
expressed within a fixed vocabulary.

Falsifiable: shape mining produces zero new shapes that pass
cross-precision threshold on OQ#1 / long5k / OQ#2 → null result,
the shape library is structurally complete for these substrates.

### E2 — Object lifting (medium priority)

Promote stable structural patterns in the R network to
**candidate objects**. Constitution commitment 4 ("identity is
token-based") means objects are tokens; lifting therefore mints
a new token whose identity is grounded in graph structure (a
specific equivalence class, a stable subgraph, a hub node by
some structural fingerprint), and registers it via meta-R
("type X has Y") so subsequent axioms can quantify over it.

Constitutional risk: the bridge between "structural fingerprint"
and "token name" must not silently create different tokens for
the same structural pattern across episodes (commitment 5,
"similarity is structural"). The lifting machinery has to be
deterministic given the structure.

Deferred until E1 produces enough vocabulary that "object of
type T" becomes meaningful — currently theory membership (axiom
subset) is the only "type-like" abstraction available.

### E3 — Intrinsic drive (parallel priority with E1)

Replace the rule-based scheduler with one that **picks attention
focus by what is currently unexplained**. Concrete signal: an R
instance in the stream that no current axiom in any current
theory predicts. The scheduler attends to such phenomena
preferentially — this is the bridge from "system observes a
stream" to "system constructs explanations under intrinsic
drive".

This is a precondition for E1's autonomy: shape mining without
an attention signal will scan blindly. With E3, mining is
directed at the unexplained R subset.

E3 has historical precedent in v1's ADR 0031 (intrinsic drive +
global abstraction score). v2's version inherits the principle
but reframes around the R-only ontology: drive is computed from
"how much of stream R is unexplained by current theories",
not from a manually-chosen meta-metric.

## Alternatives considered

**Alt A: Continue refining the intervention loop.** Threshold
sensitivity scans (Jaccard 0.50, MERGE_QUALITY_FLOOR 0.70 on
more substrates), recommendation execution loop (O1), automatic
recommendation consumption. These remain technically valid but
do not address the diagnosis. Curation cannot become creation
by sharpening curation rules. Rejected as premature optimization
of a closed sub-loop.

**Alt B: Skip the pivot, go straight to E2 (object lifting).**
Objects feel like the most "philosophical" capability and match
constitution commitment 5 most directly. Rejected because: until
E1 ships, the system has no "type-of" vocabulary for the lifted
objects to participate in. Lifting a hub node into a token "h_1"
is meaningless if no shape can quantify over hub-typed nodes.

**Alt C: Build E3 first.** Intrinsic drive could surface
unexplained phenomena and motivate the rest. Rejected as
sequencing: an attention signal without anywhere to direct the
attention is wasted. E1 + E3 are paired — drive provides the
"where to look", mining provides the "what to mint when looked".

**Alt D: Wait for empirical pressure.** Some interventionist
view says concept emergence will be forced by failure of
existing machinery; until that failure happens, work the current
loop. Rejected because the v2 failure bar (one reproducible
structural failure is sufficient) is met: every Mixed theory
discovered to date instantiates only existing shapes. That's not
"the system explored and found nothing new" — that's "the system
cannot explore."

## Consequences

**Closed:**
- Phase 0070, 0071, 0072 + Addenda 1/2/3 — production
- Phase 0072-A intervention ablation (OQ#1 + long5k) — done
- Phase 0072-B threshold scan — done

**Now easy:**
- Build shape mining as a first-class layer parallel to ADR 0070's
  shape-family layer (which currently classifies but does not
  mint new shapes)
- Reuse the cross-precision machinery (`axiom_cross_precision`,
  `theory_quality_report`) as the validator for proposed shapes
- Reuse `recommend_intervention` as the curator that prunes
  newly-minted shapes that fail validation

**Now harder:**
- Token mint policies (E2's risk): how to guarantee that the
  same emergent object across episodes maps to the same token,
  given that constitution 4 forbids implicit dedup
- Intrinsic-drive metric calibration (E3's risk): "unexplained
  R" depends on the current theory set, which is itself
  evolving; the drive signal must be stable enough to direct
  mining without thrashing

**Deferred until E1/E3 ship:**
- Recommendation execution loop (O1) — when the system is
  actively minting + retracting concepts, auto-consuming
  recommendations becomes much riskier; gate it behind concept-
  emergence validation
- Multi-substrate ablation expansion — the substrates that
  currently exist may not be diverse enough to exercise emergent
  shapes; if E1 produces shapes that work on OQ#1 but fail on a
  novel substrate, that's the new test, replacing the current
  ablation suite
- Sensitivity scans on Jaccard 0.50 — irrelevant under emergence
  if the merge geometry changes when shapes mint new families

**Workstream order (proposed):**
1. **Phase Emergence-1**: shape mining proposal-and-validate
   loop (E1) — propose composite shapes from co-occurring
   axioms, validate via cross-precision, register surviving
   shapes
2. **Phase Emergence-2**: intrinsic drive scheduler (E3) — track
   unexplained-R, focus mining and discovery on it
3. **Phase Emergence-3**: object lifting (E2) — once E1
   produces shape candidates referring to structural roles,
   mint candidate-object tokens and meta-R register them

Each phase is its own ADR. This ADR (0073) records only the
pivot, not the implementation.

## Implementation

This ADR is non-technical (no code change). It records the
phase boundary.

**Pre-pivot last commit**: `07c1ed6` (Phase 0072-B threshold
scan).

**Forward directions doc** (`docs/forward-directions-2026-05-01.md`)
should be updated to mark E1/E2/E3 as the new highest priority,
demoting earlier items that addressed curation refinement.

## Open questions

- What does "axiom shape" mean precisely as a data type that the
  system can mint? Currently the shape library is implemented as
  Rust enums + match arms; minting requires a runtime
  representation. Proposal: shape-as-meta-R (analog to ADR 0010
  for patterns), but for axioms.
- Does shape mining belong inside the runtime loop or as a
  separate "consolidation pass" (analogous to ADR 0067's
  refactor)? The autonomy criterion says inside; the
  computational cost says separate.
- How does shape mining interact with ADR 0068's existing shape
  families? Possible answer: shape mining proposes *new* shapes,
  which then enter the family discovery in ADR 0070 — i.e.
  mining is upstream of family clustering.

These questions are deferred to Phase Emergence-1's ADR.
