# E.3 — Drive synthesis sketch (H2.2)

**Status**: ✓ done (sketch only, no code)
**Format**: Architectural sketch document, supplements ADR 0063 H2.2.

## Goal

ADR 0063 sketched H2.2 — runtime composition of new drives from primitive metrics. E.3 produces a more concrete design before implementation: identify the primitive metric inventory, define the composition grammar, scope the synthesizer's search strategy, and outline the lifecycle.

## Where this fits

| level | what it does | status |
|---|---|---|
| **H2.0** (shipped) | A/B-test fixed weight blend over compile-time drives | ✓ landed |
| **H2.1** (proposed) | Drives as meta-R objects (`DRIVE_MARKER`); ESTABLISHED-promotion | ✓ Phase H2.1.0 registration shipped (E.1 verified); promotion deferred |
| **H2.2** (research) | Synthesize NEW drive function bodies from primitive metrics | not started |

E.3 = sketch for H2.2 specifically.

## Primitive metric inventory

What scalars does v2 already expose per tick? These are the "atoms" a synthesized drive can compose from:

| metric | shape | provenance |
|---|---|---|
| `compression_delta` | f64 | ADR 0031 abstraction_score delta |
| `prediction_error_delta` | f64 | ADR 0059 G1.5 hit-rate change |
| `mode_thrash_count` | u64 | mode transitions in window |
| `axiom_count` | usize | rset.axioms().len() |
| `theory_count` | usize | rset.theories().len() |
| `pattern_count` | usize | rset.patterns().len() |
| `family_count` | usize | rset.axiom_shape_families().len() (Beta-1+) |
| `episode_ep_delta` | f64 | last episode's post-EP-delta |
| `axiom_hit_rate_max` | f64 | best axiom hit rate, gated by sample floor |
| `axiom_hit_rate_mean` | f64 | mean across registered axioms |
| `cross_precision_max` | f64 | best column mean (Alpha-7+) |
| `tick` | u64 | global tick counter |

## Composition grammar

```
metric := primitive
        | mean(metric, window)
        | variance(metric, window)
        | ratio(metric, metric)
        | lag_diff(metric, k)
        | clamped(metric, lo, hi)
        | scaled(metric, factor)
```

Window operations require N-tick history (memory.episodes) per ADR 0063 H2.0 precedent.

`ratio` and `lag_diff` are second-order: capture *change in dynamics*, not static values.

## Synthesizer design

### Search strategy: bounded-depth proposal + score

Per v2 search-mode (propose/score/refine, NOT exhaustive enumeration):

