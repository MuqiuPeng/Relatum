# B.5.1 — Scheduler picks DiscoverAxiomShapeFamilies autonomously

**Status**: ✓ done (2026-04-29)
**Log**: [`logs/2026-04-29_phase_b51_scheduler_shape_family.log`](../../logs/2026-04-29_phase_b51_scheduler_shape_family.log)
**Example**: [`examples/phase_b51_scheduler_shape_family.rs`](../../examples/phase_b51_scheduler_shape_family.rs)

## Goal

B.5 added `ActionKind::DiscoverAxiomShapeFamilies` and the `execute_action` arm. B.5.1 closes the autonomous loop: scheduler should *select* this action when there's structural-family discovery work pending, without external dispatch.

## Implementation

1. New `FrontierKind::ShapeFamilyDiscoveryCandidate`
2. New `Frontier::refresh_shape_family_candidates` — surfaces the candidate when:
   - ≥ 2 registered template axioms share a canonicalized premise
   - That premise's `shape_premise_<...>` family doesn't yet exist
3. `RuleBasedScheduler::execute_for_kind` routes the new FrontierKind → `ActionKind::DiscoverAxiomShapeFamilies`
4. Expand mode's `pick_top_biased` accepts `ShapeFamilyDiscoveryCandidate` items
5. Frontier refresh in `AutonomousRuntime` calls `refresh_shape_family_candidates` after composites

## Result on OQ#1 @ 1000 ticks

- 16 axioms registered (vs 13 in earlier runs)
- 5 theories (vs 4)
- **6 shape families discovered**
- **DiscoverAxiomShapeFamilies episodes fired: 1**

The scheduler autonomously dispatched the action exactly once in 1000 ticks (idempotent: after first dispatch mints families, the freshness check sees no work to do until new axioms appear).

## Behavioral change observation

The axiom/theory counts differ from prior runs (16/5 vs 13/4). Reason: adding a new ActionKind to the scheduler's choice set redirects the trajectory — the runtime spent some tick budget on family discovery instead of other actions, and the resulting episode mix changed which patterns/theories got discovered.

This is **direct evidence the runtime integration is real**, not just structural plumbing. Family discovery is now a participant in the autonomous decision process.

## Verdict

**POSITIVE**. The scheduler autonomously discovers shape families. Closes the loop opened by Beta-1 (which was external-call-only).

## What this slice produced

1. New FrontierKind variant + scheduler routing
2. `Frontier::refresh_shape_family_candidates` — cheap structural surface check
3. Expand mode accepts the new candidate kind
4. Empirical confirmation: 1 autonomous dispatch on OQ#1 @ 1000 ticks → 6 families discovered
5. Behavioral evidence: trajectory changed (16 axioms / 5 theories vs prior 13/4)

## Future implications

- Future Beta-X experiments don't need to manually dispatch family discovery
- Long-run experiments will see family discovery happen at natural points in the loop
- The runtime now has 9 ActionKinds in its catalogue (was 8); each one autonomously selectable
