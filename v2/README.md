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

Observation through pattern naming in place:
`R`, `RSet`, `IdentifierProfile`, `Signature`, `equivalence_classes()`,
`RSignature`, `r_equivalence_classes()`, `LocalityProfile`,
`locality_profile()`, `EdgeFingerprint`, `edge_fingerprint()`,
`Subgraph`, `Subgraph::connected_components_of`,
`compound_class_subgraphs()`, `Subgraph::canonicalize()`,
`Subgraph::is_isomorphic_to()`, `CanonicalForm`, `PATTERN_MARKER`,
`PatternError`, `name_pattern_instances()`, `patterns()`,
`instances_of()`, `participants_of()`, `find_pattern_matching()`.

β is underway in four ADRs: 0008 (subgraph representation — done),
0009 (canonicalization / isomorphism — done), 0010 (pattern naming as
meta-R per commitment 3 — done), 0011 (γ drive policy — next).

See [docs/progress.md](docs/progress.md) for the current frontier.
