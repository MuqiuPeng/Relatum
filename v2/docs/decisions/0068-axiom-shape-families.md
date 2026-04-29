# ADR 0068 — Axiom shape families: first runtime extension of structural vocabulary (2026-04-29)

## Status

Accepted; landed.

## Context

After 9 phases of Phase Alpha (theory tournament, demote, repair,
merge, perf, dream phase, cross-precision), the user observed that
**none of these phases extended the system's structural
vocabulary**:

> "9 个 phase 里，0 个拓展了系统能识别的结构性词汇。最后一次真正的
> 词汇拓展是 Phase H1（4-27 左右）—— action sequence promotion。
> 那之后 8 天的工作都是在现有词汇上抛光元机制。"

The user's framing reframed v2's progress goal: not "build a target
concept" (their probe was about integers, but they clarified it
isn't a specific goal), but rather "extend what kinds of structures
the system can spontaneously identify."

ADR 0068 is the first concrete response.

## What is hardcoded vs what is discovered

Pre-ADR-0068, v2's system catalog included these compile-time
declared types:
- `PATTERN_MARKER` (subgraphs)
- `THEORY_MARKER` (axiom conjunctions)
- `AXIOM_MARKER` (rules)
- `EXTENDS_MARKER` / `INDEPENDENT_MARKER` / `PARALLEL_MARKER`
  (theory relations)
- `ESTABLISHED_MARKER` / `SHARED_AXIOM_MARKER` (lifecycle)
- `ACTION_SEQ_MARKER` (composite actions)
- `DRIVE_MARKER` / `PENALTY_MARKER` (drives)

These are all **declared** in source. The system discovers
INSTANCES (specific theories, specific patterns) but not new
TYPES.

The H1 line (action sequences) was the closest prior precedent: the
system mints `seq_N` instances at runtime. But the *type*
`ACTION_SEQ_MARKER` is still source-declared.

ADR 0068 introduces `SHAPE_FAMILY_MARKER` — a meta-class whose
**instances are discovered structurally from data**, not declared.

## Decision

Introduce `SHAPE_FAMILY_MARKER` and `RSet::discover_axiom_shape_families`.

The first family kind: **shared canonicalized premise**. Axioms
with byte-identical premise edge sets (after canonicalization) but
possibly different conclusions form a family. The family is named
`shape_premise_<premise_canonical>` and registered as
`R(SHAPE_FAMILY_MARKER, shape_id)`. Membership recorded as
`R(shape_id, ax_id)` per member.

Gates:
- Only template axioms participate (predicate axioms have no
  premise structure)
- Empty-premise families excluded (every axiom would group together
  uselessly)
- Minimum member count threshold (default 2)
- Idempotent — re-running with same axiom set produces no
  duplicates

API surface:
- `discover_axiom_shape_families(min_members) -> Vec<String>` (mint)
- `is_axiom_shape_family(id) -> bool`
- `axiom_shape_families() -> Vec<&str>`
- `shape_family_members(shape_id) -> Vec<&str>`

## Constitution check

- C1 (R singular): ✓ — family relations use R
- C2 (R binary): ✓
- **C3 (types as meta-R)**: ✓ ⭐ — this is the strongest realization
  of commitment 3 to date. Previously the type was declared in source
  and the system found instances; now the type-instances are
  discovered structurally. The marker `SHAPE_FAMILY_MARKER` is still
  declared, but the actual structural categories beneath it
  (`shape_premise_p0-0_p1-2`, etc.) emerge from data.
- C4 (token identity): ✓ — shape ids are deterministic,
  byte-stable derivations from premise canonical form
- C5 (structural similarity): ✓ — family membership is purely
  structural (premise edge set equality after canonicalization)

## Empirical result on OQ#1

[`examples/phase_beta_1_shape_families.rs`](../../examples/phase_beta_1_shape_families.rs)
runs Phase 0 (1000 ticks), then calls
`discover_axiom_shape_families(2)`.

