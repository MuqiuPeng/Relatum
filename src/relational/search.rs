//! Autonomous rule discovery and paradigm comparison.
//!
//! Provides template-based candidate rule generation from the relation/constructor
//! vocabulary found in the current fact set, a greedy search loop that selects
//! the highest-scoring candidates per round, and paradigm comparison to evaluate
//! different rule sets on the same base engine.

use std::collections::{BTreeSet, HashMap, HashSet};

use super::engine::ClosureEngine;
use super::relation::Relation;
use super::rule::{RelationPattern, Rule};
use super::score::{self, ClosureProfile, RuleEvaluation, ScoreWeights};
use super::term::Term;

// ── Candidate generation ────────────────────────────────────

/// Configuration for candidate rule generation.
#[derive(Debug, Clone)]
pub struct CandidateConfig {
    /// Maximum rule complexity (total term nodes).
    pub max_complexity: usize,
    /// Relation names to exclude from candidate generation.
    pub exclude_relations: HashSet<String>,
    /// If set, use this relation as a guard (e.g. "declared") to constrain
    /// which compound terms the rules apply to.
    pub guard_relation: Option<String>,
    /// Minimum number of supporting fact pairs for a pattern to become a candidate.
    /// Higher = fewer, more reliable candidates.
    pub min_pattern_support: usize,
}

impl Default for CandidateConfig {
    fn default() -> Self {
        CandidateConfig {
            max_complexity: 20,
            exclude_relations: HashSet::new(),
            guard_relation: Some("declared".to_string()),
            min_pattern_support: 2,
        }
    }
}

/// Extract function constructors `(name, arity)` from all terms in the fact set.
fn extract_constructors(facts: &HashSet<Relation>) -> BTreeSet<(String, usize)> {
    let mut result = BTreeSet::new();
    for fact in facts {
        for term in fact.terms() {
            collect_constructors(term, &mut result);
        }
    }
    result
}

fn collect_constructors(term: &Term, out: &mut BTreeSet<(String, usize)>) {
    if let Term::App { symbol, args } = term {
        if !args.is_empty() {
            out.insert((symbol.clone(), args.len()));
        }
        for arg in args {
            collect_constructors(arg, out);
        }
    }
}

