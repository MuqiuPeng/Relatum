# B.5 — Runtime integration of shape-family discovery

**Status**: ✓ done (2026-04-29)
**Log**: [`logs/2026-04-29_phase_beta_5_runtime_family.log`](../../logs/2026-04-29_phase_beta_5_runtime_family.log)
**Example**: [`examples/phase_beta_5_runtime_family.rs`](../../examples/phase_beta_5_runtime_family.rs)

## Goal

Wire `RSet::discover_axiom_shape_families` into the runtime as a first-class `ActionKind`, so it can be dispatched as part of the autonomous loop (vs Beta-1..4 where the API was invoked from external examples only).

## Implementation

1. Added `ActionKind::DiscoverAxiomShapeFamilies` enum variant in `src/runtime/action.rs`
2. Updated `action_kind_to_str` and `parse_action_kind` (B2-format checkpoint round-trip)
3. Added match arm in `AutonomousRuntime::execute_action`:
   ```rust
   ActionKind::DiscoverAxiomShapeFamilies => {
       let minted = self.rset.discover_axiom_shape_families(2);
       return Some(minted.len() as f64);
   }
   ```
   Episode delta = count of newly minted families
4. Made `execute_action` `pub` (was `pub(crate)`) so examples can dispatch directly

## Result

Demo example dispatches the new action manually after Phase 0 (1000 ticks):

| dispatch | delta | families total | verdict |
|---|---|---|---|
| 1 | 6 | 6 | new families minted |
| 2 | 0 | 6 | idempotent ✓ |

The 6 families minted by runtime dispatch match Beta-1's external-call result exactly:
- `shape_premise_p0-0_p1-2` (4 members) — variance-zero noise
- `shape_premise_p0-1` (3)
- `shape_premise_p0-1_p1-2` (2)
- `shape_conclusion_c0-2` (3)
- `shape_conclusion_c1-0` (2)
- `shape_conclusion_c2-0` (2)

## Verdict

**POSITIVE**. The shape family discovery is now a runtime action. Four properties confirmed:
1. Dispatched via `ActionPlan { kind: DiscoverAxiomShapeFamilies, ... }`
2. Episode delta correctly reports newly minted family count
3. Idempotent on re-dispatch
4. Round-trip through B2 checkpoint format works (str/parse helpers updated)

## What this slice does NOT do

- **No scheduler integration**: this is "wiring only". The default scheduler (`RuleBasedScheduler`) does not yet pick `DiscoverAxiomShapeFamilies` autonomously. A future B.5.1 would add it as a frontier item kind.
- **No drive feedback**: episode delta from this action contributes to abstraction-score deltas, but no drive currently uses family count as a quality signal.
- **No periodic re-dispatch logic**: idempotent on re-dispatch is correct, but the runtime won't naturally trigger this action — it has to be called externally for now.

## Future implications

- B.5.1: scheduler frontier item `ShapeFamilyDiscoveryCandidate` that fires when registered axiom count grows beyond the count at last dispatch
- Drive layer could use family count as a "structural complexity" signal — distinct from abstraction-score
- Family discovery can now be checkpointed in episodes (B2 round-trip works for the new ActionKind)

## What this slice produced

1. New ActionKind variant + str/parse helpers + execute_action arm
2. `AutonomousRuntime::execute_action` made `pub` for external dispatch
3. Demo example with 4-property verdict (mint, idempotent, count, query)
4. 545 lib tests still pass; 51 examples + new B.5 demo all build
