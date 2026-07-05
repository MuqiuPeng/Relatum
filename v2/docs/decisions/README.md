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
- [0053](0053-selective-declarativization.md) — Selective declarativization (M1)
- [0054](0054-meta-meta-pattern-discovery.md) — Meta-meta-pattern discovery (Phase D)
- [0055](0055-canonical-direction-distinction.md) — Direction-distinguishing canonical form (Phase E0)
- [0056](0056-phase-d-verification-battery.md) — Phase D verification battery (Phase F0)
- [0057](0057-anomaly-coverage-drive.md) — Anomaly-coverage drive (Phase G0; system-level effect bounded — null result documented)
- [0058](0058-axiom-forward-application.md) — Axiom forward-application semantics (Phase G1.0)
- [0059](0059-prediction-error-drive.md) — Prediction-error drive (Phases G1.3–G1.5)
- [0060](0060-meta-mechanism.md) — Meta-mechanism: runtime self-tuning via prediction error (Phase H0)
- [0061](0061-action-sequence-mining.md) — Action-sequence mining (Phases H1.0–H1.2)
- [0062](0062-sequence-demotion-and-ngrams.md) — Sequence demotion + N-gram extension (Phases H1.3/H1.4)
- [0063](0063-drive-self-modification.md) — Drive self-modification (Phase H2; H2.0 shipped through step 3b, H2.2 never implemented — frozen per 0084)
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
- [0074](0074-phase-emergence-1-shape-co-occurrence-mining.md) — Phase Emergence-1: shape co-occurrence mining (concept lifting). Mints second-order concepts from co-occurring shape-families across Signal-class theories; validates via cross-precision; registers as meta-R. **Re-classified 2026-05-06 as "implicit conceptualization"** under the constitution's heavy-reading amendment (does not register participating tokens; not a concept-creation act). Useful as a curatorial layer. Successor: ADR 0075 audit identifying the existing pattern pipeline as the genuine concept-creation kernel.
- [0075](0075-emergence-kernel-audit-and-runtime-integration.md) — Emergence kernel audit + runtime integration. Audits `discover_motifs` + `name_pattern_instances` + `autonomous_pass` (ADRs 0009/0010/0016/0018/0029) against the constitution's strict reading; finds the existing pipeline is already a compliant concept-creation kernel (atomic mint + explicit participant registration). Specifies runtime integration: promote `DiscoverPatterns` priority in scheduler. Empirical: 172 instances on OQ#2 (the substrate axiom path skipped). Proposed; piece 1 (audit) shipped, pieces 2 (scheduler) and 3 (canonical-form comparison) pending.
- [0076](0076-micro-agent-reframing.md) — Micro-agent reframing — transient agents over the episode log. Re-reads v2's existing dispatch system as a multi-agent cognitive substrate without extending ontology: each `(ActionKind, target-kind)` pair is an agent class, each `Episode` an agent instance, all "agent state" derived from queries. Constitution-compliant under heavy reading. Phase 0 (ADR) + phase 1 (query helpers + audit) + phase 2 (episode-log enrichments: outcome distribution, temporal density, target overlap) shipped 2026-05-06.
- [0077](0077-pattern-quality-and-intervention.md) — Pattern quality framework + intervention recommendations. Mirrors ADR 0071/0072 for patterns: `PatternQualityReport` (canonical_size, instance_count, mdl_gain, overlap_score, summary_class) + `recommend_pattern_intervention` (None / ShadowMonitor / PatternRetract / PatternMergeWith / Manual). 5-class taxonomy (Signal / Mixed / Redundant / Anomalous / Indeterminate). Shipped 2026-05-06; empirically identifies OQ#2's 84-instance 3-cycle as the highest-MDL Signal pattern in v2 (mdl_gain 249). 2026-05-07: added `sample_instances_of`-based cross-substrate validation (was deferred at first ship).
- [0078](0078-pattern-aware-drive-metric.md) — Pattern-aware drive metric (constitution-compliant). `UnexplainedDriveSignal` groups unexplained R by connected-component canonical form (subgraph-level, never per-token). Replaces the withdrawn 2026-05-06 first form (per-edge `EdgeFingerprint`, forbidden by heavy reading). Shipped 2026-05-07; reveals OQ#2 leaves **91% of edges unexplained at maturity** organized into 5 canonical buckets matching its stream regimes. Scheduler integration deferred to follow-up ADR.
- [0079](0079-drive-driven-frontier-candidate.md) — Drive→scheduler integration (sustained cognition). Three coordinated changes: drive-driven `PatternCandidate` in `Frontier::refresh`, drive-wake in `run_bounded` sleep short-circuit, drive bypass in `RuleBasedScheduler` stagnation gate. Shipped 2026-05-08; v2 crosses **reactive→proactive**: OQ#2 jumps from 2 patterns / 10 episodes (single-shot) to 7 patterns / 24 episodes (sustained). OQ#1-clade unchanged (their drive is silent, bypass doesn't engage). Partial fix — Phase 3 freeze observed at long horizon, resolved by ADR 0079.1.
- [0079.1](0079-1-drive-aware-thrash-bypass.md) — Drive-aware mode-thrash bypass. The OQ#2 long-horizon observation revealed Phase 3 freeze (wake-on-drive triggered but `would_thrash` gate sent runtime back to Sleep without dispatching). 5-line bypass in `switch_or_sleep` mirrors ADR 0079's stagnation bypass. Shipped 2026-05-08; second-half episodes +14 (was 0), pattern instances +11 (was 0), drive metric now drains as a thermostat (stays at 0 throughout) instead of plateau-ing at 124. Pattern count remains 7 — that's the structural canonical ceiling, the correct constitution-compliant upper bound.
- [0080](0080-learning-progress-aware-drive.md) — Learning-progress-aware drive (LP gates on canonical-bucket unexplained counts; thresholds tuned 2026-05-19: LP_WINDOW 30→10, LP_DRIVE_THRESHOLD 0.05→0.20)
- [0081](0081-vibe-proving-bridge.md) — vibe-proving bridge / external substrate ingestion (Phase 0 ran on synthetic layered random DAG; headline substrate-sensitivity claim retracted via 8/9-round ARIS review; closed by 0084, folded into N1 real Mathlib)
- [0082](0082-recommendation-execution-loop.md) — Recommendation execution loop (theory-side policy execution; shipped + verified — runtime autonomously demoted t_0 on OQ#1 at tick 511, stable through tick 2400)
- [0083](0083-pattern-policy-execution-loop.md) — Pattern policy execution loop (pattern-side mirror of 0082; PatternRetract executable, PatternMergeWith skipped pending merge API)
- [0084](0084-direction-housekeeping.md) — Direction housekeeping: freeze H2.2 + C.3a–d (handed to v3), close Alpha-2 MCTS + vibe-proving bridge (folded into N1), formalize N3/N5 reopen triggers, declare v3 additive-noise out of scope, errata + index backfill. Every item carries a written reopen trigger.
