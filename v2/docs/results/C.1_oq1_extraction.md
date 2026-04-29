# C.1 — Extract OQ#1 stream to shared library

**Status**: ✓ done (2026-04-29)
**Log**: [`logs/2026-04-29_phase_c1_oq1_extraction.log`](../../logs/2026-04-29_phase_c1_oq1_extraction.log)

## Goal

Eliminate ~100 lines of `build_long_stream` and ~125 lines of `build_5k_stream` copy-paste across 17 example files. Move to a shared module under `src/test_substrates/`.

## What changed

New library subtree:
- `src/test_substrates/mod.rs` — module index
- `src/test_substrates/oq1.rs` — `build_long_stream()` (the OQ#1 4-regime, 2400-tick stream)
- `src/test_substrates/long5k.rs` — `build_5k_stream()` (the 5-regime, 5000-tick stream)

Lib declares `pub mod test_substrates;` so examples can call:
- `relatum_v2::test_substrates::oq1::build_long_stream()`
- `relatum_v2::test_substrates::long5k::build_5k_stream()`

Refactor mechanics: a small Python script (`scripts/refactor_oq1_examples.py`) handled the bulk of the work — find each example's local `fn build_long_stream() -> Vec<(u64, Event)> { ... }` block, delete it, insert the import. The script ran cleanly across 15 examples in seconds; the long5k example was edited manually because of its different fn name.

## Tests + examples

- 545 lib tests pass (no change from B.4)
- All 51 examples build (`cargo build --release --examples` clean)
- `cargo fix` cleaned up unused imports automatically; no behavior change

## Counts

| | before | after |
|---|---|---|
| examples with local `build_long_stream` | 16 | **0** |
| examples with local `build_5k_stream` | 1 | **0** |
| total stream-fn copies | 17 | **2** (in lib) |
| net lines removed (approx) | — | **~1,300** |

## Verdict

**POSITIVE**. Pure cleanup. No empirical claim, but enables:
- Future stream additions (e.g., OQ#2, varying-density variants) in one place
- B.5 (runtime integration) and B.6 (family of families) can use shared substrates without copy-paste
- Easier to compare results across phases — no risk of one example having stale stream code

## What this slice produced

1. `src/test_substrates/{mod,oq1,long5k}.rs` — single source of truth for shared streams
2. 16+1 example refactors (all build clean)
3. `scripts/refactor_oq1_examples.py` — reusable for future bulk refactors
4. ~1300 lines net deletion from the codebase

## Notes

- Used `cargo fix --release --examples --allow-dirty` to clean up unused imports after the refactor (some examples no longer need `PATTERN_MARKER`, `Event`, etc. since the stream constructor is gone)
- One side effect: `cargo fix` also cleaned up some unused imports in the runtime submodules from earlier refactors (Phase 0067), making those files slightly cleaner too