1. **Propose**: at depth k = 1, all primitive metrics. At depth k = 2, all primitive ⊕ primitive via grammar combinators. Cap k ≤ 3 (per ADR 0063 OQ #2).
2. **Score**: each candidate gets registered under `DRIVE_MARKER` with weight 0 (passive). Run for an evaluation window. Track correlation with overall EP delta improvement.
3. **Refine**: candidates with positive correlation get non-zero weight (promoted to H2.1's ESTABLISHED status). Candidates with zero or negative correlation get retracted.

### Identifier naming (commitment 4)

Each synthesized drive needs a deterministic id:

```
drive_synth_<hash_of_canonical_expression>
```

Where canonical expression is the grammar tree's string form:
```
"variance(prediction_error_delta, 50)" → drive_synth_<sha-prefix>
```

Hash is the first 8 hex chars of SHA-1 over the canonical string. Deterministic across runs.

### Search ceiling

- Depth ≤ 3
- ≤ 20 candidates active at any time (oldest unestablished gets retracted)
- Re-synthesize when no established drive has produced positive contribution in N evaluation windows (proxy: stagnation)

## Example synthesized candidates (3 plausible)

### Candidate A — "stability of improvement"
```
expression: -variance(episode_ep_delta, 50)
intent: reward consistency over volatility
```

Predicted behavior: high when EP delta is consistently positive but small. Low when EP delta is volatile (some big wins, some big losses). Encourages the runtime to prefer steady accumulation over thrashy exploration.

### Candidate B — "growth saturation gradient"
```
expression: lag_diff(family_count, 100)
intent: reward periods when the family count is still growing; decay to 0 when saturated
```

Predicted behavior: high during early discovery; saturates (signal → 0) once shape families plateau. Could trigger creation drives (G.5) at saturation.

### Candidate C — "axiom efficiency ratio"
```
expression: ratio(axiom_hit_rate_mean, axiom_count)
intent: reward small axiom sets with high mean precision over large sets with mediocre precision
```

Predicted behavior: high when 3 axioms each at 0.9; low when 30 axioms each at 0.4. Pulls the runtime toward parsimony.

## Lifecycle (reusing H2.1's machinery)

1. **Synthesis pass**: every K ticks (e.g., 200), the synthesizer proposes 1-3 new candidates from the grammar (gradient descent over recent stagnation signals)
2. **Registration**: candidate registered as `R(DRIVE_MARKER, drive_synth_<hash>)` with weight 0
3. **Evaluation**: passive observation for N windows (~3 × 50 episodes)
4. **Promotion**: if window-mean correlation with EP delta > threshold, promote weight to non-zero (ESTABLISHED)
5. **Demotion**: if no positive contribution in M windows, retract entirely (frees the catalogue slot)

Reuses ADR 0033 defeasibility + ADR 0053 ESTABLISHED-promotion already shipped for patterns. Constitutionally consistent.

## Constitutional review

Each commitment, scored against H2.2:

1. **C1 (R singular)**: synthesized drives register under existing `DRIVE_MARKER`. No new R class. ✓
2. **C2 (R binary)**: registration edge is 2-arity. Composition grammar is computational, not relational. ✓
3. **C3 (types as meta-R)**: synthesized drive ids are `R(DRIVE_MARKER, drive_synth_<hash>)` — same shape as patterns/theories. ✓
4. **C4 (token identity)**: the hash-based naming is deterministic. Same expression tree → same hash → same id. ✓
5. **C5 (similarity is structural)**: drive comparison is via numeric scores, not structure. Sidesteps the question. ✓ (vacuous compliance)

**Highest risk**: C4. The hash function must be stable across rust versions, machine architectures, etc. Use a deterministic hash (SHA-1 / SHA-256, not std `Hasher` which is randomized per-run for security).

## Risks (5 named)

### Risk 1: synthesis explosion

Depth-3 grammar over 12 primitives × 6 combinators = 12 × 6³ × 12² ≈ 38,000 candidates. Even with depth 2 it's 12 × 6 × 12 ≈ 864. Need aggressive pruning.

**Mitigation**: only propose 1-3 NEW candidates per synthesis pass. Most candidates never get evaluated. Pruning happens implicitly — only "interesting" branches get explored, biased by recent signal trajectories.

### Risk 2: synthesized drive masks problems

A weird drive might score positively on EP delta but for the WRONG reason (correlation, not causation). The runtime promotes it; the underlying issue stays masked.

**Mitigation**: H2.1's lifecycle requires SUSTAINED positive contribution (multiple windows). Ephemeral correlations get filtered out. Long-run validation required before commit.

### Risk 3: combinatorial drive interaction

If 3 synthesized drives all promote and interact, the runtime's behavior is hard to predict from any individual drive's spec.

**Mitigation**: cap simultaneously-active synthesized drives (e.g., ≤ 2). Any 3rd promotion retracts the lowest-scored.

### Risk 4: hash collision

Two different expressions hash to the same id. Unlikely but possible.

**Mitigation**: use 8-byte (16 hex char) hash prefix. Probability of collision in 1000 candidates is ~10⁻¹². Negligible.

### Risk 5: orthogonality with H2.0/H2.1

H2.0's MetaScheduler tunes weight ratios. H2.1's ESTABLISHED-promotion governs activation. H2.2 introduces yet another loop (synthesis). Three nested A/B-like loops create coupling.

**Mitigation**: phase-shifted windows (per ADR 0063 OQ #5 already partial-resolved). H2.2 synthesis pass at multiples of 200 ticks; H2.1 evaluation at 50 episodes; H2.0 at 50 episodes. Different cadences reduce interaction.

## What this slice produced

1. Grammar for drive composition (8 combinators on top of 12 primitive metrics)
2. Bounded-depth propose-score-refine search strategy
3. 3 plausible synthesized candidates with predicted behaviors
4. Lifecycle reusing H2.1's ESTABLISHED-promotion machinery
5. Constitutional review (5 commitments, all PASS or vacuous)
6. 5 named risks with mitigations
7. Complementary specification to ADR 0063's H2.2 sketch

## Future implications

- **H2.1 promotion lifecycle**: H2.2 depends on H2.1 being shipped. H2.1's drive promotion mechanism is the lifecycle H2.2 plugs into.
- **First synthesized drive in production**: would be a notable v2 milestone — the runtime authors a new evaluation standard the implementation never wrote
- **Convergence with G.5**: G.5's `structural_growth_drive` could itself be a synthesized drive. If H2.2 synthesizes something semantically equivalent (e.g., `lag_diff(family_count, 100)` → growth saturation), G.5 retroactively becomes a hand-coded version of an emergent drive
- **Constitutional progress**: H2.2 is the slice that makes commitment 3 (types as meta-R) hold for *evaluation* axes the same way H1's action sequence promotion already made it hold for *operational* axes

## Honest assessment

H2.2 is the most speculative direction in v2. It's where the system actually starts authoring its own evaluation criteria. Risks are real (synthesis explosion, masking, coupling) and the empirical payoff is unclear before the run.

Recommended: ship H2.1 fully first (ESTABLISHED-promotion of compile-time drives), gather empirical evidence about what drive characteristics correlate with productive runs, THEN attempt H2.2 with that data informing the synthesizer's biases.

E.3 is the design map; the implementation work is years-of-research-or-1-careful-PhD-thesis territory, not a 1-slice landing.
