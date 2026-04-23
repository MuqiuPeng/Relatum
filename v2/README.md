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

Observation through subgraph-extraction in place:
`R`, `RSet`, `IdentifierProfile`, `Signature`, `equivalence_classes()`,
`RSignature`, `r_equivalence_classes()`, `LocalityProfile`,
`locality_profile()`, `EdgeFingerprint`, `edge_fingerprint()`,
`Subgraph`, `Subgraph::connected_components_of`,
`compound_class_subgraphs()`.

β is underway in four ADRs: 0008 (subgraph representation — done),
0009 (canonicalization / isomorphism — next), 0010 (pattern naming as
meta-R per commitment 3), 0011 (γ drive policy).

See [docs/progress.md](docs/progress.md) for the current frontier.
