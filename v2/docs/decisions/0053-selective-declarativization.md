# 0053: Selective declarativization (M1)

Status: Accepted (Phases C0 + C1 + C2 implemented)
Date: 2026-04-26

## Context

ADR 0052 split runtime memory into two layers:

- **M0 — durable operational memory.** Episodes, mode and
  lifecycle transitions, `ObjectHistory`, `PolicyStats`. Survives a
  process restart but is *not* knowledge. Now landed end-to-end
  through Phase B0/B1/B1+/B2/B3.
- **M1 — declarativized memory (meta-R).** A subset of M0 promoted
  to first-class meta-R facts that downstream discovery can pattern
  over. Explicitly deferred in ADR 0052 (`§ Memory policy / Deferred
  M1`).

Phase B is now complete. The runtime feeds history *and* acts on
it (B1+ pattern-cooldown, B3 stale-prune). The next phase is C —
"selective declarativize." This ADR scopes that phase before any
code lands.

The trap to avoid is the one ADR 0052 named: **conflating
durability with declarativeness.** Every M0 record is durable.
Most M0 records are not, and should never be, declarative.
Declarative facts are (a) stable enough to be referenced, (b)
referenced enough to matter, (c) carrying clear knowledge
semantics — not operational accounting.

## Decision

### Two layers of meta-R, not one

Today's meta-R already holds the system's discovered ontology:

- Patterns named via `name_pattern` (`PATTERN_MARKER` edges).
- Axioms named via `name_axiom` (`AXIOM_MARKER`).
- Theories named via `name_theory` (`THEORY_MARKER`).
- Theory relations: `EXTENDS_MARKER`, `INDEPENDENT_MARKER`,
  `PARALLEL_MARKER` (ADRs 0034 / 0042 / 0046).

These are meta-R facts whose lifetime is tied to the act of
discovery itself. Once `discover_theory` names a theory, the
theory exists in meta-R. There is no separate "is this discovery
durable enough to keep?" gate — the rset is the source of truth.

M1 introduces a **second class of meta-R facts** about the
runtime's *experience* of those discoveries:

- "pattern X has been stable for K passes"
- "theory Y was the focus of N positive episodes"
- "axiom Z appears in two distinct theories"

These are not derivable from rset state alone. They are derived
from `ObjectHistory` + `PolicyStats` over time, then *promoted*
into meta-R when they meet a stability threshold.

### Promotion rule (ADR 0052 § Deferred M1, restated)

A memory item graduates to meta-R when **all** of:

1. **Stable for ≥ K passes.** For a named pattern / theory /
   axiom: `tick - last_change_tick ≥ K` where `last_change_tick`
   is the last tick when the object's `ObjectHistory` saw a
   structural update (first_seen, last_improved, or pruned).
2. **Referenced by ≥ M episodes.** Cumulative count: appearances
   in `Episode.target` (or membership in a theory whose
   `Episode.target` was the rset). Captures "the runtime has
   actually used this." Patterns that exist but are never
   selected don't pass this gate.
3. **Has a clear knowledge name.** Restricted to objects already
   named in the rset. Anonymous patterns (set-of-instances with
   no `p_*` id) are not eligible — they have nothing to reference.

`K` and `M` defaults: `K = 100`, `M = 5`. Tunable per category.

### Phase C0 — smallest implementable slice

Just **established patterns**. One new meta-R marker:

```
ESTABLISHED_MARKER = "established_v2"
```

When a pattern `p_x` passes the promotion gate, the runtime adds
the edge `R(p_x, ESTABLISHED_MARKER)` to the rset.

Why patterns first:

- They are the most common named object type — gate fires
  earliest, easiest to test.
- They already have a well-defined identity (`p_*` id, instance
  set).
- Demotion is clean: if `retract_pattern(p_x)` runs, the
  ESTABLISHED edge falls with it (rset-level cascade — same
  policy as PATTERN_MARKER edges).

