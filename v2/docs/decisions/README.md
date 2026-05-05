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
- [0010](0010-pattern-naming-as-meta-r.md) — Pattern naming as meta-R instances (three-shape encoding)
- [0011](0011-meta-r-feedback-probe.md) — Meta-R feedback probe (observation before γ)
- [0012](0012-gamma-naming-pass.md) — γ naming-pass driver and relevance filter (closes β)
- [0013](0013-pattern-query-api.md) — Pattern query API (first use of named meta-R)
- [0014](0014-attach-only-mode.md) — Attach-only mode for naming pass
- [0015](0015-subgraph-matching.md) — Subgraph matching against named patterns (fixes fragmentation)
- [0016](0016-motif-discovery-via-sampling.md) — Motif discovery via sample-score-select (first non-enumeration search)
- [0017](0017-representative-refinement.md) — Representative refinement via targeted re-sampling
- [0018](0018-autonomous-pass.md) — Autonomous pass: compose discover + refine + name to close the abstraction loop
- [0019](0019-mdl-scoring.md) — MDL-gain scoring (opt-in reusability filter)
- [0020](0020-pattern-retraction.md) — Pattern retraction (remove named patterns)
- [0021](0021-autonomous-sweep.md) — Multi-size autonomous sweep
- [0022](0022-autonomous-and-attach.md) — Autonomous + attach composition (incremental workflow)
- [0023](0023-cross-graph-transfer.md) — Cross-graph pattern transfer (canonical library)
- [0024](0024-sample-instances.md) — Sampling-based `sample_instances_of`
- [0025](0025-hierarchical-discovery-probe.md) — Hierarchical discovery probe (opt-in meta inclusion)
- [0026](0026-gradient-refine-probe.md) — Gradient-descent refinement probe
- [0027](0027-axiom-discovery-probe.md) — Axiom discovery probe (extensional → intensional)
- [0028](0028-axiom-subsumption.md) — Axiom subsumption (structural canonicalization + redundancy filters)
- [0029](0029-intension-extension-split.md) — Intension vs extension for pattern naming (partial supersession of 0010)
- [0030](0030-theory-objects.md) — Theory objects (conjunctive concept naming)
- [0031](0031-intrinsic-drive.md) — Intrinsic drive + global abstraction score
- [0032](0032-axiom-intension.md) — Axiom intension as meta-R (extends 0030)
- [0033](0033-defeasible-axioms.md) — Defeasible axioms (rate < 1.0 with support threshold)
- [0034](0034-theory-extension-relations.md) — Theory extension relations (first higher-order meta-R)
- [0035](0035-counterfactual-value.md) — Counterfactual value / meta-metric (second-order signal)
- [0036](0036-empty-premise-templates.md) — Empty-premise templates (reflexivity as template)
- [0037](0037-compositional-subsumption.md) — Compositional subsumption (forward-chaining derivation)
- [0038](0038-persistence.md) — RSet text persistence
- [0039](0039-totality-predicate.md) — Totality as predicate axiom
- [0040](0040-auto-prune.md) — Drive auto-prune via counterfactual value
- [0041](0041-scale-benchmark.md) — Scale benchmark (measurement only)
- [0042](0042-theory-independence.md) — Theory independence relations
- [0043](0043-indexed-rset-and-sampling-path.md) — Indexed RSet + sampling-path for autonomous_pass
- [0044](0044-extended-template-language.md) — Extended template language (equality + disjunctive conclusions)
- [0045](0045-axiom-confidence.md) — Axiom confidence (Wilson score + null-baseline probability)
- [0046](0046-theory-parallel.md) — Theory parallel relations
- [0047](0047-extended-axiom-ids.md) — Extended axiom id codec (equality + disjunctive)
- [0048](0048-confidence-filters.md) — Confidence thresholds in AxiomDiscoveryConfig
- [0049](0049-theory-relation-classifier.md) — Theory relation classifier + neighborhood
- [0050](0050-sampling-scale-benchmark.md) — Large-scale sampling-mode benchmark
- [0051](0051-adaptive-drive-config.md) — Adaptive drive config (RSet-aware auto-tuning)
- [0052](0052-autonomous-runtime-architecture.md) — Autonomous runtime architecture (Proposed)
- [0063](0063-drive-self-modification.md) — Drive self-modification (Phase H2, Proposed)
- [0064](0064-drives-as-meta-r.md) — Drives as meta-R objects (Phase H2.1, Proposed)
- [0065](0065-ucb-composite-selection.md) — UCB1 composite selection (Phase Alpha-1, Accepted with negative empirical finding)
- [0066](0066-theory-tournament.md) — Theory self-play tournament (Phase Alpha-3, Accepted with strong positive empirical finding)
- [0067](0067-source-tree-refactor.md) — Source tree refactor (Phase Alpha-9 cleanup)
- [0068](0068-axiom-shape-families.md) — Axiom shape families (Phase Beta-1, first runtime extension of structural vocabulary post-H1)
- [0069](0069-identifier-minting.md) — Identifier minting / generative axioms (Phase G, contract for growing the identifier space)
- [0070](0070-shape-family-abstraction-layer.md) — Shape-family abstraction layer (consolidation of B.2-B.8.1 + F.1.1 into a formal cognitive layer; supersedes 0068's narrower scope)
- [0071](0071-unified-theory-quality-report.md) — Unified theory-quality report (Level 1.5 — facts surface; consolidates primary + cross + family + neighborhood signals; gates ADR 0072's intervention classifier)
- [0072](0072-intervention-policy-classifier.md) — Intervention policy classifier (consolidates 6 scattered intervention types into RecommendedIntervention enum + recommend_intervention() classifier; completes the 0070/0071/0072 consolidation triad)
- [0073](0073-phase-pivot-concept-emergence.md) — v2 phase pivot from concept curation (0070-0072) to concept emergence (E1 shape mining + E2 object lifting + E3 intrinsic drive); records the pivot only, not the implementation
- [0074](0074-phase-emergence-1-shape-co-occurrence-mining.md) — Phase Emergence-1: shape co-occurrence mining (concept lifting). Mints second-order concepts from co-occurring shape-families across Signal-class theories; validates via cross-precision; registers as meta-R. **Accepted, shipped 2026-05-06.** First concept (`concept_4c2d2fde3b2d8360`) minted on OQ#1 + long5k with portable identity.
