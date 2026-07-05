# Cognitive game framing

A long-horizon framing document for v2. **Not an ADR**: ADRs
record specific technical decisions; this captures architectural
positioning. Continuous-editing doc — no `Status:` header. First
written 2026-04-28 from a discussion comparing Relatum to
AlphaGo.

## Errata (2026-07-06, per ADR 0084 §7)

Two corrections that were overdue. Original text below is kept
unchanged so the errors stay visible.

1. **The search-vs-construction table and the "roughly 70%
   construction" characterization are pre-amendment.** They were
   written 2026-04-28, before the constitution's 2026-05-06
   strict-reading amendment. Under reflection 0001's four-way
   classification (curation / explicit naming / implicit
   conceptualization / genuine emergence), most operations this
   doc labels "construction" — `DiscoverPatterns`,
   `DiscoverTheory`, `DiscoverMetaMetaPatterns`,
   `UpdateTheoryRelations` — are curation or naming acts, not
   construction of new ontology. ADR 0075's audit later
   identified which subset qualifies as a compliant
   concept-creation kernel. Read the table as "mints runtime
   objects" rather than "constructs concepts"; the 70/30 split
   overstates C.
2. **MCTS has a second obstacle this doc missed: low branching
   factor.** ADR 0065's empirical result (UCB1 ≡ greedy,
   byte-identical over 2000 ticks) showed v2 substrates produce
   0–1 eligible composites per decision point — search collapses
   to selection regardless of cost. The MCTS discussion below
   flags only cost asymmetry; both obstacles must fall before any
   Alpha-2-style work reopens (see ADR 0084 §2 for the closure
   and its reopen trigger).

## What this doc is for

ADRs 0061 / 0062 / 0063 / 0064 sketched the H1 and H2 phases
piece by piece. The cumulative direction is clear from the
git log but not stated as a single architectural framing
anywhere. This doc fills that gap so future ADRs (H2.2 / H3)
have a concrete reference for what kind of system v2 is
trying to become.

## The proposal: a six-tuple cognitive game

A natural framing for v2's runtime is a sequential decision
process over relational states:

```
G = (S, A, T, V, D, C)

S: relational cognitive states  (RSet + memory + drive state)
A: available cognitive operations  (ActionKinds, composites, ...)
T: transition function induced by executing operations
V: value functional over explanatory / predictive / compression quality
D: drive policy determining which V components matter when
C: constructor — operations that mint new elements of S, A, V, D
```

The first five elements parallel a Markov decision process or a
game-theoretic formulation. **The sixth, C, is the part that
makes this framing distinctive.** Standard MDPs assume S and A
are given; AlphaGo's S, A, T, V are fixed by the rules of Go.
v2 has none of these as fixed givens. The runtime mints its own
states (new pattern names, theory ids, drive registrations),
its own actions (composite ActionKinds), and its own value
components (DriveMix mutations, eventually drive synthesis in
H2.2).

## Why six, not the AlphaGo-five

The "S and A are variable" framing is correct but understates
the difference. There are two distinct ways an MDP can have
"variable" elements:

1. **Variable-by-environment** — S and A change because the
   environment is non-stationary. The agent observes change but
   does not cause it.
2. **Variable-by-self-construction** — S and A change because
   the agent constructs new elements. The agent *causes* the
   variation.

AlphaGo handles neither. Standard non-stationary MDPs handle
(1). v2 is doing (2). Lumping (2) under "variable S, variable
A" obscures what's actually new: **the system is constructing
its own game pieces, not just searching among them.**

The C element makes this explicit:

| MDP | AlphaGo | v2 H1.x | v2 H2.0 | v2 H2.1.0 (current) |
|---|---|---|---|---|
| C is empty | ✓ | ✓ | partial | partial | partial |
| Adds new actions | — | — | ✓ (composites) | — | — |
| Adds new value weights | — | — | — | ✓ (DriveMix mutate) | — |
| Drives in meta-R class hierarchy | — | — | — | — | ✓ (DRIVE_MARKER) |

The trajectory is clear: each H phase has been adding to C, not
just to A or V. The H2.1 series in particular is dedicated to
making C constitutionally legitimate (drives become meta-R
objects rather than compile-time Rust constructs — see ADR 0064).

## Mapping cognitive operations to search-vs-construction

A cleaner way to read the v2 ActionKinds is to ask, for each:
"is this *picking* among existing elements, or *creating* new
ones?"

