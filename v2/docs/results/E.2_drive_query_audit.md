# E.2 — Drive query meta-R full audit

**Status**: ✓ done (2026-04-29) — verification only, no code change

## Goal

Audit all `Drive::is_penalty()` compile-time call sites to confirm that runtime decision paths use the meta-R query (`is_drive_penalty_via_meta_r`) as canonical truth, with the compile-time fast path retained only where structurally necessary.

## Audit results

| Call site | Method | Verdict |
|---|---|---|
| `register_drives_in_rset` line 137 | `drive.is_penalty()` | **MUST keep** — source-of-truth being copied INTO meta-R |
| `combined_drive_signal` line 173 | `is_drive_penalty_via_meta_r` | ✓ already meta-R |
| `normalized_drive_signal` line 209 | `is_drive_penalty_via_meta_r` | ✓ already meta-R |
| `drive.rs:31/45/162` | trait definition + 2 impls | ✓ not call sites |
| `runtime/tests.rs` (4 sites) | `drive.is_penalty()` | ✓ verifies trait method itself |

## Verdict

**ALREADY POSITIVE** (from prior work in H2.1.0+ era). The runtime decision paths use meta-R queries; the compile-time fast path remains only at the registration source-of-truth point, which is structurally correct.

The original direction E.2 framed this as "needs full audit"; actual state shows the work is done. This slice converts pending → confirmed.

## Why one fast-path call site is necessary

`register_drives_in_rset` reads each drive's `is_penalty()` to decide whether to write `R(PENALTY_MARKER, drive_<id>)`. If we replaced this with `is_drive_penalty_via_meta_r`, we'd query an empty marker (since no PENALTY_MARKER edges exist before registration) → no edges ever written → the meta-R query path would always return false → bug.

This is the standard "boot circular dependency" pattern: a type's identity must come from somewhere; for predicate marker setup, it has to come from compile-time trait at boot.

## What this confirms

- `Drive::is_penalty()` and `is_drive_penalty_via_meta_r` agree on default runtime (E.1 verified)
- All decision paths use meta-R
- Compile-time fast path persists only as source-of-truth seed at registration

## Future implications

- If we add the ability to **mutate** penalty status at runtime (e.g., demote a penalty drive to non-penalty by retracting `R(PENALTY_MARKER, drive_<id>)`), the meta-R query path will pick up the change without redeploying the compile-time trait
- This is the constitutional foundation for runtime-mutable drive lifecycles (which would be needed for H2.2 drive synthesis)

## What this slice produced

1. Audit confirmation: all decision paths already use meta-R
2. Methodological note: registration is the source-of-truth seed; replacing it would break the boot
3. No code change required
