//! Scoring functions for evaluating rules and closures.
//!
//! Provides multiple dimensions for measuring the "value" of inference rules
//! and their derived conclusions, enabling loss-driven search over rule space.
//!
//! # Score dimensions
//!
//! | Metric | Measures | Use |
//! |--------|----------|-----|
//! | Generativity | New facts produced per unit complexity | Rule discovery |
//! | Compression | Existing facts explained per unit complexity | Pattern extraction |
//! | Ablation | Facts lost when a rule is removed | Rule importance |
//! | Depth profile | Distribution of term complexity in closure | Structural richness |

use std::collections::HashSet;

use super::engine::{ClosureEngine, ClosureResult};
use super::relation::Relation;
use super::rule::{self, RelationPattern, Rule, Substitution};
use super::term::Term;

// ── Structural complexity ───────────────────────────────────

/// Count the number of nodes in a term tree.
/// Atoms and variables count as 1; compounds count as 1 + sum of args.
pub fn term_size(t: &Term) -> usize {
    match t {
        Term::Var(_) => 1,
        Term::App { args, .. } => 1 + args.iter().map(term_size).sum::<usize>(),
    }
}

/// Total structural size of a relation pattern (relation name + all terms).
pub fn pattern_size(p: &RelationPattern) -> usize {
    1 + p.terms().iter().map(|t| term_size(t)).sum::<usize>()
}

/// Total structural size of a rule (all premises + all conclusions).
pub fn rule_complexity(r: &Rule) -> usize {
    let premises: usize = r.premises().iter().map(pattern_size).sum();
    let conclusions: usize = r.conclusions().iter().map(pattern_size).sum();
    premises + conclusions
}

// ── Rule-level scoring ──────────────────────────────────────

/// Generativity: how many new facts does adding this rule produce,
/// normalized by rule complexity.
///
/// `base_facts` is the current fact set (before adding the rule).
/// The engine is cloned, the rule is added, closure is re-derived,
/// and the delta is measured.
///
/// Returns `(delta_facts, raw_generativity)` where
/// `raw_generativity = delta / complexity`.
pub fn generativity(engine: &ClosureEngine, rule: &Rule) -> (usize, f64) {
    let baseline = engine.facts().len();
    let complexity = rule_complexity(rule).max(1);

    let mut trial = engine.clone();
    trial.add_rule(rule.clone());
    let result = trial.derive_closure();

    let total = result.facts.len();
    let delta = total.saturating_sub(baseline);
    (delta, delta as f64 / complexity as f64)
}

/// Compression: how many existing facts are instances of this rule's
/// conclusion patterns, normalized by rule complexity.
///
/// A high compression score means the rule "explains" many existing facts
/// with a compact pattern — it captures a regularity in the fact set.
///
/// Returns `(matched_facts, raw_compression)`.
pub fn compression(rule: &Rule, facts: &HashSet<Relation>) -> (usize, f64) {
    let complexity = rule_complexity(rule).max(1);
    let mut matched = 0usize;

    for conclusion in rule.conclusions() {
        for fact in facts {
            let mut sub = Substitution::new();
            if rule::match_relation(conclusion, fact, &mut sub) {
                matched += 1;
            }
        }
    }

    (matched, matched as f64 / complexity as f64)
}

/// Ablation: how many facts are lost if this rule is removed from
/// the engine, normalized by rule complexity.
///
/// `rule_index` identifies which rule to remove (by position in `engine.rules()`).
/// The engine is cloned, the rule is removed, closure is re-derived,
/// and the deficit is measured.
///
/// Returns `(lost_facts, raw_ablation)`.
pub fn ablation(engine: &ClosureEngine, rule_index: usize) -> (usize, f64) {
    // Full closure with all rules
    let mut full = engine.clone();
    let full_result = full.derive_closure();
    let full_count = full_result.facts.len();

    let removed_rule = &engine.rules()[rule_index];
    let complexity = rule_complexity(removed_rule).max(1);

    // Closure without the target rule
    let mut reduced = engine.clone();
    reduced.remove_rule(rule_index);
    let reduced_result = reduced.derive_closure();
    let reduced_count = reduced_result.facts.len();

    let lost = full_count.saturating_sub(reduced_count);
    (lost, lost as f64 / complexity as f64)
}