| Operation | Search | Construction |
|---|---|---|
| `DiscoverPatterns` | — | ✓ (mints a new named pattern; new S element) |
| `DiscoverTheory` | — | ✓ (mints a new theory token; new S element) |
| `Declarativize` (ESTABLISHED promotion) | — | ✓ (lifts a runtime fact into meta-R) |
| `EvaluatePredictions` | search-like | — (observes; doesn't add elements) |
| `ExecuteComposite` | ✓ (picks a learned composite) | indirect (composites themselves are constructed by H1.1) |
| `PruneLowValueObjects` | search-like | ✓ (retracts elements) |
| `DiscoverMetaMetaPatterns` | — | ✓ (mints meta-meta abstractions) |
| `UpdateTheoryRelations` | — | ✓ (writes new theory-relation edges) |

Most of v2's primitives are **construction**, not search. This
is the inverse of AlphaGo, where every move is a pure-search
operation over a fixed action space.

## What transfers from AlphaGo

### Value-policy decoupling

AlphaZero's value network and policy network are separable
estimators of "how good is this state" and "what should I do
here." v2 has the analogue:

- `combined_drive_signal` / `normalized_drive_signal` is a
  value estimator (how productive does the current state look).
- The scheduler's frontier-priority logic is a policy
  estimator (what action should fire next).

This decoupling is already present and load-bearing as of step 3b
(α). H2.0 step 3+ work could make the analogy more explicit —
e.g., a value-conditional policy that picks differently when
drive signal is deeply negative.

### Self-play as a curriculum mechanism

AlphaGo's self-play generates training data through symmetric
mirroring: the same agent plays both sides. v2 cannot do this
directly (see "what does NOT transfer" below) — but a weaker
form is plausible: the runtime trains itself by alternately
playing **theory-author** and **theory-tester** over its own
output stream. See "self-play candidates" section.

### MCTS, with significant caveats

AlphaGo's MCTS is cheap because rollouts are cheap simulations
over a closed-form board state. v2's potential rollouts are
**expensive cognitive operations** (DiscoverTheory runs axiom
discovery; Declarativize walks ESTABLISHED chains; ...). A
naive translation that does N simulated rollouts per real
action would cost ~N times the runtime budget for ~N times the
information. The trade may not pay off.

Two more realistic transfer paths:

1. **MCTS only at the composite layer.** Composites (length 2
   or 3) are short and their step-kinds are known. A 3-deep
   MCTS over composite candidates is bounded and cheap. The
   primitive ActionKinds layer keeps its current scheduler.
2. **Cheap value estimator instead of rollouts.** Use the
   already-computed `normalized_drive_signal` as an MCTS leaf
   evaluator. Skip rollouts entirely; only descend the search
   tree with value-network-guided expansion.

The MCTS ambition is worth flagging but should not be H3's
opening move. Save it for after the constitutional + value-
function work in H2.1 / H2.2 stabilizes.

## What does NOT transfer

### Naive MCTS rollouts (cost asymmetry)

See above. The cost asymmetry between AlphaGo's 1µs board
simulations and v2's millisecond-scale cognitive operations
makes a direct port impractical.

### Adversarial generator as "self-play"

It's tempting to frame "a generator producing R streams that
the runtime can't explain" as a self-play setup. **It isn't.**

Self-play in AlphaGo works because of *symmetry*: both sides
have the same action space, the same observation, the same
value function. A generator producing R streams and a runtime
trying to explain them are **structurally asymmetric** — the
generator has no theory of mind about the runtime's
explanatory state, and vice versa. This is **curriculum
learning** in self-play's clothing: a useful technique, but
distinct.

The label matters because curriculum learning has its own
failure modes (the curriculum can become a Goodhart target —
generator over-fits to runtime's current weakness; runtime
over-fits to generator's quirks; neither generalizes). True
self-play sidesteps these because both sides are equally
incentivized to find general weaknesses in the *other* — only
because they share strategy space.

### Single terminal reward

AlphaGo's reward is `{+1, 0, -1}` at game end. v2 has no game
end. Cumulative reward shaping (sum of EP-deltas, sum of
abstraction-score gains, etc.) substitutes — but each
substitution is a *choice*, not a fact. H2.0's DriveMix is the
mechanism for tuning that choice; H2.1+ may make the choice
itself a meta-R object.

### Closed-form value function

AlphaGo's value is given by the rules. v2's value is
*constructed* by the system (compression / prediction-error /
mode-thrash / future synthesized drives). The H2.2 design
space is exactly "what does it mean for a system to author its
own value function?" — that question doesn't even arise in
AlphaGo.

## Self-play candidates worth pursuing

These are speculative — they preserve the symmetry property
that makes AlphaGo self-play work, while adapting to cognitive
substrate.

### (a) Internal theory competition

Two co-existing theory candidates (or two ESTABLISHED-state
configurations) predict overlapping streams. Whichever
predicts better in a held-out window survives; the other is
demoted. This has true symmetry: both candidates share the
prediction-target, the observation stream, and the metric.

### (b) Cross-substrate runtime clones

Two clones of the same runtime see different substrates
(diamond posets vs bipartite injections, say). Each runs to
saturation; theories from each get evaluated on the *other's*
substrate. Theories that transfer survive; substrate-specific
ones don't. Symmetric because both clones use identical
discovery machinery.

### (c) Adversarial-but-symmetric self-play

Drop the "generator vs runtime" framing entirely. Run two
runtime clones, each trying to *predict each other's
predictions*. The "adversarial" element is implicit: each
clone's accuracy is measured against the other, but they have
matching strategy space. This is closer to multi-agent RL's
self-play than to curriculum-learning.

None of these are committed work. Recording them so the design
space is mapped.

## The refined positioning sentence

The original draft was:

> AlphaGo solves search over a fixed game of board states;
> Relatum attempts search over a self-extending game of
> relational abstractions.

The refinement adds the construction axis:

> **AlphaGo solves search over a fixed game with given pieces
> and given value; Relatum attempts search-and-construction
> over a self-extending game whose pieces, value, and even
> the rules are partially constructed by the system itself.**

The "search-and-construction" hyphenation matters: v2's
operations are roughly 70% construction, 30% search (see the
table earlier in this doc). Calling it "search over a
self-extending game" reads as "search where the action space
happens to grow" — but the actual generator of growth is the
search agent itself. The refined sentence puts the agent's
constructive role at the center.

A complementary inversion is also useful as a headline:

> **AlphaGo is Relatum-class with C empty.**

This reframes "AlphaGo as the example we're trying to imitate"
as "AlphaGo as a degenerate case of what we're actually after."
The reframing is informative: it tells us the fundamental
difference is not "more variables" but "constructor non-empty."

## Open questions for H3 and beyond

1. **What does it mean for a system to *author* its own value
   function?** H2.0 mutates weights over a fixed catalogue.
   H2.2 (deferred research) would mint new value components
   from primitive metrics. The constitutional question is
   commitment 4 (token identity) — synthesized values need
   stable, derivable identifiers.

2. **Where does C-the-constructor itself sit ontologically?**
   Currently each construction operation is a Rust ActionKind.
   But if drives can become meta-R via H2.1, why not the
   constructor operations themselves? "DiscoverTheory" as a
   meta-R-registered operation that the runtime could itself
   demote / replace is a logically clean continuation. It's
   also constitutionally novel — commitment 3 explicitly says
   types are meta-R; if construction operations are types,
   they should be meta-R too.

3. **Composite-of-composites.** ADR 0062 deferred this. The
   AlphaGo analogy makes the case sharper: AlphaGo's tree
   search is recursive over moves; v2's ExecuteComposite is
   currently flat. Deep composites would let the runtime
   author hierarchical operational programs.

4. **The transfer property as a primary metric.** Not in scope
   today (cumulative compression / prediction-error are the
   load-bearing metrics) but: is "theories that transfer
   across substrates" a candidate value component for H2.2?
   Self-play candidate (b) above gestures at this.

5. **What's the v2-equivalent of AlphaGo's "tablebase" or
   pre-computed terminal positions?** Probably nothing —
   there's no terminal in v2. But "pre-computed common
   abstractions" (a starting library of well-known patterns)
   is conceptually nearby. ADR 0023 (cross-graph pattern
   transfer) is the closest existing precedent.

## What this doc does NOT decide

- Whether to implement MCTS-style cognitive search (deferred).
- Whether to commit to drives-as-meta-R queries (H2.1.1+ work,
  not gated on this framing).
- Whether C should be made an explicit Rust trait (probably
  unnecessary; the framing exists at the design-philosophy
  layer, not the API layer).
- Anything about H2.2 drive synthesis specifics (research
  territory; ADR 0063 Addendum 5 closed step 3b but H2.2 is
  untouched).

## What this doc does decide

- The (S, A, T, V, D, **C**) six-tuple is the canonical
  high-level architectural framing for v2's H-series work.
- The "search-and-construction" distinction is recorded as
  the dominant mode characterization — most v2 ActionKinds
  are construction, not search.
- The MCTS analogy is admissible but bounded (composite layer
  only; no naive rollouts).
- Adversarial-generator setups are explicitly *not* called
  "self-play." Curriculum learning is a different label.
- Three symmetric self-play candidates (internal theory
  competition, cross-substrate clones, mutual-prediction) are
  recorded as design space without committing.

## Pointers

- ADR 0063 — Phase H2 / drive self-modification (the value-
  function-construction work in progress).
- ADR 0064 — Phase H2.1 / drives as meta-R (the
  constitutionally load-bearing slice for C).
- ADR 0061 — Phase H1 / action-sequence mining (the first
  C-extending mechanism).
- ADR 0023 — Cross-graph pattern transfer (precedent for the
  transfer-property direction).
- progress.md — phased history.
- retrospective-2026-04-26.md, retrospective-2026-04-27.md,
  retrospective-2026-04-27-late.md — phase-boundary notes.
