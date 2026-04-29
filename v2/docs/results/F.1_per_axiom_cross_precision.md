# F.1 — Per-axiom cross-precision API

**Status**: ✓ done (2026-04-29)
**Log**: [`logs/2026-04-29_phase_f1_axiom_cross_precision.log`](../../logs/2026-04-29_phase_f1_axiom_cross_precision.log)

## Goal

Beta-1's `phase_beta_1_shape_families.rs` example computed `axiom_mean_cross_precision` internally as a helper. F.1 promotes it to a public RSet method for direct use by future slices (D.X, B.X, F.X) without reimplementing.

## Implementation

New API:
```rust
pub fn axiom_cross_precision(
    &self,
    axiom_id: &str,
    substrates: &[RSet],
) -> Option<f64>
```

For each substrate where the axiom produces non-empty predictions, compute precision = |predicted ∩ actual| / |predicted|, then return the mean. `None` if no substrate produces predictions.

2 unit tests: positive case (transitivity on saturated poset → 1.0), edge case (empty substrates → None).

## Bonus fix discovered

While verifying F.1, the full lib test suite caught a regression in `runtime::tests::a_verification_8_case_battery_matches_direct_discovery` — the equivalence_3_classes case suddenly produced 2 theories instead of 1.

**Root cause**: `SHAPE_FAMILY_MARKER` (Beta-1) and `META_SHAPE_FAMILY_MARKER` (B.6) weren't added to `RSet::collect_meta_ids`. After B.5.1 made the runtime autonomously dispatch shape-family discovery, the new meta-R edges accidentally counted as "data" identifiers in `compute_data_ids`, perturbing axiom validity calculations.

**Fix**: extend `collect_meta_ids` to include `SHAPE_FAMILY_MARKER`, `META_SHAPE_FAMILY_MARKER`, and all their `left_of` ids. Now shape-family registrations don't pollute data-axiom evaluation. 552 lib tests pass.

This bug had been latent since Beta-1; it only surfaced now because B.5.1 made the runtime itself trigger family discovery, putting shape edges in rsets that previously would have been used directly by tests.

## Verdict

**POSITIVE on mechanism, latent-bug-fix BONUS**. The API ships clean. The companion fix to `collect_meta_ids` is small (12 lines) but architecturally important — keeps Beta-1's meta-R class isolated from data-identifier accounting.

## What this slice produced

1. New `RSet::axiom_cross_precision` API (single-axiom version of Alpha-7's column means)
2. 2 unit tests
3. **Latent-bug fix**: `collect_meta_ids` now includes shape family markers
4. 552 lib tests pass

## Future implications

- B.7, F.2, F.3 etc can call `axiom_cross_precision` directly without redoing the substrate-+-precision plumbing
- The `collect_meta_ids` fix means future markers must be added there or face the same data-pollution bug; checklist for new meta-R classes
