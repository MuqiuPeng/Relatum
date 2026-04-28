//! Type definitions for motif discovery, axiom templates,
//! axiom evidence, drive actions/steps/traces, and the intrinsic
//! drive configuration. Pure data — no methods on RSet here.
//!
//! ADR 0016, 0027, 0030, 0031, 0044, 0045, 0047.

use crate::{
    AutonomousConfig, CanonicalForm, NamingPolicy, RefinementConfig,
    SamplingMatchConfig, Subgraph,
};

/// Configuration for motif discovery. ADR 0016.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryConfig {
    /// Edge count of each candidate subgraph.
    pub target_size: usize,
    /// Number of random-walk candidates to propose.
    pub sample_count: usize,
    /// Keep the top-M distinct canonical forms by score.
    pub top_m: usize,
    /// Seed for the inline xorshift64 PRNG. Zero is replaced with a
    /// non-zero default internally so that sampling always runs.
    pub rng_seed: u64,
    /// ADR 0025 probe: when true, `discover_motifs` samples from
    /// *all* edges (data + meta-R) rather than only data edges.
    /// Defaults to `false`; leave off unless probing for hierarchical
    /// structure.
    pub include_meta_in_discovery: bool,
}

/// A motif candidate found by `discover_motifs`. ADR 0016.
#[derive(Debug, Clone)]
pub struct MotifCandidate {
    pub canonical: CanonicalForm,
    pub representative: Subgraph,
    pub sample_frequency: usize,
    pub score: f64,
}

// GradientRefineConfig removed alongside the gradient refinement
// primitives. See ADR 0026 and its log for rationale.

/// Axiom discovery: template of a single edge with variable indices
/// on both endpoints. ADR 0027.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EdgeTemplate {
    pub x_var: usize,
    pub y_var: usize,
}

/// Axiom discovery: a rule `premise ⇒ conclusion` where premise is a
/// conjunction of edge templates and conclusion is a single edge.
/// Variables are identified by small integer indices 0..num_vars.
/// ADR 0027.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AxiomTemplate {
    pub num_vars: usize,
    pub premise: Vec<EdgeTemplate>,
    pub conclusion: EdgeTemplate,
}

/// Axiom discovery — equality-conclusion family. ADR 0044. A rule
/// `premise ⇒ (v_a = v_b)`. The canonical instance is antisymmetry:
/// `R(0, 1) ∧ R(1, 0) ⇒ v_0 = v_1`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EqualityAxiomTemplate {
    pub num_vars: usize,
    pub premise: Vec<EdgeTemplate>,
    pub equal_vars: (usize, usize),
}

/// Axiom discovery — disjunctive-conclusion family. ADR 0044. A rule
/// `premise ⇒ R(c_1) ∨ R(c_2) ∨ …`. The canonical instance is
/// totality: `(empty) ⇒ R(0, 1) ∨ R(1, 0)`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DisjunctiveAxiomTemplate {
    pub num_vars: usize,
    pub premise: Vec<EdgeTemplate>,
    pub conclusions: Vec<EdgeTemplate>,
}

/// ADR 0044: unified evidence for an extended-axiom-family template.
#[derive(Debug, Clone)]
pub enum ExtendedAxiomEvidence {
    Edge(AxiomEvidence),
    Equality {
        template: EqualityAxiomTemplate,
        premise_bindings: usize,
        conclusion_satisfied: usize,
        rate: f64,
    },
    Disjunctive {
        template: DisjunctiveAxiomTemplate,
        premise_bindings: usize,
        conclusion_satisfied: usize,
        rate: f64,
    },
}

impl ExtendedAxiomEvidence {
    pub fn rate(&self) -> f64 {
        match self {
            ExtendedAxiomEvidence::Edge(e) => e.rate,
            ExtendedAxiomEvidence::Equality { rate, .. } => *rate,
            ExtendedAxiomEvidence::Disjunctive { rate, .. } => *rate,
        }
    }
    pub fn premise_bindings(&self) -> usize {
        match self {
            ExtendedAxiomEvidence::Edge(e) => e.premise_bindings,
            ExtendedAxiomEvidence::Equality { premise_bindings, .. } => *premise_bindings,
            ExtendedAxiomEvidence::Disjunctive { premise_bindings, .. } => *premise_bindings,
        }
    }
}

