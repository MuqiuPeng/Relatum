# v3 Design Notes

Internal reference. Subject to revision. Authoritative spec lives in
`constitution.md`.

---

## Episode: the basic unit

A v3 sample is an **episode**, not a single labeled instance. An episode is
a sequence of anonymized states over a fixed node set, optionally
punctuated by external interventions, with derived relation signatures as
the (cached) target.

Sketch (JSON, illustrative):

```json
{
  "episode_id": "E0001",
  "nodes": ["N1", "N2", "N3", "N4"],
  "observations": [
    { "t": 0, "states": { "N1": [...], "N2": [...], "N3": [...], "N4": [...] } },
    { "t": 1, "states": { ... } }
  ],
  "interventions": [
    { "t": 1, "target": "N1", "set_state": [0.8, 0.6] }
  ],
  "relation_signatures": [
    {
      "source": "N1", "target": "N2",
      "effect_size": 0.74,
      "directionality": 0.89,
      "constraint_effect": 0.66,
      "latency": 1,
      "reversibility": 0.11,
      "stability": 0.83
    }
  ]
}
```

Node ids inside an episode are stable (v2 #4 narrowed). Across episodes,
ids carry no meaning — random renaming is the standard augmentation.

---

## Fingerprint: operational, derived (A3)

All fields are computed from observation only.

| field              | meaning                                                            |
|--------------------|--------------------------------------------------------------------|
| `effect_size`      | magnitude of target's state change conditioned on source's change  |
| `directionality`   | asymmetry of conditional dependence: `P(Δt | Δs) − P(Δs | Δt)`     |
| `constraint_effect`| reduction in target's reachable state set when source changes      |
| `latency`          | argmax-k cross-correlation lag of source change vs target change   |
| `reversibility`    | probability that undoing the source change undoes the target's     |
| `stability`        | persistence of the recovered signature across sub-episode windows  |

Each field has a fallback estimator for small-sample regimes. Recovery
must work without semantic input. None of these are stored on R as
intrinsic attributes (A3).

---

## The four-layer dataset

Built up bottom-to-top. Each layer is a separate JSONL shard.

1. **L1 — anonymous-node isomorphism.** Same structural template, many
   renamings. Target: `is_isomorphic`, `structure_similarity`. Trains the
   substrate to ignore names.
2. **L2 — first-order fingerprint.** State sequences with a single hidden
   mechanism. Target: the operational vector above.
3. **L3 — second-order similarity.** Pairs of fingerprints from
   different node sets. Target: structural similarity between
   *relations* (the de-naming of relation types).
4. **L4 — compositions.** Three or more relations forming a structure
   (chain, loop, fan-in, fan-out). Target: composed properties — feedback
   presence, propagation depth, overall stability.

Layer order matters. Skipping L1 produces a substrate that secretly
memorizes node names.

---

## Mechanism library (simulator side)

The synthetic simulator generates episodes from a small library of hidden
mechanisms. Internal names exist for the generator; they are never
exposed to the recovery side.

| code | effect                                                     | arity |
|------|------------------------------------------------------------|-------|
| A    | source change compresses target's reachable state set      | 2     |
| B    | source change propagates to target after fixed latency     | 2     |
| C    | source and target synchronize                              | 2     |
| D    | source suppresses target's change magnitude                | 2     |
| E    | source's state gates whether T1 can influence T2           | 3     |
| F    | sources jointly determine target (XOR-style)               | 3     |

E and F are n-ary and must ship with their binary projection +
irreducibility test (A4). The expectation is that A–D are binary-only,
E is irreducible (gating), F is irreducible (XOR).

---

## Training tasks

Trained jointly when possible.

- **T1. Next-state prediction.** `observations[0..t]` → `observations[t+1]`.
- **T2. Fingerprint inference.** Anonymous episode → fingerprint vector.
- **T3. Structural isomorphism.** Episode pair → similarity scalar.
- **T4. Relation clustering.** Fingerprint set → unsupervised partition.
- **T5. Composition prediction.** Two fingerprints → indirect-effect
  fingerprint.

T2 is the core; T1 supplies dense self-supervision; T3 enforces A1; T4 is
where "relation types" are allowed to emerge; T5 is the path toward
recovery of multi-hop structure.

---

## Anonymization protocol

For every training episode, generate `k` renamed variants by sampling
random bijections over `nodes`. Augmentation must preserve `episode_id`
so isomorphism evaluation can pair them.

Forbidden augmentations: any that change the temporal order, the state
trajectories, or the intervention timeline. Only the name mapping moves.

---

## Milestones

| milestone | substrate                              | intrinsic drive       | extraction quality              |
|-----------|----------------------------------------|-----------------------|---------------------------------|
| M1        | L1 + L2 (A–D only) on synthetic        | shadow only           | T2 stable above noise floor     |
| M2        | + L3 cross-template                    | shadow                | T3 isomorphism robust to rename |
| M3        | + L4 chains and small loops            | scheduling enters     | T5 indirect-effect emerges      |
| M4        | + E, F with projection / irreducibility| self-curriculum       | n-ary irreducibility detected   |
| M5        | bridge to v2 (recovered R → v2 closure)| coordinated with v2   | (out of scope for v3 alone)     |

---

## Implementation status

### M1 (2026-05-29)

- `sim::MechanismA` — state-space compression simulator.
- `Fingerprint`: 3 fields (`constraint_effect`, `effect_size`,
  `directionality`).
- A1 guard at the fingerprint level (5 seeds).
- Recovery test: forward S→T constraint_effect dominates backward T→S.

### M2 binary-mechanism completion (2026-05-29)

All four binary mechanisms shipped: `MechanismA` (state-space
compression), `MechanismB` (latency propagation), `MechanismC`
(shared-hidden synchronization), `MechanismD` (velocity suppression).

`Fingerprint` is now seven fields:

- `constraint_effect` — position spread-ratio (A signature)
- `position_effect`   — mean-shift eta-squared (C signature)
- `velocity_effect`   — delta spread-ratio (D signature)
- `latency`           — argmax cross-correlation lag (B signature)
- `reversibility`     — observational same-source / same-target
- `stability`         — episode-half signature consistency
- `effect_size` = `max(CE, PE, VE)`, `directionality` = asymmetry of
  `effect_size`.

Guards in place:

- A1 invariance covers 4 mechanisms × 3 seeds × all 8 fields
  (7 numerical via `to_bits()` + integer `latency`).
- Cross-mechanism distinguishability holds across A / B / C / D.
- Reversibility ordering recovered observationally: C > A > D.
- Stability > 0.7 holds for all stationary mechanisms.

Estimator notes / lessons:

- `position_effect`, `velocity_effect`, `constraint_effect` all share
  the same quartile-extreme binning over source; they decompose the
  signal under different distributional moments (mean, delta-variance,
  position-variance).
- `latency_lag` scans `k ∈ [0, LATENCY_MAX_LAG]` and reports `argmax`
  only when Pearson correlation exceeds `LATENCY_CORR_THRESHOLD`
  (0.25). The threshold is above the finite-sample noise floor of the
  multi-lag search; proper noise-floor (lag-shuffle baseline) is M3+.
- **Mechanism overlap is real.** Both A and D activate `CE` and `VE`
  to partially overlapping degrees: A's snap-to-center reduces velocity
  in the active bin; D's frozen-positions across runs create position
  spread asymmetry between source bins. A and D are distinguished by
  the **absolute strength** of `VE` (D ≈ 0.999, A ≈ 0.47) — not by
  which field "wins" the argmax. Future classifiers should be ranked
  by calibrated absolute magnitudes per field, not by argmax over
  fields.
- Observational reversibility is lag-blind: mechanism B reads low
  because target(t) reflects past source, not current source.
  Intervention-based reversibility is M3+ work.

### M2 L1 + T3 (2026-05-29)

Done:

- `similarity::fingerprint_similarity` — L3 primitive. Euclidean
  distance in normalised 6-d space `(CE, PE, VE, latency / LMAX, rev,
  stab)`, converted to similarity via `1 − dist / sqrt(6)`.
- `similarity::episode_similarity_2node` — T3 score. Maximises mean
  pairwise similarity over the two node bijections (identity, swap).
  2-node only; n-node generalisation is M3 work.
- `similarity::L1Pair` — dataset pair struct with `is_isomorphic` label
  and a `similarity()` method.

Guards in place:

- Renamed-variant similarity is bit-equal to `1.0` (A1 bubbles up
  through T3: bit-identical fingerprints → 0 distance).
- Distinguishability matrix is diagonal-dominant on A/B/C/D:
  `sim(i, i') > sim(i, j)` for every off-diagonal `(i, j)`.
- Positive L1 pair similarity strictly exceeds negative L1 pair
  similarity.

Lesson confirmed: even with mechanism A/D partial overlap on CE and
VE, the full 6-d Euclidean distance absorbs the absolute-magnitude
gap (A's VE ≈ 0.47 vs D's VE ≈ 0.999 contributes ~0.53 to dimensional
distance) and distinguishability holds. The "Euclidean over the full
vector" approach is the right L3/T3 primitive; no per-field argmax
classifier is needed at M2 scope.

Pending (M3 territory):

- n-node episode similarity (bijection search blows up factorially;
  use canonical-form pre-filtering and graph-matching heuristics).
- Negative-lag scan so backward latency is recoverable directly
  (currently the scan is `k ≥ 0`, so backward B reads `latency = 0`).
- Calibrated absolute magnitudes per field — for the eventual T4
  unsupervised clustering task where Euclidean alone may not suffice.
- L4 dataset: chains, loops, fan-in, fan-out (3+ nodes).
- T5 composition prediction.
- Intrinsic drive enters scheduling.

## Open questions

- How is the fingerprint dimensionality fixed? Six fields above are a
  starting set, not a closed set.
- What does "structural similarity between fingerprints" (L3 / T3) mean
  formally? Cosine on the operational vector is the obvious default; we
  should expect to need more.
- The recovery side has no access to the simulator's internal mechanism
  codes. The discovery question — can the system find structure the
  simulator did not explicitly encode — must wait for the bridge in M5.
- Continuous state vectors interact awkwardly with v2's discrete graph.
  At the bridge layer (M5), some quantization or symbolization step is
  required. Spec it in v3/M3 or later; do not pre-commit now.