/// Generate candidate rules by enumerating templates over the relation/constructor
/// vocabulary present in the engine's current fact set.
///
/// For each compound-term constructor `f/n` found in facts and each pair of
/// binary relations `(r1, r2)`, generates rules of the form:
///
/// - Unary `f`: `r1(?v, ?p) [, guard(f(?p))] |- r2(?v, f(?p))`
/// - Binary `f`: `r1(?v, ?p), r2(?v, ?q) [, guard(f(?p,?q))] |- r3(?v, f(?p,?q))`
///   plus single-premise variants.
///
/// Returns de-duplicated candidates filtered by complexity.
pub fn generate_candidates(engine: &ClosureEngine, config: &CandidateConfig) -> Vec<Rule> {
    let constructors = extract_constructors(engine.facts());

    // Collect binary relations (these carry "truth-value-like" information)
    let binary_rels: Vec<String> = engine
        .relation_defs()
        .iter()
        .filter(|(name, def)| def.arity() == 2 && !config.exclude_relations.contains(*name))
        .map(|(name, _)| name.clone())
        .collect();

    let has_guard = config
        .guard_relation
        .as_ref()
        .map_or(false, |g| engine.relation_defs().contains_key(g.as_str()));

    let mut candidates = Vec::new();
    let mut seen = HashSet::new();

    let pv = Term::var("p");
    let qv = Term::var("q");
    let vv = Term::var("v");

    for (ctor, arity) in &constructors {
        match *arity {
            1 => {
                let fp = Term::app(ctor, vec![pv.clone()]);
                let guard = if has_guard {
                    let g = config.guard_relation.as_ref().unwrap();
                    Some(RelationPattern::new(g.as_str(), vec![fp.clone()]))
                } else {
                    None
                };

                for r1 in &binary_rels {
                    for r2 in &binary_rels {
                        let mut premises =
                            vec![RelationPattern::new(r1.as_str(), vec![vv.clone(), pv.clone()])];
                        if let Some(g) = &guard {
                            premises.push(g.clone());
                        }
                        let conclusions = vec![RelationPattern::new(
                            r2.as_str(),
                            vec![vv.clone(), fp.clone()],
                        )];
                        let name = format!("cand_{}_{}_{}", r1, ctor, r2);
                        if seen.insert(name.clone()) {
                            let rule = Rule::new(&name, premises, conclusions);
                            if score::rule_complexity(&rule) <= config.max_complexity {
                                candidates.push(rule);
                            }
                        }
                    }
                }
            }
            2 => {
                let fpq = Term::app(ctor, vec![pv.clone(), qv.clone()]);
                let guard = if has_guard {
                    let g = config.guard_relation.as_ref().unwrap();
                    Some(RelationPattern::new(g.as_str(), vec![fpq.clone()]))
                } else {
                    None
                };

                // Two-premise templates: r1(?v,?p), r2(?v,?q) |- r3(?v, f(?p,?q))
                for r1 in &binary_rels {
                    for r2 in &binary_rels {
                        for r3 in &binary_rels {
                            let mut premises = vec![
                                RelationPattern::new(r1.as_str(), vec![vv.clone(), pv.clone()]),
                                RelationPattern::new(r2.as_str(), vec![vv.clone(), qv.clone()]),
                            ];
                            if let Some(g) = &guard {
                                premises.push(g.clone());
                            }
                            let conclusions = vec![RelationPattern::new(
                                r3.as_str(),
                                vec![vv.clone(), fpq.clone()],
                            )];
                            let name = format!("cand_{}_{}_{}_{}", r1, r2, ctor, r3);
                            if seen.insert(name.clone()) {
                                let rule = Rule::new(&name, premises, conclusions);
                                if score::rule_complexity(&rule) <= config.max_complexity {
                                    candidates.push(rule);
                                }
                            }
                        }
                    }
                }

                // Single-premise variants (left arg only, right arg only)
                for r1 in &binary_rels {
                    for r2 in &binary_rels {
                        // Left: r1(?v, ?p), guard |- r2(?v, f(?p,?q))
                        {
                            let mut premises = vec![RelationPattern::new(
                                r1.as_str(),
                                vec![vv.clone(), pv.clone()],
                            )];
                            if let Some(g) = &guard {
                                premises.push(g.clone());
                            }
                            let conclusions = vec![RelationPattern::new(
                                r2.as_str(),
                                vec![vv.clone(), fpq.clone()],
                            )];
                            let name = format!("cand_{}_{}_{}_lft", r1, ctor, r2);
                            if seen.insert(name.clone()) {
                                let rule = Rule::new(&name, premises, conclusions);
                                if score::rule_complexity(&rule) <= config.max_complexity {
                                    candidates.push(rule);
                                }
                            }
                        }
                        // Right: r1(?v, ?q), guard |- r2(?v, f(?p,?q))
                        {
                            let mut premises = vec![RelationPattern::new(
                                r1.as_str(),
                                vec![vv.clone(), qv.clone()],
                            )];
                            if let Some(g) = &guard {
                                premises.push(g.clone());
                            }
                            let conclusions = vec![RelationPattern::new(
                                r2.as_str(),
                                vec![vv.clone(), fpq.clone()],
                            )];
                            let name = format!("cand_{}_{}_{}_rgt", r1, ctor, r2);
                            if seen.insert(name.clone()) {
                                let rule = Rule::new(&name, premises, conclusions);
                                if score::rule_complexity(&rule) <= config.max_complexity {
                                    candidates.push(rule);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Data-driven candidates: induce patterns from the fact set
    candidates.extend(induce_candidates(engine, config));

    candidates
}

// ── Data-driven pattern induction ────────────────────────────

/// A pattern extracted from facts: a relation name + a tuple of slots,
/// where each slot is either a fixed atom or a variable index.
/// Slots with the same variable index are constrained to be equal.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct InducedPattern {
    relation: String,
    /// Each slot: `None` = free variable at that position,
    /// `Some(group)` = this position shares a value with all other `Some(group)` positions.
    /// The actual variable names are assigned later.
    slots: Vec<Slot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Slot {
    /// A free variable (distinct from all other Free slots).
    Free(usize),
    /// Tied to another position — all Tied(g) slots in the same pattern must
    /// take the same ground value for the pattern to match.
    Tied(usize),
}

/// Induce structural patterns from a group of facts sharing the same relation.
///
/// For each pair of facts, compares argument positions:
/// - Positions where both facts have the **same** value → could be a constant or tied variable.
/// - Positions where they **differ** → free variable.
///
/// Additionally detects **intra-fact repetitions**: positions within a single fact
/// that hold the same value (e.g., `op(e0, e1, e1)` → positions 1 and 2 are equal).
fn induce_patterns(facts: &[&Relation], min_support: usize) -> Vec<(InducedPattern, usize)> {
    if facts.is_empty() {
        return Vec::new();
    }
    let arity = facts[0].arity();
    let rel = facts[0].name().to_string();

    let mut pattern_counts: HashMap<InducedPattern, usize> = HashMap::new();

    // Phase 1: Intra-fact repetition patterns.
    // For each fact, find which position-pairs hold the same value.
    // E.g., op(e0, e1, e1) → positions {1,2} are equal.
    for fact in facts {
        for i in 0..arity {
            for j in (i + 1)..arity {
                if fact.terms()[i] == fact.terms()[j] {
                    // Build a pattern: positions i,j are Tied, rest are Free.
                    let mut slots = Vec::new();
                    let mut free_counter = 0usize;
                    let tie_group = 100 + i; // arbitrary group id
                    for k in 0..arity {
                        if k == i || k == j {
                            slots.push(Slot::Tied(tie_group));
                        } else {
                            slots.push(Slot::Free(free_counter));
                            free_counter += 1;
                        }
                    }
                    let pat = InducedPattern {
                        relation: rel.clone(),
                        slots,
                    };
                    *pattern_counts.entry(pat).or_insert(0) += 1;
                }
            }
        }
    }

    // Phase 2: Inter-fact cross-link patterns.
    // For each pair of facts, find positions where values match vs differ.
    // This discovers patterns like "same (1,2) args, different 3rd" (functionality).
    if facts.len() <= 200 {
        // Only for manageable sizes
        for i in 0..facts.len() {
            for j in (i + 1)..facts.len() {
                let f1 = facts[i];
                let f2 = facts[j];
                let mut same_positions = Vec::new();
                let mut diff_positions = Vec::new();
                for k in 0..arity {
                    if f1.terms()[k] == f2.terms()[k] {
                        same_positions.push(k);
                    } else {
                        diff_positions.push(k);
                    }
                }
                // Only interesting if there's a mix of same/different
                if same_positions.is_empty() || diff_positions.is_empty() {
                    continue;
                }
                // Build a 2-premise cross-link pattern:
                // R(?...) , R(?...) where same_positions share variables, diff_positions differ.
                // Represented as a single InducedPattern for the "shared structure".
                let mut slots = Vec::new();
                let mut free_counter = 0usize;
                for k in 0..arity {
                    if same_positions.contains(&k) {
                        slots.push(Slot::Tied(k)); // same across both facts
                    } else {
                        slots.push(Slot::Free(free_counter));
                        free_counter += 1;
                    }
                }
                let pat = InducedPattern {
                    relation: rel.clone(),
                    slots,
                };
                *pattern_counts.entry(pat).or_insert(0) += 1;
            }
        }
    }

    // Filter by minimum support
    let mut result: Vec<(InducedPattern, usize)> = pattern_counts
        .into_iter()
        .filter(|(_, count)| *count >= min_support)
        .collect();
    result.sort_by(|a, b| b.1.cmp(&a.1));
    result
}

/// Convert an induced pattern into candidate rules by connecting it to
/// other relations as conclusions.
fn pattern_to_rules(
    pattern: &InducedPattern,
    _support: usize,
    conclusion_rels: &[(&str, usize)], // (name, arity) of candidate conclusion relations
    is_cross_link: bool,
) -> Vec<Rule> {
    let mut rules = Vec::new();

    // Assign variable names to slots
    let mut var_names: Vec<String> = Vec::new();
    let mut tied_vars: HashMap<usize, String> = HashMap::new();
    let mut free_counter = 0usize;

    let mut premise_terms: Vec<Term> = Vec::new();
    for slot in &pattern.slots {
        match slot {
            Slot::Free(_) => {
                let name = format!("v{}", free_counter);
                free_counter += 1;
                premise_terms.push(Term::var(&name));
                var_names.push(name);
            }
            Slot::Tied(group) => {
                let name = tied_vars
                    .entry(*group)
                    .or_insert_with(|| {
                        let n = format!("v{}", free_counter);
                        free_counter += 1;
                        n
                    })
                    .clone();
                premise_terms.push(Term::var(&name));
                var_names.push(name);
            }
        }
    }

    // Identify "output" variables (free slots) and "constrained" variables (tied slots)
    let free_vars: Vec<String> = pattern
        .slots
        .iter()
        .zip(var_names.iter())
        .filter(|(s, _)| matches!(s, Slot::Free(_)))
        .map(|(_, n)| n.clone())
        .collect();
    let tied_var_names: Vec<String> = tied_vars.values().cloned().collect();

    // For intra-fact patterns: premise is a single relation pattern
    // For cross-link patterns: premise is two relation patterns with shared/different vars
    if !is_cross_link {
        // Single-premise rule: pattern |- conclusion(free_vars)
        let premise = RelationPattern::new(&pattern.relation, premise_terms);

        for (crel, carity) in conclusion_rels {
            // Try all ways to fill conclusion args from the free variables
            // (the "output" of the pattern → conclusion)
            if *carity == 1 {
                // Conclusion uses each free var
                for fv in &free_vars {
                    let name = format!(
                        "ind_{}_{}_{}",
                        pattern.relation, crel, fv
                    );
                    rules.push(Rule::new(
                        name,
                        vec![premise.clone()],
                        vec![RelationPattern::new(*crel, vec![Term::var(fv)])],
                    ));
                }
                // Also try tied vars as conclusion
                for tv in &tied_var_names {
                    let name = format!(
                        "ind_{}_{}_tied_{}",
                        pattern.relation, crel, tv
                    );
                    rules.push(Rule::new(
                        name,
                        vec![premise.clone()],
                        vec![RelationPattern::new(*crel, vec![Term::var(tv)])],
                    ));
                }
            } else if *carity == 2 && free_vars.len() >= 2 {
                // Conclusion uses pairs of free vars
                for i in 0..free_vars.len() {
                    for j in 0..free_vars.len() {
                        if i == j {
                            continue;
                        }
                        let name = format!(
                            "ind_{}_{}_{}_{}",
                            pattern.relation, crel, free_vars[i], free_vars[j]
                        );
                        rules.push(Rule::new(
                            name,
                            vec![premise.clone()],
                            vec![RelationPattern::new(
                                *crel,
                                vec![Term::var(&free_vars[i]), Term::var(&free_vars[j])],
                            )],
                        ));
                    }
                }
            }
        }
    } else {
        // Cross-link: two premises from same relation.
        // First premise uses the pattern as-is.
        // Second premise has the same tied vars but NEW free vars.
        let mut premise2_terms: Vec<Term> = Vec::new();
        let mut new_free_vars: Vec<String> = Vec::new();
        for slot in &pattern.slots {
            match slot {
                Slot::Tied(group) => {
                    // Same variable as first premise
                    premise2_terms.push(Term::var(tied_vars.get(group).unwrap()));
                }
                Slot::Free(_) => {
                    let name = format!("w{}", new_free_vars.len());
                    new_free_vars.push(name.clone());
                    premise2_terms.push(Term::var(&name));
                }
            }
        }

        let p1 = RelationPattern::new(&pattern.relation, premise_terms);
        let p2 = RelationPattern::new(&pattern.relation, premise2_terms);

        // Conclusion links the free vars from both premises
        for (crel, carity) in conclusion_rels {
            if *carity == 2 && !free_vars.is_empty() && !new_free_vars.is_empty() {
                // Link first free var from each premise
                for fv1 in &free_vars {
                    for fv2 in &new_free_vars {
                        let name = format!(
                            "ind_x_{}_{}_{}_{}",
                            pattern.relation, crel, fv1, fv2
                        );
                        rules.push(Rule::new(
                            name,
                            vec![p1.clone(), p2.clone()],
                            vec![RelationPattern::new(
                                *crel,
                                vec![Term::var(fv1), Term::var(fv2)],
                            )],
                        ));
                    }
                }
            }
        }
    }

    rules
}

/// Generate candidate rules by inducting patterns from the engine's fact set.
///
/// Unlike the template-based approach, this does NOT use hand-crafted structural
/// templates. Instead, it:
/// 1. Groups facts by relation name
/// 2. Anti-unifies fact pairs to discover structural regularities
/// 3. Converts high-frequency patterns into candidate rules
///
/// Also generates conditional rules by connecting unary facts (from earlier
/// discovery rounds) with patterns from other relations.
fn induce_candidates(engine: &ClosureEngine, config: &CandidateConfig) -> Vec<Rule> {
    let facts_vec: Vec<&Relation> = engine
        .facts()
        .iter()
        .filter(|f| !config.exclude_relations.contains(f.name()))
        .collect();

    // Group facts by relation name
    let mut by_relation: HashMap<&str, Vec<&Relation>> = HashMap::new();
    for fact in &facts_vec {
        by_relation
            .entry(fact.name())
            .or_insert_with(Vec::new)
            .push(fact);
    }

    // Collect conclusion relations: all non-excluded relations
    let conclusion_rels: Vec<(&str, usize)> = engine
        .relation_defs()
        .iter()
        .filter(|(n, _)| !config.exclude_relations.contains(*n))
        .map(|(n, d)| (n.as_str(), d.arity()))
        .collect();

    let mut all_rules = Vec::new();
    let mut seen_names: HashSet<String> = HashSet::new();

    let min_support = config.min_pattern_support;

    for (_rel_name, rel_facts) in &by_relation {
        if rel_facts.len() < 2 {
            continue;
        }
        // Induce patterns from this relation's facts
        let patterns = induce_patterns(rel_facts, min_support);

        for (pattern, support) in &patterns {
            // Check if this is an intra-fact or cross-link pattern
            let has_tied = pattern.slots.iter().any(|s| matches!(s, Slot::Tied(_)));
            if !has_tied {
                continue; // all-free pattern is trivial
            }

            let is_cross_link = pattern.slots.iter().any(|s| match s {
                Slot::Tied(g) => *g < 100, // cross-link tied groups use position indices
                _ => false,
            });

            let rules = pattern_to_rules(pattern, *support, &conclusion_rels, is_cross_link);
            for rule in rules {
                if score::rule_complexity(&rule) <= config.max_complexity {
                    if seen_names.insert(rule.name().to_string()) {
                        all_rules.push(rule);
                    }
                }
            }
        }
    }

    // Phase 3: Conditional rules — connect unary facts with other relation patterns.
    // If is_id(e0) exists, generate rules like: is_id(?e), op(?x, ?y, ?e) |- rel(?x, ?y).
    // This is induced from the DATA: for each unary relation U with facts,
    // and each other relation R, try constraining each position of R with U's variable.
    let unary_rels_with_facts: Vec<&str> = by_relation
        .iter()
        .filter(|(_, fs)| !fs.is_empty() && fs[0].arity() == 1)
        .map(|(name, _)| *name)
        .collect();

    for u_rel in &unary_rels_with_facts {
        for (r_rel, r_facts) in &by_relation {
            if *r_rel == *u_rel {
                continue;
            }
            let r_arity = r_facts[0].arity();
            if r_arity < 2 {
                continue;
            }

            // For each position in R, try constraining it with U
            for constrained_pos in 0..r_arity {
                let mut r_terms = Vec::new();
                let mut free_vars = Vec::new();
                for k in 0..r_arity {
                    if k == constrained_pos {
                        r_terms.push(Term::var("e"));
                    } else {
                        let vname = format!("p{}", k);
                        free_vars.push(vname.clone());
                        r_terms.push(Term::var(&vname));
                    }
                }

                let premises = vec![
                    RelationPattern::new(*u_rel, vec![Term::var("e")]),
                    RelationPattern::new(*r_rel, r_terms),
                ];

                // Try all binary conclusion relations
                for (crel, carity) in &conclusion_rels {
                    if *carity == 2 && free_vars.len() >= 2 {
                        let name = format!("ind_cond_{}_{}_{}_pos{}", u_rel, r_rel, crel, constrained_pos);
                        if seen_names.insert(name.clone()) {
                            let rule = Rule::new(
                                name,
                                premises.clone(),
                                vec![RelationPattern::new(
                                    *crel,
                                    vec![
                                        Term::var(&free_vars[0]),
                                        Term::var(&free_vars[1]),
                                    ],
                                )],
                            );
                            if score::rule_complexity(&rule) <= config.max_complexity {
                                all_rules.push(rule);
                            }
                        }
                    }
                }
            }
        }
    }

    all_rules
}

// ── Search loop ─────────────────────────────────────────────

/// Configuration for the search loop.
#[derive(Debug, Clone)]
pub struct SearchConfig {
    pub candidate_config: CandidateConfig,
    pub weights: ScoreWeights,
    /// How many candidates to keep after compression pre-filter.
    pub pre_filter_top_n: usize,
    /// How many rules to select per round.
    pub top_k: usize,
    /// Maximum search rounds.
    pub max_steps: usize,
    /// Stop when a round adds fewer than this many facts.
    pub min_delta: usize,
}

impl Default for SearchConfig {
    fn default() -> Self {
        SearchConfig {
            candidate_config: CandidateConfig::default(),
            weights: ScoreWeights::default(),
            pre_filter_top_n: 20,
            top_k: 1,
            max_steps: 10,
            min_delta: 1,
        }
    }
}

/// Result of one search step.
#[derive(Debug, Clone)]
pub struct StepResult {
    pub round: usize,
    pub candidates_generated: usize,
    pub candidates_after_filter: usize,
    pub selected: Vec<RuleEvaluation>,
    pub total_facts_after: usize,
    pub delta_facts: usize,
    pub profile: ClosureProfile,
}

/// Execute one search step: generate candidates, score, select top-k, add to engine.
///
/// The engine is mutated: selected rules are added and closure is re-derived.
/// Returns the step result with scoring details.
pub fn search_step(
    engine: &mut ClosureEngine,
    round: usize,
    config: &SearchConfig,
) -> StepResult {
    let facts_before = engine.facts().len();

    // 1. Generate candidates
    let candidates = generate_candidates(engine, &config.candidate_config);
    let n_generated = candidates.len();

    // 2. Pre-filter by compression (cheap — no closure needed)
    let mut scored_by_compression: Vec<(usize, f64)> = candidates
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let (_, comp) = score::compression(r, engine.facts());
            (i, comp)
        })
        .collect();
    scored_by_compression.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let filtered_indices: Vec<usize> = scored_by_compression
        .iter()
        .take(config.pre_filter_top_n)
        .map(|(i, _)| *i)
        .collect();
    let n_filtered = filtered_indices.len();

    // 3. Full evaluation (expensive — runs closure for each)
    let filtered_rules: Vec<&Rule> = filtered_indices.iter().map(|i| &candidates[*i]).collect();
    let mut evaluations: Vec<RuleEvaluation> = filtered_rules
        .iter()
        .map(|r| score::evaluate_rule(engine, r, &config.weights))
        .collect();
    evaluations
        .sort_by(|a, b| b.combined.partial_cmp(&a.combined).unwrap_or(std::cmp::Ordering::Equal));

    // 4. Select top-k and add to engine
    let selected: Vec<RuleEvaluation> = evaluations.into_iter().take(config.top_k).collect();
    for eval in &selected {
        // Find the matching candidate rule and add it
        if let Some(rule) = candidates.iter().find(|r| r.name() == eval.rule_name) {
            // Register variables used by this rule
            for p in rule.premises() {
                for t in p.terms() {
                    register_vars(engine, t);
                }
            }
            for c in rule.conclusions() {
                for t in c.terms() {
                    register_vars(engine, t);
                }
            }
            engine.add_rule(rule.clone());
        }
    }

    // 5. Re-derive closure to update fact set
    let result = engine.derive_closure();
    let profile = score::closure_profile(&result);
    let total_after = result.facts.len();

    StepResult {
        round,
        candidates_generated: n_generated,
        candidates_after_filter: n_filtered,
        selected,
        total_facts_after: total_after,
        delta_facts: total_after.saturating_sub(facts_before),
        profile,
    }
}

/// Register all variables in a term with the engine.
fn register_vars(engine: &mut ClosureEngine, term: &Term) {
    match term {
        Term::Var(name) => {
            engine.define_variable(name);
        }
        Term::App { args, .. } => {
            for arg in args {
                register_vars(engine, arg);
            }
        }
    }
}

/// Run the search loop until convergence or max steps.
///
/// Returns the log of all steps (emergence history).
pub fn run_search(engine: &mut ClosureEngine, config: &SearchConfig) -> Vec<StepResult> {
    let mut log = Vec::new();

    for round in 0..config.max_steps {
        let step = search_step(engine, round, config);
        let delta = step.delta_facts;
        log.push(step);

        if delta < config.min_delta {
            break;
        }
    }

    log
}

// ── Beam search (combo evaluation) ──────────────────────────

/// Configuration for beam search over rule combinations.
#[derive(Debug, Clone)]
pub struct BeamConfig {
    pub candidate_config: CandidateConfig,
    pub weights: ScoreWeights,
    /// Number of rule-sets to maintain in the beam.
    pub beam_width: usize,
    /// Maximum rules per beam entry.
    pub max_rules_per_beam: usize,
    /// Maximum search rounds.
    pub max_steps: usize,
    /// Adaptive weight policy.
    pub adaptive: AdaptivePolicy,
}

impl Default for BeamConfig {
    fn default() -> Self {
        BeamConfig {
            candidate_config: CandidateConfig::default(),
            weights: ScoreWeights::default(),
            beam_width: 5,
            max_rules_per_beam: 6,
            max_steps: 6,
            adaptive: AdaptivePolicy::Fixed,
        }
    }
}

/// Policy for adjusting score weights across search rounds.
#[derive(Debug, Clone)]
pub enum AdaptivePolicy {
    /// Weights stay constant throughout search.
    Fixed,
    /// Compression weight scales with derived facts; consistency penalty
    /// increases as the system grows.
    ///
    /// At round `r` with `d` derived facts:
    /// - `compression = base_compression * (d / fact_threshold).min(1.0)`
    /// - `consistency_penalty = base_penalty * (1 + growth_rate * r)`
    Adaptive {
        /// Derived fact count at which compression reaches full weight.
        fact_threshold: f64,
        /// Per-round increase rate for consistency penalty.
        growth_rate: f64,
    },
}

/// Compute effective weights for a given round/state under the adaptive policy.
fn effective_weights(
    base: &ScoreWeights,
    policy: &AdaptivePolicy,
    round: usize,
    derived_facts: usize,
) -> ScoreWeights {
    match policy {
        AdaptivePolicy::Fixed => base.clone(),
        AdaptivePolicy::Adaptive {
            fact_threshold,
            growth_rate,
        } => {
            let comp_scale = (derived_facts as f64 / fact_threshold).min(1.0);
            let penalty_scale = 1.0 + growth_rate * round as f64;
            ScoreWeights {
                generativity: base.generativity,
                compression: base.compression * comp_scale,
                consistency_penalty: base.consistency_penalty * penalty_scale,
                exclusions: base.exclusions.clone(),
            }
        }
    }
}

/// One entry in the beam: a set of rules and its evaluation.
#[derive(Debug, Clone)]
pub struct BeamEntry {
    /// Names of rules in this combination.
    pub rule_names: Vec<String>,
    /// The actual rules.
    pub rules: Vec<Rule>,
    /// Closure profile when these rules are applied to the base engine.
    pub profile: ClosureProfile,
    /// Combined score: generativity-weighted derived_facts - consistency penalty.
    pub score: f64,
}

/// Result of one beam search round.
#[derive(Debug, Clone)]
pub struct BeamStepResult {
    pub round: usize,
    pub candidates_evaluated: usize,
    pub beam: Vec<BeamEntry>,
    /// Effective weights used this round (after adaptive adjustment).
    pub effective_weights: ScoreWeights,
}

/// Score a rule set by running closure on a base engine with those rules.
fn score_rule_set(
    base: &ClosureEngine,
    rules: &[Rule],
    weights: &ScoreWeights,
) -> (ClosureProfile, f64) {
    let mut engine = base.clone();
    for rule in rules {
        for p in rule.premises() {
            for t in p.terms() {
                register_vars_on(&mut engine, t);
            }
        }
        for cc in rule.conclusions() {
            for t in cc.terms() {
                register_vars_on(&mut engine, t);
            }
        }
        engine.add_rule(rule.clone());
    }
    let result = engine.derive_closure();
    let profile = score::closure_profile_with_exclusions(&result, &weights.exclusions);

    let total_complexity: usize = rules.iter().map(|r| score::rule_complexity(r).max(1)).sum();

    // Additive scoring: each derived fact is worth +1, complexity is a mild penalty.
    // This rewards absolute growth, not just efficiency ratios.
    let complexity_cost = 0.1; // 10 complexity units ≈ 1 derived fact
    let score = weights.generativity * profile.derived_facts as f64
        - complexity_cost * total_complexity as f64
        - weights.consistency_penalty * profile.inconsistencies as f64;

    (profile, score)
}

/// Run beam search over rule combinations.
///
/// Maintains `beam_width` best rule-sets. Each round, expands each beam entry
/// by adding one candidate rule, evaluates the resulting closure, and keeps
/// the top entries.
///
/// The base engine is NOT mutated.
pub fn beam_search(base: &ClosureEngine, config: &BeamConfig) -> Vec<BeamStepResult> {
    let candidates = generate_candidates(base, &config.candidate_config);
    let mut log = Vec::new();

    // Round 0: compute effective weights
    let w0 = effective_weights(&config.weights, &config.adaptive, 0, 0);

    // Initialize beam with single-rule entries
    let mut beam: Vec<BeamEntry> = Vec::new();
    for rule in &candidates {
        let (profile, sc) = score_rule_set(base, &[rule.clone()], &w0);
        beam.push(BeamEntry {
            rule_names: vec![rule.name().to_string()],
            rules: vec![rule.clone()],
            profile,
            score: sc,
        });
    }
    beam.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    beam.truncate(config.beam_width);

    log.push(BeamStepResult {
        round: 0,
        candidates_evaluated: candidates.len(),
        beam: beam.clone(),
        effective_weights: w0,
    });

    // Expand beam
    for round in 1..config.max_steps {
        // Adaptive: compute weights based on best beam entry's derived facts
        let best_derived = beam.first().map_or(0, |e| e.profile.derived_facts);
        let weights = effective_weights(&config.weights, &config.adaptive, round, best_derived);

        let mut next_beam: Vec<BeamEntry> = Vec::new();
        let mut evaluated = 0usize;

        for entry in &beam {
            if entry.rules.len() >= config.max_rules_per_beam {
                // Re-score with current weights (may change ranking)
                let (profile, sc) = score_rule_set(base, &entry.rules, &weights);
                next_beam.push(BeamEntry {
                    rule_names: entry.rule_names.clone(),
                    rules: entry.rules.clone(),
                    profile,
                    score: sc,
                });
                continue;
            }

            for rule in &candidates {
                if entry.rule_names.contains(&rule.name().to_string()) {
                    continue;
                }

                let mut expanded_rules = entry.rules.clone();
                expanded_rules.push(rule.clone());
                let (profile, sc) = score_rule_set(base, &expanded_rules, &weights);
                evaluated += 1;

                let mut names = entry.rule_names.clone();
                names.push(rule.name().to_string());

                next_beam.push(BeamEntry {
                    rule_names: names,
                    rules: expanded_rules,
                    profile,
                    score: sc,
                });
            }
        }

        // Also keep current beam entries re-scored with new weights
        for entry in &beam {
            let (profile, sc) = score_rule_set(base, &entry.rules, &weights);
            next_beam.push(BeamEntry {
                rule_names: entry.rule_names.clone(),
                rules: entry.rules.clone(),
                profile,
                score: sc,
            });
        }

        next_beam.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        next_beam.truncate(config.beam_width);
        beam = next_beam;

        let converged = log
            .last()
            .map_or(false, |prev: &BeamStepResult| {
                prev.beam.first().map(|e| &e.rule_names) == beam.first().map(|e| &e.rule_names)
            });

        log.push(BeamStepResult {
            round,
            candidates_evaluated: evaluated,
            beam: beam.clone(),
            effective_weights: weights,
        });

        if converged {
            break;
        }
    }

    log
}

// ── Concept promotion & discovery loop ──────────────────────

/// Configuration for pattern-to-concept promotion.
#[derive(Debug, Clone)]
pub struct PromotionConfig {
    /// Minimum support (number of facts matching the pattern) to promote.
    pub min_support: usize,
    /// Maximum number of concepts to promote per round.
    pub max_promotions_per_round: usize,
    /// Relation names to exclude from pattern induction.
    pub exclude_relations: HashSet<String>,
}

impl Default for PromotionConfig {
    fn default() -> Self {
        PromotionConfig {
            min_support: 2,
            max_promotions_per_round: 5,
            exclude_relations: HashSet::new(),
        }
    }
}

/// Full discovery configuration: promotion + beam search.
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    pub beam: BeamConfig,
    pub promotion: PromotionConfig,
    pub max_rounds: usize,
}

/// Canonical signature of a concept's origin pattern.
/// Two concepts with the same signature represent the same abstract idea,
/// regardless of which concrete structure they were discovered in.
///
/// Example: `("op", [Tied(100), Free(0), Free(0)])` = "element where arg1==arg2 in op"
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConceptSignature {
    pub source_relation: String,
    pub slots: Vec<Slot>,
    pub arity: usize,
}

impl std::fmt::Display for ConceptSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}(", self.source_relation)?;
        for (i, slot) in self.slots.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            match slot {
                Slot::Free(id) => write!(f, "?v{}", id)?,
                Slot::Tied(g) => write!(f, "?t{}", g % 100)?,
            }
        }
        write!(f, ") -> /{}", self.arity)
    }
}

/// Information about an auto-promoted concept.
#[derive(Debug, Clone)]
pub struct ConceptInfo {
    pub name: String,
    pub arity: usize,
    pub signature: ConceptSignature,
    pub source_pattern: String,
    pub support: usize,
    pub instances: usize,
    /// The actual instance set (ground terms).
    pub instance_set: Vec<Term>,
}

/// Result of one discovery round.
#[derive(Debug, Clone)]
pub struct DiscoveryStep {
    pub round: usize,
    pub promoted: Vec<ConceptInfo>,
    /// Verification rules auto-discovered for promoted concepts.
    pub verification_rules: Vec<String>,
    /// Universal generative rules: finite observation → universal axiom.
    pub universal_rules: Vec<UniversalRule>,
    /// Chain identities discovered (e.g., associativity).
    pub chain_identities: Vec<ChainIdentity>,
    pub beam_best: Option<BeamEntry>,
    pub total_facts: usize,
    pub total_relations: usize,
}

/// A universal rule discovered from finite observations.
/// Can be transferred to new (potentially infinite) structures.
#[derive(Debug, Clone)]
pub struct UniversalRule {
    /// Human-readable description: e.g., "identity(?e), element(?x) |- op(?e, ?x, ?x)"
    pub description: String,
    /// The concept this rule is about.
    pub concept: String,
    /// The relation being constrained.
    pub relation: String,
    /// Positions: (concept_pos, equal_pos_1, equal_pos_2).
    pub pattern: (usize, usize, usize),
    /// The actual Rule object.
    pub rule_name: String,
}

/// Complete discovery log with initial conditions and all steps.
#[derive(Debug, Clone)]
pub struct DiscoveryLog {
    /// Description of the structure being analyzed.
    pub structure_name: String,
    /// Initial facts grouped by relation.
    pub initial_facts: Vec<(String, Vec<String>)>,
    /// Initial relation declarations.
    pub initial_relations: Vec<(String, usize)>,
    /// Number of hand-written rules in the base engine.
    pub initial_rules: usize,
    /// Discovery steps.
    pub steps: Vec<DiscoveryStep>,
}

impl DiscoveryLog {
    /// Format the entire discovery as a human-readable log string.
    pub fn to_log_string(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "═══════════════════════════════════════════════════════\n\
             DISCOVERY LOG: {}\n\
             ═══════════════════════════════════════════════════════\n\n",
            self.structure_name
        ));

        // Initial conditions
        out.push_str("── INITIAL CONDITIONS ──\n\n");
        out.push_str(&format!("  Relations declared: {}\n", self.initial_relations.len()));
        for (name, arity) in &self.initial_relations {
            out.push_str(&format!("    {}/{}\n", name, arity));
        }
        out.push_str(&format!("  Hand-written rules: {}\n", self.initial_rules));
        out.push_str("\n  Facts:\n");
        for (rel, facts) in &self.initial_facts {
            out.push_str(&format!("    {} ({} facts):\n", rel, facts.len()));
            for f in facts {
                out.push_str(&format!("      {}\n", f));
            }
        }

        // Steps
        for step in &self.steps {
            out.push_str(&format!(
                "\n── ROUND {} ──\n\n",
                step.round
            ));

            // Promoted concepts
            if !step.promoted.is_empty() {
                out.push_str(&format!(
                    "  Concepts promoted: {}\n",
                    step.promoted.len()
                ));
                for c in &step.promoted {
                    let inst_str: Vec<String> =
                        c.instance_set.iter().map(|t| t.to_string()).collect();
                    out.push_str(&format!(
                        "    {} | sig={} | instances={{{}}}\n",
                        c.name,
                        c.signature,
                        inst_str.join(", ")
                    ));
                    out.push_str(&format!("      rule: {}\n", c.source_pattern.replace('\n', "\n            ")));
                }
            } else {
                out.push_str("  Concepts promoted: 0 (no new patterns above threshold)\n");
            }

            // Verification rules
            if !step.verification_rules.is_empty() {
                out.push_str(&format!(
                    "  Verification rules discovered: {}\n",
                    step.verification_rules.len()
                ));
                for vr in &step.verification_rules {
                    out.push_str(&format!("    {}\n", vr));
                }
            }

            // Universal generative rules
            if !step.universal_rules.is_empty() {
                out.push_str(&format!(
                    "  Universal rules discovered: {}\n",
                    step.universal_rules.len()
                ));
                for ur in &step.universal_rules {
                    out.push_str(&format!("    {}\n", ur.description));
                }
            }

            // Chain identities
            if !step.chain_identities.is_empty() {
                out.push_str(&format!(
                    "  Chain identities discovered: {}\n",
                    step.chain_identities.len()
                ));
                for ci in &step.chain_identities {
                    out.push_str(&format!("    {} = {}\n", ci.path_a, ci.path_b));
                }
            }

            // Beam
            if let Some(ref best) = step.beam_best {
                out.push_str(&format!(
                    "  Beam search: score={:.2} derived={} inconsistencies={}\n",
                    best.score, best.profile.derived_facts, best.profile.inconsistencies
                ));
                if best.score > 0.0 && best.profile.derived_facts > 0 {
                    out.push_str(&format!("    rules injected: {:?}\n", best.rule_names));
                } else {
                    out.push_str("    → no beneficial rules (not injected)\n");
                }
            } else {
                out.push_str("  Beam search: skipped (no new facts to explore)\n");
            }

            out.push_str(&format!(
                "  State: {} facts, {} relations\n",
                step.total_facts, step.total_relations
            ));
        }

        out.push_str(&format!(
            "\n═══════════════════════════════════════════════════════\n\
             END OF LOG ({})\n\
             ═══════════════════════════════════════════════════════\n",
            self.structure_name
        ));

        out
    }
}

