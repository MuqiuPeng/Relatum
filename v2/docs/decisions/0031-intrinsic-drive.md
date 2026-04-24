# 0031: Intrinsic drive + global evaluation

Status: Accepted
Date: 2026-04-24

## Context

v2's initial capability wishlist (MEMORY.md) listed "self-driven
triggering" and "evaluation (MDL-style, specifics TBD)" as things
the system should eventually have. Up through ADR 0030, every
mechanism was invoked by external code: the caller picked which
abstraction to run, with what parameters, and when to stop. The
system could discover patterns, axioms, and theories, but it could
not decide on its own whether any of that was worth doing.

This ADR is task C of the approved A → C → B → D sequence. It
closes the "initial wishlist" gap on self-triggering and global
evaluation, by providing:

1. A scalar `abstraction_score(&self) -> f64` on RSet that measures
   how much reusable structure the RSet currently carries.
2. An action space of discrete capabilities the system can try.
3. A `drive_step` that trials each candidate action on a clone,
   measures the score delta, and applies the best-improving one.
4. An `intrinsic_drive` loop that iterates `drive_step` until no
   action improves the score above `epsilon`.

This is a small mechanism (it orchestrates existing primitives),
but it changes the system's role from "library of abstractions" to
"agent that picks its own abstractions."

## Decision

### Metric

```
score = Σ_pattern max(0, (N − 1) × k)
      + 2.0 · Σ_theory |members|
      − 0.1 · |meta-R edges|
```

- `N` = `instances_of(p).len()`.
- `k` = `pattern_roles(p).len()` (from ADR 0029 Layer A).
- `|members|` = `theory_axioms(t).len()`.
- `meta-R edges` = edges whose either endpoint is in
  `collect_meta_ids()`.

The metric is monotonic in the things that constitute "good
abstraction" (reuse, richer theory) and taxes raw meta-R overhead
so that empty naming is punished. Weights are chosen empirically on
the rigorous battery — they produce sensible action orderings;
they're not derived from information-theoretic first principles.

### Action space

```rust
pub enum DriveAction {
    DiscoverPatterns(AutonomousConfig),
    DiscoverTheory(AxiomDiscoveryConfig),
}
```

`DriveConfig::candidate_actions()` produces one
`DiscoverPatterns` per `pattern_sizes` entry (default `[2, 3, 4]`)
plus one `DiscoverTheory`. Callers can widen the action space by
customizing the config.

### Drive loop

```rust
impl RSet {
    pub fn abstraction_score(&self) -> f64;
    pub fn drive_step(&mut self, config: &DriveConfig) -> Option<DriveStep>;
    pub fn intrinsic_drive(&mut self, config: &DriveConfig) -> DriveTrace;
}
```

`drive_step`:
1. Compute `before = self.abstraction_score()`.
2. For each candidate action, clone self, apply the action to the
   clone, compute the new score and delta.
3. Keep the best-improving action (`delta > epsilon`). If any
   improvement found, replace self with the winning clone, return
   the `DriveStep` record.
4. Otherwise return `None`.

`intrinsic_drive` calls `drive_step` up to `max_steps` times,
stopping early when a step returns `None`. Returns a `DriveTrace`
with every applied step.

### DriveConfig default

```rust
pub_sizes = vec![2, 3, 4]
discovery_config = target_size_placeholder + sample_count=200, top_m=10
refinement_config = max_tries=200
naming_policy = default (min_edges=2, min_instances=1, skip_meta=true)
axiom_config = default (max_premise=2, max_vars=3, min_evidence=1)
max_steps = 10
epsilon = 0.0
```

`target_size` in `discovery_config` is overridden per action from
`pattern_sizes`.

## Alternatives considered

- **Simulated annealing or ε-greedy exploration**. Rejected for v1
  of the drive — greedy is sufficient on the current action space
  (actions are additive on score) and adds no interpretability
  cost. Can be added later if needed.
- **Retract-and-revise in the drive loop**. Rejected. Keeps the
  mechanism minimal. Retractions are available as primitives (ADR
  0020/0030) and can be folded in by a future ADR if a real use
  case shows.
- **Bit-exact MDL metric**. Rejected. Would require priors over
  the RSet and over the abstraction space; the simplified
  additive metric here does the job and is transparent. A future
  ADR could replace the metric without touching the loop.
- **Multiple-action composition per step**. Rejected. Each step
  applies exactly one action, so the trace is readable and the
  next-step decision is made from a concrete state.

## Consequences

### Self-triggering

On four inputs of different character (mixed graph / equivalence /
strict poset / random), the drive picks action orders that reflect
the input:

| input | action 1 | action 2 | final score |
|---|---|---|---:|
| mixed graph | DiscoverPatterns(size=2) Δ=+13.0 | DiscoverTheory Δ=+1.7 | 14.7 |
| equivalence | DiscoverTheory Δ=+10.7 | DiscoverPatterns(size=4) Δ=+3.9 | 14.6 |
| strict poset | DiscoverPatterns(size=3) Δ=+5.7 | DiscoverTheory Δ=+5.3 | 11.0 |
| random | DiscoverPatterns(size=2) Δ=+2.6 | DiscoverTheory Δ=+1.7 | 4.3 |

The final score reflects "how much there is to understand" — random
sits at 4.3, mixed and equivalence at ~14.7. That's the first time
v2 has any external-visible signal of abstraction depth that's not
a hand-inspected artifact.

### Termination

By construction, the loop terminates when every candidate action
yields `delta ≤ epsilon`. Each action is idempotent (autonomous
pass skips already-named patterns; `name_theory` reuses on
member-set equality), so saturation is reached in finite steps.
The test `adr0031_drive_is_idempotent_after_saturation` verifies
that re-running the drive after it has stopped does nothing.

### Interaction with existing mechanisms

- `autonomous_pass` (0018), `autonomous_sweep` (0021),
  `autonomous_and_attach` (0022) remain as standalone APIs —
  useful when the caller knows exactly what to do.
  `intrinsic_drive` sits above them as the "don't tell me what,
  I'll figure it out" entry point.
- Score depends on ADR 0029 Layer A (uses `pattern_roles`). Legacy
  RSets without Layer A would get k=0 for every pattern, making
  reuse savings evaluate to 0 and patterns invisible to the drive.
  Acceptable — 0029 is the present default.

### Small first step, large principle

Mechanism-wise this is ~150 lines. Principle-wise it closes the
gap between "a library of abstractions" and "an agent that runs
them on itself." The v2 stance ("cognition = abstraction") already
required abstraction machinery; the drive makes the system
responsible for *applying* the machinery.

## Verification

- `cd v2 && cargo test` → 155 → 162 (7 new tests).
- `cd v2 && cargo run --example intrinsic_drive` → the four-input
  table above.
- `logs/2026-04-24_intrinsic_drive.log` — analysis.

## Limits

See the log's "Limits" section. Highlights:
- Metric weights are hand-tuned.
- Action space is fixed.
- Purely greedy, no exploration.
- No revision (retract-in-loop) in this ADR.
- Score is a proxy, not a definition of "understanding."

## Implementation

- `v2/src/lib.rs` — `abstraction_score`, `drive_step`,
  `intrinsic_drive`, plus `DriveAction`, `DriveActionResult`,
  `DriveStep`, `DriveTrace`, `DriveConfig` types.
- `v2/examples/intrinsic_drive.rs` — four-input demo.
- `v2/logs/2026-04-24_intrinsic_drive.log` — log.
- `v2/docs/decisions/0031-intrinsic-drive.md` — this ADR.
- `v2/docs/progress.md`, `v2/README.md`, decisions index.
