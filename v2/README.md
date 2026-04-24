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

Full autonomous-abstraction pipeline in place:
`R`, `RSet`, `IdentifierProfile`, `Signature`, `equivalence_classes()`,
`RSignature`, `r_equivalence_classes()`, `LocalityProfile`,
`locality_profile()`, `EdgeFingerprint`, `edge_fingerprint()`,
`Subgraph`, `Subgraph::connected_components_of`,
`compound_class_subgraphs()`, `Subgraph::canonicalize()`,
`Subgraph::is_isomorphic_to()`, `CanonicalForm`, `PATTERN_MARKER`,
`PatternError`, `name_pattern_instances()`, `patterns()`,
`instances_of()`, `participants_of()`, `find_pattern_matching()`,
`NamingPolicy`, `SkipReason`, `NamingDecision`, `consider_naming()`,
`run_naming_pass()`.

β closed: 0008 (subgraph extraction), 0009 (canonicalization), 0010
(pattern naming as meta-R), 0012 (γ policy + driver). Probe ADRs
0007 and 0011 recorded the observations that informed β's shape.
Query API in ADR 0013; attach-only mode in ADR 0014; subgraph
matching in ADR 0015 (replaces the attach path with direct
enumeration, fixes compound-class fragmentation on asymmetric
structures).

Three search primitives cover different jobs:
- `compound_class_subgraphs` (0007): discovery heuristic via
  structural grouping — efficient for symmetric recurrent motifs.
- `find_instances_of` (0015): exhaustive matching against a known
  canonical — for attach verification; cleanness-filtered.
- `discover_motifs` (0016): sample-score-select — the first
  non-enumeration search, finds asymmetric motifs the other two
  cannot.
- `refine_candidates` (0017): targeted re-sampling to promote
  embedded motif representatives to clean ones.
- `autonomous_pass` (0018): composes the above with naming into a
  single "sample → score → refine → name" loop. The system
  proposes and records new pattern types without any external
  canonical or instance hints.

Plus `is_clean_subgraph` (0017) exposed as a public helper; sorted
data-edge enumeration keeps all sampling / matching deterministic
across process runs.

**Autonomous abstraction loop operational.** On the canonical mixed
graph at target_size=3, one `autonomous_pass` names four distinct
structural types — 3-chain, 3-cycle, 3-star, and the 3-tree that
compound-class discovery could not reach.

Naming policy includes a **MDL-gain filter** (ADR 0019) — opt-in
reusability threshold via `NamingPolicy::min_mdl_gain`. Zero-gain
singletons are automatically excluded when the threshold is set.

Registry is **bidirectional** (ADR 0020): `RSet::retract_pattern`
removes a named pattern and all of its meta-R while leaving data
edges intact. Enables experimentation loops (try naming, roll back,
try different policy).

ADR 0026 probed gradient-descent refinement (sigmoid-gated edge
selection + analytical gradient + multi-start). Verdict: usable
but not cheaper than random re-sample on β-scale graphs;
implementation was removed per minimum-first, ADR and log retained
as record.

ADR 0027 added **axiom discovery** (intensional inference): from a
bounded template space of positive-implication axioms
(≤ 2-edge premise, 1-edge conclusion, ≤ 3 variables), enumerate
and evaluate against the RSet. Discovers transitivity on a
poset, symmetry on a symmetric relation. `check_poset()` returns
a three-axiom verdict. First v2 mechanism that reaches first-order
axiomatic properties; axioms live as Rust values, not yet encoded
as meta-R.

ADR 0028 added **axiom subsumption** on top of 0027: structural
template canonicalization (permutation-aware, collapsing transitivity's
two previous forms into one), subsumption by universal reflexivity
(drops trivially-true `⇒ R(v,v)` conclusions), and subsumption by
premise weakening (drops axioms dominated by a stronger-premise
variant). Exposed as `RSet::discover_axioms_minimal`. On the rigorous
8-case blind battery: 45 → 5 on equivalence, 37 → 1 on tolerance,
25 → 1 on total order; other cases unchanged or clean already.

ADR 0029 made commitment 3 land properly by adding an **intension /
extension split** to pattern naming. Layer A (always written) stores
the type itself — registry, roles, structural edges among roles —
so a type's definition is fully present in meta-R without depending
on any surviving instance. Layer B (configurable via
`PatternRecordingPolicy::{Intensional, InstancesOnly, FullBindings}`)
controls how much per-instance extension data is also written.
`FullBindings` remains the default for backward compatibility.
`find_pattern_matching` now reads Layer A directly; legacy RSets
still work via the old first-instance recovery fallback.
Constitution gained a clarifying footnote: commitment 3 is about the
type's *intension*, extension is instrumentation policy.

ADR 0030 added **theory objects** (conjunctive concept naming) as
the jump from individual axioms to theory-level structure. Each
axiom carries a deterministic id — template-based axioms serialize
from their canonical form, predicate axioms use fixed strings
(`ax_reflexivity`, `ax_antisymmetry`). `discover_theory` returns
the minimal axiom bundle that holds on the RSet; `name_theory`
verifies membership and writes `R(__theory__, t_N)` +
`R(t_N, ax_i)` membership edges, reusing existing ids when the
member set matches. Fingerprints on the 8-case battery separate
strict partial order from total order from equivalence from
tolerance cleanly, by structural identity alone. Axiom intension
itself stays name-only until task B.

ADR 0031 added **intrinsic drive + global evaluation**. The system
now has a scalar `abstraction_score` (reuse savings + theory
richness − meta-R overhead tax) and a `drive_step` / `intrinsic_drive`
loop that explores its own action space (pattern discovery at
configurable sizes, theory discovery), applies the best-improving
action per step, and halts on saturation. First v2 mechanism where
the system chooses *what* to do and *when to stop* from its own
value signal, with no external trigger. On four different-shaped
inputs the drive picks action orders that reflect the input (structure-
rich → patterns first, rule-rich → theory first); final score
discriminates structured inputs (~14–15) from unstructured (~4).

ADR 0032 gave template axioms their own intension in meta-R via a
chain encoding `var_x → edge_node → var_y` (direction of R encodes
source vs. target — no extra markers per edge needed). New reserved
markers `__axiomvar__`, `__premise__`, `__conclusion__`. Full
reconstruction roundtrip verified for transitivity and symmetry.
`register_axiom_with_intension` is called automatically from
`name_theory`; `retract_axiom` tears the intension down. Commitment 3
now lands for every named meta-R object in v2 (patterns, theories,
template axioms) — predicate axioms remain registry-only because
the current template language can't express them.

ADR 0033 added **defeasible axioms** via `AxiomDiscoveryConfig::min_rate`
(default 1.0, preserves strict ADR 0027 behavior). Lowering the
threshold admits rules that hold on a fraction of premise bindings,
with rate and support reported on every `AxiomEvidence`. `discover_
axioms_minimal` gates subsumption to strict mode only, since the
soundness arguments behind ADR 0028's subsumption break under
rate < 1.0. On the "almost-transitive" case that previously returned
nothing, defeasible discovery surfaces transitivity at rate 0.667
(support 2/3) — the system can now report "this almost holds"
rather than stay silent.

Open directions (refinements, not completions): multi-size
autonomous passes, attach-only integration, cross-graph pattern
transfer, sampling-based `find_instances_of` replacement,
hierarchical / composed patterns.

See [docs/progress.md](docs/progress.md) for the current frontier.
