# Relatum v3 — world-model substrate

> Recover de-named relation structure from anonymous state sequences.

v3 is a parallel engine to [v2](../v2/), not a replacement. v2 remains the
R-closure / theory runtime. v3 is the substrate beneath it: it takes
anonymous state sequences and recovers the relation structure that
explains them.

- Five ontological commitments (three inherited from v2, two changed):
  [docs/constitution.md](docs/constitution.md)
- Episode schema, fingerprint definition, four-layer dataset, training
  tasks, milestones: [docs/design-notes.md](docs/design-notes.md)
- Minimal scaffold: `src/lib.rs` — `Episode`, `Observation`,
  `Intervention`, three commitment-guard tests.

## Build

```
cargo test
```

## What is *not* here yet

- The simulator (mechanism library A–F).
- The fingerprint engine.
- Any training code.
- Any link to v2.

The minimum scaffold exists only to guard the ontological commitments
under code. Everything else is design work in `docs/design-notes.md`.
