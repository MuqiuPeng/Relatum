# Relatum v2 Design Notes (Internal)

> Internal design reference. Not external spec. Captures the thinking that
> produced the constitution and the v2 direction. Subject to revision as
> implementation proceeds.

## Core philosophical position

### Fundamental ontology
- **No truth, only a fact space.** What we call "truth" is a projection or
  slice of the fact space. Cognition itself is an abstraction process.
- **Objects emerge from relations.** "Apple" = intersection of red ∩ spherical
  ∩ fruity ∩ hard in sensory space — not a primitive.
- **Cognition = abstraction.** The running system's activity *is* ongoing
  abstraction.

### Stance
- This is a theoretical commitment. v2 is that commitment made concrete.
- Relatum is a tool for practicing the idea, not an end in itself.
- Architecture is subordinate to the stance. When closure is insufficient,
  the architecture changes.

## v2 goal

**Under system-intrinsic drive, construct — from a minimal given relation —
new relations that explain new phenomena.**

Decomposed:
1. **Minimal relation:** start from very little.
2. **Self-driven:** system decides direction; no external steering.
3. **Constructive:** generate new relations (not discover existing ones).
4. **Explanatory:** new relations render new phenomena intelligible.

## Primitive choice

**Only `R(x, y)`** — a binary directed relation with no intrinsic meaning.

Why not `Corr(x, y, s)` (correlation with strength):
- Strength `s` lives in a numeric domain with order built in.
- That order is a smuggled structure. Not minimal.

Why `R(x, y)`:
- Directionality is given, but meaning is not.
- No coordinate frame, no anchor, no strength axis.
- Graph structure; meaning emerges from usage patterns.

## Required system capabilities

1. **Object emergence** — recognize stable identifiers in the R instance stream.
2. **Pattern abstraction** — detect recurring structural patterns in R distribution.
3. **Relation composition** — let newly abstracted relations feed further abstraction.
4. **Type naming** — name abstracted patterns, making them first-class.
5. **Evaluation** — judge whether a newly constructed abstraction is worth keeping
   (MDL-style compression criterion; specifics TBD).

## Concrete target: the integer set

An illustrative — not prescriptive — construction path:

Input: R(a1, a2), R(a2, a3), ... and R(a5, a4), R(a4, a3), ...

Possible emergent path:
1. Identify stable identifiers (a1, a2, ...) as objects.
2. Notice role patterns (each `ai` appears as both left and right).
3. Detect chain patterns (connected linear sequences).
4. Abstract chain to ordering.
5. Abstract local adjacency to successor.
6. Recognize bidirectional chain pairing.
7. Recognize unbounded extension.
8. Name the whole structure as type_1 (integer-like).
9. Explain new chain inputs by matching type_1.

**This path is one possibility, not a requirement.** If the system discovers
a different abstraction (cycles, stars, density clusters), that is also a
success — the success criterion is abstraction happening, not a specific target.

## Experiment design

### Minimal frame
- Input: a set of R instances with no pre-declared objects.
- Process: object emergence → pattern abstraction → relation construction →
  type naming → new-phenomenon explanation.
- Output: one or more emergent relation types.

### Success indicators
1. **Autonomy** — abstraction happens without external prompt.
2. **Constructivity** — new relations appear, not just retrieved.
3. **Explanatory power** — new relations describe inputs not seen during construction.
4. **Compression** — emergent types are shorter descriptions than the raw instance set.

### Failure diagnosis axes
- Object emergence fails — identifiers don't stabilize.
- Pattern abstraction fails — no named types emerge.
- Evaluation fails — junk types outnumber useful ones.
- Composition fails — types don't feed further abstraction.

## Relation to v1

- **Paradigm shift:** closure → continuous abstraction; static → dynamic;
  discovery → construction.
- **Preserved value:** Phase 1–9 results in `v1/` serve as the v1 benchmark.
  v2 is not "v1 + features"; it is a different direction tested against the
  same question space.

## Implementation roadmap (sketch)

1. **Infrastructure** — R data structure, token-based identity, basic
   graph-structure queries.
2. **Minimal experiment** — feed a chain dataset, observe what (if anything)
   the system abstracts.
3. **Capability extension** — more complex patterns, cross-type composition.
4. **System integration** — continuous abstraction loop with self-driven triggers.

## Design principles (repeated for emphasis)

1. Minimize assumptions — only R(x, y); everything else must earn its place.
2. Autonomy first — the system chooses direction and timing.
3. Constructive over discovery — generation is the goal.
4. Verifiability — every abstraction must be inspectable.
5. Open-endedness — admit system limits; pursue continued expansion.

## Philosophical checks at each design decision

- Does this preserve "cognition = abstraction"?
- Does this emerge from the primitive, or is it imported?
- Does this reflect system autonomy?
- Does this aid construction, not just discovery?

## Known risks

### Technical
- Pattern abstraction is computationally heavy.
- Self-drive can loop unproductively.
- Evaluation criteria are ill-defined.

### Philosophical
- The stance can be compromised in implementation.
- "Self-drive" may secretly depend on structural hints we injected.
- Abstraction may diverge without bound.

### Mitigation
- Start with the simplest possible example.
- Mark every compromise explicitly when it occurs.
- Treat failure as diagnostic, not fatal.

---

**Note.** This document snapshots current thinking. It will be revised as
implementation proceeds. v2 is the embodiment of a philosophical position,
not a fixed technical specification.