/// Axiom discovery: evidence for one template against an RSet.
/// ADR 0027. Extended by ADR 0045 with Bayesian posterior (Wilson
/// score) fields and `null_baseline_prob` for significance filtering.
#[derive(Debug, Clone)]
pub struct AxiomEvidence {
    pub template: AxiomTemplate,
    pub premise_bindings: usize,
    pub conclusion_satisfied: usize,
    pub rate: f64,
    /// Lower bound of the 95% Wilson score confidence interval on the
    /// posterior rate. For support N and successes s, this is an
    /// interval-estimate of the "true" rate that corrects the raw
    /// `s/N` estimator for small N. ADR 0045.
    pub posterior_lower_95: f64,
    /// Upper bound of the 95% Wilson score confidence interval.
    pub posterior_upper_95: f64,
    /// Null-baseline probability: if edges were iid Bernoulli with
    /// p = |data edges| / |ids|², the chance of seeing this template
    /// hold on all N premise bindings by accident. A small value
    /// (e.g. < 0.01) indicates the axiom is statistically surprising
    /// relative to a random-edge null hypothesis. ADR 0045.
    pub null_baseline_prob: f64,
}

/// Configuration for axiom discovery. ADR 0027, extended by ADR
/// 0033 (defeasible rules) and ADR 0036 (empty-premise templates).
#[derive(Debug, Clone)]
pub struct AxiomDiscoveryConfig {
    pub max_premise_edges: usize,
    pub max_vars: usize,
    pub min_evidence: usize,
    /// Minimum satisfaction rate for a template to be reported. Default
    /// `1.0` preserves ADR 0027's strict "holds universally" semantics.
    /// Lowering it to e.g. `0.8` admits defeasible rules that hold on
    /// ≥ 80% of premise bindings. ADR 0033.
    pub min_rate: f64,
    /// When true, enumerate empty-premise templates `[] ⇒ R(0,0)`
    /// (single-variable self-loop conclusion — universally quantified
    /// reflexivity in template form). Default `false` preserves
    /// ADR 0027's "premise must have at least one edge" shape. ADR 0036.
    pub include_empty_premise: bool,
    /// Minimum posterior lower-95 bound for an axiom to be reported
    /// (uses ADR 0045's Wilson score). Default `0.0` = no filter.
    /// Raising to e.g. 0.8 rejects axioms that are technically at
    /// rate 1.0 but have too few premise bindings for meaningful
    /// confidence. ADR 0048.
    pub min_posterior_lower: f64,
    /// Maximum null-baseline probability (iid-Bernoulli-edges null)
    /// for an axiom to be reported. Default `1.0` = no filter.
    /// Lowering to e.g. 0.01 rejects axioms whose rate = 1.0 is
    /// explainable as a coincidence under uniform-random edges.
    /// ADR 0048.
    pub max_null_baseline: f64,
}

impl Default for AxiomDiscoveryConfig {
    fn default() -> Self {
        AxiomDiscoveryConfig {
            max_premise_edges: 2,
            max_vars: 3,
            min_evidence: 1,
            min_rate: 1.0,
            include_empty_premise: false,
            min_posterior_lower: 0.0,
            max_null_baseline: 1.0,
        }
    }
}

/// Reflexivity check result. ADR 0027. Does not fit the positive-
/// implication template (premise "x is an identifier" has no edge).
#[derive(Debug, Clone)]
pub struct ReflexivityEvidence {
    pub identifiers_total: usize,
    pub self_loops_present: usize,
    pub rate: f64,
}

/// Antisymmetry check result. ADR 0027. Does not fit the template
/// (conclusion "x = y" is not an edge).
#[derive(Debug, Clone)]
pub struct AntisymmetryEvidence {
    pub directed_pairs_checked: usize,
    pub violations: usize,
    pub holds: bool,
}

/// Totality check result. ADR 0039. Does not fit the template
/// (disjunctive conclusion).
#[derive(Debug, Clone)]
pub struct TotalityEvidence {
    pub unordered_pairs_checked: usize,
    pub violations: usize,
    pub holds: bool,
}

/// Combined poset check. ADR 0027.
#[derive(Debug, Clone)]
pub struct PosetCheck {
    pub reflexive: ReflexivityEvidence,
    pub antisymmetric: AntisymmetryEvidence,
    pub transitive: Option<AxiomEvidence>,
    pub is_poset: bool,
}

