# v2 Experiment Logs

One file per experiment run. Logs are durable records; commit them.

## Naming

```
YYYY-MM-DD_descriptor.log
```

Example: `2026-04-23_chain_profile.log`.

If multiple runs happen on the same day with the same descriptor, suffix
with an index: `2026-04-23_chain_profile_02.log`.

## Content

Each log should be self-contained. A future reader should not need to
consult git or external context to understand what was run.

At minimum include:
1. **Header** — date, descriptor, purpose (one sentence).
2. **Inputs** — the R instances fed in (verbatim or hash + source pointer).
3. **Config** — any tunables or mechanism versions.
4. **Output** — what the mechanism produced.
5. **Observations** — what was interesting, surprising, or indicative.
6. **Next** — what the result suggests doing next, if anything.

## When to log

- Every time a mechanism is run on non-trivial data.
- When a small input produces a surprising result.
- When comparing two mechanism variants (log each; include the comparison).

Don't log trivial unit-test outputs — those already live in the test suite.