// ── Consistency checking ─────────────────────────────────────

/// A pair of relation names that are mutually exclusive on the same arguments.
/// E.g., `("tv_t", "tv_f")` means `tv_t(v, f)` and `tv_f(v, f)` cannot both hold.
pub type ExclusionPair = (String, String);

/// Count facts that violate mutual exclusion constraints.
///
/// For each exclusion pair `(r1, r2)`, counts how many argument tuples `(t1, ..., tn)`
/// appear in both `r1(t1,...,tn)` and `r2(t1,...,tn)`.
pub fn inconsistency_count(facts: &[Relation], exclusions: &[ExclusionPair]) -> usize {
    let mut total = 0;
    for (r1, r2) in exclusions {
        let r1_args: HashSet<Vec<&Term>> = facts
            .iter()
            .filter(|f| f.name() == r1)
            .map(|f| f.terms().iter().collect())
            .collect();
        total += facts
            .iter()
            .filter(|f| f.name() == r2)
            .filter(|f| r1_args.contains(&f.terms().iter().collect::<Vec<_>>()))
            .count();
    }
    total
}

// ── Closure-level scoring ───────────────────────────────────

/// Summary metrics for an entire closure result.
#[derive(Debug, Clone)]
pub struct ClosureProfile {
    /// Total facts (initial + derived).
    pub total_facts: usize,
    /// Only derived facts.
    pub derived_facts: usize,
    /// Number of distinct relation names in derived facts.
    pub relation_diversity: usize,
    /// Distribution of max term depth across derived facts.
    /// `depth_histogram[d]` = number of derived facts whose deepest term has depth `d`.
    pub depth_histogram: Vec<usize>,
    /// Mean max-term-depth across derived facts.
    pub mean_depth: f64,
    /// Whether closure reached a fixed point.
    pub saturated: bool,
    /// Number of rounds to converge.
    pub rounds: usize,
    /// Number of facts violating mutual exclusion constraints.
    pub inconsistencies: usize,
}

/// Compute a structural profile of a closure result.
pub fn closure_profile(result: &ClosureResult) -> ClosureProfile {
    closure_profile_with_exclusions(result, &[])
}

/// Compute a structural profile with mutual exclusion checking.
pub fn closure_profile_with_exclusions(
    result: &ClosureResult,
    exclusions: &[ExclusionPair],
) -> ClosureProfile {
    let mut rel_names: HashSet<&str> = HashSet::new();
    let mut depth_hist: Vec<usize> = Vec::new();
    let mut depth_sum = 0usize;

    for fact in &result.derived {
        rel_names.insert(fact.name());
        let max_d = fact.terms().iter().map(|t| t.depth()).max().unwrap_or(0);
        if max_d >= depth_hist.len() {
            depth_hist.resize(max_d + 1, 0);
        }
        depth_hist[max_d] += 1;
        depth_sum += max_d;
    }

    let n = result.derived.len().max(1);
    let incon = inconsistency_count(&result.facts, exclusions);

    ClosureProfile {
        total_facts: result.facts.len(),
        derived_facts: result.derived.len(),
        relation_diversity: rel_names.len(),
        depth_histogram: depth_hist,
        mean_depth: depth_sum as f64 / n as f64,
        saturated: result.saturated,
        rounds: result.rounds,
        inconsistencies: incon,
    }
}

// ── Combined scoring ────────────────────────────────────────

/// Weights for combining score dimensions.
#[derive(Debug, Clone)]
pub struct ScoreWeights {
    pub generativity: f64,
    pub compression: f64,
    /// Penalty per inconsistency (subtracted from combined score).
    pub consistency_penalty: f64,
    /// Mutual exclusion pairs for consistency checking.
    pub exclusions: Vec<ExclusionPair>,
}

