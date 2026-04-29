# G.5 — Drive for identifier creation (design)

**Status**: ✓ done (design only, no runtime integration)
**Format**: Design document; no example or log file. Code integration deferred to a future "G.5 implementation" slice.

## Goal

Currently no Drive triggers G-series generation. G.1-G.7 work mechanically but the runtime never *invokes* them. Without a triggering drive, generative axioms are inert. G.5 designs the surface.

## Why this is harder than it looks

Naive: "drive that wants minting" → rewards creating identifiers → unbounded minting → rset bloat.

The existing drive `compression` actively penalizes rset growth. So a "creation drive" sits in tension with compression. The right question is not "create vs not create" but **"under what conditions does creation outweigh compression's cost?"**

## Proposed design

### Drive name: `structural_growth_drive`

### Signal

Rather than reward creation per se, reward creation that **closes prediction gaps**. Counterfactual:

```
signal(τ) = max(0, prediction_error(τ) − prediction_error_estimated_after_minting(τ + Δ))
```

Where `prediction_error_estimated_after_minting` is a bounded look-ahead: imagine minting K new ids via the available generative axioms; estimate prediction_error on the resulting rset.

### Saturation

If `prediction_error < EPS` (system already predicts data well), drive returns 0. No creation pressure when nothing to explain.

### Anti-rapacity

Hard cap: at most K_MAX (default 10) new ids per dispatch. Even if the drive saturates positive, the action layer mints in bounded chunks. Idle time elapses between dispatches; the runtime can re-evaluate.

### Composition with existing drives

`structural_growth_drive` joins the standard drive suite alongside `compression`, `prediction_error`, etc. The composite scheduler signal arbitrates:

- High prediction_error + low compression saturation → creation favored
- Low prediction_error → creation suppressed (compression wins)
- High mode_thrash → creation deferred (system instability)

This pattern is the same DriveMix model used elsewhere — the new drive is just one term.

## Integration sketch

1. Register drive: `R(DRIVE_MARKER, "drive_structural_growth")` (per ADR 0064)
2. Drive evaluator function: computes counterfactual signal per tick
3. Frontier item: `FrontierKind::GenerativeAxiomCandidate` (proposes a recipe + seed pair)
4. ActionKind: `ActionKind::ApplyGenerativeRule` (executes K mints; see G.8)
5. Scheduler routes high-signal frontier items to action when arbitration favors creation

## Risks

### Risk 1: counterfactual look-ahead is expensive

`prediction_error_estimated_after_minting` requires a hypothetical rset state. Could be a forward-pass simulation, costly per tick.

**Mitigation**: amortize by sampling. Don't compute every tick; compute every K ticks. Cache result.

### Risk 2: drive becomes rapacious if the recipe space is unbounded

If many generative recipes are available, the drive could keep minting indefinitely with marginal gain.

**Mitigation**: K_MAX cap + diminishing-returns curve — signal returns proportional to log(num_existing_minted_ids).

### Risk 3: collision with compression's intent

Creation always adds rset entries; compression penalizes additions. Pure tension.

**Mitigation**: this is BY DESIGN. The drive composite resolves the tension via arbitration; if compression's penalty exceeds creation's reward, no minting happens. The system finds the equilibrium where new ids justify their compression cost.

## Why this slice is design-only

Implementation requires:
- Lib changes for new drive type
- Scheduler integration (analog to B.5.1)
- New ActionKind + FrontierKind (G.8 territory)
- Counterfactual evaluation infrastructure
- Tests for stability (no unbounded minting under any drive composition)

Each is its own slice. G.5 specifies the surface; implementation is G.5.1 / G.8 follow-ups.

## Comparison with existing drives

| drive | reward signal | pressure | works with G-series? |
|---|---|---|---|
| compression | smaller rset | shrinks | NO (active conflict) |
| prediction_error | better predictions | improvement | indirect (via predicted-edge gain) |
| mode_thrash | stability | cooling | mode-orthogonal |
| **structural_growth** | counterfactual prediction-error reduction via mints | growth | YES (the missing piece) |

The "missing piece" framing is correct: G-series generative recipes are mechanically working; they need exactly this drive class to be runtime-driven.

## What this slice produced

1. Design document for `structural_growth_drive`
2. Counterfactual signal specification (`prediction_error - estimated_after_minting`)
3. Saturation + anti-rapacity guards
4. Integration sketch with existing drive suite
5. Risk analysis (3 named risks + mitigations)

## Future implications

- **G.5 implementation**: ship the drive. Probably a single-tick experiment first (mint K=3 ids, observe prediction-error delta), then iterate on saturation curves
- **G.8 (ActionKind)**: depends on G.5 having a frontier-item / action-routing surface
- **Drive interaction analysis**: once shipped, analyze what fraction of dispatches choose creation vs other actions on OQ#1 / long5k
- **The autonomy claim**: until this drive ships, G-series cannot honestly be called "autonomous extension". G.5's implementation is the bridge.