Why a new marker rather than reusing PATTERN_MARKER:

- PATTERN_MARKER says "this id is a pattern."
  ESTABLISHED_MARKER says "this id has earned trust through
  use." Different semantics; conflating them loses the second.
- Downstream discovery can pattern over established patterns
  specifically: the meta-R subgraph `R(?, ESTABLISHED_MARKER)`
  becomes a queryable concept. A pattern over those is a
  meta-meta-pattern.

### Phase C1 — theory promotion (implemented)

Same `ESTABLISHED_MARKER`, same M ≥ 1 cheap path as C0, applied to
named theories with a more conservative age threshold:
`min_theory_age_for_promotion` defaults to **200 ticks**. Tighter
M ≥ 3 (the original ADR sketch) is deferred — same reasoning as C0,
needs an explicit contribution counter on `ObjectHistory`.

Demotion piggybacks on `RSet::retract_theory`, which gains a final
step removing `R(theory_id, ESTABLISHED_MARKER)` symmetric to
`retract_pattern`'s step (7).

### Phase C2 — shared-axiom promotion (implemented)

`SHARED_AXIOM_MARKER` ("__shared_axiom__") is emitted as
`R(<axiom_id>, SHARED_AXIOM_MARKER)` whenever
`theories_containing(axiom_id).len() >= 2`. Unlike C0/C1, the
gate is purely structural — no `ObjectHistory` lookup, no age,
no contribution counter. The promotion item rides the same
`Declarativize` action as C0/C1, but the dispatcher selects the
marker by target type:

- `FrontierTarget::Pattern(id)` → ESTABLISHED
- `FrontierTarget::Theory(id)`  → ESTABLISHED
- `FrontierTarget::Axiom(id)`   → SHARED_AXIOM

Demotion is automatic via `RSet::retract_theory`'s cascade —
when a theory is removed, every member axiom whose surviving
theory count drops below 2 has its `SHARED_AXIOM` edge stripped.
Multi-share scenarios stay correct: an axiom in three theories
keeps the marker after one is retracted (still ≥ 2).

### Demotion

ESTABLISHED edges follow their target's lifecycle. If the target
pattern is retracted, the ESTABLISHED edge is retracted with it.
The runtime does not separately demote based on staleness — once
a pattern earned ESTABLISHED, that fact stays until the pattern
itself goes. (Stale-prune from B3 is the path; if the pattern
becomes stale enough to be pruned, ESTABLISHED falls with it.)

### Where the gate runs

Implementation reuses the Frontier / scheduler path rather than
adding a Reflect-entry hook:

- `Frontier::refresh_established_promotions(&rset, &history, tick)`
  runs alongside `refresh()` and `refresh_stale_prune()` whenever
  the frontier is dirty. Eligible patterns become
  `FrontierKind::EstablishedPromotion` items at fixed priority
  1.5 — above stale-prune (0.5), below typical negative-cv
  prune (≥ 1.0).
- Scheduler picks promotion items in Consolidate mode (alongside
  the existing `LowValueObjectForPrune` /
  `TheoryNeedsRelations`).
- Dispatch runs `ActionKind::Declarativize`, which calls
  `rset.add(R::new(id, ESTABLISHED_MARKER))`. The episode records
  with whatever `abstraction_score` delta the new edge produces;
  the runtime already handles non-positive episodes uniformly.

This was cleaner than a Reflect-entry hook: it reuses the
mode-thrash gate, the budget mechanism, and the episode log
without special-casing declarativization.

## Alternatives considered

- **Skip C entirely; let pattern over `PATTERN_MARKER` do the
  work.** Loses the experience signal. A brand-new pattern and a
  pattern referenced 100 times look identical to downstream
  discovery; the runtime's accumulated history is wasted.
- **Encode stability as a numeric attribute on the existing
  PATTERN_MARKER edge.** v2 commitment 2: R is binary. No edge
  attributes. The only way to express "stable for K" is via a
  separate edge.