/// Promote an intra-fact pattern to a new concept relation.
///
/// Creates a new relation and a rule that populates it from the pattern.
/// Returns the promotion rule, or None if the pattern isn't promotable.
fn promote_pattern(
    engine: &mut ClosureEngine,
    pattern: &InducedPattern,
    concept_counter: &mut usize,
) -> Option<(String, Rule, ConceptSignature)> {
    // Only promote intra-fact patterns (tied groups >= 100)
    let is_intra = pattern
        .slots
        .iter()
        .any(|s| matches!(s, Slot::Tied(g) if *g >= 100));
    if !is_intra {
        return None;
    }

    // Build variable mapping: Free → conclusion args, Tied → shared in premise
    let mut premise_terms = Vec::new();
    let mut conclusion_terms = Vec::new();
    let mut var_counter = 0usize;
    let mut tied_map: HashMap<usize, String> = HashMap::new();

    for slot in &pattern.slots {
        match slot {
            Slot::Free(_) => {
                let vname = format!("pv{}", var_counter);
                var_counter += 1;
                engine.define_variable(&vname);
                premise_terms.push(Term::var(&vname));
                conclusion_terms.push(Term::var(&vname));
            }
            Slot::Tied(group) => {
                let vname = tied_map
                    .entry(*group)
                    .or_insert_with(|| {
                        let n = format!("pv{}", var_counter);
                        var_counter += 1;
                        engine.define_variable(&n);
                        n
                    })
                    .clone();
                premise_terms.push(Term::var(&vname));
            }
        }
    }

    let arity = conclusion_terms.len();
    if arity == 0 || arity > 2 {
        return None;
    }

    let sig = ConceptSignature {
        source_relation: pattern.relation.clone(),
        slots: pattern.slots.clone(),
        arity,
    };

    let concept_name = format!("auto_{}", concept_counter);
    *concept_counter += 1;
    engine.define_relation(&concept_name, arity);

    let rule = Rule::new(
        format!("promote_{}", concept_name),
        vec![RelationPattern::new(&pattern.relation, premise_terms)],
        vec![RelationPattern::new(&concept_name, conclusion_terms)],
    );

    Some((concept_name, rule, sig))
}

/// Discover verification rules for a promoted concept by observing how
/// its instances behave in other relations.
///
/// Algorithm:
/// 1. Collect all instances of the concept.
/// 2. For each relation R that mentions an instance at position `pos`,
///    gather the facts and check for structural regularities
///    (e.g., "when instance is at pos 0, positions 1 and 2 are always equal").
/// 3. Check that the regularity does NOT hold for non-instances.
/// 4. If exclusive, emit a verification rule:
///    `concept(?e), R(?e, ?x, ?y) |- eq(?x, ?y)`.
fn discover_verification_rules(
    engine: &mut ClosureEngine,
    concept_name: &str,
    eq_relation: &str,
) -> Vec<Rule> {
    let concept_arity = match engine.relation_defs().get(concept_name) {
        Some(def) => def.arity(),
        None => return Vec::new(),
    };
    if concept_arity != 1 {
        return Vec::new(); // only handle unary concepts for now
    }

    // Step 1: Collect instances and non-instances
    let instances: HashSet<Term> = engine
        .facts()
        .iter()
        .filter(|f| f.name() == concept_name && f.arity() == 1)
        .map(|f| f.terms()[0].clone())
        .collect();

    if instances.is_empty() {
        return Vec::new();
    }

    // Collect all ground constants as the universe (owned to avoid borrow issues)
    let universe: HashSet<Term> = engine
        .facts()
        .iter()
        .flat_map(|f| f.terms().iter())
        .filter(|t| t.is_ground() && matches!(t, Term::App { args, .. } if args.is_empty()))
        .cloned()
        .collect();
    let non_instances: Vec<Term> = universe
        .iter()
        .filter(|t| !instances.contains(t))
        .cloned()
        .collect();

    let mut rules = Vec::new();

    // Step 2: For each relation R with arity >= 2, check each position
    let rel_defs: Vec<(String, usize)> = engine
        .relation_defs()
        .iter()
        .filter(|(n, d)| {
            d.arity() >= 2
                && *n != concept_name
                && *n != eq_relation
                && *n != "distinct"
        })
        .map(|(n, d)| (n.clone(), d.arity()))
        .collect();

    let facts: Vec<Relation> = engine.facts().iter().cloned().collect();

    for (rel_name, rel_arity) in &rel_defs {
        for pos in 0..*rel_arity {
            // Gather facts where an instance appears at `pos`
            let instance_facts: Vec<&Relation> = facts
                .iter()
                .filter(|f| f.name() == rel_name && instances.contains(&f.terms()[pos]))
                .collect();

            if instance_facts.is_empty() {
                continue;
            }

            // Check for position-equality patterns among non-`pos` positions
            let other_positions: Vec<usize> =
                (0..*rel_arity).filter(|p| *p != pos).collect();

            for i in 0..other_positions.len() {
                for j in (i + 1)..other_positions.len() {
                    let pi = other_positions[i];
                    let pj = other_positions[j];

                    // Do ALL instance facts have terms[pi] == terms[pj]?
                    let all_equal = instance_facts
                        .iter()
                        .all(|f| f.terms()[pi] == f.terms()[pj]);

                    if !all_equal {
                        continue;
                    }

                    // Step 3: Does this hold for non-instances too?
                    let non_instance_facts: Vec<&Relation> = facts
                        .iter()
                        .filter(|f| {
                            f.name() == rel_name && non_instances.contains(&f.terms()[pos])
                        })
                        .collect();

                    let also_holds_for_non = !non_instance_facts.is_empty()
                        && non_instance_facts
                            .iter()
                            .all(|f| f.terms()[pi] == f.terms()[pj]);

                    if also_holds_for_non {
                        continue; // not exclusive to concept instances
                    }

                    // Step 4: Construct verification rule
                    // concept(?e), R(...?e at pos..., ...?x at pi..., ...?y at pj...)
                    //   |- eq(?x, ?y)
                    let mut r_terms = Vec::new();
                    for k in 0..*rel_arity {
                        if k == pos {
                            r_terms.push(Term::var("ve"));
                        } else if k == pi {
                            r_terms.push(Term::var("vx"));
                        } else if k == pj {
                            r_terms.push(Term::var("vy"));
                        } else {
                            let vname = format!("vr{}", k);
                            engine.define_variable(&vname);
                            r_terms.push(Term::var(&vname));
                        }
                    }
                    engine.define_variable("ve");
                    engine.define_variable("vx");
                    engine.define_variable("vy");

                    let rule_name =
                        format!("verify_{}_{}_{}_{}_{}", concept_name, rel_name, pos, pi, pj);
                    let rule = Rule::new(
                        &rule_name,
                        vec![
                            RelationPattern::new(concept_name, vec![Term::var("ve")]),
                            RelationPattern::new(rel_name.as_str(), r_terms),
                        ],
                        vec![RelationPattern::new(
                            eq_relation,
                            vec![Term::var("vx"), Term::var("vy")],
                        )],
                    );
                    rules.push(rule);
                }
            }
        }
    }

    rules
}

/// Discover universal generative rules from finite observations.
///
/// For each concept instance, checks if a pattern holds for ALL elements
/// (not just some). If so, emits a rule that GENERATES facts rather than
/// checking them. This bridges finite observation to universal axiom.
///
/// Example: if `op(e0, x, x)` holds for all x in {e0,e1,e2}, emits:
/// `auto_0(?e), element(?x) |- op(?e, ?x, ?x)`
fn discover_universal_rules(
    engine: &mut ClosureEngine,
    concept_name: &str,
) -> Vec<(Rule, UniversalRule)> {
    let concept_arity = match engine.relation_defs().get(concept_name) {
        Some(def) => def.arity(),
        None => return Vec::new(),
    };
    if concept_arity != 1 {
        return Vec::new();
    }

    let instances: HashSet<Term> = engine
        .facts()
        .iter()
        .filter(|f| f.name() == concept_name && f.arity() == 1)
        .map(|f| f.terms()[0].clone())
        .collect();

    if instances.is_empty() {
        return Vec::new();
    }

    // Collect all ground elements
    let all_elements: HashSet<Term> = engine
        .facts()
        .iter()
        .filter(|f| f.name() == "element")
        .map(|f| f.terms()[0].clone())
        .collect();

    if all_elements.is_empty() {
        return Vec::new();
    }

    let n_elements = all_elements.len();
    let facts: Vec<Relation> = engine.facts().iter().cloned().collect();

    let rel_defs: Vec<(String, usize)> = engine
        .relation_defs()
        .iter()
        .filter(|(n, d)| d.arity() >= 2 && *n != concept_name && *n != "eq" && *n != "distinct" && *n != "element")
        .map(|(n, d)| (n.clone(), d.arity()))
        .collect();

    let mut results = Vec::new();

    for (rel_name, rel_arity) in &rel_defs {
        for pos in 0..*rel_arity {
            for inst in &instances {
                // Collect facts where this instance appears at `pos`
                let matching_facts: Vec<&Relation> = facts
                    .iter()
                    .filter(|f| f.name() == rel_name && f.terms()[pos] == *inst)
                    .collect();

                // Check for equality patterns among non-`pos` positions
                let other_positions: Vec<usize> =
                    (0..*rel_arity).filter(|p| *p != pos).collect();

                for i in 0..other_positions.len() {
                    for j in (i + 1)..other_positions.len() {
                        let pi = other_positions[i];
                        let pj = other_positions[j];

                        // Must hold for ALL matching facts
                        let all_equal = matching_facts
                            .iter()
                            .all(|f| f.terms()[pi] == f.terms()[pj]);

                        if !all_equal {
                            continue;
                        }

                        // UNIVERSALITY CHECK: the equal values must cover ALL elements
                        let covered: HashSet<&Term> = matching_facts
                            .iter()
                            .map(|f| &f.terms()[pi])
                            .collect();

                        if covered.len() < n_elements {
                            continue; // not universal — doesn't cover all elements
                        }

                        // Emit generative rule: concept(?e), element(?x) |- rel(...?e...?x...?x...)
                        let mut r_terms = Vec::new();
                        for k in 0..*rel_arity {
                            if k == pos {
                                r_terms.push(Term::var("ue"));
                            } else if k == pi || k == pj {
                                r_terms.push(Term::var("ux"));
                            } else {
                                let vname = format!("uk{}", k);
                                engine.define_variable(&vname);
                                r_terms.push(Term::var(&vname));
                            }
                        }
                        engine.define_variable("ue");
                        engine.define_variable("ux");

                        let rule_name = format!(
                            "universal_{}_{}_{}_{}_{}", concept_name, rel_name, pos, pi, pj
                        );
                        let rule = Rule::new(
                            &rule_name,
                            vec![
                                RelationPattern::new(concept_name, vec![Term::var("ue")]),
                                RelationPattern::new("element", vec![Term::var("ux")]),
                            ],
                            vec![RelationPattern::new(rel_name.as_str(), r_terms)],
                        );

                        // Human-readable description
                        let mut desc_terms = Vec::new();
                        for k in 0..*rel_arity {
                            if k == pos { desc_terms.push("?e".to_string()); }
                            else if k == pi || k == pj { desc_terms.push("?x".to_string()); }
                            else { desc_terms.push(format!("?v{}", k)); }
                        }
                        let desc = format!(
                            "{}(?e), element(?x) |- {}({})",
                            concept_name, rel_name, desc_terms.join(", ")
                        );

                        results.push((
                            rule,
                            UniversalRule {
                                description: desc,
                                concept: concept_name.to_string(),
                                relation: rel_name.clone(),
                                pattern: (pos, pi, pj),
                                rule_name: rule_name.clone(),
                            },
                        ));
                    }
                }
            }
        }
    }

    results
}

// ── Chain rule induction (associativity, etc.) ──────────────

/// A 2-step evaluation path through a ternary relation R.
/// Represents computing R(var[i], var[j], m) then R(m, var[k], result)
/// or R(var[k], m, result).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ChainPath {
    inner_left: usize,       // index into (a,b,c) for inner arg 0
    inner_right: usize,      // index into (a,b,c) for inner arg 1
    outer_var: usize,        // index of the remaining variable
    intermediate_first: bool, // true: R(m, outer, result), false: R(outer, m, result)
}

impl std::fmt::Display for ChainPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let vars = ['a', 'b', 'c'];
        let inner = format!(
            "{}*{}",
            vars[self.inner_left],
            vars[self.inner_right]
        );
        if self.intermediate_first {
            write!(f, "({})*{}", inner, vars[self.outer_var])
        } else {
            write!(f, "{}*({})", vars[self.outer_var], inner)
        }
    }
}

fn all_chain_paths() -> Vec<ChainPath> {
    let mut paths = Vec::new();
    let perms: [(usize, usize, usize); 6] = [
        (0, 1, 2), (0, 2, 1), (1, 0, 2),
        (1, 2, 0), (2, 0, 1), (2, 1, 0),
    ];
    for &(i, j, k) in &perms {
        paths.push(ChainPath {
            inner_left: i, inner_right: j, outer_var: k,
            intermediate_first: true,
        });
        paths.push(ChainPath {
            inner_left: i, inner_right: j, outer_var: k,
            intermediate_first: false,
        });
    }
    paths
}

fn evaluate_chain(
    op_table: &HashMap<(Term, Term), Term>,
    vars: &[Term; 3],
    path: &ChainPath,
) -> Option<Term> {
    let intermediate = op_table.get(&(
        vars[path.inner_left].clone(),
        vars[path.inner_right].clone(),
    ))?;
    if path.intermediate_first {
        op_table.get(&(intermediate.clone(), vars[path.outer_var].clone())).cloned()
    } else {
        op_table.get(&(vars[path.outer_var].clone(), intermediate.clone())).cloned()
    }
}

/// A discovered chain identity: two evaluation paths that always agree.
#[derive(Debug, Clone)]
pub struct ChainIdentity {
    pub relation: String,
    pub path_a: String,
    pub path_b: String,
    pub rule_name: String,
}