impl Default for ScoreWeights {
    fn default() -> Self {
        ScoreWeights {
            generativity: 1.0,
            compression: 1.0,
            consistency_penalty: 0.0,
            exclusions: Vec::new(),
        }
    }
}

impl ScoreWeights {
    /// Create weights with consistency checking enabled for a set of exclusion pairs.
    pub fn with_consistency(mut self, exclusions: Vec<ExclusionPair>, penalty: f64) -> Self {
        self.exclusions = exclusions;
        self.consistency_penalty = penalty;
        self
    }
}

/// Combined evaluation of a rule against the current engine state.
#[derive(Debug, Clone)]
pub struct RuleEvaluation {
    pub rule_name: String,
    pub complexity: usize,
    /// Number of new facts this rule produces.
    pub delta_facts: usize,
    /// Generativity = delta_facts / complexity.
    pub generativity: f64,
    /// Number of existing facts matching the conclusion pattern.
    pub matched_facts: usize,
    /// Compression = matched_facts / complexity.
    pub compression: f64,
    /// Number of mutual-exclusion violations introduced by this rule.
    pub inconsistencies: usize,
    /// Weighted combination (after consistency penalty).
    pub combined: f64,
}

/// Evaluate a rule on all dimensions and combine with weights.
pub fn evaluate_rule(
    engine: &ClosureEngine,
    rule: &Rule,
    weights: &ScoreWeights,
) -> RuleEvaluation {
    let complexity = rule_complexity(rule);
    let (delta, gen) = generativity(engine, rule);
    let (matched, comp) = compression(rule, engine.facts());

    // Consistency: run trial closure and check for violations
    let incon = if !weights.exclusions.is_empty() {
        let mut trial = engine.clone();
        trial.add_rule(rule.clone());
        let result = trial.derive_closure();
        let baseline = inconsistency_count(
            &engine.facts().iter().cloned().collect::<Vec<_>>(),
            &weights.exclusions,
        );
        let after = inconsistency_count(&result.facts, &weights.exclusions);
        after.saturating_sub(baseline)
    } else {
        0
    };

    let combined = weights.generativity * gen + weights.compression * comp
        - weights.consistency_penalty * incon as f64;

    RuleEvaluation {
        rule_name: rule.name().to_string(),
        complexity,
        delta_facts: delta,
        generativity: gen,
        matched_facts: matched,
        compression: comp,
        inconsistencies: incon,
        combined,
    }
}

/// Evaluate all rules currently in the engine (ablation analysis).
/// Returns evaluations sorted by ablation score descending.
pub fn ablation_analysis(engine: &ClosureEngine) -> Vec<(String, usize, f64)> {
    let mut results = Vec::new();
    for i in 0..engine.rules().len() {
        let name = engine.rules()[i].name().to_string();
        let (lost, score) = ablation(engine, i);
        results.push((name, lost, score));
    }
    results.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    results
}

