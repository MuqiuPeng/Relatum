# 0076: Micro-agent reframing — transient agents over the episode log

Status: Proposed
Date: 2026-05-06

Parents:
- [Reflection 0001 — meaning emerges with concept](../reflections/0001-meaning-emerges-with-concept.md)
- [Constitution amendment — strict reading](../constitution.md#strict-reading-differentiation-requires-registration)
- [0075 — Emergence kernel audit](0075-emergence-kernel-audit-and-runtime-integration.md)

## Context

The user proposed extending Relatum from a monolithic relational
closure engine into a population of *micro-agents* — local,
limited cognitive units that observe, propose, criticize, and
revise local relational structures, with global cognition
emerging from their interaction.

After examining three implementation paths against the
constitution's strict reading (heavy reading of commitments
1/3/4/5):

- **A — Implementation-only agents**: agents as Rust traits
  with private fields (`local_memory`, `confidence`,
  `attention_weight`). Rejected: private fields not registered
  as meta-R constitute a phantom registry of object
  differentiation; this is exactly what reflection 0001 forbids
- **B — Fully ontologized agents**: agents as first-class meta-R
  entities, every state field as a meta-R edge. Rejected for
  this slice: heavy registration cost; loses agent privacy by
  design; not necessarily required for the user's actual goals
- **C — Transient agents**: agents as behaviour patterns over
  the episode log; no persistent private state; agent
  "identity" derives from (action_kind, target, episode_id)
  tuples that already exist in `Memory::episodes`. **Selected.**

This ADR formalizes path C.

## Decision

**Reframe — do not extend.** v2's existing dispatch system is
already a micro-agent architecture under a different name. This
ADR commits to the reinterpretation without adding new ontology
entities or modifying the dispatch path itself.

### Definition

An **agent class** = `(ActionKind, FrontierTarget-type)` pair.
Currently nine classes exist:

| ActionKind | typical target | role |
|---|---|---|
| `DiscoverPatterns` | `PatternSize(N)` | pattern miner (observer + theorist) |
| `DiscoverTheory` | `WholeRSet` | theory builder (theorist) |
| `PruneLowValueObjects` | `Pattern(id)` | pruner (critic) |
| `UpdateTheoryRelations` | `WholeRSet` | relation maintainer (curator) |
| `Declarativize` | `Pattern(id)` / `Theory(id)` | promoter (curator) |
| `DiscoverMetaMetaPatterns` | `WholeRSet` | meta-pattern miner (observer + theorist at meta level) |
| `EvaluatePredictions` | `WholeRSet` | prediction evaluator (critic) |
| `ExecuteComposite` | `ActionSequence(id)` | composite dispatcher (executor) |
| `DiscoverAxiomShapeFamilies` | `WholeRSet` | shape family miner (theorist at L2) |
| `RetractShapeFamily` | `ShapeFamily(id)` | family critic (critic) |

An **agent instance** = a single dispatch invocation = one
`Episode` record in `Memory::episodes` with id, tick, action_kind,
target, and outcome delta.

### What replaces "private agent state"

The user's `trait MicroAgent` proposal lists fields like
`local memory`, `confidence`, `attention weight`,
`prediction history`, `specialization`, `failure record`. Under
path C, none of these are stored as agent-private state.
Instead, all are **derivable from the episode log via queries**:

| user concept | path-C derivation |
|---|---|
| local memory | `Memory::episodes.iter().filter(|e| matches(e, this_agent_class))` |
| confidence | `policy_stats.action_positive_delta_counts[kind] / action_counts[kind]` (already exists) |
| attention weight | dispatch-frequency derivative: how often was this class picked recently |
| prediction history | per-axiom hit-rate trace in `prediction_state` (already exists) |
| specialization | which target subset does this class predominantly act on |
| failure record | episodes with `delta <= 0` for this class |

**No new state is added** to memory or rset. The reframing is
expository: the same episode log is reread as "many transient
agents leaving behaviour traces" rather than "one runtime
dispatching actions".

### Why this satisfies the constitution heavy reading