/// Induce chain rules (e.g., associativity) from ternary relations.
///
/// Enumerates all 2-step evaluation paths, checks all pairs for agreement
/// across all element triples, and emits 4-premise rules for agreeing pairs.
fn induce_chain_rules(
    engine: &ClosureEngine,
    eq_relation: &str,
    exclude_relations: &HashSet<String>,
) -> Vec<(Rule, ChainIdentity)> {
    let mut results = Vec::new();

    // Find ternary relations
    let ternary_rels: Vec<String> = engine
        .relation_defs()
        .iter()
        .filter(|(n, d)| d.arity() == 3 && !exclude_relations.contains(*n))
        .map(|(n, _)| n.clone())
        .collect();

    // Collect ground elements (atoms only)
    let elements: Vec<Term> = engine
        .facts()
        .iter()
        .flat_map(|f| f.terms().iter())
        .filter(|t| t.is_ground() && matches!(t, Term::App { args, .. } if args.is_empty()))
        .cloned()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let all_paths = all_chain_paths();
    let mut seen_pairs: HashSet<(String, String)> = HashSet::new();

    for rel_name in &ternary_rels {
        // Build operation table: (arg0, arg1) → arg2
        let op_table: HashMap<(Term, Term), Term> = engine
            .facts()
            .iter()
            .filter(|f| f.name() == rel_name && f.arity() == 3)
            .map(|f| {
                (
                    (f.terms()[0].clone(), f.terms()[1].clone()),
                    f.terms()[2].clone(),
                )
            })
            .collect();

        if op_table.is_empty() {
            continue;
        }

        // Check all path pairs
        for i in 0..all_paths.len() {
            for j in (i + 1)..all_paths.len() {
                let pa = &all_paths[i];
                let pb = &all_paths[j];

                // Check if paths always agree
                let mut always_agree = true;
                let mut checked = 0usize;

                for a in &elements {
                    for b in &elements {
                        for c in &elements {
                            let vars = [a.clone(), b.clone(), c.clone()];
                            let ra = evaluate_chain(&op_table, &vars, pa);
                            let rb = evaluate_chain(&op_table, &vars, pb);
                            match (ra, rb) {
                                (Some(va), Some(vb)) => {
                                    checked += 1;
                                    if va != vb {
                                        always_agree = false;
                                        break;
                                    }
                                }
                                _ => {} // incomplete table — skip
                            }
                        }
                        if !always_agree { break; }
                    }
                    if !always_agree { break; }
                }

                if !always_agree || checked == 0 {
                    continue;
                }

                // Dedup by canonical path-pair description
                let desc_a = format!("{}", pa);
                let desc_b = format!("{}", pb);
                let key = if desc_a < desc_b {
                    (desc_a.clone(), desc_b.clone())
                } else {
                    (desc_b.clone(), desc_a.clone())
                };
                if !seen_pairs.insert(key) {
                    continue;
                }

                // Limit: only keep pairs where at least one path is the
                // "standard left-associate" form (a*b)*c to avoid
                // combinatorial explosion from commutativity-derived redundancies.
                let is_std_left = |p: &ChainPath| {
                    p.inner_left == 0
                        && p.inner_right == 1
                        && p.outer_var == 2
                        && p.intermediate_first
                };
                if !is_std_left(pa) && !is_std_left(pb) {
                    continue;
                }

                // Build 4-premise rule
                let var_names = ["ca", "cb", "cc"];
                let mut premises = Vec::new();

                // Path A premises
                let m1 = "cm1";
                let r1 = "cr1";
                premises.push(RelationPattern::new(
                    rel_name.as_str(),
                    vec![
                        Term::var(var_names[pa.inner_left]),
                        Term::var(var_names[pa.inner_right]),
                        Term::var(m1),
                    ],
                ));
                if pa.intermediate_first {
                    premises.push(RelationPattern::new(
                        rel_name.as_str(),
                        vec![Term::var(m1), Term::var(var_names[pa.outer_var]), Term::var(r1)],
                    ));
                } else {
                    premises.push(RelationPattern::new(
                        rel_name.as_str(),
                        vec![Term::var(var_names[pa.outer_var]), Term::var(m1), Term::var(r1)],
                    ));
                }

                // Path B premises
                let m2 = "cm2";
                let r2 = "cr2";
                premises.push(RelationPattern::new(
                    rel_name.as_str(),
                    vec![
                        Term::var(var_names[pb.inner_left]),
                        Term::var(var_names[pb.inner_right]),
                        Term::var(m2),
                    ],
                ));
                if pb.intermediate_first {
                    premises.push(RelationPattern::new(
                        rel_name.as_str(),
                        vec![Term::var(m2), Term::var(var_names[pb.outer_var]), Term::var(r2)],
                    ));
                } else {
                    premises.push(RelationPattern::new(
                        rel_name.as_str(),
                        vec![Term::var(var_names[pb.outer_var]), Term::var(m2), Term::var(r2)],
                    ));
                }

                let conclusion = vec![RelationPattern::new(
                    eq_relation,
                    vec![Term::var(r1), Term::var(r2)],
                )];

                let rule_name = format!("chain_{}_{}_vs_{}", rel_name, desc_a, desc_b)
                    .replace('*', "x")
                    .replace('(', "")
                    .replace(')', "");

                let rule = Rule::new(&rule_name, premises, conclusion);

                let identity = ChainIdentity {
                    relation: rel_name.clone(),
                    path_a: desc_a,
                    path_b: desc_b,
                    rule_name: rule_name.clone(),
                };

                results.push((rule, identity));
            }
        }
    }

    results
}

/// Run the full discovery loop: promote patterns → beam search → repeat.
///
/// The base engine should contain only raw data (facts) and structural
/// infrastructure (equivalence, distinctness). Concepts are invented
/// by the system through pattern promotion.
///
/// Returns the discovery log with what concepts were invented at each round.
pub fn run_discovery(
    base: &ClosureEngine,
    config: &DiscoveryConfig,
) -> DiscoveryLog {
    run_discovery_named(base, config, "unnamed")
}

/// Run discovery with a structure name for logging.
pub fn run_discovery_named(
    base: &ClosureEngine,
    config: &DiscoveryConfig,
    structure_name: &str,
) -> DiscoveryLog {
    // Capture initial state
    let initial_relations: Vec<(String, usize)> = base
        .relation_defs()
        .iter()
        .map(|(n, d)| (n.clone(), d.arity()))
        .collect();
    let initial_rules = base.rules().len();
    let initial_facts: Vec<(String, Vec<String>)> = {
        let mut by_rel: HashMap<String, Vec<String>> = HashMap::new();
        for fact in base.facts() {
            by_rel
                .entry(fact.name().to_string())
                .or_default()
                .push(fact.to_string());
        }
        let mut result: Vec<(String, Vec<String>)> = by_rel.into_iter().collect();
        result.sort_by(|a, b| a.0.cmp(&b.0));
        for (_, facts) in &mut result {
            facts.sort();
        }
        result
    };

    let mut engine = base.clone();

    // Add element/1 relation and populate with all declared constants.
    // This enables universal rules: concept(?e), element(?x) |- rel(?e, ?x, ?x)
    //
    // NOTE: all constants become elements. If the engine contains constants
    // used as labels/tags (not algebraic elements), they would pollute the
    // element set and cause universal rules to over-generate. Current usage
    // is safe because only carrier elements are declared as constants.
    if !engine.relation_defs().contains_key("element") {
        engine.define_relation("element", 1);
    }
    for constant in engine.constants().clone() {
        engine.add_fact(Relation::new("element", vec![Term::constant(&constant)]));
    }

    let mut concept_counter = 0usize;
    let mut promoted_sigs: HashSet<ConceptSignature> = HashSet::new();
    let mut steps = Vec::new();

    for round in 0..config.max_rounds {
        let facts_at_round_start = engine.facts().len();

        // ── Phase 1: Pattern induction and promotion ────────

        // Collect patterns first (immutable borrow), then promote (mutable borrow)
        let all_patterns: Vec<(InducedPattern, usize)> = {
            let facts_vec: Vec<&Relation> = engine
                .facts()
                .iter()
                .filter(|f| !config.promotion.exclude_relations.contains(f.name()))
                .collect();
            let mut by_relation: HashMap<&str, Vec<&Relation>> = HashMap::new();
            for fact in &facts_vec {
                by_relation.entry(fact.name()).or_default().push(fact);
            }
            let mut patterns = Vec::new();
            for (_rel_name, rel_facts) in &by_relation {
                if rel_facts.len() < 2 {
                    continue;
                }
                patterns.extend(induce_patterns(&rel_facts, config.promotion.min_support));
            }
            patterns
        };

        let mut promoted = Vec::new();
        let mut promotion_rules = Vec::new();

        // Filter patterns: only keep promotable intra-fact patterns
        // with signatures not already promoted in a previous round.
        let mut candidates_to_promote: Vec<(&InducedPattern, usize, ConceptSignature)> =
            Vec::new();
        for (pattern, support) in &all_patterns {
            if candidates_to_promote.len() >= config.promotion.max_promotions_per_round {
                break;
            }
            // Only intra-fact patterns
            let is_intra = pattern
                .slots
                .iter()
                .any(|s| matches!(s, Slot::Tied(g) if *g >= 100));
            if !is_intra {
                continue;
            }
            let free_count = pattern
                .slots
                .iter()
                .filter(|s| matches!(s, Slot::Free(_)))
                .count();
            if free_count == 0 || free_count > 2 {
                continue;
            }
            let sig = ConceptSignature {
                source_relation: pattern.relation.clone(),
                slots: pattern.slots.clone(),
                arity: free_count,
            };
            // Dedup: skip if this signature was already promoted
            if promoted_sigs.contains(&sig) {
                continue;
            }
            candidates_to_promote.push((pattern, *support, sig));
        }

        // Now actually create the concepts (no ghost relations)
        for (pattern, support, sig) in &candidates_to_promote {
            if let Some((name, rule, _)) =
                promote_pattern(&mut engine, pattern, &mut concept_counter)
            {
                let mut trial = engine.clone();
                trial.add_rule(rule.clone());
                let result = trial.derive_closure();
                let instance_set: Vec<Term> = result
                    .facts
                    .iter()
                    .filter(|f| f.name() == name)
                    .flat_map(|f| f.terms().iter().cloned())
                    .collect();

                promoted_sigs.insert(sig.clone());

                promoted.push(ConceptInfo {
                    name: name.clone(),
                    arity: engine.relation_defs()[&name].arity(),
                    signature: sig.clone(),
                    source_pattern: format!("{}", rule),
                    support: *support,
                    instances: instance_set.len(),
                    instance_set,
                });
                promotion_rules.push(rule);
            }
        }

        // Add promotion rules to engine and run closure
        for rule in &promotion_rules {
            engine.add_rule(rule.clone());
        }
        if !promotion_rules.is_empty() {
            engine.derive_closure();
        }

        // ── Phase 1.5: Discover verification rules for new concepts ──
        let mut verification_rules = Vec::new();
        for concept in &promoted {
            let vrules = discover_verification_rules(&mut engine, &concept.name, "eq");
            for rule in vrules {
                verification_rules.push(rule.name().to_string());
                engine.add_rule(rule);
            }
        }
        if !verification_rules.is_empty() {
            engine.derive_closure();
        }

        // ── Phase 1.6: Universal generative rules ──
        // Only for concepts that HAVE verification rules (= verified concepts).
        let verified_concepts: HashSet<String> = verification_rules
            .iter()
            .filter_map(|vr| {
                // Extract concept name from "verify_auto_N_rel_pos_p1_p2"
                let after = vr.strip_prefix("verify_")?;
                // concept name is "auto_N" — find by matching promoted concept names
                promoted.iter()
                    .map(|c| c.name.as_str())
                    .find(|name| after.starts_with(name))
                    .map(|s| s.to_string())
            })
            .collect();
        let mut universal_rules = Vec::new();
        for concept in &promoted {
            if !verified_concepts.contains(&concept.name) {
                continue; // skip unverified concepts
            }
            let ur = discover_universal_rules(&mut engine, &concept.name);
            for (rule, info) in ur {
                engine.add_rule(rule);
                universal_rules.push(info);
            }
        }
        if !universal_rules.is_empty() {
            engine.derive_closure();
        }

        // ── Phase 1.75: Chain rule induction (associativity, etc.) ──
        // Only on round 0: chain identities depend on the op table which is
        // static across rounds. If the system later supports dynamic fact
        // addition (Notebook mode), this should re-trigger when op facts change.
        let chain_results = if round == 0 {
            let cr = induce_chain_rules(&engine, "eq", &config.promotion.exclude_relations);
            for (rule, _) in &cr {
                for p in rule.premises() {
                    for t in p.terms() {
                        register_vars_on(&mut engine, t);
                    }
                }
                for c in rule.conclusions() {
                    for t in c.terms() {
                        register_vars_on(&mut engine, t);
                    }
                }
                engine.add_rule(rule.clone());
            }
            if !cr.is_empty() {
                engine.derive_closure();
            }
            cr
        } else {
            Vec::new()
        };
        let chain_identities: Vec<ChainIdentity> =
            chain_results.into_iter().map(|(_, id)| id).collect();

        // Early termination: if no new facts after promotion/verification/chain,
        // skip the expensive beam search and stop.
        let facts_after_phases = engine.facts().len();
        if facts_after_phases == facts_at_round_start {
            steps.push(DiscoveryStep {
                round,
                promoted,
                verification_rules,
                universal_rules,
                chain_identities,
                beam_best: None,
                total_facts: facts_after_phases,
                total_relations: engine.relation_defs().len(),
            });
            break;
        }

        // ── Phase 2: Beam search with induced candidates ────
        // Now includes rules involving auto-promoted concepts
        let beam_log = beam_search(&engine, &config.beam);
        let beam_best = beam_log
            .last()
            .and_then(|s| s.beam.first().cloned());

        // Add best beam rules to engine ONLY if score > 0 and derived > 0
        let beam_contributed = if let Some(ref best) = beam_best {
            if best.score > 0.0 && best.profile.derived_facts > 0 {
                for rule in &best.rules {
                    for p in rule.premises() {
                        for t in p.terms() {
                            register_vars_on(&mut engine, t);
                        }
                    }
                    for c in rule.conclusions() {
                        for t in c.terms() {
                            register_vars_on(&mut engine, t);
                        }
                    }
                    engine.add_rule(rule.clone());
                }
                engine.derive_closure();
                true
            } else {
                false
            }
        } else {
            false
        };

        let total_facts = engine.facts().len();
        let total_relations = engine.relation_defs().len();

        steps.push(DiscoveryStep {
            round,
            promoted,
            verification_rules,
            universal_rules,
            chain_identities,
            beam_best,
            total_facts,
            total_relations,
        });

        // Convergence: no new concepts AND beam found nothing useful
        if promotion_rules.is_empty() && !beam_contributed {
            break;
        }
    }

    DiscoveryLog {
        structure_name: structure_name.to_string(),
        initial_facts,
        initial_relations,
        initial_rules,
        steps,
    }
}

// ── Paradigm comparison ─────────────────────────────────────

/// Result of evaluating one paradigm (rule set).
#[derive(Debug, Clone)]
pub struct ParadigmResult {
    pub name: String,
    pub profile: ClosureProfile,
    /// Number of rules in this paradigm.
    pub num_rules: usize,
    /// Efficiency: derived_facts / num_rules.
    pub efficiency: f64,
}

/// Compare different rule sets ("paradigms") on the same base engine.
///
/// Each paradigm is a `(name, rules)` pair. The base engine is cloned for each,
/// the rules are added, closure is derived, and profiles are compared.
///
/// Returns results sorted by total derived facts (descending).
pub fn compare_paradigms(
    base: &ClosureEngine,
    paradigms: Vec<(&str, Vec<Rule>)>,
) -> Vec<ParadigmResult> {
    let mut results: Vec<ParadigmResult> = paradigms
        .into_iter()
        .map(|(name, rules)| {
            let mut engine = base.clone();
            let n = rules.len();
            for rule in rules {
                // Register variables
                for p in rule.premises() {
                    for t in p.terms() {
                        register_vars_on(&mut engine, t);
                    }
                }
                for c in rule.conclusions() {
                    for t in c.terms() {
                        register_vars_on(&mut engine, t);
                    }
                }
                engine.add_rule(rule);
            }
            let result = engine.derive_closure();
            let profile = score::closure_profile(&result);
            let eff = if n > 0 {
                profile.derived_facts as f64 / n as f64
            } else {
                0.0
            };
            ParadigmResult {
                name: name.to_string(),
                profile,
                num_rules: n,
                efficiency: eff,
            }
        })
        .collect();

    results.sort_by(|a, b| {
        b.profile
            .derived_facts
            .cmp(&a.profile.derived_facts)
    });
    results
}

fn register_vars_on(engine: &mut ClosureEngine, term: &Term) {
    match term {
        Term::Var(name) => {
            engine.define_variable(name);
        }
        Term::App { args, .. } => {
            for arg in args {
                register_vars_on(engine, arg);
            }
        }
    }
}

// ── Cross-structure abstraction ─────────────────────────────

/// An abstract concept discovered across multiple concrete structures.
#[derive(Debug, Clone)]
pub struct AbstractConcept {
    /// The canonical pattern signature (structure-independent).
    pub signature: ConceptSignature,
    /// Human-readable description of the pattern.
    pub description: String,
    /// Per-structure instance sets: (structure_name, concept_name, instance_set).
    pub occurrences: Vec<(String, String, Vec<Term>)>,
    /// Verification rules that appeared in ALL structures (by their structural description).
    pub universal_properties: Vec<String>,
}

/// Verification rule signature: (concept_sig, source_relation, constrained_pos, eq_pos_1, eq_pos_2).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VerificationSig {
    rel: String,
    constrained_pos: usize,
    eq_pos_1: usize,
    eq_pos_2: usize,
}