Pre-Beta-1 state:
- 13 axioms registered, 4 theories registered
- **0 shape families** (the type was unrealized)

Post-Beta-1: **3 families minted**.

| family | members | mean xprec | variance | verdict |
|---|---|---|---|---|
| **shape_premise_p0-0_p1-2** | 4 | 0.4162 | **0.000000** | STRUCTURAL NOISE FAMILY |
| shape_premise_p0-1 | 3 | 0.9016 | 0.019367 | MIXED |
| shape_premise_p0-1_p1-2 | 2 | 0.8524 | 0.021788 | MIXED |

The first family is the load-bearing finding: 4 members with
**variance zero** in cross-precision profile. The system has
spontaneously identified what we (in Phase Alpha-7+) called the
"noise axioms in t_0" — 4 distinct axiom IDs that share the
`p0-0_p1-2` premise structure and behave identically in
cross-validation.

The mechanism behind the variance-zero result: shared premise →
shared binding set → conclusions span the same identifier-pair
space → identical precision under the same substrate set.

The other 2 families (mixed) show that the "shared premise"
abstraction doesn't always capture quality dimension. Their
conclusions vary across `R(x,y)`, `R(y,x)`, `R(x,x)`, `R(y,y)` and
their cross-precision behavior diverges (spread ~0.30). This is
fine — it tells us shared-premise is one structural axis but not the
only one.

## Why this is the first real auto-extension since H1

| Phase | Extended structural vocabulary? |
|---|---|
| Alpha-1 (UCB1) | No — selection over existing actions |
| Alpha-3..3++ (tournament) | No — judgment on existing theories |
| Alpha-3+++ (repair) | No — existing-theory operation |
| Alpha-3++++ (naive merge) | No |
| Alpha-5 (smart merge) | No |
| Alpha-6 (ILP perf) | No |
| Alpha-7 (dream phase) | New observation, not new vocabulary |
| Alpha-8 (cross-prec demote) | New signal use, not new vocabulary |
| Alpha-9 (varying-T) | Validation experiment |
| **Beta-1 (this ADR)** | **Yes — `SHAPE_FAMILY_MARKER` instances discovered, not declared** |

Pre-Beta-1: rset.right_of(SHAPE_FAMILY_MARKER) = ∅
Post-Beta-1: rset.right_of(SHAPE_FAMILY_MARKER) = {3 minted families}

The structural vocabulary (set of available type instances) grew.

## What this is NOT

This is a **first** step on the auto-extension front, not a
finished one:

1. The TYPE itself (`SHAPE_FAMILY_MARKER`) is still source-declared.
   For deeper extension, the system would need to discover that
   "shape family" itself is a useful category — currently I program
   it in.
2. Only one **kind** of family is recognized (shared premise).
   Future Beta-1.X could add: shared conclusion, shared variable
   arity, shared structural symmetries, compositions thereof.
3. No DRIVE consumes the discovered families. Currently they are
   inert observations. Future slices could:
   - Demote entire families when their cross-precision is uniformly
     low
   - Treat axioms in a high-precision family as "trustworthy"
     priors for new theory composition
   - Use family membership to prune the template enumeration
     space (skip premises that already have a noise family)

## Future deferred slices

- **Beta-1.1**: shared conclusion family. Test on OQ#1 — does it
  capture useful clusters?
- **Beta-2**: family-level demote. If a family's mean cross-precision
  is below threshold, retract all members.
- **Beta-3**: family-aware template enumeration. Bias the discovery
  away from premise shapes already in low-precision families.
- **Beta-1 wired into runtime**: add a `DiscoverAxiomShapeFamilies`
  ActionKind that runs periodically. Currently the API is invoked
  externally from the example.
- **Family of families**: groups of shape families that themselves
  share structure. Genuinely nested abstraction.

## Status

Beta-1 Accepted with strong empirical evidence. First post-H1
auto-extension of structural vocabulary on v2. The
`SHAPE_FAMILY_MARKER` meta-class is now part of v2's runtime
ontology; its instance population is data-driven.