/// Rank a set of candidate rules by combined score.
/// Returns evaluations sorted by combined score descending.
pub fn rank_candidates(
    engine: &ClosureEngine,
    candidates: &[Rule],
    weights: &ScoreWeights,
) -> Vec<RuleEvaluation> {
    let mut evals: Vec<RuleEvaluation> = candidates
        .iter()
        .map(|r| evaluate_rule(engine, r, weights))
        .collect();
    evals.sort_by(|a, b| b.combined.partial_cmp(&a.combined).unwrap_or(std::cmp::Ordering::Equal));
    evals
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(s: &str) -> Term {
        Term::constant(s)
    }

    #[test]
    fn test_term_size() {
        assert_eq!(term_size(&c("a")), 1);
        assert_eq!(term_size(&Term::var("x")), 1);
        assert_eq!(term_size(&Term::app("f", vec![c("a")])), 2);
        assert_eq!(
            term_size(&Term::app("f", vec![c("a"), Term::app("g", vec![c("b")])])),
            4
        );
    }

    #[test]
    fn test_rule_complexity() {
        // parent(?x, ?y) |- ancestor(?x, ?y)
        let rule = Rule::new(
            "base",
            vec![RelationPattern::new(
                "parent",
                vec![Term::var("x"), Term::var("y")],
            )],
            vec![RelationPattern::new(
                "ancestor",
                vec![Term::var("x"), Term::var("y")],
            )],
        );
        // premises: 1 (rel) + 2 (vars) = 3
        // conclusions: 1 (rel) + 2 (vars) = 3
        assert_eq!(rule_complexity(&rule), 6);
    }

    #[test]
    fn test_generativity_basic() {
        let mut engine = ClosureEngine::new();
        engine.define_relation("p", 1);
        engine.define_relation("q", 1);
        engine.define_variable("x");
        engine.define_constant("a");
        engine.define_constant("b");

        engine.add_fact(Relation::new("p", vec![c("a")]));
        engine.add_fact(Relation::new("p", vec![c("b")]));

        // Rule: p(?x) |- q(?x)
        let rule = Rule::new(
            "p_to_q",
            vec![RelationPattern::new("p", vec![Term::var("x")])],
            vec![RelationPattern::new("q", vec![Term::var("x")])],
        );

        let (delta, score) = generativity(&engine, &rule);
        assert_eq!(delta, 2, "should derive q(a) and q(b)");
        assert!(score > 0.0);
    }

    #[test]
    fn test_compression_basic() {
        let mut engine = ClosureEngine::new();
        engine.define_relation("q", 1);
        engine.define_variable("x");
        engine.define_constant("a");
        engine.define_constant("b");

        // Facts that match q(?x)
        engine.add_fact(Relation::new("q", vec![c("a")]));
        engine.add_fact(Relation::new("q", vec![c("b")]));

        let rule = Rule::new(
            "p_to_q",
            vec![RelationPattern::new("p", vec![Term::var("x")])],
            vec![RelationPattern::new("q", vec![Term::var("x")])],
        );

        let (matched, _score) = compression(&rule, engine.facts());
        assert_eq!(matched, 2, "both q(a) and q(b) match conclusion q(?x)");
    }

    #[test]
    fn test_closure_profile() {
        let mut engine = ClosureEngine::new();
        engine.define_equivalence("eq");
        engine.add_fact(Relation::binary("eq", c("a"), c("b")));
        engine.add_fact(Relation::binary("eq", c("b"), c("c")));

        let result = engine.derive_closure();
        let profile = closure_profile(&result);

        assert!(profile.derived_facts > 0);
        assert_eq!(profile.relation_diversity, 1); // only "eq"
        assert!(profile.saturated);
    }

    #[test]
    fn test_ablation_measures_importance() {
        let mut engine = ClosureEngine::new();
        engine.define_relation("p", 1);
        engine.define_relation("q", 1);
        engine.define_relation("r", 1);
        engine.define_variable("x");
        engine.define_constant("a");

        engine.add_fact(Relation::new("p", vec![c("a")]));

        // Rule 0: p(?x) |- q(?x)  — enables rule 1
        engine.add_rule(Rule::new(
            "p_to_q",
            vec![RelationPattern::new("p", vec![Term::var("x")])],
            vec![RelationPattern::new("q", vec![Term::var("x")])],
        ));
        // Rule 1: q(?x) |- r(?x)  — depends on rule 0
        engine.add_rule(Rule::new(
            "q_to_r",
            vec![RelationPattern::new("q", vec![Term::var("x")])],
            vec![RelationPattern::new("r", vec![Term::var("x")])],
        ));

        // Removing rule 0 should lose more facts (q(a) and r(a))
        let (lost_0, _) = ablation(&engine, 0);
        // Removing rule 1 should lose fewer facts (only r(a))
        let (lost_1, _) = ablation(&engine, 1);

        assert!(
            lost_0 > lost_1,
            "rule 0 (p_to_q) should be more important: lost {} vs {}",
            lost_0,
            lost_1,
        );
    }

    /// Build the propositional logic engine (same as the engine test)
    /// but return it BEFORE running closure, so we can score individual rules.
    fn build_propositional_engine() -> (ClosureEngine, Vec<String>) {
        let mut engine = ClosureEngine::new();

        engine.define_relation("tv_t", 2);
        engine.define_relation("tv_f", 2);
        engine.define_relation("declared", 1);
        engine.define_relation("tautology", 1);
        engine.define_relation("contradiction", 1);

        engine.define_variable("v");
        engine.define_variable("p");
        engine.define_variable("q");
        engine.define_variable("r");
        engine.define_variable("f");

        engine.define_constant("v_tt");
        engine.define_constant("v_tf");
        engine.define_constant("v_ft");
        engine.define_constant("v_ff");
        let p = engine.define_constant("p");
        let q = engine.define_constant("q");

        // Atomic truth assignments
        engine.add_fact(Relation::binary("tv_t", c("v_tt"), p.clone()));
        engine.add_fact(Relation::binary("tv_t", c("v_tt"), q.clone()));
        engine.add_fact(Relation::binary("tv_t", c("v_tf"), p.clone()));
        engine.add_fact(Relation::binary("tv_f", c("v_tf"), q.clone()));
        engine.add_fact(Relation::binary("tv_f", c("v_ft"), p.clone()));
        engine.add_fact(Relation::binary("tv_t", c("v_ft"), q.clone()));
        engine.add_fact(Relation::binary("tv_f", c("v_ff"), p.clone()));
        engine.add_fact(Relation::binary("tv_f", c("v_ff"), q.clone()));

        // Declared formulas
        let neg_p = Term::app("neg", vec![p.clone()]);
        let neg_q = Term::app("neg", vec![q.clone()]);
        let and_pq = Term::app("and", vec![p.clone(), q.clone()]);
        let or_pq = Term::app("or", vec![p.clone(), q.clone()]);
        let imp_pq = Term::app("imp", vec![p.clone(), q.clone()]);
        let imp_pp = Term::app("imp", vec![p.clone(), p.clone()]);
        let or_p_negp = Term::app("or", vec![p.clone(), neg_p.clone()]);
        let and_p_negp = Term::app("and", vec![p.clone(), neg_p.clone()]);
        let imp_andpq_p = Term::app("imp", vec![and_pq.clone(), p.clone()]);
        let imp_p_orpq = Term::app("imp", vec![p.clone(), or_pq.clone()]);
        let imp_p_negp = Term::app("imp", vec![p.clone(), neg_p.clone()]);
        let and_p_imppq = Term::app("and", vec![p.clone(), imp_pq.clone()]);
        let mp = Term::app("imp", vec![and_p_imppq.clone(), q.clone()]);

        for f in &[
            &neg_p, &neg_q, &and_pq, &or_pq, &imp_pq, &imp_pp,
            &or_p_negp, &and_p_negp, &imp_andpq_p, &imp_p_orpq,
            &imp_p_negp, &and_p_imppq, &mp,
        ] {
            engine.add_fact(Relation::new("declared", vec![(*f).clone()]));
        }

        // Rules (same as engine test)
        let vv = Term::var("v");
        let pv = Term::var("p");
        let qv = Term::var("q");
        let fv = Term::var("f");

        let mut rule_names = Vec::new();

        let rules = vec![
            Rule::new("neg_t",
                vec![
                    RelationPattern::new("tv_f", vec![vv.clone(), pv.clone()]),
                    RelationPattern::new("declared", vec![Term::app("neg", vec![pv.clone()])]),
                ],
                vec![RelationPattern::new("tv_t", vec![vv.clone(), Term::app("neg", vec![pv.clone()])])],
            ),
            Rule::new("neg_f",
                vec![
                    RelationPattern::new("tv_t", vec![vv.clone(), pv.clone()]),
                    RelationPattern::new("declared", vec![Term::app("neg", vec![pv.clone()])]),
                ],
                vec![RelationPattern::new("tv_f", vec![vv.clone(), Term::app("neg", vec![pv.clone()])])],
            ),
            Rule::new("and_t",
                vec![
                    RelationPattern::new("tv_t", vec![vv.clone(), pv.clone()]),
                    RelationPattern::new("tv_t", vec![vv.clone(), qv.clone()]),
                    RelationPattern::new("declared", vec![Term::app("and", vec![pv.clone(), qv.clone()])]),
                ],
                vec![RelationPattern::new("tv_t", vec![vv.clone(), Term::app("and", vec![pv.clone(), qv.clone()])])],
            ),
            Rule::new("and_f1",
                vec![
                    RelationPattern::new("tv_f", vec![vv.clone(), pv.clone()]),
                    RelationPattern::new("declared", vec![Term::app("and", vec![pv.clone(), qv.clone()])]),
                ],
                vec![RelationPattern::new("tv_f", vec![vv.clone(), Term::app("and", vec![pv.clone(), qv.clone()])])],
            ),
            Rule::new("and_f2",
                vec![
                    RelationPattern::new("tv_f", vec![vv.clone(), qv.clone()]),
                    RelationPattern::new("declared", vec![Term::app("and", vec![pv.clone(), qv.clone()])]),
                ],
                vec![RelationPattern::new("tv_f", vec![vv.clone(), Term::app("and", vec![pv.clone(), qv.clone()])])],
            ),
            Rule::new("or_t1",
                vec![
                    RelationPattern::new("tv_t", vec![vv.clone(), pv.clone()]),
                    RelationPattern::new("declared", vec![Term::app("or", vec![pv.clone(), qv.clone()])]),
                ],
                vec![RelationPattern::new("tv_t", vec![vv.clone(), Term::app("or", vec![pv.clone(), qv.clone()])])],
            ),
            Rule::new("or_t2",
                vec![
                    RelationPattern::new("tv_t", vec![vv.clone(), qv.clone()]),
                    RelationPattern::new("declared", vec![Term::app("or", vec![pv.clone(), qv.clone()])]),
                ],
                vec![RelationPattern::new("tv_t", vec![vv.clone(), Term::app("or", vec![pv.clone(), qv.clone()])])],
            ),
            Rule::new("or_f",
                vec![
                    RelationPattern::new("tv_f", vec![vv.clone(), pv.clone()]),
                    RelationPattern::new("tv_f", vec![vv.clone(), qv.clone()]),
                    RelationPattern::new("declared", vec![Term::app("or", vec![pv.clone(), qv.clone()])]),
                ],
                vec![RelationPattern::new("tv_f", vec![vv.clone(), Term::app("or", vec![pv.clone(), qv.clone()])])],
            ),
            Rule::new("imp_t1",
                vec![
                    RelationPattern::new("tv_f", vec![vv.clone(), pv.clone()]),
                    RelationPattern::new("declared", vec![Term::app("imp", vec![pv.clone(), qv.clone()])]),
                ],
                vec![RelationPattern::new("tv_t", vec![vv.clone(), Term::app("imp", vec![pv.clone(), qv.clone()])])],
            ),
            Rule::new("imp_t2",
                vec![
                    RelationPattern::new("tv_t", vec![vv.clone(), qv.clone()]),
                    RelationPattern::new("declared", vec![Term::app("imp", vec![pv.clone(), qv.clone()])]),
                ],
                vec![RelationPattern::new("tv_t", vec![vv.clone(), Term::app("imp", vec![pv.clone(), qv.clone()])])],
            ),
            Rule::new("imp_f",
                vec![
                    RelationPattern::new("tv_t", vec![vv.clone(), pv.clone()]),
                    RelationPattern::new("tv_f", vec![vv.clone(), qv.clone()]),
                    RelationPattern::new("declared", vec![Term::app("imp", vec![pv.clone(), qv.clone()])]),
                ],
                vec![RelationPattern::new("tv_f", vec![vv.clone(), Term::app("imp", vec![pv.clone(), qv.clone()])])],
            ),
            Rule::new("taut",
                vec![
                    RelationPattern::new("tv_t", vec![c("v_tt"), fv.clone()]),
                    RelationPattern::new("tv_t", vec![c("v_tf"), fv.clone()]),
                    RelationPattern::new("tv_t", vec![c("v_ft"), fv.clone()]),
                    RelationPattern::new("tv_t", vec![c("v_ff"), fv.clone()]),
                ],
                vec![RelationPattern::new("tautology", vec![fv.clone()])],
            ),
            Rule::new("contra",
                vec![
                    RelationPattern::new("tv_f", vec![c("v_tt"), fv.clone()]),
                    RelationPattern::new("tv_f", vec![c("v_tf"), fv.clone()]),
                    RelationPattern::new("tv_f", vec![c("v_ft"), fv.clone()]),
                    RelationPattern::new("tv_f", vec![c("v_ff"), fv.clone()]),
                ],
                vec![RelationPattern::new("contradiction", vec![fv.clone()])],
            ),
        ];

        for rule in rules {
            rule_names.push(rule.name().to_string());
            engine.add_rule(rule);
        }

        (engine, rule_names)
    }

    #[test]
    fn test_propositional_ablation_analysis() {
        let (engine, _rule_names) = build_propositional_engine();
        let analysis = ablation_analysis(&engine);

        // Print for inspection
        for (name, lost, score) in &analysis {
            eprintln!("  {:<10} lost={:<3} score={:.3}", name, lost, score);
        }

        // imp_t1 (falsity of antecedent → truth of implication) should be
        // among the most important, since it's needed for most tautologies
        // involving implication
        let imp_t1 = analysis.iter().find(|(n, _, _)| n == "imp_t1").unwrap();
        let taut = analysis.iter().find(|(n, _, _)| n == "taut").unwrap();

        // Both should lose facts when removed
        assert!(imp_t1.1 > 0, "imp_t1 should be important");
        assert!(taut.1 > 0, "taut rule should be important");
    }

    #[test]
    fn test_propositional_closure_profile() {
        let (mut engine, _) = build_propositional_engine();
        let result = engine.derive_closure();
        let profile = closure_profile(&result);

        eprintln!("  total_facts:    {}", profile.total_facts);
        eprintln!("  derived_facts:  {}", profile.derived_facts);
        eprintln!("  diversity:      {}", profile.relation_diversity);
        eprintln!("  mean_depth:     {:.2}", profile.mean_depth);
        eprintln!("  depth_hist:     {:?}", profile.depth_histogram);
        eprintln!("  saturated:      {}", profile.saturated);
        eprintln!("  rounds:         {}", profile.rounds);

        assert!(profile.saturated);
        assert!(profile.derived_facts > 30, "should derive many facts");
        // Should have tv_t, tv_f, tautology, contradiction in derived
        assert!(profile.relation_diversity >= 3);
        // Should have compound terms (depth > 1)
        assert!(profile.mean_depth > 1.0, "derived facts should include compound terms");
    }

    #[test]
    fn test_rank_candidates() {
        let mut engine = ClosureEngine::new();
        engine.define_relation("p", 1);
        engine.define_relation("q", 1);
        engine.define_relation("r", 1);
        engine.define_variable("x");
        engine.define_constant("a");
        engine.define_constant("b");
        engine.define_constant("c");

        engine.add_fact(Relation::new("p", vec![c("a")]));
        engine.add_fact(Relation::new("p", vec![c("b")]));
        engine.add_fact(Relation::new("p", vec![c("c")]));

        // Candidate A: p(?x) |- q(?x)  — generates 3 new facts
        let rule_a = Rule::new(
            "high_gen",
            vec![RelationPattern::new("p", vec![Term::var("x")])],
            vec![RelationPattern::new("q", vec![Term::var("x")])],
        );

        // Candidate B: p(a) |- r(a)  — generates 1 new fact, but more complex
        // (ground terms in premise = lower generality)
        let rule_b = Rule::new(
            "low_gen",
            vec![RelationPattern::new("p", vec![c("a")])],
            vec![RelationPattern::new("r", vec![c("a")])],
        );

        let ranked = rank_candidates(&engine, &[rule_a, rule_b], &ScoreWeights::default());
        assert_eq!(ranked[0].rule_name, "high_gen");
        assert!(ranked[0].generativity > ranked[1].generativity);
    }

    // ── Consistency penalty tests ───────────────────────────

    #[test]
    fn test_inconsistency_count_basic() {
        let facts = vec![
            Relation::binary("tv_t", c("v1"), c("p")),
            Relation::binary("tv_f", c("v1"), c("p")), // conflict!
            Relation::binary("tv_t", c("v2"), c("p")),
            Relation::binary("tv_f", c("v2"), c("q")), // no conflict
        ];
        let excl = vec![("tv_t".to_string(), "tv_f".to_string())];
        assert_eq!(inconsistency_count(&facts, &excl), 1);
    }

    #[test]
    fn test_consistency_penalty_distinguishes_correct_from_wrong() {
        // Build propositional base (same as propositional_engine helper)
        let (engine, _) = build_propositional_engine();

        let excl = vec![("tv_t".to_string(), "tv_f".to_string())];
        let weights = ScoreWeights {
            generativity: 1.0,
            compression: 0.0,
            consistency_penalty: 2.0,
            exclusions: excl,
        };

        let vv = Term::var("v");
        let pv = Term::var("p");

        // Correct: tv_f(?v, ?p), declared(neg(?p)) |- tv_t(?v, neg(?p))
        let correct = Rule::new(
            "neg_t_correct",
            vec![
                RelationPattern::new("tv_f", vec![vv.clone(), pv.clone()]),
                RelationPattern::new("declared", vec![Term::app("neg", vec![pv.clone()])]),
            ],
            vec![RelationPattern::new(
                "tv_t",
                vec![vv.clone(), Term::app("neg", vec![pv.clone()])],
            )],
        );

        // Wrong: tv_t(?v, ?p), declared(neg(?p)) |- tv_t(?v, neg(?p))
        // This produces tv_t(v, neg(p)) when p is ALREADY true — contradiction
        let wrong = Rule::new(
            "neg_t_wrong",
            vec![
                RelationPattern::new("tv_t", vec![vv.clone(), pv.clone()]),
                RelationPattern::new("declared", vec![Term::app("neg", vec![pv.clone()])]),
            ],
            vec![RelationPattern::new(
                "tv_t",
                vec![vv.clone(), Term::app("neg", vec![pv.clone()])],
            )],
        );

        let eval_correct = evaluate_rule(&engine, &correct, &weights);
        let eval_wrong = evaluate_rule(&engine, &wrong, &weights);

        eprintln!(
            "  correct: gen={:.3} incon={} combined={:.3}",
            eval_correct.generativity, eval_correct.inconsistencies, eval_correct.combined
        );
        eprintln!(
            "  wrong:   gen={:.3} incon={} combined={:.3}",
            eval_wrong.generativity, eval_wrong.inconsistencies, eval_wrong.combined
        );

        // Wrong rule may even produce MORE facts (interacts with other rules)
        // but the key signal is inconsistency

        // Wrong rule introduces inconsistencies
        assert_eq!(eval_correct.inconsistencies, 0);
        assert!(
            eval_wrong.inconsistencies > 0,
            "wrong neg rule should cause inconsistencies"
        );

        // So correct rule has higher combined score
        assert!(
            eval_correct.combined > eval_wrong.combined,
            "correct rule should score higher: {:.3} vs {:.3}",
            eval_correct.combined,
            eval_wrong.combined,
        );
    }
}