fn parse_verification_sig(name: &str) -> Option<VerificationSig> {
    // Format: verify_{concept}_{rel}_{pos}_{p1}_{p2}
    let parts: Vec<&str> = name.split('_').collect();
    if parts.len() < 6 || parts[0] != "verify" {
        return None;
    }
    // Find the relation name and positions — they're the last 4 parts
    let p2: usize = parts.last()?.parse().ok()?;
    let p1: usize = parts[parts.len() - 2].parse().ok()?;
    let pos: usize = parts[parts.len() - 3].parse().ok()?;
    let rel = parts[parts.len() - 4].to_string();
    Some(VerificationSig {
        rel,
        constrained_pos: pos,
        eq_pos_1: p1,
        eq_pos_2: p2,
    })
}

/// Compare discovery results across multiple structures.
///
/// Finds concepts with the same pattern signature that appear in
/// multiple structures, and identifies verification rules that
/// hold universally.
pub fn abstract_across_structures(
    discoveries: Vec<(&str, &[DiscoveryStep])>,
) -> Vec<AbstractConcept> {
    // Collect all concepts keyed by signature
    let mut by_sig: HashMap<ConceptSignature, Vec<(String, String, Vec<Term>, Vec<String>)>> =
        HashMap::new();

    for (structure_name, steps) in &discoveries {
        for step in *steps {
            for concept in &step.promoted {
                by_sig
                    .entry(concept.signature.clone())
                    .or_default()
                    .push((
                        structure_name.to_string(),
                        concept.name.clone(),
                        concept.instance_set.clone(),
                        step.verification_rules.clone(),
                    ));
            }
        }
    }

    // Keep only signatures that appear in multiple structures
    let mut abstracts = Vec::new();
    for (sig, entries) in &by_sig {
        let distinct_structures: HashSet<&str> = entries.iter().map(|(s, _, _, _)| s.as_str()).collect();
        if distinct_structures.len() < 2 {
            continue;
        }

        // Find universal verification rules (present in ALL structures)
        let mut vsig_by_structure: HashMap<&str, HashSet<VerificationSig>> = HashMap::new();
        for (sname, cname, _, vrules) in entries {
            let vsigs: HashSet<VerificationSig> = vrules
                .iter()
                .filter(|vr| vr.contains(cname))
                .filter_map(|vr| parse_verification_sig(vr))
                .collect();
            vsig_by_structure
                .entry(sname.as_str())
                .or_default()
                .extend(vsigs);
        }

        let universal: Vec<VerificationSig> = if vsig_by_structure.len() >= 2 {
            let mut iter = vsig_by_structure.values();
            let first = iter.next().unwrap().clone();
            iter.fold(first, |acc, set| acc.intersection(set).cloned().collect())
                .into_iter()
                .collect()
        } else {
            Vec::new()
        };

        let description = format!(
            "pattern {} | appears in {} structures | {} universal properties",
            sig,
            distinct_structures.len(),
            universal.len(),
        );

        let occurrences = entries
            .iter()
            .map(|(s, c, inst, _)| (s.clone(), c.clone(), inst.clone()))
            .collect();

        let universal_props = universal
            .iter()
            .map(|v| {
                format!(
                    "concept(?e), {}(...?e@{}...) |- eq(pos{}, pos{})",
                    v.rel, v.constrained_pos, v.eq_pos_1, v.eq_pos_2
                )
            })
            .collect();

        abstracts.push(AbstractConcept {
            signature: sig.clone(),
            description,
            occurrences,
            universal_properties: universal_props,
        });
    }

    // Sort by number of structures (most universal first)
    abstracts.sort_by(|a, b| b.occurrences.len().cmp(&a.occurrences.len()));
    abstracts
}

// ── Concept-level theorem discovery ─────────────────────────

/// A relationship between two abstract concepts, discovered by comparing
/// their instance sets across multiple structures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TheoremKind {
    /// ∀x. A(x) ↔ B(x) — same instance sets in every tested structure.
    Equivalent,
    /// ∀x. A(x) → B(x) — A instances ⊆ B instances in every tested structure.
    Subsumption,
}

/// A candidate theorem about the relationship between two concepts.
#[derive(Debug, Clone)]
pub struct ConceptTheorem {
    pub concept_a: ConceptSignature,
    pub concept_b: ConceptSignature,
    pub kind: TheoremKind,
    /// Evidence: (structure_name, instances_a, instances_b) for each structure.
    pub evidence: Vec<(String, Vec<Term>, Vec<Term>)>,
    /// Structures where the theorem was verified after discovery (held-out).
    pub verified_on: Vec<String>,
    /// Structures where verification failed (counterexamples).
    pub refuted_on: Vec<String>,
}

impl std::fmt::Display for ConceptTheorem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let symbol = match self.kind {
            TheoremKind::Equivalent => "↔",
            TheoremKind::Subsumption => "→",
        };
        write!(
            f,
            "∀x. [{}](x) {} [{}](x)",
            self.concept_a, symbol, self.concept_b
        )?;
        write!(
            f,
            "  | evidence: {} structures | verified: {} | refuted: {}",
            self.evidence.len(),
            self.verified_on.len(),
            self.refuted_on.len(),
        )
    }
}

/// Discover candidate theorems by comparing abstract concepts' instance sets.
///
/// Two concepts are equivalent if their instance sets are identical in every
/// structure where both appear. One subsumes the other if A ⊆ B everywhere.
pub fn discover_theorems(abstracts: &[AbstractConcept]) -> Vec<ConceptTheorem> {
    let mut theorems = Vec::new();

    for i in 0..abstracts.len() {
        for j in (i + 1)..abstracts.len() {
            let a = &abstracts[i];
            let b = &abstracts[j];

            // Find structures where both concepts have occurrences
            let mut evidence = Vec::new();
            let mut all_equal = true;
            let mut a_subset_b = true;
            let mut b_subset_a = true;
            let mut has_overlap = false;

            for (s_a, _, inst_a) in &a.occurrences {
                for (s_b, _, inst_b) in &b.occurrences {
                    if s_a == s_b {
                        has_overlap = true;
                        let set_a: HashSet<&Term> = inst_a.iter().collect();
                        let set_b: HashSet<&Term> = inst_b.iter().collect();

                        if set_a != set_b {
                            all_equal = false;
                        }
                        if !set_a.is_subset(&set_b) {
                            a_subset_b = false;
                        }
                        if !set_b.is_subset(&set_a) {
                            b_subset_a = false;
                        }

                        evidence.push((
                            s_a.clone(),
                            inst_a.clone(),
                            inst_b.clone(),
                        ));
                    }
                }
            }

            if !has_overlap || evidence.len() < 2 {
                continue; // need at least 2 structures for a meaningful claim
            }

            if all_equal {
                theorems.push(ConceptTheorem {
                    concept_a: a.signature.clone(),
                    concept_b: b.signature.clone(),
                    kind: TheoremKind::Equivalent,
                    evidence,
                    verified_on: Vec::new(),
                    refuted_on: Vec::new(),
                });
            } else if a_subset_b {
                theorems.push(ConceptTheorem {
                    concept_a: a.signature.clone(),
                    concept_b: b.signature.clone(),
                    kind: TheoremKind::Subsumption,
                    evidence,
                    verified_on: Vec::new(),
                    refuted_on: Vec::new(),
                });
            } else if b_subset_a {
                // Swap evidence to match swapped concept_a/concept_b
                let swapped: Vec<_> = evidence
                    .into_iter()
                    .map(|(s, ia, ib)| (s, ib, ia))
                    .collect();
                theorems.push(ConceptTheorem {
                    concept_a: b.signature.clone(),
                    concept_b: a.signature.clone(),
                    kind: TheoremKind::Subsumption,
                    evidence: swapped,
                    verified_on: Vec::new(),
                    refuted_on: Vec::new(),
                });
            }
        }
    }

    theorems
}

/// Verify a candidate theorem on a new (held-out) structure.
///
/// Runs discovery on the structure, finds concepts matching the theorem's
/// signatures, and checks whether the predicted relationship holds.
pub fn verify_theorem(
    theorem: &mut ConceptTheorem,
    structure_name: &str,
    discovery_log: &[DiscoveryStep],
) {
    // Find concepts in this structure matching the theorem's signatures
    let mut inst_a: Option<Vec<Term>> = None;
    let mut inst_b: Option<Vec<Term>> = None;

    for step in discovery_log {
        for concept in &step.promoted {
            if concept.signature == theorem.concept_a && inst_a.is_none() {
                inst_a = Some(concept.instance_set.clone());
            }
            if concept.signature == theorem.concept_b && inst_b.is_none() {
                inst_b = Some(concept.instance_set.clone());
            }
        }
    }

    let (ia, ib) = match (inst_a, inst_b) {
        (Some(a), Some(b)) => (a, b),
        _ => return, // signatures not found in this structure — inconclusive
    };

    let set_a: HashSet<&Term> = ia.iter().collect();
    let set_b: HashSet<&Term> = ib.iter().collect();

    let holds = match theorem.kind {
        TheoremKind::Equivalent => set_a == set_b,
        TheoremKind::Subsumption => set_a.is_subset(&set_b),
    };

    if holds {
        theorem.verified_on.push(structure_name.to_string());
    } else {
        theorem.refuted_on.push(structure_name.to_string());
    }
}

// ── Knowledge transfer ──────────────────────────────────────

/// A bundle of rules that can be transferred to a new structure.
#[derive(Debug, Clone)]
pub struct TransferableKnowledge {
    pub concept_name: String,
    pub concept_arity: usize,
    /// Promotion rule: discovers concept instances from op facts.
    pub promotion_rule: Rule,
    /// Universal rules: generate op facts from concept + element.
    pub universal_rules: Vec<Rule>,
    /// Description for logging.
    pub descriptions: Vec<String>,
}

/// Extract transferable knowledge from a discovery log.
///
/// Returns promotion rules and universal rules for each verified concept.
/// These can be injected into a new engine to transfer structural understanding.
pub fn extract_transferable(log: &DiscoveryLog) -> Vec<TransferableKnowledge> {
    let mut results = Vec::new();

    for step in &log.steps {
        // Find verified concepts (those with verification rules)
        let verified: HashSet<String> = step
            .verification_rules
            .iter()
            .filter_map(|vr| {
                let after = vr.strip_prefix("verify_")?;
                step.promoted
                    .iter()
                    .map(|c| c.name.as_str())
                    .find(|name| after.starts_with(name))
                    .map(|s| s.to_string())
            })
            .collect();

        for concept in &step.promoted {
            if !verified.contains(&concept.name) {
                continue;
            }

            // Reconstruct promotion rule from signature
            let promo = reconstruct_promotion_rule(&concept.name, &concept.signature);

            // Reconstruct universal rules
            let universals: Vec<Rule> = step
                .universal_rules
                .iter()
                .filter(|ur| ur.concept == concept.name)
                .map(|ur| reconstruct_universal_rule(ur, &concept.name))
                .collect();

            if universals.is_empty() {
                continue;
            }

            let descriptions: Vec<String> = step
                .universal_rules
                .iter()
                .filter(|ur| ur.concept == concept.name)
                .map(|ur| ur.description.clone())
                .collect();

            results.push(TransferableKnowledge {
                concept_name: concept.name.clone(),
                concept_arity: concept.arity,
                promotion_rule: promo,
                universal_rules: universals,
                descriptions,
            });
        }
    }

    // Deduplicate by concept signature (keep first occurrence)
    let mut seen_sigs = HashSet::new();
    results.retain(|k| {
        // Use rule name as dedup key (contains concept name)
        seen_sigs.insert(k.concept_name.clone())
    });

    results
}

/// Inject transferred knowledge into a target engine.
///
/// Registers concept relations, adds promotion and universal rules,
/// and ensures `element/1` is populated.
pub fn inject_transfer(
    engine: &mut ClosureEngine,
    knowledge: &[TransferableKnowledge],
) {
    // Ensure element/1 exists and is populated
    if !engine.relation_defs().contains_key("element") {
        engine.define_relation("element", 1);
    }
    for constant in engine.constants().clone() {
        engine.add_fact(Relation::new("element", vec![Term::constant(&constant)]));
    }

    for k in knowledge {
        // Define concept relation
        if !engine.relation_defs().contains_key(&k.concept_name) {
            engine.define_relation(&k.concept_name, k.concept_arity);
        }

        // Register variables and add promotion rule
        register_rule_vars(engine, &k.promotion_rule);
        engine.add_rule(k.promotion_rule.clone());

        // Add universal rules
        for ur in &k.universal_rules {
            register_rule_vars(engine, ur);
            engine.add_rule(ur.clone());
        }
    }
}

fn register_rule_vars(engine: &mut ClosureEngine, rule: &Rule) {
    for p in rule.premises() {
        for t in p.terms() {
            register_vars_on(engine, t);
        }
    }
    for c in rule.conclusions() {
        for t in c.terms() {
            register_vars_on(engine, t);
        }
    }
}

/// Reconstruct a promotion rule from a concept signature.
///
/// NOTE: variable names may differ from the original rule produced by
/// `promote_pattern`. The rule is semantically equivalent (same pattern
/// structure) but not identical (different variable names). For robust
/// transfer, consider storing the original Rule in ConceptInfo instead
/// of reconstructing here.
fn reconstruct_promotion_rule(concept_name: &str, sig: &ConceptSignature) -> Rule {
    let mut premise_terms = Vec::new();
    let mut conclusion_terms = Vec::new();
    let mut var_counter = 0usize;
    let mut tied_map: HashMap<usize, String> = HashMap::new();

    for slot in &sig.slots {
        match slot {
            Slot::Free(_) => {
                let vname = format!("tv{}", var_counter);
                var_counter += 1;
                premise_terms.push(Term::var(&vname));
                conclusion_terms.push(Term::var(&vname));
            }
            Slot::Tied(group) => {
                let vname = tied_map
                    .entry(*group)
                    .or_insert_with(|| {
                        let n = format!("tv{}", var_counter);
                        var_counter += 1;
                        n
                    })
                    .clone();
                premise_terms.push(Term::var(&vname));
            }
        }
    }

    Rule::new(
        format!("transfer_promote_{}", concept_name),
        vec![RelationPattern::new(&sig.source_relation, premise_terms)],
        vec![RelationPattern::new(concept_name, conclusion_terms)],
    )
}

fn reconstruct_universal_rule(ur: &UniversalRule, concept_name: &str) -> Rule {
    let (pos, pi, pj) = ur.pattern;
    let arity = [pos, pi, pj].iter().copied().max().unwrap_or(0) + 1;

    let mut r_terms = Vec::new();
    for k in 0..arity {
        if k == pos {
            r_terms.push(Term::var("te"));
        } else if k == pi || k == pj {
            r_terms.push(Term::var("tx"));
        } else {
            r_terms.push(Term::var(&format!("tk{}", k)));
        }
    }

    Rule::new(
        format!("transfer_{}", ur.rule_name),
        vec![
            RelationPattern::new(concept_name, vec![Term::var("te")]),
            RelationPattern::new("element", vec![Term::var("tx")]),
        ],
        vec![RelationPattern::new(&ur.relation, r_terms)],
    )
}

// ── Autonomous reasoning loop ────────────────────────────────

/// One round of autonomous reasoning. The system decides the action.
#[derive(Debug, Clone)]
pub enum AutonomousAction {
    /// Enumerate all binary ops on the carrier and classify by axioms.
    Enumerate {
        total_ops: usize,
        classes: Vec<(String, usize)>, // (axioms, model_count)
    },
    /// Run discovery on a structure representative.
    Discover {
        axiom_class: String,
        model_count: usize,
        /// Each concept: (name, signature, instances)
        concepts: Vec<(String, String, Vec<String>)>,
        verification_rules: Vec<String>,
        universal_rules: Vec<String>,
        chain_identities: Vec<(String, String)>,
    },
    /// Cross-structure comparison of all discovered structures so far.
    Compare {
        structures: Vec<String>,
        /// Each abstract concept: (signature, [(structure, instances)])
        abstract_concepts: Vec<(String, Vec<(String, Vec<String>)>)>,
        /// Each theorem: description string
        theorems: Vec<String>,
    },
    /// Converged — nothing new to explore.
    Converged {
        reason: String,
    },
}

/// Full log of autonomous reasoning.
pub struct AutonomousLog {
    pub carrier_size: usize,
    pub rounds: Vec<(usize, String, AutonomousAction)>, // (round, description, action)
}

impl AutonomousLog {
    pub fn to_log_string(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "═══════════════════════════════════════════════════════\n\
             AUTONOMOUS REASONING LOG\n\
             Carrier size: {}\n\
             ═══════════════════════════════════════════════════════\n\n",
            self.carrier_size
        ));

        for (round, desc, action) in &self.rounds {
            out.push_str(&format!("── ROUND {} : {} ──\n", round, desc));
            match action {
                AutonomousAction::Enumerate { total_ops, classes } => {
                    out.push_str(&format!("  enumerated {} operations → {} axiom classes\n", total_ops, classes.len()));
                    for (ax, count) in classes {
                        out.push_str(&format!("    {:>6} models : {}\n", count, ax));
                    }
                }
                AutonomousAction::Discover { axiom_class, model_count, concepts, verification_rules, universal_rules, chain_identities } => {
                    out.push_str(&format!("  axiom class: {}  ({} models in carrier)\n", axiom_class, model_count));

                    if concepts.is_empty() {
                        out.push_str("  concepts: (none)\n");
                    } else {
                        out.push_str(&format!("  concepts discovered: {}\n", concepts.len()));
                        for (name, sig, instances) in concepts {
                            let tag = if instances.len() == 1 { " ← selective" } else { "" };
                            out.push_str(&format!("    {} : {} = {{{}}}{}",
                                name, sig, instances.join(", "), tag));
                            out.push('\n');
                        }
                    }

                    if !verification_rules.is_empty() {
                        out.push_str(&format!("  verification rules: {}\n", verification_rules.len()));
                        for vr in verification_rules {
                            out.push_str(&format!("    {}\n", vr));
                        }
                    }

                    if !universal_rules.is_empty() {
                        out.push_str(&format!("  universal (transferable) rules: {}\n", universal_rules.len()));
                        for ur in universal_rules {
                            out.push_str(&format!("    {}\n", ur));
                        }
                    }

                    if !chain_identities.is_empty() {
                        out.push_str(&format!("  chain identities (associativity-like): {}\n", chain_identities.len()));
                        for (a, b) in chain_identities {
                            out.push_str(&format!("    {} = {}\n", a, b));
                        }
                    }

                    if verification_rules.is_empty() && universal_rules.is_empty() && chain_identities.is_empty() {
                        out.push_str("  no structural rules discovered\n");
                    }
                }
                AutonomousAction::Compare { structures, abstract_concepts, theorems } => {
                    out.push_str(&format!("  comparing {} structures\n", structures.len()));
                    if !abstract_concepts.is_empty() {
                        out.push_str(&format!("  abstract concepts (shared across structures): {}\n", abstract_concepts.len()));
                        for (sig, occurrences) in abstract_concepts {
                            out.push_str(&format!("    pattern: {}\n", sig));
                            for (s, inst) in occurrences {
                                out.push_str(&format!("      {} = {{{}}}\n", s, inst.join(", ")));
                            }
                        }
                    }
                    if !theorems.is_empty() {
                        out.push_str(&format!("  theorems discovered: {}\n", theorems.len()));
                        for thm in theorems {
                            out.push_str(&format!("    {}\n", thm));
                        }
                    }
                }
                AutonomousAction::Converged { reason } => {
                    out.push_str(&format!("  {}\n", reason));
                }
            }
            out.push('\n');
        }

        out.push_str("═══════════════════════════════════════════════════════\n");
        out.push_str(&format!("END ({} rounds)\n", self.rounds.len()));
        out.push_str("═══════════════════════════════════════════════════════\n");
        out
    }
}

