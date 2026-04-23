# 0025: Hierarchical discovery probe

Status: Accepted
Date: 2026-04-23

## Context

"Patterns composed of other patterns" is the last open direction.
A full hierarchical-pattern mechanism is ambitious: it would
require canonicals that carry sub-pattern labels, a matching
procedure that traverses them, and meta-R edges that link composed
patterns to their sub-pattern instances.

ADR 0011 already probed the relevant phenomenon: re-running
`compound_class_subgraphs` on the RSet *after* naming, so meta-R
edges participate in discovery. The finding then was that most
"new" structure is predictable encoding artifact (out-stars
produced by the three-shape encoding itself), with only a small
amount of genuinely new structure.

This ADR is a **focused second probe** using the newer pipeline
(sample-score-select + MDL filter). It asks: now that we have
principled filtering, does running `discover_motifs` against the
*full* post-naming RSet (data + meta-R) surface candidates that
look like genuine higher-order structure, or does MDL suppress
them all as trivial?

Rather than commit to a full hierarchical mechanism, this ADR
adds one opt-in flag and records what the probe finds.

## Decision

Add `include_meta_in_discovery: bool` (default `false`) to
`DiscoveryConfig`. When true, `discover_motifs` samples from *all*
edges, not just data edges. Everything else (refine, find,
naming) remains data-only so the probe doesn't accidentally build
on its own output.

```rust
pub struct DiscoveryConfig {
    pub target_size: usize,
    pub sample_count: usize,
    pub top_m: usize,
    pub rng_seed: u64,
    pub include_meta_in_discovery: bool,  // NEW (ADR 0025)
}
```

Internal helper: `all_edges_sorted` returns every edge (meta and
data) in a deterministic order, parallel to `data_edges_sorted`.

The probe experiment (in the log) compares `discover_motifs` at
`target_size=3` on the post-autonomous mixed graph with the flag
off vs. on, and reports the canonicals and their MDL gains.

## Alternatives considered

- **Implement full hierarchical canonical form.** Rejected as far
  beyond minimum-first — would require canonicals carrying
  pattern-id labels, matching that resolves them, and storage
  conventions for composed patterns. Not scoped for this ADR.
- **Enable meta in all operations (discovery, refine, find).**
  Rejected. The probe intentionally leaves other operations
  data-only so we can isolate whether discovery alone surfaces
  useful meta-involving structure.
- **Make a separate `hierarchical_discover` method.** Rejected.
  The flag keeps the API small; semantics are governed by one bit.
- **Run the probe without any code change, using an already-built
  RSet externally.** Rejected — `sample_connected_subgraph`
  excludes meta via `data_edges_sorted`. A code change was
  needed to sample meta edges at all.

## Consequences

- `discover_motifs` becomes dual-purpose: data-only by default,
  data+meta when opted in.
- Probe results either motivate a future full hierarchical ADR
  (if genuinely useful structure emerges) or close the direction
  (if only encoding artifacts).
- The flag defaults to `false`, so all existing callers are
  unaffected.
- All previously explicit `DiscoveryConfig` constructions gain a
  new field; tests and examples get the default value added.

## Implementation

- Source: `v2/src/lib.rs` — add field, add
  `RSet::all_edges_sorted`, branch `discover_motifs` on the flag.
- Tests: 2 new — flag off matches prior behavior; flag on can
  surface canonicals containing meta-R edges after naming.
- Example: `v2/examples/hierarchical_probe.rs` — compare
  data-only vs meta-included discovery on the post-autonomous
  mixed graph.
- Experiment log: `v2/logs/2026-04-23_hierarchical_probe.log`
  with the probe's findings and a honest verdict.
