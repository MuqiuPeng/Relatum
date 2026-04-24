# 0038: RSet text persistence

Status: Accepted
Date: 2026-04-24

## Context

The RSet lived only in process memory until ADR 0038. Every ADR
mechanism operated on an in-memory value; any cross-session
continuation required re-running the construction pipeline from
raw data. For v2 to be useful as a medium for accumulated
abstraction (patterns, theories, axioms named across sessions),
some persistence is necessary.

Task 5 of the 1→5 extension, final phase. Adds the smallest
possible persistence — a deterministic text format — without
introducing external dependencies or format sophistication.

## Decision

### Text format

Line-oriented TSV: one R instance per line, `x\ty\n`. Blank lines
and lines starting with `#` on read are skipped. Output is sorted
lex by (x, y) so the same RSet always serializes to the same bytes.

```
# example
a	b
a	c
b	c
```

No header, no metadata, no version tag. The file *is* the set of
edges; meta-R objects (patterns, roles, theories, axioms,
extensions) are all encoded as R instances under ADR 0029 / 0030
/ 0032 / 0034, so they persist automatically.

### API

```rust
impl RSet {
    pub fn to_text(&self) -> Result<String, PersistenceError>;
    pub fn from_text(s: &str) -> Result<RSet, PersistenceError>;
}

pub enum PersistenceError {
    TabInIdentifier(String),
    NewlineInIdentifier(String),
    MalformedLine(usize),
}
```

`to_text` rejects any identifier containing tab or newline (they
would break line-based parsing). `from_text` reports the 1-based
line number on malformed input. Blank / `#`-prefixed lines are
ignored on read for easy hand-editing.

### PartialEq on RSet

Also added `#[derive(PartialEq, Eq)]` so roundtrip tests can do
`assert_eq!(a, b)`. Since `RSet` is a `HashSet<R>` wrapper and `R`
already derives `PartialEq, Eq, Hash`, this is safe.

## Alternatives considered

- **Serde + JSON / CBOR / bincode**. Rejected — requires an
  external crate; v2 has deliberately stayed no-deps. TSV is
  enough for what v2 needs.
- **Binary format with length prefixes**. Rejected — text format
  is diffable, hand-editable, and compressible by standard tools.
  Binary gains nothing at β-scale and costs readability.
- **Versioned format with header**. Rejected for now — no format
  evolution planned; if one arises, a header can be added later.
- **Save / load meta-R separately from data**. Rejected. Commitment
  1 (only R) makes separation arbitrary: meta-R *is* R. One file
  is more honest.
- **Escape characters for tab / newline**. Considered but
  rejected — typical v2 identifiers don't contain them, rejecting
  outright keeps the format simple. Can add escape support if a
  caller ever needs it.

## Consequences

### Cross-session continuation

```rust
let text = rs.to_text()?;
std::fs::write("my_abstraction.rset", text)?;
// … later, possibly another session …
let rs = RSet::from_text(&std::fs::read_to_string("my_abstraction.rset")?)?;
```

Everything is preserved: data edges, pattern definitions (Layer A
intension), instances (Layer B), theories, axioms with intension,
extensions. Because every named object is already meta-R, the
save/load is a single Set round-trip.

### Cross-machine reproduction

Deterministic sorted output means two processes, given the same
in-memory RSet, produce byte-identical files. Diffing is
meaningful; version control works.

### What's not persistent

- **PRNG state** for drive / discovery is not stored. Re-running
  a stochastic operation after load may produce different results.
  Acceptable — the config carries a seed; deterministic pipelines
  replay exactly.
- **Drive history** (`DriveTrace`) is a Rust value, not meta-R.
  If a caller wants the trace persistent, they can serialize it
  separately. Out of scope.

### Commitment check

- 1: all persistence is over R instances. ✓
- 2–5: unaffected.

## Verification

- 200 → 209 tests pass (9 new covering: empty roundtrip, simple
  roundtrip, determinism across insertion order, full-meta-R
  roundtrip with patterns/theories/axioms, reject tab, reject
  newline, reject malformed, skip blank/comment, bytes reproduce
  exactly).

## Implementation

- `v2/src/lib.rs` — `PersistenceError`, `RSet::to_text`,
  `RSet::from_text`; PartialEq derive on `RSet`.
- `v2/docs/decisions/0038-persistence.md` — this ADR.
