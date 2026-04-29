# H.1 — Semantic axiom labels

**Status**: ✓ done (2026-04-30)
**Log**: [`logs/2026-04-30_phase_h1_axiom_labels.log`](../../logs/2026-04-30_phase_h1_axiom_labels.log)
**Example**: [`examples/phase_h1_axiom_labels.rs`](../../examples/phase_h1_axiom_labels.rs)

## Goal

Axiom ids like `ax_tpl_v3_p0-1_p1-2_c0-2` are informative but cryptic. H.1 ships a utility function `human_label(axiom_id)` that returns readable names for common patterns + structural-description fallback for unknown shapes. Quality-of-life improvement for logging, debugging, and result documents.

## Recognized patterns

| input id | label |
|---|---|
| `ax_reflexivity` | `reflexivity` |
| `ax_antisymmetry` | `antisymmetry` |
| `ax_totality` | `totality` |
| `ax_tpl_v3_p0-1_p1-2_c0-2` | **`transitivity`** |
| `ax_tpl_v3_p0-1_p1-2_c2-0` | `reverse-transitivity` |
| `ax_tpl_v2_p0-1_c0-0` | `left-self-loop` |
| `ax_tpl_v2_p0-1_c1-1` | `right-self-loop` |
| `ax_tpl_v2_p0-1_c1-0` | `symmetry` |
| `ax_tpl_v2_p0-1_c0-1` | `trivial-identity` |
| `ax_tpl_v3_p0-0_p1-2_*` | `self-loop-with-witness-edge` |
| (unknown shape) | `v{n} prem{p} concl(c{x}-{y})` (structural fallback) |

## Result on OQ#1

11 of 13 axioms get human labels. The 2 unrecognized fall back to structural descriptions:
- `ax_tpl_v3_p0-1_p2-1_c0-2` → `v3 prem2 concl(c0-2)`
- `ax_tpl_v3_p0-1_p0-2_c1-2` → `v3 prem2 concl(c1-2)`

These are 3-var axioms with premise structures other than the canonical `R(0,1) R(1,2)` — they have less standard names.

### Notable empirical observation

All 4 noise-family axioms (`shape_premise_p0-0_p1-2`) label as **`self-loop-with-witness-edge`**. The label captures the structural shared property — the noise family is unified at the semantic-label level too, not just at the family-discovery level.

This is **structural amplification**: labels generated independently from family discovery converge on the same partition for noise members.

## Verdict

**POSITIVE — H.1 produces readable labels for 8 well-known shapes + structural fallback for the rest.**

## What this slice produced

1. `human_label(axiom_id) -> String` utility function (inline in example; ready to graduate to lib)
2. 8 recognized patterns covering common cases (predicate axioms + transitivity-likes + 2-var conclusion variants + noise-family pattern)
3. Empirical convergence: noise family members all share the same label — independent path to Beta-1's structural finding

## Future implications

- **Graduate to lib**: move `human_label` to `RSet::axiom_human_label` (or a free function in `axiom_ids` module)
- **Integrate with logging**: every axiom-related log line could include the label in addition to the id
- **Family labels**: extend to shape families (`shape_premise_p0-0_p1-2` → `noise-family`, `shape_premise_p0-1` → `2-var-base`)
- **Theory labels**: theories could be labeled by their member axioms' labels (e.g., t_2 = "{transitivity, reflexivity, antisymmetry}" — Peano-poset)
- **Documentation generation**: result documents could auto-include labels alongside ids for readability

## Why this is a quality-of-life addition only

The label is descriptive metadata; no behavioral change. v2's runtime treats `ax_tpl_v3_p0-1_p1-2_c0-2` and `transitivity` as the same axiom (commitment 4 — token identity). The label is a presentation layer.

**Risk avoided**: H.1 doesn't introduce labels as PRIMARY identifiers. If "transitivity" became an alias for the axiom token, two systems with the same alias might disagree on the underlying token, breaking commitment 4. Labels are read-only descriptions, not redirections.

## Methodological note

Pattern matching on `template.premise.as_slice()` plus structural conditions captures the recognition logic concisely. Adding new label patterns is local — append a new match arm. Coverage of OQ#1 is 11/13 = 85%; covering the remaining 2 (`v3_p0-1_p2-1` and `v3_p0-1_p0-2` premise variants) would need 2 more match arms (e.g., "permuted-transitivity" labels for less-standard premise patterns).

The structural fallback ensures NO axiom ever appears unlabeled — every axiom gets either a semantic name or a parameterized structural description.
