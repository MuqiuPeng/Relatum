# Relatum v2

Autonomous abstraction from a single binary directed relation.

## Primitive

```
R(x, y)
```

One relation. Two slots. A direction. No meaning.
Everything else must emerge.

## Documentation

- [docs/constitution.md](docs/constitution.md) — five non-negotiable ontological commitments
- [docs/design-notes.md](docs/design-notes.md) — internal design reference for the v2 direction
- [docs/practices.md](docs/practices.md) — working practices (traceability, minimum-first, constitution check)
- [docs/decisions/](docs/decisions/) — ADRs for all non-trivial technical decisions
- [docs/progress.md](docs/progress.md) — chronological progress log
- [logs/](logs/) — experiment output logs

## Build

```
cargo test
```

## Status

Full autonomous-abstraction pipeline in place:
`R`, `RSet`, `IdentifierProfile`, `Signature`, `equivalence_classes()`,
`RSignature`, `r_equivalence_classes()`, `LocalityProfile`,
`locality_profile()`, `EdgeFingerprint`, `edge_fingerprint()`,
`Subgraph`, `Subgraph::connected_components_of`,
`compound_class_subgraphs()`, `Subgraph::canonicalize()`,
`Subgraph::is_isomorphic_to()`, `CanonicalForm`, `PATTERN_MARKER`,
`PatternError`, `name_pattern_instances()`, `patterns()`,
`instances_of()`, `participants_of()`, `find_pattern_matching()`,
`NamingPolicy`, `SkipReason`, `NamingDecision`, `consider_naming()`,
`run_naming_pass()`.

β closed: 0008 (subgraph extraction), 0009 (canonicalization), 0010
(pattern naming as meta-R), 0012 (γ policy + driver). Probe ADRs
0007 and 0011 recorded the observations that informed β's shape.
Query API in ADR 0013; attach-only mode in ADR 0014; subgraph
matching in ADR 0015 (replaces the attach path with direct
enumeration, fixes compound-class fragmentation on asymmetric
structures).

Three search primitives cover different jobs:
- `compound_class_subgraphs` (0007): discovery heuristic via
  structural grouping — efficient for symmetric recurrent motifs.
- `find_instances_of` (0015): exhaustive matching against a known
  canonical — for attach verification; cleanness-filtered.
- `discover_motifs` (0016): sample-score-select — the first
  non-enumeration search, finds asymmetric motifs the other two
  cannot.
- `refine_candidates` (0017): targeted re-sampling to promote
  embedded motif representatives to clean ones.
- `autonomous_pass` (0018): composes the above with naming into a
  single "sample → score → refine → name" loop. The system
  proposes and records new pattern types without any external
  canonical or instance hints.

Plus `is_clean_subgraph` (0017) exposed as a public helper; sorted
data-edge enumeration keeps all sampling / matching deterministic
across process runs.

**Autonomous abstraction loop operational.** On the canonical mixed
graph at target_size=3, one `autonomous_pass` names four distinct
structural types — 3-chain, 3-cycle, 3-star, and the 3-tree that
compound-class discovery could not reach.

Naming policy includes a **MDL-gain filter** (ADR 0019) — opt-in
reusability threshold via `NamingPolicy::min_mdl_gain`. Zero-gain
singletons are automatically excluded when the threshold is set.

Registry is **bidirectional** (ADR 0020): `RSet::retract_pattern`
removes a named pattern and all of its meta-R while leaving data
edges intact. Enables experimentation loops (try naming, roll back,
try different policy).

Open directions (refinements, not completions): multi-size
autonomous passes, attach-only integration, cross-graph pattern
transfer, sampling-based `find_instances_of` replacement,
hierarchical / composed patterns.

See [docs/progress.md](docs/progress.md) for the current frontier.
