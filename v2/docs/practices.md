# v2 Working Practices

Standing practices for how v2 is built. These are orthogonal to the
ontological commitments in [constitution.md](constitution.md) — constitution
is *what* v2 is, practices are *how* v2 is built.

## 1. Traceability

Every non-trivial step must produce a durable record beyond the git commit.

- **Decisions** → `docs/decisions/NNNN-slug.md` (ADR format, see
  [decisions/README.md](decisions/README.md))
- **Progress** → `docs/progress.md` (chronological, append-only)
- **Experiments** → `logs/YYYY-MM-DD_descriptor.log` (text, self-contained)

The git commit references the ADR. The ADR must exist and be current at
commit time. An undocumented commit is incomplete.

## 2. Minimum-first, expand on failure

Start every mechanism with the simplest version that could plausibly work.
Record richer alternatives as "deferred candidates" in the source or ADR.
Only implement them when the simple version demonstrably fails.

Rationale: the constitution constrains what mechanisms are allowed; premature
sophistication makes it harder to see when a mechanism is actually inadequate
vs. merely under-tuned.

## 3. Constitution check before commit

Before committing any mechanism, check each of the five commitments
([constitution.md](constitution.md)) against the new code. If a commitment is
at risk of being violated, either:
- revise the mechanism, or
- document the risk in the ADR and raise it to the user.

Silent drift is the failure mode. See the closing note in constitution.md.

## 4. Tests encode invariants

Where a commitment or design decision has a testable consequence, write a
test that will fail if it is violated. Tests are the commitments' guard.

Example: `r_is_directional` guards the asymmetry of R(x, y) vs R(y, x);
`identity_is_token_based` guards string-equality identity.