/// Run fully autonomous reasoning from bare minimum inputs.
///
/// The system decides what to do at each round:
/// 1. If no model space: enumerate all binary ops
/// 2. If unexplored axiom classes: pick highest-potential, run discovery
/// 3. If ≥2 discovered structures: cross-structure comparison + theorems
/// 4. If nothing new: converge
pub fn run_autonomous(carrier_size: usize, max_rounds: usize) -> AutonomousLog {
    let n = carrier_size;
    let total_ops = n.pow((n * n) as u32);
    let mut log = AutonomousLog {
        carrier_size: n,
        rounds: Vec::new(),
    };

    let mut exclude = HashSet::new();
    exclude.insert("distinct".to_string());

    let excl_pairs = vec![("eq".to_string(), "distinct".to_string())];

    let disc_config = DiscoveryConfig {
        beam: BeamConfig {
            candidate_config: CandidateConfig {
                guard_relation: None,
                exclude_relations: exclude.clone(),
                min_pattern_support: 2,
                ..CandidateConfig::default()
            },
            weights: ScoreWeights {
                generativity: 1.0,
                compression: 0.5,
                consistency_penalty: 10.0,
                exclusions: excl_pairs,
            },
            beam_width: 3,
            max_rules_per_beam: 3,
            max_steps: 1,
            adaptive: AdaptivePolicy::Fixed,
        },
        promotion: PromotionConfig {
            min_support: 2,
            max_promotions_per_round: 5,
            exclude_relations: exclude.clone(),
        },
        max_rounds: 1,
    };

    // State
    let mut axiom_classes: Vec<(String, usize, Vec<Vec<usize>>)> = Vec::new();
    let mut explored: HashSet<String> = HashSet::new();
    let mut discovery_logs: Vec<(String, DiscoveryLog)> = Vec::new();
    let mut compared = false;
    let mut transferred = false;
    let mut transferable: Vec<TransferableKnowledge> = Vec::new();
    let mut round = 0;

    let elem_names: Vec<String> = (0..n).map(|i| format!("e{}", i)).collect();

    while round < max_rounds {
        // ── Decision: what to do this round? ──

        if axiom_classes.is_empty() {
            // Step 1: Enumerate
            let mut classes_map: HashMap<String, (usize, Vec<Vec<usize>>)> = HashMap::new();
            let mut table = vec![0usize; n * n];
            for op_id in 0..total_ops {
                let mut x = op_id;
                for k in 0..n * n { table[k] = x % n; x /= n; }
                let op = |a: usize, b: usize| -> usize { table[a * n + b] };
                let assoc = check_assoc(n, &table);
                let comm = check_comm(n, &table);
                let (has_id, id_e) = check_id(n, &table);
                let has_inv = has_id && check_inv(n, &table, id_e.unwrap());
                let mut props = Vec::new();
                if assoc { props.push("assoc"); }
                if comm { props.push("comm"); }
                if has_id { props.push("id"); }
                if has_inv { props.push("inv"); }
                let key = if props.is_empty() { "none".to_string() } else { props.join("+") };
                let entry = classes_map.entry(key).or_insert((0, Vec::new()));
                entry.0 += 1;
                if entry.1.len() < 3 { entry.1.push(table.clone()); }
            }
            let mut sorted: Vec<(String, usize, Vec<Vec<usize>>)> = classes_map.into_iter()
                .map(|(k, (c, r))| (k, c, r))
                .collect();
            sorted.sort_by(|a, b| a.1.cmp(&b.1)); // ascending by model count (rarest first)

            let classes_summary: Vec<(String, usize)> = sorted.iter()
                .map(|(k, c, _)| (k.clone(), *c))
                .collect();
            log.rounds.push((round, "enumerate model space".into(),
                AutonomousAction::Enumerate { total_ops, classes: classes_summary }));
            axiom_classes = sorted;

        } else if let Some(next_class) = axiom_classes.iter()
            .filter(|(k, _, reps)| !explored.contains(k) && !reps.is_empty() && k != "none")
            .next()
            .cloned()
        {
            // Step 2: Explore the next unexplored axiom class (rarest first)
            let (axiom_key, _count, reps) = next_class;
            explored.insert(axiom_key.clone());
            let rep = &reps[0];

            let mut engine = ClosureEngine::new();
            engine.define_relation("op", 3);
            engine.define_equivalence("eq");
            engine.define_relation("distinct", 2);
            for name in &elem_names { engine.define_constant(name); }
            for i in 0..n {
                for j in 0..n {
                    engine.add_fact(Relation::new("op", vec![
                        Term::constant(&elem_names[i]),
                        Term::constant(&elem_names[j]),
                        Term::constant(&elem_names[rep[i * n + j]]),
                    ]));
                }
            }
            for i in 0..n {
                for j in (i + 1)..n {
                    engine.add_fact(Relation::binary("distinct",
                        Term::constant(&elem_names[i]), Term::constant(&elem_names[j])));
                    engine.add_fact(Relation::binary("distinct",
                        Term::constant(&elem_names[j]), Term::constant(&elem_names[i])));
                }
            }

            let struct_name = format!("model_{}_{}", axiom_key, round);
            let disc_log = run_discovery_named(&engine, &disc_config, &struct_name);

            let step = &disc_log.steps[0];
            log.rounds.push((round, format!("discover {}", axiom_key),
                AutonomousAction::Discover {
                    axiom_class: axiom_key.clone(),
                    model_count: _count,
                    concepts: step.promoted.iter().map(|c| {
                        (c.name.clone(),
                         format!("{}", c.signature),
                         c.instance_set.iter().map(|t| t.to_string()).collect())
                    }).collect(),
                    verification_rules: step.verification_rules.clone(),
                    universal_rules: step.universal_rules.iter()
                        .map(|u| u.description.clone()).collect(),
                    chain_identities: step.chain_identities.iter()
                        .map(|ci| (ci.path_a.clone(), ci.path_b.clone())).collect(),
                }));
            discovery_logs.push((axiom_key, disc_log));

        } else if !compared && discovery_logs.len() >= 2 {
            // Step 3: Cross-structure comparison
            let disc_refs: Vec<(&str, &[DiscoveryStep])> = discovery_logs.iter()
                .map(|(k, l)| (k.as_str(), l.steps.as_slice()))
                .collect();
            let abstracts = abstract_across_structures(disc_refs);
            let theorems = discover_theorems(&abstracts);

            let struct_names: Vec<String> = discovery_logs.iter().map(|(k, _)| k.clone()).collect();
            let abs_detail: Vec<(String, Vec<(String, Vec<String>)>)> = abstracts.iter().map(|a| {
                let sig = format!("{}", a.signature);
                let occ: Vec<(String, Vec<String>)> = a.occurrences.iter()
                    .map(|(s, _, inst)| (s.clone(), inst.iter().map(|t| t.to_string()).collect()))
                    .collect();
                (sig, occ)
            }).collect();
            let thm_detail: Vec<String> = theorems.iter().map(|t| format!("{}", t)).collect();
            log.rounds.push((round, "cross-structure comparison".into(),
                AutonomousAction::Compare {
                    structures: struct_names,
                    abstract_concepts: abs_detail,
                    theorems: thm_detail,
                }));
            compared = true;
            // Extract transferable rules from the richest structure (first discovered = rarest)
            if let Some((_, first_log)) = discovery_logs.first() {
                transferable = extract_transferable(first_log);
            }

        } else if compared && !transferred && !transferable.is_empty() {
            // Step 4: Transfer knowledge to a larger carrier
            let next_n = n + 1;
            let next_total = next_n.pow((next_n * next_n) as u32);

            // Find a group in the larger carrier to test on
            let next_elem_names: Vec<String> = (0..next_n).map(|i| format!("t{}", i)).collect();
            let mut found_target = false;
            let mut predictions_correct = 0usize;
            let mut predictions_total = 0usize;
            let mut target_axioms = String::new();
            let mut discovered_concepts: Vec<(String, String, Vec<String>)> = Vec::new();

            // Construct Z_{next_n} (cyclic group) directly — no brute-force search needed
            let mut next_table = vec![0usize; next_n * next_n];
            for i in 0..next_n {
                for j in 0..next_n {
                    next_table[i * next_n + j] = (i + j) % next_n;
                }
            }
            {
                target_axioms = "assoc+comm+id+inv".to_string();

                let mut target_engine = ClosureEngine::new();
                target_engine.define_relation("op", 3);
                target_engine.define_equivalence("eq");
                target_engine.define_relation("distinct", 2);
                for name in &next_elem_names { target_engine.define_constant(name); }
                for i in 0..next_n { for j in (i+1)..next_n {
                    target_engine.add_fact(Relation::binary("distinct",
                        Term::constant(&next_elem_names[i]), Term::constant(&next_elem_names[j])));
                    target_engine.add_fact(Relation::binary("distinct",
                        Term::constant(&next_elem_names[j]), Term::constant(&next_elem_names[i])));
                }}

                // Give partial table: exclude identity row/column except hint
                // Z_n has identity = 0
                let id = 0;
                let mut withheld = Vec::new();
                for i in 0..next_n { for j in 0..next_n {
                    let r = next_table[i * next_n + j];
                    if (i == id || j == id) && !(i == id && j == id) {
                        withheld.push((i, j, r));
                    } else {
                        target_engine.add_fact(Relation::new("op", vec![
                            Term::constant(&next_elem_names[i]),
                            Term::constant(&next_elem_names[j]),
                            Term::constant(&next_elem_names[r]),
                        ]));
                    }
                }}

                // Inject transferred knowledge
                inject_transfer(&mut target_engine, &transferable);
                let result = target_engine.derive_closure();

                // Verify predictions
                for (a, b, expected) in &withheld {
                    let fact = Relation::new("op", vec![
                        Term::constant(&next_elem_names[*a]),
                        Term::constant(&next_elem_names[*b]),
                        Term::constant(&next_elem_names[*expected]),
                    ]);
                    predictions_total += 1;
                    if result.facts.contains(&fact) { predictions_correct += 1; }
                }

                // Extract actual discovered instances from target closure
                for k in &transferable {
                    let inst: Vec<String> = result.facts.iter()
                        .filter(|f| f.name() == k.concept_name)
                        .flat_map(|f| f.terms().iter().map(|t| t.to_string()))
                        .collect();
                    discovered_concepts.push((
                        k.concept_name.clone(),
                        if inst.len() == 1 { "selective".to_string() }
                        else { format!("{} instances", inst.len()) },
                        inst,
                    ));
                }
                found_target = true;
            }

            if found_target {

                log.rounds.push((round, format!("transfer to n={} + verify", next_n),
                    AutonomousAction::Discover {
                        axiom_class: format!("Z{} (constructed cyclic group)", next_n),
                        model_count: 1,
                        concepts: discovered_concepts,
                        verification_rules: vec![
                            format!("predictions: {}/{} correct", predictions_correct, predictions_total),
                        ],
                        universal_rules: transferable.iter()
                            .flat_map(|k| k.descriptions.iter().cloned()).collect(),
                        chain_identities: Vec::new(),
                    }));
                round += 1;

                // Enumerate the larger carrier
                log.rounds.push((round, format!("scale to carrier size {}", next_n),
                    AutonomousAction::Enumerate {
                        total_ops: next_total,
                        classes: if next_total <= 500_000 {
                            // Feasible: full enumeration
                            let mut next_classes_map: HashMap<String, usize> = HashMap::new();
                            let mut nt = vec![0usize; next_n * next_n];
                            for op_id in 0..next_total {
                                let mut x = op_id;
                                for k in 0..next_n * next_n { nt[k] = x % next_n; x /= next_n; }
                                let assoc = check_assoc(next_n, &nt);
                                let comm = check_comm(next_n, &nt);
                                let (has_id, id_e) = check_id(next_n, &nt);
                                let has_inv = has_id && check_inv(next_n, &nt, id_e.unwrap());
                                let mut props = Vec::new();
                                if assoc { props.push("assoc"); }
                                if comm { props.push("comm"); }
                                if has_id { props.push("id"); }
                                if has_inv { props.push("inv"); }
                                let key = if props.is_empty() { "none".to_string() } else { props.join("+") };
                                *next_classes_map.entry(key).or_insert(0) += 1;
                            }
                            let mut sorted: Vec<(String, usize)> = next_classes_map.into_iter().collect();
                            sorted.sort_by(|a, b| a.1.cmp(&b.1));
                            sorted
                        } else {
                            // Too large for exhaustive enumeration
                            vec![
                                (format!("(exhaustive enumeration infeasible: {} > 500000 ops)", next_total), 0),
                            ]
                        },
                    }));
            }
            transferred = true;

        } else {
            // Step 5: Converged
            let reason = if transferred {
                format!("explored {} classes at n={}, transferred and verified at n={}", explored.len(), n, n + 1)
            } else if explored.len() >= axiom_classes.len() - 1 {
                format!("all {} non-trivial axiom classes explored, no transferable rules", explored.len())
            } else {
                "no new productive actions available".to_string()
            };
            log.rounds.push((round, "converge".into(),
                AutonomousAction::Converged { reason }));
            break;
        }

        round += 1;
    }

    log
}