/// ADR 0031 (task C): one candidate action the intrinsic drive may
/// try. The list of candidates is produced by `DriveConfig::candidate_actions`.
#[derive(Debug, Clone)]
pub enum DriveAction {
    /// Run `autonomous_pass` with the given config.
    DiscoverPatterns(AutonomousConfig),
    /// Run `discover_theory` + `name_theory`.
    DiscoverTheory(AxiomDiscoveryConfig),
    /// Retract every named object whose counterfactual value is
    /// strictly below `threshold`. ADR 0040.
    Prune(f64),
}

/// ADR 0031: structured outcome of one applied drive action.
#[derive(Debug, Clone)]
pub enum DriveActionResult {
    PatternsDiscovered {
        target_size: usize,
        new_patterns: usize,
    },
    TheoryDiscovered {
        theory_id: Option<String>,
        member_count: usize,
    },
    Pruned {
        object_ids: Vec<String>,
    },
}

/// ADR 0031: one step of the intrinsic drive, after it has been
/// applied to the RSet.
#[derive(Debug, Clone)]
pub struct DriveStep {
    pub action: DriveAction,
    pub score_before: f64,
    pub score_after: f64,
    pub delta: f64,
    pub result: DriveActionResult,
}

/// ADR 0031: the full trace of an `intrinsic_drive` run.
#[derive(Debug, Clone)]
pub struct DriveTrace {
    pub initial_score: f64,
    pub final_score: f64,
    pub steps: Vec<DriveStep>,
}

/// ADR 0031: configuration for the intrinsic drive. The driver
/// explores every action in `candidate_actions()` each step and
/// applies the best-improving one if it exceeds `epsilon`. Loops
/// for at most `max_steps` iterations.
#[derive(Debug, Clone)]
pub struct DriveConfig {
    pub pattern_sizes: Vec<usize>,
    pub discovery_config: DiscoveryConfig,
    pub refinement_config: RefinementConfig,
    pub naming_policy: NamingPolicy,
    pub axiom_config: AxiomDiscoveryConfig,
    pub max_steps: usize,
    pub epsilon: f64,
    /// When true, include `DriveAction::Prune(prune_threshold)` as a
    /// candidate action each step. ADR 0040. Default `true`.
    pub enable_prune: bool,
    /// Counterfactual-value threshold below which Prune retracts an
    /// object. Default `0.0` (retract only net-negative contributors).
    pub prune_threshold: f64,
    /// When `Some(cfg)`, pattern-discovery actions route through
    /// `sample_instances_of` instead of exhaustive `find_instances_of`
    /// — trades completeness for tractability on large graphs.
    /// ADR 0043. Default `None`.
    pub instance_sampling: Option<SamplingMatchConfig>,
}

impl DriveConfig {
    pub(crate) fn candidate_actions(&self) -> Vec<DriveAction> {
        let mut out: Vec<DriveAction> = self
            .pattern_sizes
            .iter()
            .map(|&size| {
                let mut d = self.discovery_config.clone();
                d.target_size = size;
                DriveAction::DiscoverPatterns(AutonomousConfig {
                    discovery: d,
                    refinement: self.refinement_config.clone(),
                    naming: self.naming_policy.clone(),
                    instance_sampling: self.instance_sampling.clone(),
                })
            })
            .collect();
        out.push(DriveAction::DiscoverTheory(self.axiom_config.clone()));
        if self.enable_prune {
            out.push(DriveAction::Prune(self.prune_threshold));
        }
        out
    }
}

impl Default for DriveConfig {
    fn default() -> Self {
        DriveConfig {
            pattern_sizes: vec![2, 3, 4],
            discovery_config: DiscoveryConfig {
                target_size: 3,
                sample_count: 200,
                top_m: 10,
                rng_seed: 2024,
                include_meta_in_discovery: false,
            },
            refinement_config: RefinementConfig {
                max_tries: 200,
                rng_seed: 999,
            },
            naming_policy: NamingPolicy::default(),
            axiom_config: AxiomDiscoveryConfig::default(),
            max_steps: 10,
            epsilon: 0.0,
            enable_prune: true,
            prune_threshold: 0.0,
            instance_sampling: None,
        }
    }
}
