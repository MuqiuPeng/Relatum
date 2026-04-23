# 0001: Project restructure — v1 archived, v2 scaffolded

Status: Accepted
Date: 2026-04-23

## Context

The original Relatum (now v1) is a relational closure engine. Phase 1–9
established that closure can discover axiom classes and derive universally
quantified consequences. It does not, however, *construct* new relations
from intrinsic drive.

The user's philosophical stance — cognition is abstraction, objects emerge
from relations — calls for a system whose ongoing activity is the generation
of new relations, not the closure of given ones. This cannot be added on top
of v1; it requires a different primitive.

## Decision

Archive v1 in place and scaffold v2 in the same repository.

- `v1/` contains everything previously at the repo root (`src/`, `tests/`,
  `www/`, `docs/`, `logs/`, `Cargo.toml`, etc.) untouched.
- Tag `v1.0` marks the final v1 commit before the move.
- `v2/` is a fresh Rust package with its own `Cargo.toml`, `src/`, `docs/`,
  and `logs/` trees.
- Root `README.md` becomes the v1-vs-v2 index.
- `.github/workflows/deploy.yml` updated (`path: www` → `path: v1/www`) so
  GitHub Pages continues serving the v1 playground.

## Alternatives considered

- **Separate repositories.** Rejected: loses the ability to reference v1
  results directly (essay, phase logs) as the v2 benchmark.
- **Branch isolation** (tag v1 on main, clear main, start v2). Rejected:
  makes v1-v2 cross-reference harder; `main` should remain the active line.
- **Modify v1 in place.** Rejected: destroys v1's provenance. Phase 1–9 results
  must remain reproducible from their original source.
- **v2 copies v1 code, then prunes.** Rejected: v1's abstractions (Term,
  Fact, Rule, Relation) carry v1-specific commitments that would leak into v2.

## Consequences

- v1 is frozen. Any future v1 work would branch from tag `v1.0`.
- v2 starts with zero accumulated assumptions — the single primitive is
  `R(x, y)`, the five ontological commitments are the only rules.
- The essay (`v1/docs/essay/`), phase logs (`v1/logs/archive/`), and v1
  playground remain available as the benchmark against which v2 is judged.
- GitHub Pages deployment continues unchanged from the user's perspective.

## Implementation

- Commit `b8aaa84` — saved pending v1 content before restructure.
- Tag `v1.0` at `b8aaa84`.
- Commit `79541ea` — `restructure: archive v1 under v1/, scaffold v2`.
