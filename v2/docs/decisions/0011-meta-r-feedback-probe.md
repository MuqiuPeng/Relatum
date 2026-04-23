# 0011: Meta-R feedback probe (observation before γ)

Status: Accepted
Date: 2026-04-23

## Context

ADR 0010's Consequences flagged a feedback loop: after naming, the
RSet contains both original data and meta-R (registry / ownership /
participant) edges. Running the existing observation pipeline
(ADR 0007 `compound_class_subgraphs`) on the enlarged RSet surfaces
compound classes that involve meta-R tokens. Whether that yields
useful "patterns of patterns" is an empirical question γ (ADR 0012)
must answer to choose its iteration policy.

Following the precedent of ADR 0007 before β, this ADR runs a focused
probe and records findings instead of committing to a mechanism. It
adds no new code to `lib.rs`. Its output is one example binary, one
log, and the framing for γ's scope.

(Numbering note: γ was originally planned as ADR 0011. With this
probe inserted in front of it, γ becomes ADR 0012.)

## Decision

Run the probe on the mixed graph from ADR 0007 under two conditions:

1. **Baseline** — `compound_class_subgraphs` on the original 14-edge
   RSet. Reproduces ADR 0007's 7-class partition.
2. **Post-naming** — same call after ADR 0010 has named all four
   canonical-form groups, yielding a 49-edge RSet.

For each compound class in the post-naming RSet, record:
- class size,
- whether every member edge is data-only, meta-only, or mixed,
- for classes of size > 1, the connected-component subgraphs and
  their canonical forms (ADR 0008 + 0009 re-applied).

Interpret: do any compound classes surface structural repetition
*among meta-R edges* that would constitute patterns of patterns?
If yes, γ's iteration policy has a concrete target; if no, γ can
safely skip iteration.

## Alternatives considered

- **Skip the probe and design γ from speculation.** Rejected; same
  reason ADR 0007 ran before β.
- **Broader sweep across many graphs.** Deferred; one mixed graph is
  enough to see whether the feedback loop *shape* is informative.
  A targeted follow-up can be added if ambiguity surfaces.
- **Add a probe method to `RSet`.** Rejected as premature. The probe
  uses only existing APIs; no new surface needed.

## Consequences

This ADR produces findings, not machinery. The log section captures:
- baseline vs post-naming compound class counts and sizes,
- per-class classification as data-only / meta-only / mixed,
- canonical forms of any newly repeating subgraphs,
- a specific recommendation to γ on whether iteration should be in
  its scope.

## Implementation

- Example: `v2/examples/meta_feedback_probe.rs` — builds the mixed
  graph, runs baseline, runs naming, runs post-naming pipeline.
- Experiment log: `v2/logs/2026-04-23_meta_feedback_probe.log` with
  the baseline / post-naming diff and γ recommendations.
- No `v2/src/lib.rs` changes.
