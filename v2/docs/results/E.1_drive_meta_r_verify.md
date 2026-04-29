# E.1 — Verify H2.1.0 drive-as-meta-R registration

**Status**: ✓ done (2026-04-29)
**Log**: [`logs/2026-04-29_phase_e1_drive_meta_r_verify.log`](../../logs/2026-04-29_phase_e1_drive_meta_r_verify.log)
**Example**: [`examples/phase_e1_drive_meta_r_verify.rs`](../../examples/phase_e1_drive_meta_r_verify.rs)

## Goal

H2.1.0 (ADR 0064) was supposedly already done — `register_drives_in_rset` adds `R(DRIVE_MARKER, drive_<id>)` and `R(PENALTY_MARKER, drive_<id>)` for penalty drives. E.1 is a defensive verification that:
1. Registration actually fires from `AutonomousRuntime::new`
2. The expected drives appear under `DRIVE_MARKER`
3. The expected drives appear under `PENALTY_MARKER`
4. EP path still fires (H2 known risk: drive changes can break EP)
5. `Drive::is_penalty()` matches the meta-R query (canonical truth check)

## Result on default runtime + OQ#1 stream

| check | result |
|---|---|
| 1. Drives registered (count=3) | ✓ {compression, prediction_error, mode_thrash} |
| 2. Penalty drives (count=1) | ✓ {mode_thrash} |
| 3. EP path runs (200 ticks → 14 EP episodes) | ✓ |
| 4. trait/meta-R consistency (3/3 drives) | ✓ |

All four checks passed → **POSITIVE**.

## API gotcha encountered + fixed

First-attempt example used `right_of(DRIVE_MARKER)` to enumerate drives. Returned 0 entries. **Reason**: `R(DRIVE_MARKER, drive_X)` has DRIVE_MARKER on the LEFT, so `left_of(DRIVE_MARKER)` is the correct query. `right_of` would only return edges where DRIVE_MARKER is on the right (none).

Fixed in same slice. Worth noting in any future "query drive catalogue" code: use `left_of(DRIVE_MARKER)`.

## What this slice produced

1. Verification example confirming H2.1.0 is intact and consistent
2. API-usage note: `left_of(MARKER)` is the right query for `R(MARKER, ?)` patterns
3. Empirical confirmation that meta-R query path is consistent with `Drive::is_penalty()` compile-time fast path
4. EP path fires after drive registration (no shadow-mode breakage)

## Confirms what already shipped

- `combined_drive_signal` already reads penalty status from meta-R (`is_drive_penalty_via_meta_r`) as the canonical source — Direction E.2 (replacing fast paths) is therefore already partially shipped
- The compile-time `Drive::is_penalty()` is consistent with meta-R queries; either source is currently equivalent

## Future implications

- Future Beta-X could extend the drive registry meta-R structure: e.g., `R(drive_X, weight_string_repr)` for round-tripped weights
- `DRIVE_MARKER` being a queryable meta-R class enables drive-discovery analogous to Beta-1's shape-family discovery (E.3 / H2.2 territory)
- The verification pattern (4 checks) can be reused for future meta-R class additions