- No new meta-R markers introduced (commitment 3 not extended)
- No agent token registered in the rset (commitment 4 unchanged
  — token identity remains "string equality on identifiers
  representing observable structure")
- All "agent state" is a query, not a stored attribute
  (commitment 3 strict reading: differentiation requires
  registration → here, we register no differentiation; agents
  are *patterns we read off the log*, not entities the system
  declares)
- Single global RSet preserved (commitment 1 unchanged)

The reframing operates entirely at the **interpretation layer**.
A reader / researcher / engineer may now describe v2 as a
multi-agent cognitive substrate without invoking phantom
registries.

### What changes in code

Minimum: a small set of read-only query helpers that surface
agent-shaped views of the existing episode log:

```rust
impl AutonomousRuntime {
    /// Group episodes by (action_kind, target-type) and return
    /// agent-class summaries: episode count, success rate,
    /// most-recent-tick, target-distribution.
    pub fn agent_classes(&self) -> Vec<AgentClassSummary>;

    /// All episode records for a specific (kind, target-type)
    /// agent class — its full "memory" by query.
    pub fn agent_episodes(
        &self,
        kind: ActionKind,
        target_kind: TargetTypeFilter,
    ) -> Vec<&Episode>;

    /// Recent attention proxy: dispatches in the last `n_episodes`
    /// for this class, divided by total. A class with rising
    /// dispatch share is "gaining attention".
    pub fn agent_attention_share(
        &self,
        kind: ActionKind,
        n_episodes: usize,
    ) -> f64;
}
```

Plus a struct `AgentClassSummary` carrying counts, rates, and
target distribution per class. Pure-query helpers; do not
modify any dispatch path.

### What does NOT change

- No new `ActionKind`
- No new `FrontierKind`
- No `trait MicroAgent` (agents are not Rust types — they are
  query results over `Memory`)
- No new markers in `markers.rs`
- No `agent_id` registry
- No `broadcast` / `send_signal` interface (the global workspace
  is the rset; agents already "broadcast" by editing it via
  meta-R mints in their dispatch)

## Alternatives considered

**Alt A — Implementation-only `trait MicroAgent`**: rejected as
above (phantom registry).

**Alt B — Fully ontologized agents**: rejected for this slice;
viable as future extension if path C's expressiveness is
empirically insufficient. Key indicator that B is needed: a
problem that requires *agent privacy* (an agent maintaining
beliefs others can't read) — under path C, all agent
"thoughts" are publicly readable in the rset / episode log,
which is intentionally lacking privacy. If privacy turns out
to be cognitively essential for the architecture's value, the
B-path expansion would write each agent's private state as
meta-R entries scoped to that agent's id.

**Alt D — Don't reframe; ship a new ADR-0072-style
"reorganization" doc instead**: tempting because it avoids the
philosophical baggage. Rejected because the user explicitly
asked for the cognitive-substrate framing, and a flat
reorganization doesn't capture what the user wants — a way to
talk about Relatum as cognitive ecosystem of local rational
units.

## Consequences

**Now possible:**

- v2 is describable as "many transient micro-agents leaving
  traces in a shared workspace" without violating any
  commitment
- Agent-classes queries surface emergent patterns (e.g.,
  "DiscoverPatterns/PatternSize(2) is the most-active agent
  class on OQ#1; it has dispatched 18 times with 0% success
  rate") that the existing `policy_stats` already implies but
  doesn't make first-class
- Future writing / papers can frame Relatum as a
  consciousness-inspired multi-agent cognitive architecture
  per the user's last paragraph, with a defensible technical
  description

**Now harder:**

- Implementing features that genuinely require agent privacy
  (e.g., "agent A maintains a hypothesis it doesn't yet share")
  — under path C this is impossible by design. If such a need
  arises, path B becomes necessary
- Implementing features that require agent identity to persist
  across episodes (e.g., "this is the same observer that
  noticed pattern X yesterday") — possible under path C only
  via behaviour-pattern matching in the episode log, which is
  fuzzy. If identity continuity is essential, path B again

**Now easy:**

- Documenting Relatum as a "society of mind" / "global workspace"
  / "society of small reasoners" architecture in publications
  without resorting to phantom typing
- Cross-referencing micro-agent literature (Minsky 1986, global
  workspace theory, multi-agent cognitive architectures) without
  having to claim Relatum *is* one of them — only that it is
  amenable to that *interpretation*
- Surfacing scheduler decisions to the user as "which agent
  class got attention this tick" rather than "which action got
  dispatched"

## Implementation

### Phase 0 — this ADR (no code)

Document the reframing. Establishes the conceptual move.

### Phase 1 — query helpers (small)

Add the three query methods listed above on `AutonomousRuntime`,
plus an `AgentClassSummary` struct. Add 3-5 unit tests covering
counts, rates, attention share. Add an example
`phase_emergence_agent_audit.rs` running a substrate to maturity
and printing the agent-class summary table — this is the empirical
demonstration that v2 already operates as a multi-agent cognitive
substrate.

### Phase 2 — episode-log enrichments (deferred)

If users find specific path-C limits painful, add per-class
query helpers for finer-grained behaviour patterns:
- per-class outcome distribution (NewPattern / Existing /
  Skipped / etc.)
- per-class temporal density (peak attention windows)
- per-class target overlap (which patterns/theories does this
  class predominantly act on)

These are all derivable from `Memory::episodes` plus rset reads;
none touches dispatch.

### Phase 3 — path B if needed (deferred indefinitely)

If path-C expressiveness proves insufficient (privacy /
identity-continuity needs not met), revisit ontologization. A
new ADR + reflection 0002 would be required. Indicators:
- a feature naturally requiring agent privacy
- a feature naturally requiring persistent agent identity
- empirical limit of episode-log reading

## Open questions

- **What's an "agent class"'s natural granularity?** This ADR
  uses `(ActionKind, target-type)` pairs (~15 classes). Could
  be coarser (just `ActionKind`, ~10 classes) or finer
  (specific target instances, ~hundreds of classes). Decided
  empirically by the audit.
- **How is "attention share" exactly defined?** The `agent_
  attention_share` proposal is a recent-window dispatch fraction.
  Could also use: priority sum, hit-rate weighted, etc.
  Implementation will pick one default + leave config open.
- **Is `AutonomousRuntime` the right surface?** Could put the
  query helpers on `Memory` instead. Decided: `AutonomousRuntime`
  because some queries (e.g., scheduler-related "attention
  share") need access beyond just memory.

These are decided in implementation.

## Implementation note (the deeper alignment)

This reframing has an interesting alignment with the constitution's
strict reading that wasn't obvious until path C was selected:

> Under the strict reading, agents *cannot* be private entities.
> But agents naturally *should not* be private entities — a private
> agent is just one with information the system collectively can't
> see. Hidden information is exactly what creates phantom typing
> the strict reading forbids.

So the strict reading and path C agree by construction:
**a v2-compliant agent is one whose entire existence is publicly
readable from the rset + episode log**. There is no "hidden agent
state" because the strict reading forbids it. The commitment is
saying the same thing as the architecture: cognition is public.
