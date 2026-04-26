# v2 retrospective — 2026-04-27 (late)

Twelve hours after the 2026-04-27 morning retrospective.
That retrospective named four next directions in priority
order:

1. Long-run empirical cycle (HORIZON ≥ 2000 on richer
   streaming substrate).
2. Triple demotion (small slice mirroring H1.3 for
   triples).
3. Recent-window stats checkpoint persistence.
4. H2 ADR drafting.

Three of the four shipped today. (#3 was deferred — long-
run empirics didn't surface it as load-bearing yet.) Plus
two empirical findings discovered along the way that
turned into shipped fixes / extensions.

## What landed in the past 12 hours

| # | ADR | What it added |
|---|---|---|
| H1.x retro #1 | 0062 | Long-run empirics (HORIZON=2000) — captured `phase_h1_long_run.rs` over 4-regime substrate |
| H1.x retro #2 | 0062 | Triple demotion — `triple_recent_post_ep_count` / `triple_recent_post_ep_delta_sum` mirrors; demotion sweep extended to triples |
| H1.x retro #3 | 0062 | Composite dispatch EP gap fix — `EvaluatePredictions` synthesized as always-present in eligibility; `ExecuteComposite` arm dispatches EP via `WholeRSet` |
| H1.x retro (post #3) | 0062 | Regime B/C inert closed without code (downstream of #3) |
| H2 spec | 0063 | Drive self-modification ADR (Proposed → Accepted) — three sub-slices defined; constitutional review of all five v2 commitments |
| H2.0 step 1 | 0063 | `Drive` trait + 3 baseline impls (`CompressionDrive`, `PredictionErrorDrive`, `ModeThrashPenalty`) — shadow-only |
| H2.0 step 2 | 0063 | `DriveMix` struct + A/B mutation + checkpoint round-trip — shadow-only |

487 unit/integration tests pass (was 466 at start of day).
Two new ADRs (0063 + 0062 retro additions). +21 tests.

## The bigger move: a load-bearing finding turned into a fix

The long-run #2 finding was the most consequential moment.
On its surface it looked minor: "composite dispatch fired
0 times." Diagnosis revealed an architectural gap —
`EvaluatePredictions` is dispatched outside the frontier,
so no `FrontierKind` mapped to it, so the
`refresh_composite_candidates` eligibility gate excluded
EP-containing pairs (which were the only ones that could
get promoted under stream-shaped substrates).

Pre-fix vs post-fix on the same 2000-tick substrate:

| metric | pre-fix | post-fix |
|---|---|---|
| episodes | 49 | 268 |
| EP attempts | 23 | 129 |
| composite attempts | 0 | 1 |
| pairs currently named | 1 | 4 |
| triples currently named | 1 | 8 |
| live demotion events | 0 | 2 |

The fix was 6 lines of code. The empirical change is
qualitative: post-fix, the runtime discovers
`(Decl, Decl, Decl)`, `(PruneLow, EP, PruneLow)`,
`(Decl, Decl, EvaluatePredictions)` and 5 other diverse
sequences that pre-fix it could never reach.

Long-run finding #3 (regime B/C inert) dissolved into
finding #2 — there was never a separate wake-gate problem;
the runtime woke fine on regime-B/C edges, but with no
productive composite dispatch available it returned to
sleep before the next snapshot. The diagnostic value of
2000-tick horizons over 300-tick batteries: longer windows
expose secondary consequences of architectural defects that
short windows mask.

## H2 entered implementation tracks

H2 was deliberately deferred to "research direction" in
ADR 0060 / Phase H. The morning retrospective said:

> H2 has more potential for getting wrong than any prior
> phase.

So the implementation strategy is *phased*: ADR 0063
specs three sub-slices (H2.0 / H2.1 / H2.2). H2.0 itself
is split into three steps (trait + impls / DriveMix / wake-
gate refactor). Step 1 + step 2 shipped today. Step 3 is
the load-bearing integration; gated on whatever step 2
empirics show.

#### Step 2 empirics (long-run, captured today)

Over 2000 ticks the DriveMix layer:

- Cycled A/B 5 times (5 windows × 50 EP episodes).
- Mutated the loser at each window boundary:
  - `candidate_a.mode_thrash`: 0.10 → 0.125 (×1.25)
  - `candidate_b.compression`: 0.50 → 0.40 (×0.8)
- Stayed within [0, 1] bounds; all mutations clamped
  cleanly.
- **Did not perturb runtime behaviour**: episode count,
  theories, named sequences all byte-identical to the
  pre-step-2 post-fix run. Shadow-only property holds.

This is the empirical green light for H2.0 step 3 (wake-
gate refactor). The mutation loop is responsive but not
load-bearing yet; switching the gate to read from
`active_weights()` is the minimum further code change to
make DriveMix actually decide things.

## The constitutional question revisited

The morning retrospective named one constitutional concern:

> `ACTION_SEQ_MARKER` introduces a second-order operational
> meta-R class. Is that commitment-compatible?

Today's H2.0 work raised the analogous question for drives:
when (and how) do drives become first-class meta-R objects
(commitment 3)?

ADR 0063's answer: H2.0 keeps drives as compile-time Rust
constructs (`Drive` trait + struct impls + ids). The
DriveMix `candidate_a.compression = 0.5` is a Rust
HashMap entry, not an R fact. **No new meta-R class
introduced at H2.0**.

H2.1 (sketched, not yet implemented) is where drives
become `R(DRIVE_MARKER, drive_compression)`. The
constitutional shape is identical to PATTERN_MARKER /
THEORY_MARKER chains — a clean extension, not a drift.

Today's commits stay strictly within the H2.0 boundary.
Commitments 1-5 PASS by construction.

## What still doesn't exist (12-hour-newer view)

Most items from the morning retrospective remain open.
Some now have sharper specifications:

- **Self-extending action atoms** (vs. compositions of
  existing 7+1 atoms). Not implemented; H2.2 is the
  closest design path but doesn't directly create new
  atoms either.
- **Self-modifying drive (load-bearing)**. H2.0 step 1 +
  step 2 establish the machinery; step 3 (wake-gate
  refactor) makes it actually self-modifying behaviour.
  Identified as the next implementation move.
- **Cross-context generalisation**. Same as morning.
- **Falsifiability of promoted sequences**. Same.
- **Recent-window stats checkpoint persistence** (morning
  retro #3). Still deferred. Long-run did not surface it
  as load-bearing — the existing reset-on-tick mechanism
  worked fine over 2000 ticks.

## Post-mortem on today's deltas

#### What went well

1. **The composite-EP-fix found by long-run was a 6-line
   change with 5× empirical impact.** The retrospective's
   choice to start with empirics paid off.
2. **Phasing H2.0 into three steps was right.** Step 1 + 2
   shipped without breaking anything; the riskiest step 3
   stays separable.
3. **Triple demotion was straightforward** — symmetry with
   H1.3's pair-demotion path made the implementation
   essentially mechanical.

#### What was harder than expected

1. **Diagnosing the EP-frontier gap took reading
   `execute_for_kind`'s mapping table closely.** The bug
   isn't visible from the call site; it's structural. A
   note on which ActionKinds are "frontier-mapped" vs
   "scheduler-special" would have caught this earlier.
2. **DriveMix checkpoint format** was a small design
   exercise. K/V format with `candidate_a:` / `candidate_b:`
   prefixes worked; mixing weight entries with scalar
   fields in the same section is slightly less clean than
   two sub-sections, but well-contained.

#### What might bite later

1. **Two A/B loops on the same EP signal**. MetaScheduler
   (H0) and DriveMix (H2.0) both window-mutate based on
   mean EP delta. Currently they don't see each other; if
   step 3 wires DriveMix into the gate, the two loops
   could interact unpredictably. ADR 0063 OQ #5 flagged
   this; needs phase-shifted windows or a single unified
   feedback controller.
2. **DriveMix mutation magnitude**. ×0.8 / ×1.25 borrowed
   from MetaScheduler. Drive weights live in [0, 1]; the
   multiplicative scheme produces large relative steps
   near 0 (e.g., 0.10 → 0.125 = +25%) and small steps near
   1 (0.95 → 1.0 capped). May need an additive mode
   eventually.
3. **rng_state determinism across checkpoint restore**.
   Round-trips through the [drive_mix] section — verified
   by `h2_0_drive_mix_round_trips_through_checkpoint` —
   but if the runtime ever runs concurrently or
   deserializes from different versions, drift could
   accumulate. Out of scope for now; flagged.

## Distance covered, in one sentence

12 hours ago: "the runtime mints new dispatch units from
its own behaviour, encodes them as meta-R facts,
dispatches them as units, retracts them when they stop
helping, and tracks length-3 sequences."

Today: "the runtime now has a self-tuning blend of
intrinsic drives — compression / prediction-error /
mode-thrash — operating in shadow mode with checkpoint
round-trip; one architectural gap diagnosed and fixed
along the way; long-run empirics validate that the
mutation feedback loop works on real substrates."

## Next directions

In rough priority:

1. **H2.0 step 3 — wake-gate refactor.** The load-bearing
   slice. Replace the existing zero-streak / mode-thrash
   gates with `combined_signal = Σ active_weights[id] *
   drive.evaluate()`. Risk: high — step 2's shadow
   property is the safety net that's about to come off.
   Recommend phase-shifting DriveMix windows vs
   MetaScheduler windows before this step (OQ #5).
2. **Long-run rerun under step 3.** Once gates read from
   DriveMix, observe whether self-tuning drifts to
   sensible weights or thrashes. The morning
   retrospective's "does the system drift to sensible
   values, or thrash between extremes?" question becomes
   answerable for drives.
3. **Empirical comparison: hand-tuned vs equal-weighted
   initialization** (ADR 0063 OQ #1). With step 3 wired,
   re-initialize DriveMix with `(0.33, 0.33, 0.33)` and
   compare F0 battery output to the hand-tuned baseline.
4. **H2.1 ADR drafting** — drives as `R(DRIVE_MARKER, X)`
   meta-R objects. Empirically motivated only if step 3
   reveals that DriveMix's catalogue (3 compile-time
   drives) is too narrow. Likely deferred.
5. **Recent-window stats checkpoint persistence** —
   carrying forward from morning retro #3. Still
   deferred; revisit if long-run substrates expose it.

## Author's note

12 hours of guided iteration, 6 implementation phases (H1
retros 1–4 + H2.0 steps 1–2), 2 ADR additions, ~1500 lines
of new code, +21 tests (466 → 487), 1 architectural fix
that 5×'d the runtime's empirical engagement, 1 second-
generation retrospective that explicitly compares against
the morning retrospective. Each phase was a verified
slice. The cumulative architectural shift is qualitative:
the system now has a *shadow self-tuning evaluation loop*
that didn't exist 12 hours ago, *and* the long-run
diagnostic infrastructure that revealed the architectural
gap is now standing infrastructure (long-run example +
log capture).

H2.0 step 3 is the natural next move. The empirical case
is concrete; the design space is mapped; the testing
infrastructure exists. What's left is the integration
itself, where H2 was originally said to "have more
potential for getting wrong than any prior phase."

The phasing strategy means that potential is now bounded
to one focused change.
