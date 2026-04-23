# 0026: Gradient-descent refinement probe

Status: Accepted
Date: 2026-04-23

## Context

User asked: "can we consider continuous quantization + gradient
descent?" Of the four possible meanings (edge weights, identifier
embeddings, decision relaxation, differentiable matching), decision
relaxation (**C** in the discussion) is the only one that doesn't
break v2's five commitments. It keeps R boolean, identifiers as
tokens, canonicals discrete, and similarity structural — gradient
descent acts only on the *search* process over those discrete
primitives.

ADR 0017's refinement uses targeted re-sampling: draw random walks,
accept if they improve representative cleanness. This works but
has a failure mode: when the current representative is stuck in a
tight structural neighborhood (e.g., a 2-chain embedded in a
3-cycle), re-sampling might take many attempts to escape.

ADR 0026 probes whether gradient descent over a continuous
relaxation of edge selection does better on that specific failure
mode. The probe's job is to return an honest answer — improves,
doesn't improve, or improves with caveats.

## Decision

Implement `RSet::gradient_refine_candidate` as a **probe**. Do not
change existing refinement or autonomous passes. The probe exists
alongside `refine_candidates` so comparisons can be run head-to-head
on the same input.

### Parameterization

For each data edge `i` (meta-R excluded), maintain an unconstrained
weight `w_i ∈ ℝ`. Let `p_i = sigmoid(w_i) ∈ (0, 1)` be the "selection
probability" of that edge.

### Objective

Combine two terms:

1. **Edge-count term** `(Σ p_i − k)²` — prefer exactly `k` edges
   worth of total selection (where `k = |target canonical|`).

2. **Cleanness term** `α · Σ_j (1 − p_j) · π(x_j) · π(y_j)` —
   penalize edges that *would leak cleanness* if the current
   soft-selection were rounded. Here `π(v)` is a soft-participant
   indicator for identifier `v`:
   `π(v) = sigmoid(Σ p_i − θ)` summed over edges touching `v`
   (θ a small bias, e.g. 0.5). Edge `j` leaks iff it is *not*
   selected (`1 − p_j`) but both endpoints are participants
   (`π(x_j) · π(y_j)`). Minimizing this pushes the algorithm toward
   selections whose participants induce exactly the selected edges —
   a differentiable cleanness criterion.

### Update

Analytical gradient (sigmoid derivative + chain rule through `π`).
Standard SGD step: `w ← w − η · ∇L`.

### Initialization

`w_i = +init_scale` if edge is in the candidate's current
representative; `−init_scale` otherwise. With `init_scale = 3.0`,
initial `p_i ≈ 0.95` or `0.05`.

### Rounding

After `steps` iterations, select the top-`k` edges by `p_i`. Build
a Subgraph; canonicalize; check cleanness. If both pass AND the
canonical matches the target, return the refined candidate.
Otherwise return the input unchanged.

### Scope

- Only `gradient_refine_candidate`. No gradient variant of
  `discover_motifs`, `find_instances_of`, or
  `autonomous_pass`. The probe lives alongside existing ADR 0017
  refinement as an *alternative*, not a replacement.
- Not integrated into `autonomous_pass`. Callers who want gradient
  refinement call it explicitly.

## Alternatives considered

- **Gumbel-softmax over edges.** Rejected — heavier machinery,
  adds a temperature hyperparameter, and the probe can assess the
  core hypothesis without it.
- **Replace all refinement with gradient.** Rejected as premature.
  Existing random refine works for many cases; the probe measures
  whether gradient does better *on the hard cases*.
- **Use edge embeddings or learned features.** Rejected — would
  violate commitment 5 (similarity is structural).
- **Automatic differentiation crate.** Rejected — gradient is
  simple enough to write analytically; avoids a first external
  dependency.

## Consequences

- A new primitive for refinement joins the mix. No existing
  behavior changes.
- The probe's result (improves, doesn't, mixed) will be recorded
  in the experiment log and either motivate promoting gradient
  refine to production use or close the direction.
- Gradient is deterministic given initialization (no RNG in the
  forward pass), though we may add an `init_jitter` knob if the
  initial point is a bad local minimum.

## Implementation

- Source: `v2/src/lib.rs` — `GradientRefineConfig`,
  `RSet::gradient_refine_candidate`, private helpers for sigmoid,
  participant weights, gradient computation.
- Tests: 3 new — already-clean candidate is returned unchanged,
  naive-initial embedded case may or may not escape (assertion
  accepts either outcome honestly), gradient produces valid output
  (canonical match + cleanness if it returns an improved candidate).
- Example: `v2/examples/gradient_refine.rs` — runs gradient refine
  on the canonical "2-chain inside 3-cycle" case and reports
  outcomes alongside random refine.
- Experiment log: `v2/logs/2026-04-23_gradient_refine.log` with
  honest verdict.

## Update (follow-up probes in the same session)

User pressed for more variants before concluding. Added two:

- `gradient_refine_from_uniform` — deterministic start at w = 0.
  Result on the hard case: no clean match found.
- `gradient_refine_multistart(n_starts, seed)` — N random
  initializations, first valid match wins. Result: **clean
  2-chain found at n_starts = 30+**.

**Verdict revised.** The single-start failure was initialization-
driven, not fundamental. Multi-start works with sufficient budget
(~30 random starts × 300 gradient steps on this case).

Cost comparison: multi-start cost (~9 000 gradient ops) is
significantly higher than random re-sample's (~200 walks) on this
small graph — random re-sample remains the cheaper path. Gradient
multi-start is kept as a legitimate alternative for cases where
smooth-objective search has real advantages (e.g., large graphs
where random walk hit rates are low). That hypothesis is not
demonstrated by this probe but is no longer invalidated.

Methodological note: the original "do not promote" conclusion was
premature — it stopped at the first variant. This update records
the corrected verdict.

The three gradient refinement primitives all remain in the
codebase:
- `gradient_refine_candidate` (from-rep init)
- `gradient_refine_from_uniform` (w = 0 init)
- `gradient_refine_multistart` (N random inits, first match)

Random re-sample (ADR 0017) remains the default for production
refinement pipelines.
