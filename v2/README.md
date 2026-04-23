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

Observation layer + 0-hop structural signature in place:
`R`, `RSet`, `IdentifierProfile`, `Signature`, `equivalence_classes()`.

Not yet implemented: compound pattern detection, type naming, self-driven
triggering, evaluation. See [docs/progress.md](docs/progress.md) for the
current frontier.
