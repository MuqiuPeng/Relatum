# Decisions

One file per non-trivial technical decision. Numbered sequentially, zero-padded.
Filename is `NNNN-slug.md`.

## Format

```markdown
# NNNN: Short title

Status: Accepted | Superseded by NNNN | Withdrawn
Date: YYYY-MM-DD

## Context
Why was a decision needed? What state of the system prompted it?

## Decision
What was chosen. Be concrete.

## Alternatives considered
Options that were examined but not taken, with brief reasoning.

## Consequences
What follows from this. What is now easy; what is now harder.
What was deferred and under what condition it would be revisited.

## Implementation
Commit hash(es), file pointers.
```

## Rules

- ADRs are append-only in spirit. Once accepted, don't rewrite history.
  If the decision changes, write a new ADR that supersedes the old one
  and update the old one's Status to `Superseded by NNNN`.
- Distinguish **Constitution** (non-revisable ontological commitments,
  lives in `constitution.md`) from **Decisions** (revisable technical
  choices, lives here).
- Every commit implementing a decision references the ADR by number.

## Index

- [0001](0001-project-restructure.md) — project restructure: v1 archived, v2 scaffolded
- [0002](0002-rset-harness.md) — RSet as the observation harness
- [0003](0003-identifier-profile.md) — IdentifierProfile as first-pass structural observation
- [0004](0004-signature-is-profile.md) — Signature = IdentifierProfile (0-hop, first pass)
- [0005](0005-r-instance-signature.md) — R-instance signature (edge-level, endpoint profile pair)
- [0006](0006-locality-profile.md) — Locality profile: co-left / co-right / forward / reverse neighbor counts
- [0007](0007-compound-signature-probe.md) — Compound signature probe (edge_fingerprint = RSignature + LocalityProfile)
- [0008](0008-subgraph-extraction.md) — Subgraph representation and connected-component extraction (first β ADR)
- [0009](0009-subgraph-canonicalization.md) — Subgraph canonicalization via Weisfeiler–Lehman refinement