- **Promote on every tick, demote on every tick.** Churns the
  rset and breaks the principle that meta-R facts are stable
  references. The threshold gate is the point.
- **Use a richer marker per category (`STABLE_PATTERN`,
  `MATURE_PATTERN`, etc.).** Overengineering. Two states (named
  vs established) carry the signal; sub-categorization is
  premature without a use-case demanding it.

## Non-goals

- Demotion based on time alone. Only structural retraction
  removes ESTABLISHED.
- Numeric "trust scores" on the rset. The gate is binary.
- Rewriting any existing M0 records. M0 stays exactly as B-line
  defined it; M1 sits next to it and reads from it.
- Backporting to v1. v1 has no runtime layer; declarativization
  is a v2-only concept.

## Verification plan

For Phase C0:

1. Existing 358 tests pass unchanged.
2. New tests:
   - **gate negative — fresh pattern**: pattern with age < K, no
     promotion edge appears.
   - **gate negative — never-selected**: pattern with age ≥ K
     but `times_selected_as_focus + improvement count < M`, no
     promotion.
   - **gate positive**: pattern meeting both thresholds, edge
     appears after a Reflect entry, idempotent across multiple
     entries.
   - **demotion via retract**: retract the pattern, ESTABLISHED
     edge gone too.
   - **B3 interaction**: stale pattern that ALSO became
     established → still gets pruned (B3 fires), and on prune the
     ESTABLISHED edge cascades. Confirms B3 and C0 don't deadlock.
3. End-to-end: a 200-tick run on a structured input where the
   poset's named patterns last long enough to graduate. Final
   rset contains at least one `R(p_*, ESTABLISHED_MARKER)`
   edge.

## Open questions (for the implementation, not blocking acceptance)

1. **Counter source for "M references."** Options: cumulative
   `Episode` scans (expensive but exact),
   `times_selected_as_focus + (delta-positive contribution count)`
   (cheap, already in B0). Default to the cheap path; revisit if
   it underestimates.
2. **Where ESTABLISHED_MARKER is collected.** Should it be
   exposed via `collect_meta_ids` like other markers? Yes — keeps
   meta-R hygiene consistent.
3. **Reentry**: if the runtime restarts after checkpoint, the
   ESTABLISHED edge is in the rset (rset is checkpointed via
   ADR 0038). M0's `last_change_tick` is in the checkpoint
   (Phase B2). So promotion state survives restart by virtue of
   the underlying layers. No new persistence work.
4. **Naming convention**: `established_v2` to namespace away
   from any potential v1 / external collision. Bikeshed.

## Touched ADRs (declared by this ADR)

- **ADR 0030** `name_theory`, **ADR 0018** pattern naming —
  read-side only; promotion adds a new edge but does not change
  naming behavior.
- **ADR 0038** persistence — ESTABLISHED edges round-trip with
  the rest of the rset; no checkpoint format change.
- **ADR 0040** Prune lane — interaction in the B3 cascade test.
- **ADR 0052** Phase C is realized by this ADR. Implementation
  lands as Phase C0 (this slice), C1 / C2 in follow-on commits
  if the use-case stays warm.

## Summary

M1 is the smallest set of meta-R facts the runtime can declare
about its own experience without violating v2's commitments
(R-binary, R-singular, identity-by-token). Phase C0 lands one
marker: ESTABLISHED, attached to patterns that have been around
long enough and used enough. Promotion is gated; demotion piggy-
backs on existing retract paths. No new persistence machinery,
no new ontological layer — just a second meta-R class about
*experience-with*, alongside the existing meta-R class about
*kind-of*.

If this lands cleanly, downstream discovery can pattern over
ESTABLISHED edges directly: "what do all established patterns
share?" becomes a query the system can ask of itself. That is
the smallest concrete payoff Phase C should aim to make
possible.
