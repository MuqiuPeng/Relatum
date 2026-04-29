# G.4 — Generative-axiom quality via predicate compliance

**Status**: ✓ done (2026-04-29)
**Log**: [`logs/2026-04-29_phase_g4_generative_quality.log`](../../logs/2026-04-29_phase_g4_generative_quality.log)
**Example**: [`examples/phase_g4_generative_quality.rs`](../../examples/phase_g4_generative_quality.rs)

## Goal

ADR 0069 stated that cross-precision **does not apply** to generative axioms — they produce identifiers, not predictions over a fixed substrate. G.4 specifies the alternative metric.

## Approach

**Predicate compliance**: a panel of 5 structural predicates that any "well-behaved" generative recipe should satisfy on its produced chain.

| # | property | rationale |
|---|---|---|
| 1 | acyclic | no R(x,y) reaches back to x — chain forms a DAG |
| 2 | injective predecessor | every minted id has ≤ 1 predecessor — chain is linear |
| 3 | irreflexive | no R(x,x) — minted ids are distinct from themselves |
| 4 | transitive closure → strict total order | chain admits well-defined ordering after closure |
| 5 | freshness | every minted id absent from seed substrate; no chain duplicates |

Compliance rate = `satisfied / 5`. Per-recipe scalar in [0, 1], comparable across recipes — the role cross-precision plays for predicate axioms.

## Result

Three recipes evaluated against seed substrate `{0, X, Y}`:

| recipe | mint formula | acyclic | inj-pred | irrefl | total-order | fresh | rate |
|---|---|---|---|---|---|---|---|
| **successor** | `format!("succ({})", t)` | ✓ | ✓ | ✓ | ✓ | ✓ | **1.00** |
| **constant**  | `_ => "X"` | ✗ | ✗ | ✗ | ✗ | ✗ | **0.00** |
| **dbl_prefix** | `format!("p_{}", t)` | ✓ | ✓ | ✓ | ✓ | ✓ | **1.00** |

## Why constant fails everything

`mint(t) ≡ "X"`:
- step 1: mint("0") = "X" — colliding with seed substrate (no freshness)
- step 2: mint("X") = "X" — produces R("X", "X") (no irreflexivity, no acyclicity)
- step 2+: every later mint also = "X" — same predecessor R("X", "X") triggers multiple-predecessor violation as chain "grows"
- transitive closure on a self-loop trivially fails total order (R(X,X) implies neither i<j nor i>j)

A broken recipe fails all 5 properties. The metric correctly assigns it 0.00.

## Why successor and dbl_prefix both score 1.00

Both produce chains with the structural shape:
```
chain[i+1] := wrapper(chain[i])
edge:        R(chain[i+1], chain[i])
```

Where `wrapper` is a string function that:
- Always grows the input (output strictly longer than input)
- Always varies output by input (no two distinct inputs produce same output)

Both `succ(...)` and `p_...` satisfy these, hence both pass all 5 predicates. They are functionally equivalent under predicate compliance — same shape, different surface syntax. **A high score on this metric does NOT discriminate "succ" from "p_"**, but it correctly classifies both as well-behaved.

## Verdict

**POSITIVE — the metric discriminates broken recipes from well-behaved ones.**

This is the cross-precision analog for generative axioms:
- Cross-precision answers "do my predictions hold on a substrate?" — a per-axiom quality scalar in [0, 1].
- Predicate compliance answers "does my generated structure satisfy expected properties?" — also a per-axiom quality scalar in [0, 1].

Both let the runtime rank, demote, or merge axioms by quality.

## Comparison with cross-precision

| dimension | cross-precision (predicate axioms) | predicate compliance (generative axioms) |
|---|---|---|
| input | axiom + substrate | axiom + seed |
| output | rate edge predictions match substrate | rate properties satisfied by chain |
| signal | high precision = trustworthy axiom | high compliance = well-behaved recipe |
| range | [0, 1] | [0, 1] |
| discriminator | distinguishes signal from noise axioms | distinguishes well-formed from broken recipes |

The asymmetry: cross-precision varies smoothly across well-formed predicate axioms (e.g., t_0 noise = 0.32 vs t_2 universal = 1.0). Predicate compliance is more bimodal — most recipes are either correct (1.0) or broken (low). Future G.5+ refinements could add finer-grained metrics (e.g., chain length growth rate, cycle distance from "barely-fails-acyclic" recipes).

## What this slice produced

1. Predicate compliance metric — formalized cross-precision analog for generative axioms
2. Empirical demonstration: distinguishes good recipes (1.0) from broken (0.0)
3. Documentation of the bimodal nature of compliance (vs cross-precision's continuous gradient)
4. ADR 0069 commitment satisfied: G.4 specified the alternative metric promised by the ADR

## Future implications

- **G.5**: fine-grained generative quality (chain growth rate, structural diversity, etc.) — beyond binary predicate compliance
- **Generative-axiom demote**: when compliance < 0.6, retract the recipe (analog of cross-precision-driven demote)
- **Generative-axiom shape families**: recipes that produce equivalent compliance vectors might be grouped (Beta-1 logic generalized to generative)
- **Drive integration**: a "produce well-behaved structure" drive could reward high compliance, signaling toward integer-like constructs