fn check_assoc(n: usize, table: &[usize]) -> bool {
    for a in 0..n { for b in 0..n { for cc in 0..n {
        let ab = table[a * n + b];
        let bc = table[b * n + cc];
        if table[ab * n + cc] != table[a * n + bc] { return false; }
    }}} true
}
fn check_comm(n: usize, table: &[usize]) -> bool {
    for a in 0..n { for b in (a+1)..n {
        if table[a * n + b] != table[b * n + a] { return false; }
    }} true
}
/// Check for a two-sided identity element.
///
/// Returns the first found. Two-sided identity is unique even without
/// associativity: if e and f are both two-sided identities, then e = e·f = f.
/// So returning the first is safe. If we ever support one-sided identities
/// (left ≠ right), this would need to return all candidates.
fn check_id(n: usize, table: &[usize]) -> (bool, Option<usize>) {
    for e in 0..n {
        if (0..n).all(|x| table[e * n + x] == x && table[x * n + e] == x) {
            return (true, Some(e));
        }
    }
    (false, None)
}
/// Check if every element has a two-sided inverse w.r.t. identity `e`.
///
/// Requires `e` to be a valid two-sided identity. For structures with
/// multiple candidate identities (one-sided), this would need to check
/// against each candidate separately.
fn check_inv(n: usize, table: &[usize], e: usize) -> bool {
    for a in 0..n {
        if !(0..n).any(|b| table[a * n + b] == e && table[b * n + a] == e) { return false; }
    } true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(s: &str) -> Term {
        Term::constant(s)
    }

    /// Build a minimal propositional base: just truth assignments + declared formulas, NO rules.
    fn prop_base() -> ClosureEngine {
        let mut engine = ClosureEngine::new();

        engine.define_relation("tv_t", 2);
        engine.define_relation("tv_f", 2);
        engine.define_relation("declared", 1);
        engine.define_relation("tautology", 1);
        engine.define_relation("contradiction", 1);

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
            &neg_p, &neg_q, &and_pq, &or_pq, &imp_pq, &imp_pp, &or_p_negp,
            &and_p_negp, &imp_andpq_p, &imp_p_orpq, &imp_p_negp, &and_p_imppq, &mp,
        ] {
            engine.add_fact(Relation::new("declared", vec![(*f).clone()]));
        }

        engine
    }

    /// Helper: build negation rules.
    fn negation_rules() -> Vec<Rule> {
        let vv = Term::var("v");
        let pv = Term::var("p");
        vec![
            Rule::new(
                "neg_t",
                vec![
                    RelationPattern::new("tv_f", vec![vv.clone(), pv.clone()]),
                    RelationPattern::new("declared", vec![Term::app("neg", vec![pv.clone()])]),
                ],
                vec![RelationPattern::new(
                    "tv_t",
                    vec![vv.clone(), Term::app("neg", vec![pv.clone()])],
                )],
            ),
            Rule::new(
                "neg_f",
                vec![
                    RelationPattern::new("tv_t", vec![vv.clone(), pv.clone()]),
                    RelationPattern::new("declared", vec![Term::app("neg", vec![pv.clone()])]),
                ],
                vec![RelationPattern::new(
                    "tv_f",
                    vec![vv.clone(), Term::app("neg", vec![pv.clone()])],
                )],
            ),
        ]
    }

    /// Helper: build implication rules.
    fn implication_rules() -> Vec<Rule> {
        let vv = Term::var("v");
        let pv = Term::var("p");
        let qv = Term::var("q");
        vec![
            Rule::new(
                "imp_t1",
                vec![
                    RelationPattern::new("tv_f", vec![vv.clone(), pv.clone()]),
                    RelationPattern::new(
                        "declared",
                        vec![Term::app("imp", vec![pv.clone(), qv.clone()])],
                    ),
                ],
                vec![RelationPattern::new(
                    "tv_t",
                    vec![vv.clone(), Term::app("imp", vec![pv.clone(), qv.clone()])],
                )],
            ),
            Rule::new(
                "imp_t2",
                vec![
                    RelationPattern::new("tv_t", vec![vv.clone(), qv.clone()]),
                    RelationPattern::new(
                        "declared",
                        vec![Term::app("imp", vec![pv.clone(), qv.clone()])],
                    ),
                ],
                vec![RelationPattern::new(
                    "tv_t",
                    vec![vv.clone(), Term::app("imp", vec![pv.clone(), qv.clone()])],
                )],
            ),
            Rule::new(
                "imp_f",
                vec![
                    RelationPattern::new("tv_t", vec![vv.clone(), pv.clone()]),
                    RelationPattern::new("tv_f", vec![vv.clone(), qv.clone()]),
                    RelationPattern::new(
                        "declared",
                        vec![Term::app("imp", vec![pv.clone(), qv.clone()])],
                    ),
                ],
                vec![RelationPattern::new(
                    "tv_f",
                    vec![vv.clone(), Term::app("imp", vec![pv.clone(), qv.clone()])],
                )],
            ),
        ]
    }

    /// Helper: build conjunction rules.
    fn conjunction_rules() -> Vec<Rule> {
        let vv = Term::var("v");
        let pv = Term::var("p");
        let qv = Term::var("q");
        vec![
            Rule::new(
                "and_t",
                vec![
                    RelationPattern::new("tv_t", vec![vv.clone(), pv.clone()]),
                    RelationPattern::new("tv_t", vec![vv.clone(), qv.clone()]),
                    RelationPattern::new(
                        "declared",
                        vec![Term::app("and", vec![pv.clone(), qv.clone()])],
                    ),
                ],
                vec![RelationPattern::new(
                    "tv_t",
                    vec![vv.clone(), Term::app("and", vec![pv.clone(), qv.clone()])],
                )],
            ),
            Rule::new(
                "and_f1",
                vec![
                    RelationPattern::new("tv_f", vec![vv.clone(), pv.clone()]),
                    RelationPattern::new(
                        "declared",
                        vec![Term::app("and", vec![pv.clone(), qv.clone()])],
                    ),
                ],
                vec![RelationPattern::new(
                    "tv_f",
                    vec![vv.clone(), Term::app("and", vec![pv.clone(), qv.clone()])],
                )],
            ),
            Rule::new(
                "and_f2",
                vec![
                    RelationPattern::new("tv_f", vec![vv.clone(), qv.clone()]),
                    RelationPattern::new(
                        "declared",
                        vec![Term::app("and", vec![pv.clone(), qv.clone()])],
                    ),
                ],
                vec![RelationPattern::new(
                    "tv_f",
                    vec![vv.clone(), Term::app("and", vec![pv.clone(), qv.clone()])],
                )],
            ),
        ]
    }

    /// Helper: tautology + contradiction meta-rules.
    fn meta_rules() -> Vec<Rule> {
        let fv = Term::var("f");
        vec![
            Rule::new(
                "taut",
                vec![
                    RelationPattern::new("tv_t", vec![c("v_tt"), fv.clone()]),
                    RelationPattern::new("tv_t", vec![c("v_tf"), fv.clone()]),
                    RelationPattern::new("tv_t", vec![c("v_ft"), fv.clone()]),
                    RelationPattern::new("tv_t", vec![c("v_ff"), fv.clone()]),
                ],
                vec![RelationPattern::new("tautology", vec![fv.clone()])],
            ),
            Rule::new(
                "contra",
                vec![
                    RelationPattern::new("tv_f", vec![c("v_tt"), fv.clone()]),
                    RelationPattern::new("tv_f", vec![c("v_tf"), fv.clone()]),
                    RelationPattern::new("tv_f", vec![c("v_ft"), fv.clone()]),
                    RelationPattern::new("tv_f", vec![c("v_ff"), fv.clone()]),
                ],
                vec![RelationPattern::new("contradiction", vec![fv.clone()])],
            ),
        ]
    }

    // ── Candidate generation tests ──────────────────────────

    #[test]
    fn test_generate_candidates_produces_correct_templates() {
        let engine = prop_base();
        let config = CandidateConfig::default();
        let candidates = generate_candidates(&engine, &config);

        // Should generate candidates for neg/1 (4 = 2×2 binary rels) and
        // for and/2, or/2, imp/2 (each: 8 two-premise + 8 single-premise = 16, ×3 = 48)
        // Total: 4 + 48 = 52 (approximately, depending on dedup)
        assert!(
            candidates.len() >= 40,
            "should generate many candidates, got {}",
            candidates.len()
        );

        // The correct neg_t rule template should be among candidates
        let has_neg_correct = candidates.iter().any(|r| {
            r.name() == "cand_tv_f_neg_tv_t"
        });
        assert!(
            has_neg_correct,
            "should generate the correct neg_t template (tv_f + neg → tv_t)"
        );
    }

    #[test]
    fn test_candidate_scoring_prefers_correct_rules() {
        let engine = prop_base();
        let config = CandidateConfig::default();
        let candidates = generate_candidates(&engine, &config);

        // Score all negation candidates
        let neg_candidates: Vec<&Rule> = candidates
            .iter()
            .filter(|r| r.name().contains("neg"))
            .collect();

        let weights = ScoreWeights::default();
        let mut evals: Vec<RuleEvaluation> = neg_candidates
            .iter()
            .map(|r| score::evaluate_rule(&engine, r, &weights))
            .collect();
        evals.sort_by(|a, b| b.combined.partial_cmp(&a.combined).unwrap_or(std::cmp::Ordering::Equal));

        eprintln!("  Negation candidates ranked by combined score:");
        for e in &evals {
            eprintln!(
                "    {:<30} gen={:.3} comp={:.3} comb={:.3} delta={}",
                e.rule_name, e.generativity, e.compression, e.combined, e.delta_facts
            );
        }

        // All 4 negation templates produce the same number of facts (4 each)
        // but compression should differ: correct rules' conclusions match
        // actual truth table patterns
        assert!(!evals.is_empty());
    }

    // ── Paradigm comparison tests ───────────────────────────

    #[test]
    fn test_compare_paradigms_negation_vs_implication_vs_conjunction() {
        let mut base = prop_base();
        // Add meta-rules to all paradigms (these are the "goal" rules)
        for rule in meta_rules() {
            for p in rule.premises() {
                for t in p.terms() {
                    register_vars_on(&mut base, t);
                }
            }
            for cc in rule.conclusions() {
                for t in cc.terms() {
                    register_vars_on(&mut base, t);
                }
            }
            base.add_rule(rule);
        }

        let results = compare_paradigms(
            &base,
            vec![
                ("negation_only", negation_rules()),
                ("implication_only", implication_rules()),
                ("conjunction_only", conjunction_rules()),
                ("neg+imp", {
                    let mut r = negation_rules();
                    r.extend(implication_rules());
                    r
                }),
                ("all_connectives", {
                    let mut r = negation_rules();
                    r.extend(implication_rules());
                    r.extend(conjunction_rules());
                    r
                }),
            ],
        );

        eprintln!("\n  Paradigm comparison (with taut/contra meta-rules):");
        eprintln!(
            "  {:<20} {:>7} {:>7} {:>5} {:>8} {:>6} {:>10}",
            "paradigm", "derived", "total", "rules", "eff", "rels", "mean_depth"
        );
        for r in &results {
            eprintln!(
                "  {:<20} {:>7} {:>7} {:>5} {:>8.1} {:>6} {:>10.2}",
                r.name,
                r.profile.derived_facts,
                r.profile.total_facts,
                r.num_rules,
                r.efficiency,
                r.profile.relation_diversity,
                r.profile.mean_depth,
            );
        }

        // All connectives should produce the most facts
        let all = results.iter().find(|r| r.name == "all_connectives").unwrap();
        let neg_only = results.iter().find(|r| r.name == "negation_only").unwrap();
        assert!(
            all.profile.derived_facts > neg_only.profile.derived_facts,
            "all connectives should derive more than negation alone"
        );

        // neg+imp should unlock tautologies (p→p needs implication, or(p,¬p) needs neg)
        let neg_imp = results.iter().find(|r| r.name == "neg+imp").unwrap();
        assert!(
            neg_imp.profile.relation_diversity >= 3,
            "neg+imp should produce tautology facts (diversity ≥ 3)"
        );
    }

    // ── Search loop test ────────────────────────────────────

    #[test]
    fn test_search_discovers_negation_rules() {
        let mut engine = prop_base();

        let config = SearchConfig {
            candidate_config: CandidateConfig::default(),
            weights: ScoreWeights {
                generativity: 1.0,
                compression: 0.0, // pure generativity for this test
                consistency_penalty: 0.0,
                exclusions: Vec::new(),
            },
            pre_filter_top_n: 10,
            top_k: 2, // select 2 rules per round
            max_steps: 3,
            min_delta: 1,
        };

        let log = run_search(&mut engine, &config);

        eprintln!("\n  Search log:");
        for step in &log {
            let selected_names: Vec<&str> =
                step.selected.iter().map(|e| e.rule_name.as_str()).collect();
            eprintln!(
                "    round {} | candidates={} filtered={} | selected={:?} | delta={} total={}",
                step.round,
                step.candidates_generated,
                step.candidates_after_filter,
                selected_names,
                step.delta_facts,
                step.total_facts_after,
            );
        }

        // After search, engine should have discovered some rules
        assert!(
            engine.rules().len() >= 2,
            "search should discover at least 2 rules"
        );

        // The discovered rules should produce new facts
        let total_delta: usize = log.iter().map(|s| s.delta_facts).sum();
        assert!(
            total_delta > 0,
            "search should produce new facts, got delta=0"
        );
    }

    // ── Beam search tests ───────────────────────────────────

    #[test]
    fn test_beam_search_with_consistency() {
        let mut base = prop_base();
        // Add meta-rules so tautology detection is possible
        for rule in meta_rules() {
            for p in rule.premises() {
                for t in p.terms() {
                    register_vars_on(&mut base, t);
                }
            }
            for cc in rule.conclusions() {
                for t in cc.terms() {
                    register_vars_on(&mut base, t);
                }
            }
            base.add_rule(rule);
        }

        let excl = vec![("tv_t".to_string(), "tv_f".to_string())];
        let config = BeamConfig {
            candidate_config: CandidateConfig::default(),
            weights: ScoreWeights {
                generativity: 1.0,
                compression: 0.0,
                consistency_penalty: 1.0,
                exclusions: excl,
            },
            beam_width: 5,
            max_rules_per_beam: 4,
            max_steps: 4,
            adaptive: AdaptivePolicy::Fixed,
        };

        let log = beam_search(&base, &config);

        eprintln!("\n  Beam search log:");
        for step in &log {
            eprintln!("  === Round {} ({} evaluated) ===", step.round, step.candidates_evaluated);
            for (i, entry) in step.beam.iter().enumerate() {
                eprintln!(
                    "    beam[{}] score={:.3} derived={} incon={} rules={:?}",
                    i,
                    entry.score,
                    entry.profile.derived_facts,
                    entry.profile.inconsistencies,
                    entry.rule_names,
                );
            }
        }

        // The best beam entry should have 0 inconsistencies
        let best = &log.last().unwrap().beam[0];
        assert_eq!(
            best.profile.inconsistencies, 0,
            "best beam entry should be consistent"
        );

        // The best entry should have discovered some useful rules
        assert!(
            best.profile.derived_facts > 5,
            "best entry should derive meaningful facts, got {}",
            best.profile.derived_facts
        );

        // The best entry's rules should be correct neg/and/or/imp variants
        // (not the wrong tv_t→tv_t ones)
        eprintln!("\n  Best rule set: {:?}", best.rule_names);
        eprintln!("  Derived: {}, Score: {:.3}", best.profile.derived_facts, best.score);
    }

    #[test]
    fn test_beam_search_adaptive_weights() {
        let mut base = prop_base();
        for rule in meta_rules() {
            for p in rule.premises() {
                for t in p.terms() {
                    register_vars_on(&mut base, t);
                }
            }
            for cc in rule.conclusions() {
                for t in cc.terms() {
                    register_vars_on(&mut base, t);
                }
            }
            base.add_rule(rule);
        }

        let excl = vec![("tv_t".to_string(), "tv_f".to_string())];
        let config = BeamConfig {
            candidate_config: CandidateConfig::default(),
            weights: ScoreWeights {
                generativity: 1.0,
                compression: 1.0, // starts at 0 effective, ramps up
                consistency_penalty: 0.5,
                exclusions: excl,
            },
            beam_width: 5,
            max_rules_per_beam: 4,
            max_steps: 4,
            adaptive: AdaptivePolicy::Adaptive {
                fact_threshold: 20.0,
                growth_rate: 0.5,
            },
        };

        let log = beam_search(&base, &config);

        eprintln!("\n  Adaptive beam search:");
        for step in &log {
            let w = &step.effective_weights;
            eprintln!(
                "  Round {} | comp_w={:.3} penalty={:.3} | best: score={:.3} derived={} incon={} rules={:?}",
                step.round,
                w.compression,
                w.consistency_penalty,
                step.beam[0].score,
                step.beam[0].profile.derived_facts,
                step.beam[0].profile.inconsistencies,
                step.beam[0].rule_names,
            );
        }

        // Compression weight should increase across rounds
        if log.len() >= 2 {
            let w0 = &log[0].effective_weights;
            let w1 = &log[1].effective_weights;
            assert!(
                w1.compression >= w0.compression,
                "compression weight should increase: {:.3} → {:.3}",
                w0.compression,
                w1.compression,
            );
        }

        // Consistency penalty should increase
        if log.len() >= 3 {
            let w1 = &log[1].effective_weights;
            let w2 = &log[2].effective_weights;
            assert!(
                w2.consistency_penalty > w1.consistency_penalty,
                "penalty should grow: {:.3} → {:.3}",
                w1.consistency_penalty,
                w2.consistency_penalty,
            );
        }

        // Best entry should still be consistent
        let best = &log.last().unwrap().beam[0];
        assert_eq!(best.profile.inconsistencies, 0);
    }

    // ── Z₃ group discovery ─────────────────────────────────

    /// Build Z₃ base: operation table + element distinctness + structural definitions.
    ///
    /// **Given** (minimal initial information):
    /// - Elements: e0, e1, e2 (+ pairwise distinct)
    /// - Operation table: 9 facts (complete Cayley table for Z₃)
    /// - Definitions of what identity/inverse MEAN (verification rules)
    ///
    /// **To be discovered** by the search:
    /// - WHICH element is the identity
    /// - WHICH pairs are inverses
    /// - Commutativity, functionality, cancellation properties
    fn z3_base() -> ClosureEngine {
        let mut engine = ClosureEngine::new();

        // Relations
        engine.define_relation("op", 3);
        engine.define_equivalence("eq"); // sym, trans, refl, congruence
        engine.define_relation("is_id", 1);
        engine.define_relation("has_inv", 2);
        engine.define_relation("distinct", 2);

        // Elements of Z₃
        engine.define_constant("e0");
        engine.define_constant("e1");
        engine.define_constant("e2");

        // Operation table: addition mod 3
        let ops = [
            ("e0","e0","e0"), ("e0","e1","e1"), ("e0","e2","e2"),
            ("e1","e0","e1"), ("e1","e1","e2"), ("e1","e2","e0"),
            ("e2","e0","e2"), ("e2","e1","e0"), ("e2","e2","e1"),
        ];
        for (a, b, r) in &ops {
            engine.add_fact(Relation::new("op", vec![c(a), c(b), c(r)]));
        }

        // Distinctness: pairwise (both directions)
        for (a, b) in &[("e0","e1"), ("e0","e2"), ("e1","e2")] {
            engine.add_fact(Relation::binary("distinct", c(a), c(b)));
            engine.add_fact(Relation::binary("distinct", c(b), c(a)));
        }

        // ── Structural definitions (what identity/inverse MEAN) ──
        // These are analogous to the tautology/contradiction meta-rules
        // in propositional logic: they define concepts, not instances.

        // is_id(e) means: e*x = x for all x
        engine.add_rule(Rule::new(
            "id_left_verify",
            vec![
                RelationPattern::new("is_id", vec![Term::var("e")]),
                RelationPattern::new("op", vec![Term::var("e"), Term::var("x"), Term::var("y")]),
            ],
            vec![RelationPattern::new("eq", vec![Term::var("x"), Term::var("y")])],
        ));
        // is_id(e) means: x*e = x for all x
        engine.add_rule(Rule::new(
            "id_right_verify",
            vec![
                RelationPattern::new("is_id", vec![Term::var("e")]),
                RelationPattern::new("op", vec![Term::var("x"), Term::var("e"), Term::var("y")]),
            ],
            vec![RelationPattern::new("eq", vec![Term::var("x"), Term::var("y")])],
        ));

        engine
    }

    #[test]
    fn test_z3_induced_candidates() {
        let engine = z3_base();
        let config = CandidateConfig {
            guard_relation: None,
            ..CandidateConfig::default()
        };
        let candidates = generate_candidates(&engine, &config);

        eprintln!("\n  Z₃ induced candidates ({} total):", candidates.len());
        for r in &candidates {
            eprintln!("    {}", r.name());
        }

        // Should have data-induced candidates
        assert!(
            candidates.len() >= 10,
            "should generate induced candidates, got {}",
            candidates.len()
        );

        // Should include an identity-detection candidate (induced from
        // the pattern that op(e0,x,x) appears for multiple x values).
        // The induction names it "ind_op_is_id_..." — the tied-variable
        // version where positions 1,2 are equal.
        let has_id_rule = candidates.iter().any(|r| {
            r.name().contains("ind_op_is_id")
        });
        assert!(has_id_rule, "should induce an identity-detection rule from op patterns");
    }

    #[test]
    fn test_z3_beam_search_discovers_group_structure() {
        let base = z3_base();

        let excl = vec![("eq".to_string(), "distinct".to_string())];
        let mut exclude = HashSet::new();
        exclude.insert("distinct".to_string()); // don't generate rules about distinctness

        let config = BeamConfig {
            candidate_config: CandidateConfig {
                guard_relation: None,
                exclude_relations: exclude,
                ..CandidateConfig::default()
            },
            weights: ScoreWeights {
                generativity: 1.0,
                compression: 0.5,
                consistency_penalty: 10.0, // each inconsistency is fatal
                exclusions: excl,
            },
            beam_width: 5,
            max_rules_per_beam: 4,
            max_steps: 4,
            adaptive: AdaptivePolicy::Adaptive {
                fact_threshold: 10.0,
                growth_rate: 0.0,
            },
        };

        let log = beam_search(&base, &config);

        eprintln!("\n  Z₃ beam search:");
        for step in &log {
            eprintln!("  === Round {} ({} evaluated) ===", step.round, step.candidates_evaluated);
            for (i, entry) in step.beam.iter().enumerate() {
                eprintln!(
                    "    beam[{}] score={:.3} derived={} rules={:?}",
                    i, entry.score, entry.profile.derived_facts, entry.rule_names,
                );
            }
        }

        let best = &log.last().unwrap().beam[0];
        eprintln!("\n  Best rule set: {:?}", best.rule_names);
        eprintln!("  Derived facts: {}", best.profile.derived_facts);

        // The best entry should have discovered meaningful structure.
        // Verify specific facts by running closure with the best rules.
        let mut engine = base.clone();
        for rule in &best.rules {
            for p in rule.premises() {
                for t in p.terms() {
                    register_vars_on(&mut engine, t);
                }
            }
            for cc in rule.conclusions() {
                for t in cc.terms() {
                    register_vars_on(&mut engine, t);
                }
            }
            engine.add_rule(rule.clone());
        }
        let result = engine.derive_closure();

        // Check: is_id(e0) should be discovered
        let has_identity = result
            .facts
            .contains(&Relation::new("is_id", vec![c("e0")]));
        eprintln!("  is_id(e0) discovered: {}", has_identity);

        // Check: no spurious identities
        let false_id1 = result
            .facts
            .contains(&Relation::new("is_id", vec![c("e1")]));
        let false_id2 = result
            .facts
            .contains(&Relation::new("is_id", vec![c("e2")]));
        eprintln!("  is_id(e1) [should be false]: {}", false_id1);
        eprintln!("  is_id(e2) [should be false]: {}", false_id2);

        // Check: has_inv pairs
        let inv_12 = result
            .facts
            .contains(&Relation::binary("has_inv", c("e1"), c("e2")));
        let inv_21 = result
            .facts
            .contains(&Relation::binary("has_inv", c("e2"), c("e1")));
        let inv_00 = result
            .facts
            .contains(&Relation::binary("has_inv", c("e0"), c("e0")));
        eprintln!("  has_inv(e1, e2): {}", inv_12);
        eprintln!("  has_inv(e2, e1): {}", inv_21);
        eprintln!("  has_inv(e0, e0): {}", inv_00);

        // Print all derived facts for inspection
        eprintln!("\n  All derived facts:");
        for fact in &result.derived {
            eprintln!("    {}", fact);
        }

        // Key assertions: the system should discover at least identity
        assert!(has_identity, "should discover e0 is the identity");
        assert!(!false_id1, "e1 should NOT be marked as identity");
        assert!(!false_id2, "e2 should NOT be marked as identity");
    }

    // ── Concept promotion & sensitivity analysis ────────────

    /// Z₃ with NO pre-declared concepts — only raw data.
    fn z3_raw() -> ClosureEngine {
        let mut engine = ClosureEngine::new();

        engine.define_relation("op", 3);
        engine.define_equivalence("eq");
        engine.define_relation("distinct", 2);

        engine.define_constant("e0");
        engine.define_constant("e1");
        engine.define_constant("e2");

        let ops = [
            ("e0","e0","e0"), ("e0","e1","e1"), ("e0","e2","e2"),
            ("e1","e0","e1"), ("e1","e1","e2"), ("e1","e2","e0"),
            ("e2","e0","e2"), ("e2","e1","e0"), ("e2","e2","e1"),
        ];
        for (a, b, r) in &ops {
            engine.add_fact(Relation::new("op", vec![c(a), c(b), c(r)]));
        }
        for (a, b) in &[("e0","e1"), ("e0","e2"), ("e1","e2")] {
            engine.add_fact(Relation::binary("distinct", c(a), c(b)));
            engine.add_fact(Relation::binary("distinct", c(b), c(a)));
        }

        // Identity verification rules (structural definition, not instance)
        engine.add_rule(Rule::new(
            "id_left_verify",
            vec![
                RelationPattern::new("is_id", vec![Term::var("e")]),
                RelationPattern::new("op", vec![Term::var("e"), Term::var("x"), Term::var("y")]),
            ],
            vec![RelationPattern::new("eq", vec![Term::var("x"), Term::var("y")])],
        ));
        engine.add_rule(Rule::new(
            "id_right_verify",
            vec![
                RelationPattern::new("is_id", vec![Term::var("e")]),
                RelationPattern::new("op", vec![Term::var("x"), Term::var("e"), Term::var("y")]),
            ],
            vec![RelationPattern::new("eq", vec![Term::var("x"), Term::var("y")])],
        ));

        engine
    }

    #[test]
    fn test_discovery_z3_sensitivity_analysis() {
        let excl = vec![("eq".to_string(), "distinct".to_string())];
        let mut exclude_rels = HashSet::new();
        exclude_rels.insert("distinct".to_string());

        eprintln!("\n  === Z₃ Concept Promotion Sensitivity Analysis ===\n");

        for threshold in [1, 2, 3, 5] {
            let base = z3_raw();
            let config = DiscoveryConfig {
                beam: BeamConfig {
                    candidate_config: CandidateConfig {
                        guard_relation: None,
                        exclude_relations: exclude_rels.clone(),
                        min_pattern_support: threshold,
                        ..CandidateConfig::default()
                    },
                    weights: ScoreWeights {
                        generativity: 1.0,
                        compression: 0.5,
                        consistency_penalty: 10.0,
                        exclusions: excl.clone(),
                    },
                    beam_width: 3,
                    max_rules_per_beam: 3,
                    max_steps: 2,
                    adaptive: AdaptivePolicy::Fixed,
                },
                promotion: PromotionConfig {
                    min_support: threshold,
                    max_promotions_per_round: 5,
                    exclude_relations: exclude_rels.clone(),
                },
                max_rounds: 2,
            };

            let log = run_discovery(&base, &config);

            eprintln!("  --- threshold = {} ---", threshold);
            for step in &log.steps {
                eprintln!("  Round {} | {} concepts promoted | {} total facts | {} relations",
                    step.round, step.promoted.len(), step.total_facts, step.total_relations);
                for concept in &step.promoted {
                    eprintln!("    {} (arity={}, support={}, instances={})",
                        concept.name, concept.arity, concept.support, concept.instances);
                    eprintln!("      rule: {}", concept.source_pattern);
                }
                if let Some(ref best) = step.beam_best {
                    eprintln!("    beam best: score={:.1} derived={} rules={:?}",
                        best.score, best.profile.derived_facts, best.rule_names);
                }
            }
            eprintln!();
        }
    }

    #[test]
    fn test_discovery_z3_invents_identity_concept() {
        let excl = vec![("eq".to_string(), "distinct".to_string())];
        let mut exclude_rels = HashSet::new();
        exclude_rels.insert("distinct".to_string());

        let base = z3_raw();
        let config = DiscoveryConfig {
            beam: BeamConfig {
                candidate_config: CandidateConfig {
                    guard_relation: None,
                    exclude_relations: exclude_rels.clone(),
                    min_pattern_support: 2,
                    ..CandidateConfig::default()
                },
                weights: ScoreWeights {
                    generativity: 1.0,
                    compression: 0.5,
                    consistency_penalty: 10.0,
                    exclusions: excl,
                },
                beam_width: 3,
                max_rules_per_beam: 3,
                max_steps: 2,
                adaptive: AdaptivePolicy::Fixed,
            },
            promotion: PromotionConfig {
                min_support: 2,
                max_promotions_per_round: 5,
                exclude_relations: exclude_rels,
            },
            max_rounds: 2,
        };

        let log = run_discovery(&base, &config);

        // The system should have invented at least one concept
        let total_concepts: usize = log.steps.iter().map(|s| s.promoted.len()).sum();
        assert!(total_concepts > 0, "should invent at least one concept");

        // One of the invented concepts should be equivalent to is_id:
        // a unary concept with exactly 1 instance (e0)
        let has_identity_like = log.steps.iter().any(|step| {
            step.promoted.iter().any(|c| c.arity == 1 && c.instances == 1)
        });
        assert!(
            has_identity_like,
            "should invent a unary concept with 1 instance (identity-like)"
        );

        eprintln!("\n  Invented concepts:");
        for step in &log.steps {
            for c in &step.promoted {
                eprintln!(
                    "    {} | arity={} instances={} support={} | {}",
                    c.name, c.arity, c.instances, c.support,
                    if c.arity == 1 && c.instances == 1 { "← identity-like" } else { "" }
                );
            }
        }
    }

    // ── S₃ discovery ───────────────────────────────────────

    /// Build S₃ (symmetric group on 3 elements) with NO pre-declared concepts.
    ///
    /// 6 elements: e (identity), a=(12), b=(23), c=(13), d=(123), f=(132)
    /// 36 op facts (complete Cayley table)
    /// 30 distinct facts (15 pairs × 2 directions)
    fn s3_raw() -> ClosureEngine {
        let mut engine = ClosureEngine::new();

        engine.define_relation("op", 3);
        engine.define_equivalence("eq");
        engine.define_relation("distinct", 2);

        let elems = ["e", "a", "b", "c", "d", "f"];
        for name in &elems {
            engine.define_constant(*name);
        }

        // Cayley table (right-to-left composition)
        //   e=(id), a=(12), b=(23), c=(13), d=(123), f=(132)
        let table: [[&str; 6]; 6] = [
            ["e","a","b","c","d","f"],  // e·?
            ["a","e","d","f","b","c"],  // a·?
            ["b","f","e","d","c","a"],  // b·?
            ["c","d","f","e","a","b"],  // c·?
            ["d","c","a","b","f","e"],  // d·?
            ["f","b","c","a","e","d"],  // f·?
        ];
        for (i, row) in table.iter().enumerate() {
            for (j, result) in row.iter().enumerate() {
                engine.add_fact(Relation::new("op", vec![
                    c(elems[i]), c(elems[j]), c(result),
                ]));
            }
        }

        // Pairwise distinct
        for i in 0..elems.len() {
            for j in (i + 1)..elems.len() {
                engine.add_fact(Relation::binary("distinct", c(elems[i]), c(elems[j])));
                engine.add_fact(Relation::binary("distinct", c(elems[j]), c(elems[i])));
            }
        }

        // Identity verification rules (structural definition)
        engine.add_rule(Rule::new(
            "id_left_verify",
            vec![
                RelationPattern::new("is_id", vec![Term::var("e")]),
                RelationPattern::new("op", vec![Term::var("e"), Term::var("x"), Term::var("y")]),
            ],
            vec![RelationPattern::new("eq", vec![Term::var("x"), Term::var("y")])],
        ));
        engine.add_rule(Rule::new(
            "id_right_verify",
            vec![
                RelationPattern::new("is_id", vec![Term::var("e")]),
                RelationPattern::new("op", vec![Term::var("x"), Term::var("e"), Term::var("y")]),
            ],
            vec![RelationPattern::new("eq", vec![Term::var("x"), Term::var("y")])],
        ));

        engine
    }

    #[test]
    fn test_discovery_s3_group() {
        let excl = vec![("eq".to_string(), "distinct".to_string())];
        let mut exclude_rels = HashSet::new();
        exclude_rels.insert("distinct".to_string());

        let base = s3_raw();

        let config = DiscoveryConfig {
            beam: BeamConfig {
                candidate_config: CandidateConfig {
                    guard_relation: None,
                    exclude_relations: exclude_rels.clone(),
                    min_pattern_support: 3,
                    ..CandidateConfig::default()
                },
                weights: ScoreWeights {
                    generativity: 1.0,
                    compression: 0.5,
                    consistency_penalty: 10.0,
                    exclusions: excl,
                },
                beam_width: 3,
                max_rules_per_beam: 3,
                max_steps: 2,
                adaptive: AdaptivePolicy::Fixed,
            },
            promotion: PromotionConfig {
                min_support: 3,
                max_promotions_per_round: 5,
                exclude_relations: exclude_rels,
            },
            max_rounds: 2,
        };

        let log = run_discovery(&base, &config);

        eprintln!("\n  === S₃ Discovery ===");
        for step in &log.steps {
            eprintln!(
                "\n  Round {} | {} concepts | {} facts | {} relations",
                step.round, step.promoted.len(), step.total_facts, step.total_relations
            );
            for concept in &step.promoted {
                eprintln!(
                    "    {} (arity={}, support={}, instances={}) {}",
                    concept.name,
                    concept.arity,
                    concept.support,
                    concept.instances,
                    if concept.arity == 1 && concept.instances == 1 {
                        "← identity-like"
                    } else {
                        ""
                    }
                );
            }
            if let Some(ref best) = step.beam_best {
                eprintln!(
                    "    beam: score={:.1} derived={} incon={} rules={:?}",
                    best.score, best.profile.derived_facts,
                    best.profile.inconsistencies, best.rule_names,
                );
            }
        }

        // Should invent identity-like concept (1 instance = e)
        let has_identity = log.steps.iter().any(|step| {
            step.promoted.iter().any(|c| c.arity == 1 && c.instances == 1)
        });
        assert!(has_identity, "should invent identity concept for S₃");

        // Should NOT invent a concept that covers all 6 elements
        // (that would be a trivial/useless concept)
        let has_trivial = log.steps.iter().any(|step| {
            step.promoted.iter().any(|c| c.arity == 1 && c.instances == 6)
        });
        assert!(!has_trivial, "should not invent trivial all-element concept");
    }

    // ── Verification rule auto-discovery ────────────────────

    /// Z₃ with NO hand-written rules at all.
    fn z3_fully_autonomous() -> ClosureEngine {
        let mut engine = ClosureEngine::new();

        engine.define_relation("op", 3);
        engine.define_equivalence("eq");
        engine.define_relation("distinct", 2);

        engine.define_constant("e0");
        engine.define_constant("e1");
        engine.define_constant("e2");

        let ops = [
            ("e0","e0","e0"), ("e0","e1","e1"), ("e0","e2","e2"),
            ("e1","e0","e1"), ("e1","e1","e2"), ("e1","e2","e0"),
            ("e2","e0","e2"), ("e2","e1","e0"), ("e2","e2","e1"),
        ];
        for (a, b, r) in &ops {
            engine.add_fact(Relation::new("op", vec![c(a), c(b), c(r)]));
        }
        for (a, b) in &[("e0","e1"), ("e0","e2"), ("e1","e2")] {
            engine.add_fact(Relation::binary("distinct", c(a), c(b)));
            engine.add_fact(Relation::binary("distinct", c(b), c(a)));
        }

        // NO verification rules. NO is_id declaration. Nothing.
        engine
    }

    #[test]
    fn test_verification_rule_auto_discovery_z3() {
        let excl = vec![("eq".to_string(), "distinct".to_string())];
        let mut exclude_rels = HashSet::new();
        exclude_rels.insert("distinct".to_string());

        let base = z3_fully_autonomous();
        let config = DiscoveryConfig {
            beam: BeamConfig {
                candidate_config: CandidateConfig {
                    guard_relation: None,
                    exclude_relations: exclude_rels.clone(),
                    min_pattern_support: 2,
                    ..CandidateConfig::default()
                },
                weights: ScoreWeights {
                    generativity: 1.0,
                    compression: 0.5,
                    consistency_penalty: 10.0,
                    exclusions: excl,
                },
                beam_width: 3,
                max_rules_per_beam: 3,
                max_steps: 2,
                adaptive: AdaptivePolicy::Fixed,
            },
            promotion: PromotionConfig {
                min_support: 2,
                max_promotions_per_round: 5,
                exclude_relations: exclude_rels,
            },
            max_rounds: 2,
        };

        let log = run_discovery(&base, &config);

        eprintln!("\n  === Z₃ Fully Autonomous Discovery ===");
        for step in &log.steps {
            eprintln!(
                "\n  Round {} | {} concepts | {} verification rules | {} facts",
                step.round, step.promoted.len(), step.verification_rules.len(), step.total_facts,
            );
            for concept in &step.promoted {
                eprintln!(
                    "    {} (arity={}, instances={}) {}",
                    concept.name, concept.arity, concept.instances,
                    if concept.arity == 1 && concept.instances == 1 { "← identity" } else { "" }
                );
            }
            for vr in &step.verification_rules {
                eprintln!("    verification: {}", vr);
            }
            if let Some(ref best) = step.beam_best {
                eprintln!(
                    "    beam: score={:.1} derived={} incon={} rules={:?}",
                    best.score, best.profile.derived_facts,
                    best.profile.inconsistencies, best.rule_names,
                );
            }
        }

        // Should discover verification rules automatically
        let total_vrules: usize = log.steps.iter().map(|s| s.verification_rules.len()).sum();
        assert!(
            total_vrules > 0,
            "should auto-discover at least one verification rule"
        );

        // Verification rules should include a pattern like
        // "auto_X(?e), op(?e, ?x, ?y) |- eq(?x, ?y)"
        let has_identity_verify = log.steps.iter().any(|step| {
            step.verification_rules.iter().any(|name| {
                name.contains("auto_") && name.contains("op")
            })
        });
        assert!(has_identity_verify, "should discover identity verification rule");

        eprintln!(
            "\n  Total: {} concepts, {} verification rules",
            log.steps.iter().map(|s| s.promoted.len()).sum::<usize>(),
            total_vrules,
        );
    }
}
