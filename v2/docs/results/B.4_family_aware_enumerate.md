# B.4 — Family-aware template enumeration

**Status**: ✓ done (2026-04-29)
**Log**: [`logs/2026-04-29_phase_beta_4_filtered_enumerate.log`](../../logs/2026-04-29_phase_beta_4_filtered_enumerate.log)

## Goal

Once Beta-1 has surfaced a uniform-low-cross-precision premise family (variance near zero), future template discovery should not waste cycles re-finding axioms with that premise — they will all be noise variants of the same conclusion shape.

This is the **first feedback loop** between Beta-1's structural discoveries and runtime mechanism: a family discovered as noise can suppress its own re-discovery.

## Implementation

Two new APIs, purely additive (existing `enumerate_axiom_templates` unchanged):

1. `pub fn enumerate_axiom_templates_filtered(config, blocked_premise_keys: &HashSet<Vec<(usize, usize)>>) -> Vec<AxiomTemplate>`
   - Returns same templates as baseline minus those whose premise key matches `blocked_premise_keys`
   - With empty blocked set, behaves identically to baseline (verified by unit test)

2. `RSet::shape_premise_key(shape_id: &str) -> Option<Vec<(usize, usize)>>`
   - Parses a `shape_premise_<...>` family id back to its canonical premise key
   - Returns `None` for non-premise families (e.g., `shape_conclusion_*`)
   - Round-trip verified via unit test

The intended pipeline:
1. Discover families via `discover_axiom_shape_families(2)`
2. Compute per-family cross-precision profile (mean + variance)
3. For families with low mean + variance < ε, extract their premise keys via `shape_premise_key`
4. Pass the keys to `enumerate_axiom_templates_filtered` for the next discovery cycle

## Tests

3 new unit tests, 545 lib tests pass:
- `adr0068_b4_filtered_enumerate_skips_blocked_premise`: filtered count strictly less than baseline; no template in filtered has the blocked premise
- `adr0068_b4_filtered_with_empty_blocked_equals_baseline`: empty blocked set is identity
- `adr0068_b4_shape_premise_key_round_trip`: `shape_premise_p0-0_p1-2` ↔ `[(0,0), (1,2)]` is bijective for premise families; conclusion families return None

## Verdict

**POSITIVE on mechanism**. The infrastructure for family-aware enumeration is in place.

No standalone runtime experiment in this slice — the empirical impact is conditional on future use (B.5 integrates this into the autonomous loop, where it would actually reduce discovery cycles). Standalone count-comparison would just verify what unit tests already verify.

## Future implications

- B.5 (runtime integration) should call `enumerate_axiom_templates_filtered` with blocked keys derived from existing low-quality shape families
- The family abstraction now has THREE runtime functions:
  - Inert observation (Beta-1 baseline)
  - Demote-driver (Beta-2)
  - Discovery-bias (B.4)
- Each unlock further compounds the value of the structural vocabulary

## What this slice produced

1. `enumerate_axiom_templates_filtered` API (free function, pub)
2. `RSet::shape_premise_key` parser
3. 3 new unit tests; 545 lib tests pass
4. Pipeline design for using family discoveries as discovery bias
