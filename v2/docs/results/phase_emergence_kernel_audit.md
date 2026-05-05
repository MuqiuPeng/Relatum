# Phase Emergence — kernel audit

**Status**: ✓ done (2026-05-06); reverses prior diagnosis
**Log**: [`logs/2026-05-06_phase_emergence_kernel_audit.log`](../../logs/2026-05-06_phase_emergence_kernel_audit.log)
**Example**: [`examples/phase_emergence_kernel_audit.rs`](../../examples/phase_emergence_kernel_audit.rs)
**ADR**: [0075 — Emergence kernel audit and runtime integration](../decisions/0075-emergence-kernel-audit-and-runtime-integration.md)

## Goal

Reflection 0001 + the constitution heavy-reading amendment
(2026-05-06) require concept creation to be atomic in three
facets: (1) mint a concept token, (2) register participating
tokens via meta-R, (3) never use per-token derived signature as
visible behaviour outside that act.

ADR 0073 had concluded "v2 cannot create new concepts" before the
strict reading was applied. The reflection / amendment showed
that the requirement is more demanding than ADR 0074's
shape-family co-occurrence mining satisfied, but it left open
whether **other** existing v2 mechanisms might already satisfy it.
This audit checks one specific candidate: the `autonomous_pass`
pipeline (ADR 0018, wiring 0009 + 0016 + 0017 + 0010 / 0029).

## Method

Build each canonical substrate (OQ#1, long5k, narrow_a, OQ#2),
run the standard runtime to its Phase 0 horizon, then call
`autonomous_pass` directly with sizes 2-5 (sample_count=400,
top_m=20). Tabulate:
- patterns minted
- per-pattern instance count
- per-pattern distinct participating-token count

Verify that each minted pattern carries `R(p, instance_n)` plus
`R(instance_n, participant_token)` meta-R chains — i.e. that
participating tokens are *explicitly registered* and not just
implicitly counted.

## Result

### Cross-substrate summary

```
substrate       ticks  total_edges  pre  post  new  total_instances
OQ#1            1000          375    0     7    7              105
long5k          1500          420    0     7    7              140
narrow_a         500          345    0     3    3               35
OQ#2            4500          245    2     7    5              172
```

`pre` = patterns existing before the audit's autonomous_pass call.
OQ#2 had 2 from incidental discovery during Phase 0 runtime; the
audit added 5 more, for a post-audit total of 7.

### Per-pattern detail

OQ#1 minted patterns p_0..p_6:
- p_0 → 30 instances, 25 distinct participating tokens
- p_1 → 15 instances, 25 distinct participating tokens
- p_2 → 25 instances, 20 distinct participating tokens
- p_3 → 10 instances, 25 distinct participating tokens
- p_4 → 15 instances, 25 distinct participating tokens
- p_5 → 5 instances, 15 distinct participating tokens
- p_6 → 5 instances, 15 distinct participating tokens

long5k minted similarly-shaped p_0..p_6 with somewhat larger
instance counts (140 total vs OQ#1's 105). narrow_a produced
fewer (3 patterns / 35 instances), as expected from its narrower
stream. **OQ#2 produced more total instances than any other
substrate** despite the axiom path producing only 2 predicate
axioms there.

### Compliance check

For every minted pattern, the audit verified:

1. `R(PATTERN_MARKER, p)` exists (concept token registered)
2. For each instance n of p: `R(p, instance_n)` exists
3. For each instance n: `R(instance_n, participant_token)` edges
   exist for every token participating in that subgraph

All three hold for all minted patterns. The pipeline satisfies
the constitution's heavy reading: bucket key is subgraph
canonical form (computed only from subgraph-internal edges),
mint is atomic, participating tokens are registered.

## Verdict

**The emergence kernel was already in v2's library.** It was
never wired into the main runtime's high-priority action set,
so the kernel was effectively dormant — that's why the
substrate-diversity probe and ADRs 0073 / 0074 reported "no
concept creation." The diagnosis "v2 cannot create new concepts"
is now revised to "v2 has a compliant concept-creation kernel
that the runtime currently underuses."

## Diagnostic reversal: OQ#2 is the most kernel-active substrate

The Emergence-1 substrate-diversity probe had concluded OQ#2 is
the substrate where Phase 0070-0072 produced nothing useful, and
predicted that intrinsic drive would be most valuable on it.

The audit reverses this:
- OQ#2 produces 0 template axioms and 0 shape families (axiom
  path is silent)
- OQ#2 produces **172 total pattern instances across 5+ minted
  patterns** (pattern path is the most active among all
  audited substrates)

The "RSet collapse" finding from the diversity probe was a
property of the *axiom path*, not of v2's overall ability to
abstract. The pattern path is genuinely substrate-distinct and
empirically rich on OQ#2.

The reframed forecast: drive does not need to "activate v2 on
OQ#2" — v2 is already active on OQ#2 via the pattern path.
What's needed is **runtime integration** so the existing
capability operates autonomously during normal scheduling, not
only when called manually.

## What this changes

### Re-classification of past ADRs

- **ADR 0073** "system cannot create new concepts": the
  diagnosis is correct only for axiom-template invention (the
  template grammar is hard-coded). Concept emergence in general
  works, via the pattern path. Update the ADR's framing to
  scope its "hard ceiling" claim to axiom shapes specifically.

- **ADR 0074** concept mining: re-categorised under the
  constitution heavy reading as "implicit conceptualization" —
  the minted "concept" is a shape-family co-occurrence label
  that does not register participating tokens as instances of
  the new concept. Useful as a curatorial layer; not concept
  creation. The genuine concept-creation path is the pattern
  pipeline audited here.

- **Substrate-diversity probe** (Phase Emergence-1) finding:
  the "OQ#2 is the blind spot" verdict is replaced by "OQ#2 is
  the axiom-path blind spot but the pattern-path's most active
  substrate." The diagnostic generalisations should be re-read
  with this scope correction.

### Runtime integration is the next concrete step

ADR 0075 specifies the integration: bump `DiscoverPatterns`'s
priority in `RuleBasedScheduler` so the runtime calls
`autonomous_pass` periodically during normal stream processing.
The audit gives empirical support that pattern-discovery is
worth running on every canonical substrate — even ones the
axiom path skips.

### Cross-substrate canonical-form comparison is a useful follow-up

The audit's "shared pattern id" output is misleading: pattern
ids are per-RSet counters, so p_2 appearing on every substrate
is an accident, not a structural identity claim. The real
substrate-diversity test at the pattern level requires comparing
**canonical forms** (post-canonicalize bytes), not pattern ids.
This is a 1-day follow-up, deferred to a separate slice.

## What the audit did not address

- **Pattern lifecycle**: minted patterns persist forever unless
  explicitly retracted. The audit does not address when /
  whether to retract.
- **Pattern quality assessment**: the audit reports
  count-based statistics. ADR 0072-style cross-precision
  validation for patterns (analogous to what 0071 / 0072
  built for axioms / theories) is a possible follow-up but
  not in scope here.
- **Pattern-aware drive metric**: under the heavy reading, any
  drive metric must avoid per-token signature bucketing. A
  pattern-aware metric that classifies unexplained R by
  pattern-instance-membership rather than by signature would
  be compliant; left for a future ADR.

## Files

- Example: `examples/phase_emergence_kernel_audit.rs` (~ 200 lines)
- Log: `logs/2026-05-06_phase_emergence_kernel_audit.log`
- ADR: `docs/decisions/0075-emergence-kernel-audit-and-runtime-integration.md`
- This result: `docs/results/phase_emergence_kernel_audit.md`

No code changes to library; the audit is purely observational.
