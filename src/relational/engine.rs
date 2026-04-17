//! Closure engine: derives new relations from facts and rules until saturation.
//!
//! The engine applies three kinds of derivation each round:
//!
//! 1. **User rules** — pattern-matched against the current fact set.
//! 2. **Reflexivity** — `R(t, t)` for every ground term `t` and every relation
//!    marked reflexive.
//! 3. **Congruence** — if `R(a, b)`, then `R(f(…a…), f(…b…))` for every
//!    compound term containing `a` and every relation marked congruent.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use super::relation::Relation;
use super::rule::{self, RelationPattern, Rule, Substitution};
use super::term::Term;

const DEFAULT_MAX_ROUNDS: usize = 100;
const DEFAULT_MAX_FACTS: usize = 10_000;
const MAX_TERM_DEPTH: usize = 8;
/// Maximum number of ground terms used in axiom substitution pool.
const MAX_AXIOM_UNIVERSE: usize = 500;
/// Maximum depth of terms fed into axiom variable substitution.
/// Kept low to prevent combinatorial explosion with congruence closure.
const MAX_AXIOM_SUB_DEPTH: usize = 2;

/// Outcome of a closure computation.
pub struct ClosureResult {
    /// All facts after closure (initial + derived), sorted.
    pub facts: Vec<Relation>,
    /// Only the newly derived facts, sorted.
    pub derived: Vec<Relation>,
    /// Number of rounds executed.
    pub rounds: usize,
    /// `true` if the engine reached a fixed point (no new facts possible).
    pub saturated: bool,
    /// Warnings about recursive axioms, depth capping, etc.
    pub warnings: Vec<String>,
}

/// A universally quantified equation: for all ground substitutions of the
/// variables, emit `equiv_relation(subst(lhs), subst(rhs))`.
#[derive(Debug, Clone)]
pub struct Axiom {
    name: String,
    lhs: Term,
    rhs: Term,
    /// The equivalence relation to emit into (e.g. "equiv").
    equiv_relation: String,
}

impl Axiom {
    pub fn new(
        name: impl Into<String>,
        lhs: Term,
        rhs: Term,
        equiv_relation: impl Into<String>,
    ) -> Self {
        Axiom {
            name: name.into(),
            lhs,
            rhs,
            equiv_relation: equiv_relation.into(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn lhs(&self) -> &Term {
        &self.lhs
    }
    pub fn rhs(&self) -> &Term {
        &self.rhs
    }
    pub fn equiv_relation(&self) -> &str {
        &self.equiv_relation
    }
}

/// Schema for a declared relation.
#[derive(Debug, Clone, PartialEq)]
pub struct RelationDef {
    arity: usize,
}

impl RelationDef {
    pub fn arity(&self) -> usize {
        self.arity
    }
}

/// Pure relational closure engine.
///
/// Operates on arbitrary relations — no predefined mathematical semantics.
/// Equality is not built-in; declare it with [`define_equivalence`](Self::define_equivalence)
/// to get symmetry, transitivity, reflexivity, and congruence.
#[derive(Clone)]
pub struct ClosureEngine {
    // ── declarations ─────────────────────────────────────────
    constants: BTreeSet<String>,
    variables: BTreeSet<String>,
    relation_defs: BTreeMap<String, RelationDef>,
    reflexive_relations: BTreeSet<String>,
    congruent_relations: BTreeSet<String>,

    // ── runtime state ────────────────────────────────────────
    facts: HashSet<Relation>,
    rules: Vec<Rule>,
    axioms: Vec<Axiom>,
    max_rounds: usize,
    max_facts: usize,
    /// Suppress refutation phase (set to true in hypothetical branches
    /// to prevent infinite recursion).
    hypothetical_mode: bool,
}

impl Default for ClosureEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ClosureEngine {
    /// Creates an empty engine with no declarations, rules, or facts.
    pub fn new() -> Self {
        ClosureEngine {
            constants: BTreeSet::new(),
            variables: BTreeSet::new(),
            relation_defs: BTreeMap::new(),
            reflexive_relations: BTreeSet::new(),
            congruent_relations: BTreeSet::new(),
            facts: HashSet::new(),
            rules: Vec::new(),
            axioms: Vec::new(),
            max_rounds: DEFAULT_MAX_ROUNDS,
            max_facts: DEFAULT_MAX_FACTS,
            hypothetical_mode: false,
        }
    }

    /// Creates an engine with `equiv/2` defined as a full equivalence relation
    /// (symmetric, transitive, reflexive, congruent).
    pub fn with_defaults() -> Self {
        let mut engine = Self::new();
        engine.define_equivalence("equiv");
        engine
    }

    // ── entity declarations ──────────────────────────────────

    /// Declares a ground constant. Returns the corresponding [`Term`] for
    /// convenient use in fact/rule construction.
    ///
    /// Declared constants are automatically part of the universe for
    /// reflexivity and congruence, even before any fact mentions them.
    pub fn define_constant(&mut self, name: impl Into<String>) -> Term {
        let name = name.into();
        self.constants.insert(name.clone());
        Term::constant(name)
    }

    /// Declares a pattern variable. Returns the corresponding [`Term`].
    ///
    /// Variables are only meaningful inside rule patterns; they are never
    /// part of the ground universe.
    pub fn define_variable(&mut self, name: impl Into<String>) -> Term {
        let name = name.into();
        self.variables.insert(name.clone());
        Term::var(name)
    }

    /// Declares a relation schema with the given arity.
    pub fn define_relation(&mut self, name: impl Into<String>, arity: usize) {
        self.relation_defs
            .insert(name.into(), RelationDef { arity });
    }

    /// Marks a declared relation as reflexive: the engine will generate
    /// `R(t, t)` for every ground term `t` in the universe.
    ///
    /// Only meaningful for binary relations.
    pub fn mark_reflexive(&mut self, name: impl Into<String>) {
        self.reflexive_relations.insert(name.into());
    }

    /// Marks a declared relation as congruent: if `R(a, b)` holds, the engine
    /// derives `R(f(…a…), f(…b…))` for every compound term containing `a`.
    ///
    /// Only meaningful for binary relations.
    pub fn mark_congruent(&mut self, name: impl Into<String>) {
        self.congruent_relations.insert(name.into());
    }

    /// Convenience: declares a binary relation and equips it with symmetry,
    /// transitivity, reflexivity, and congruence — making it a full
    /// equivalence relation.
    pub fn define_equivalence(&mut self, name: impl Into<String>) {
        let name = name.into();
        self.define_relation(&name, 2);
        // Auto-declare the pattern variables used by the generated rules
        self.variables.insert("x".into());
        self.variables.insert("y".into());
        self.variables.insert("z".into());
        self.add_rule(rule::symmetry_for(&name));
        self.add_rule(rule::transitivity_for(&name));
        self.mark_reflexive(&name);
        self.mark_congruent(&name);
    }

    // ── accessors ────────────────────────────────────────────

    pub fn constants(&self) -> &BTreeSet<String> {
        &self.constants
    }
    pub fn variables(&self) -> &BTreeSet<String> {
        &self.variables
    }
    pub fn relation_defs(&self) -> &BTreeMap<String, RelationDef> {
        &self.relation_defs
    }
    pub fn facts(&self) -> &HashSet<Relation> {
        &self.facts
    }
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }
    pub fn axioms(&self) -> &[Axiom] {
        &self.axioms
    }

    // ── building ─────────────────────────────────────────────

    pub fn add_fact(&mut self, fact: Relation) {
        if fact.is_ground() {
            self.facts.insert(fact);
        } else {
            self.facts.insert(fact.alpha_normalize());
        }
    }

    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
    }

    /// Removes a rule by index. Used for ablation analysis.
    pub fn remove_rule(&mut self, index: usize) -> Rule {
        self.rules.remove(index)
    }

    /// Remove all rules matching a given name.
    pub fn remove_rules_by_name(&mut self, name: &str) {
        self.rules.retain(|r| r.name() != name);
    }

    /// Adds a universally quantified axiom. During closure, the engine
    /// enumerates all ground substitutions for the axiom's variables and
    /// emits `equiv_relation(subst(lhs), subst(rhs))` facts.
    pub fn add_axiom(&mut self, axiom: Axiom) {
        self.axioms.push(axiom);
    }

    // ── property definitions (Henkin second-order) ──────────

    /// Define a named property as a first-class relational object.
    ///
    /// A property is a constant term representing a predicate. The engine
    /// generates bidirectional rules linking the property to its formula:
    ///
    /// ```text
    /// property subset_of_empty(x) := subset(empty, x)
    ///
    /// generates:
    ///   (detect)  subset(empty, x) |- has_property_1(x, subset_of_empty)
    ///   (apply)   has_property_1(x, subset_of_empty) |- subset(empty, x)
    ///   (mark)    is_property(subset_of_empty)
    /// ```
    ///
    /// Properties are first-class: they can be bound to variables, quantified
    /// over (`has_property_1(?x, ?p)` matches any property), and appear in
    /// rules as regular terms.
    pub fn define_property(
        &mut self,
        name: &str,
        params: &[&str],
        formula: Vec<RelationPattern>,
    ) {
        // Property as a constant term
        self.define_constant(name);

        // Mark as property
        if !self.relation_defs.contains_key("is_property") {
            self.define_relation("is_property", 1);
        }
        self.add_fact(Relation::new("is_property", vec![Term::constant(name)]));

        // Declare has_property_N relation
        let arity = params.len() + 1; // params + property ref
        let has_rel = format!("has_property_{}", params.len());
        if !self.relation_defs.contains_key(&has_rel) {
            self.define_relation(&has_rel, arity);
        }

        // Register param variables
        for p in params {
            self.define_variable(*p);
        }

        // Build has_property_N(param1, ..., paramN, prop_name) pattern
        let mut has_terms: Vec<Term> = params.iter().map(|p| Term::var(*p)).collect();
        has_terms.push(Term::constant(name));
        let has_pattern = RelationPattern::new(&has_rel, has_terms);

        // Collect ALL variables in the formula (params + free vars like x, y)
        let mut all_vars: HashSet<String> = HashSet::new();
        for pat in &formula {
            for t in pat.terms() {
                collect_var_names(t, &mut all_vars);
            }
        }
        let all_var_names: Vec<String> = all_vars.into_iter().collect();

        // Forward: formula |- has_property_N(args, prop)
        // Ground-required on ALL variables (params + formula vars) to ensure
        // only ground instances produce has_property facts.
        self.add_rule(Rule::new(
            format!("{}_detect", name),
            formula.clone(),
            vec![has_pattern.clone()],
        ).with_ground_required(all_var_names.clone()));

        // Backward rule (apply) temporarily disabled — detect-only mode.
        // Apply rules cause cascading pattern facts in algebraic contexts.
        // TODO: re-enable with proper grounding guards.
        // self.add_rule(Rule::new(
        //     format!("{}_apply", name),
        //     vec![has_pattern],
        //     formula,
        // ).with_ground_required(param_names));
    }

    // ── property composition (phase 3) ────────────────────

    /// Define a composite property as the conjunction of two existing properties.
    ///
    /// ```text
    /// define_compose_and("two_sided_id", "left_id", "right_id")
    ///
    /// generates:
    ///   is_property(two_sided_id)
    ///   has_property_1(x, left_id), has_property_1(x, right_id)
    ///       |- has_property_1(x, two_sided_id)                    (detect)
    ///   has_property_1(x, two_sided_id) |- has_property_1(x, left_id)   (split_l)
    ///   has_property_1(x, two_sided_id) |- has_property_1(x, right_id)  (split_r)
    /// ```
    ///
    /// The composite property's extension equals the intersection of the
    /// component extensions: `ext(name) = ext(p) ∩ ext(q)`.
    pub fn define_compose_and(&mut self, name: &str, p: &str, q: &str) {
        // Register as a property constant
        self.define_constant(name);
        if !self.relation_defs.contains_key("is_property") {
            self.define_relation("is_property", 1);
        }
        self.add_fact(Relation::new("is_property", vec![Term::constant(name)]));

        if !self.relation_defs.contains_key("has_property_1") {
            self.define_relation("has_property_1", 2);
        }

        // Ensure a variable for the element parameter
        self.define_variable("_cx");

        let x = Term::var("_cx");
        let prop_p = Term::constant(p);
        let prop_q = Term::constant(q);
        let prop_name = Term::constant(name);

        // Detect: has_property_1(x, p) ∧ has_property_1(x, q) |- has_property_1(x, name)
        self.add_rule(Rule::new(
            format!("{}_compose_detect", name),
            vec![
                RelationPattern::new("has_property_1", vec![x.clone(), prop_p.clone()]),
                RelationPattern::new("has_property_1", vec![x.clone(), prop_q.clone()]),
            ],
            vec![
                RelationPattern::new("has_property_1", vec![x.clone(), prop_name.clone()]),
            ],
        ).with_ground_required(vec!["_cx".to_string()]));

        // Split left: has_property_1(x, name) |- has_property_1(x, p)
        self.add_rule(Rule::new(
            format!("{}_split_left", name),
            vec![
                RelationPattern::new("has_property_1", vec![x.clone(), prop_name.clone()]),
            ],
            vec![
                RelationPattern::new("has_property_1", vec![x.clone(), prop_p]),
            ],
        ).with_ground_required(vec!["_cx".to_string()]));

        // Split right: has_property_1(x, name) |- has_property_1(x, q)
        self.add_rule(Rule::new(
            format!("{}_split_right", name),
            vec![
                RelationPattern::new("has_property_1", vec![x.clone(), prop_name]),
            ],
            vec![
                RelationPattern::new("has_property_1", vec![x, prop_q]),
            ],
        ).with_ground_required(vec!["_cx".to_string()]));
    }

    // ── property similarity ────────────────────────────────

    /// Get the extension of a unary property: all ground elements x
    /// where has_property_1(x, prop_name) holds.
    pub fn property_extension(&self, prop_name: &str) -> HashSet<Term> {
        self.facts
            .iter()
            .filter(|f| {
                f.name() == "has_property_1"
                    && f.arity() == 2
                    && f.terms()[1] == Term::constant(prop_name)
                    && f.is_ground()
            })
            .map(|f| f.terms()[0].clone())
            .collect()
    }

    /// Jaccard similarity between two property extensions.
    /// Returns None if both extensions are empty (no data).
    pub fn extension_similarity(
        ext1: &HashSet<Term>,
        ext2: &HashSet<Term>,
    ) -> Option<f64> {
        let union_size = ext1.union(ext2).count();
        if union_size == 0 {
            None
        } else {
            let inter_size = ext1.intersection(ext2).count();
            Some(inter_size as f64 / union_size as f64)
        }
    }

    /// Compute Jaccard similarity between two named properties.
    pub fn property_similarity(&self, p: &str, q: &str) -> Option<f64> {
        let ext_p = self.property_extension(p);
        let ext_q = self.property_extension(q);
        Self::extension_similarity(&ext_p, &ext_q)
    }

    // ── combination scoring ────────────────────────────────

    /// Score a property conjunction: how informative is `p ∧ q` compared
    /// to `p` and `q` individually?
    ///
    /// Returns `(score, diagnosis)` where score ≥ 0 and diagnosis explains
    /// the result. Score = 0 with a diagnosis means the combination is
    /// degenerate (not worth keeping).
    pub fn score_combo_and(&self, p: &str, q: &str) -> (f64, String) {
        let ext_p = self.property_extension(p);
        let ext_q = self.property_extension(q);
        let ext_combo: HashSet<Term> = ext_p.intersection(&ext_q).cloned().collect();
        let domain_size = self
            .facts
            .iter()
            .filter(|f| f.name() == "element" && f.arity() == 1 && f.is_ground())
            .count();

        // Hard filters
        if ext_combo.is_empty() {
            return (0.0, "empty_extension".into());
        }
        if ext_combo == ext_p {
            return (0.0, "degenerate_to_p".into());
        }
        if ext_combo == ext_q {
            return (0.0, "degenerate_to_q".into());
        }
        if domain_size > 0 && ext_combo.len() == domain_size {
            return (0.0, "trivial_universal".into());
        }

        // Check if combo ext equals ANY existing property's ext (cross-degenerate)
        let properties: Vec<Term> = self
            .facts
            .iter()
            .filter(|f| f.name() == "is_property" && f.arity() == 1 && f.is_ground())
            .map(|f| f.terms()[0].clone())
            .collect();
        for prop in &properties {
            let prop_name = prop.to_string();
            if prop_name == p || prop_name == q {
                continue; // already checked above
            }
            let ext_existing = self.property_extension(&prop_name);
            if !ext_existing.is_empty() && ext_combo == ext_existing {
                return (0.0, format!("degenerate_to_{}", prop_name));
            }
        }

        // Soft scoring: independence × rarity × constraint
        let jaccard = |a: &HashSet<Term>, b: &HashSet<Term>| -> f64 {
            let u = a.union(b).count();
            if u == 0 {
                return 0.0;
            }
            a.intersection(b).count() as f64 / u as f64
        };

        let indep_p = 1.0 - jaccard(&ext_combo, &ext_p);
        let indep_q = 1.0 - jaccard(&ext_combo, &ext_q);
        let independence = indep_p.min(indep_q);

        let rarity = if domain_size == 0 {
            0.0
        } else {
            let ratio = ext_combo.len() as f64 / domain_size as f64;
            4.0 * ratio * (1.0 - ratio) // peaks at 0.5
        };

        let constraint = 1.0 - jaccard(&ext_p, &ext_q);

        let score = independence * rarity * constraint;
        (score, "positive".into())
    }

    /// Score and record a combination into the fact set.
    /// Records `combo_score(p, q, value)` and `combo_diagnosis(p, q, reason)`.
    pub fn record_combo_score(&mut self, name: &str, p: &str, q: &str) {
        let (score, diagnosis) = self.score_combo_and(p, q);

        if !self.relation_defs.contains_key("combo_score") {
            self.define_relation("combo_score", 3);
        }
        if !self.relation_defs.contains_key("combo_diagnosis") {
            self.define_relation("combo_diagnosis", 3);
        }

        let score_str = format!("{:.4}", score);
        self.define_constant(&score_str);
        self.define_constant(&diagnosis);

        self.add_fact(Relation::new(
            "combo_score",
            vec![
                Term::constant(name),
                Term::constant(p),
                Term::constant(&score_str),
            ],
        ));
        self.add_fact(Relation::new(
            "combo_diagnosis",
            vec![
                Term::constant(name),
                Term::constant(p),
                Term::constant(&diagnosis),
            ],
        ));
    }

    // ── property negation ──────────────────────────────────

    /// Define the negation of a property: ext(not_p) = domain \ ext(p).
    ///
    /// ```text
    /// define_negate("not_left_id", "left_id")
    ///
    /// generates:
    ///   is_property(not_left_id)
    ///   element(x), NOT has_property_1(x, left_id) |- has_property_1(x, not_left_id)
    /// ```
    ///
    /// The negated property's extension is the complement of the original
    /// within the element domain.
    pub fn define_negate(&mut self, name: &str, p: &str) {
        self.define_constant(name);
        if !self.relation_defs.contains_key("is_property") {
            self.define_relation("is_property", 1);
        }
        self.add_fact(Relation::new("is_property", vec![Term::constant(name)]));

        if !self.relation_defs.contains_key("has_property_1") {
            self.define_relation("has_property_1", 2);
        }

        self.define_variable("_nx");

        // element(x), NOT has_property_1(x, p) |- has_property_1(x, name)
        // Stratum 3: must run AFTER stratum 2 (where has_property_1 facts
        // are derived via double-negation detection for universal properties).
        self.add_rule(Rule::new(
            format!("{}_negate_detect", name),
            vec![RelationPattern::new("element", vec![Term::var("_nx")])],
            vec![RelationPattern::new(
                "has_property_1",
                vec![Term::var("_nx"), Term::constant(name)],
            )],
        ).with_negated(vec![
            RelationPattern::new(
                "has_property_1",
                vec![Term::var("_nx"), Term::constant(p)],
            ),
        ]).with_stratum(3));
    }

    // ── auto combination search ─────────────────────────────

    /// Enumerate all pairs of existing properties, score each conjunction,
    /// and return candidates sorted by score (descending). Only non-zero
    /// scores are included.
    pub fn enumerate_combo_candidates(&self) -> Vec<(String, String, f64, String)> {
        let properties: Vec<String> = self
            .facts
            .iter()
            .filter(|f| f.name() == "is_property" && f.arity() == 1 && f.is_ground())
            .map(|f| f.terms()[0].to_string())
            .collect();

        let mut candidates = Vec::new();
        for i in 0..properties.len() {
            for j in (i + 1)..properties.len() {
                let p = &properties[i];
                let q = &properties[j];
                let (score, diag) = self.score_combo_and(p, q);
                if score > 0.0 {
                    candidates.push((p.clone(), q.clone(), score, diag));
                }
            }
        }
        candidates.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        candidates
    }

    /// Automatically construct the top-K scoring property conjunctions.
    /// Returns the names of constructed combinations.
    pub fn auto_construct_top_k(&mut self, k: usize) -> Vec<String> {
        let candidates = self.enumerate_combo_candidates();
        let mut constructed = Vec::new();

        if !self.relation_defs.contains_key("auto_constructed") {
            self.define_relation("auto_constructed", 3);
        }

        for (rank, (p, q, score, _diag)) in candidates.iter().take(k).enumerate() {
            let name = format!("combo_{}_{}", p, q);
            self.define_compose_and(&name, p, q);
            self.record_combo_score(&name, p, q);

            // Record rank
            let rank_str = format!("{}", rank + 1);
            self.define_constant(&rank_str);
            self.define_constant(&name);
            self.add_fact(Relation::new(
                "auto_constructed",
                vec![
                    Term::constant(&name),
                    Term::constant(&format!("{:.4}", score)),
                    Term::constant(&rank_str),
                ],
            ));

            constructed.push(name);
        }

        constructed
    }

    // ── property implication (Henkin second-order, phase 2) ──

    /// Declare that property implication should be tracked.
    /// After closure, the engine scans all unary properties: if every element
    /// satisfying property P also satisfies property Q, derives `implies(P, Q)`.
    ///
    /// Also installs:
    /// - Modus ponens: `implies(p, q), has_property_1(x, p) |- has_property_1(x, q)`
    /// - Transitivity: `implies(p, q), implies(q, r) |- implies(p, r)`
    /// - Equivalence: `implies(p, q), implies(q, p) |- equivalent(p, q)`
    pub fn enable_property_implication(&mut self) {
        self.define_relation("implies", 2);
        self.define_relation("equivalent", 2);

        // Ensure has_property_1 and is_property exist
        if !self.relation_defs.contains_key("has_property_1") {
            self.define_relation("has_property_1", 2);
        }
        if !self.relation_defs.contains_key("is_property") {
            self.define_relation("is_property", 1);
        }

        // Modus ponens: implies(p, q), has_property_1(x, p) |- has_property_1(x, q)
        self.define_variable("_p");
        self.define_variable("_q");
        self.define_variable("_r");
        self.define_variable("_x");
        self.add_rule(Rule::new(
            "property_modus_ponens",
            vec![
                RelationPattern::new("implies", vec![Term::var("_p"), Term::var("_q")]),
                RelationPattern::new("has_property_1", vec![Term::var("_x"), Term::var("_p")]),
            ],
            vec![
                RelationPattern::new("has_property_1", vec![Term::var("_x"), Term::var("_q")]),
            ],
        ));

        // Transitivity: implies(p, q), implies(q, r) |- implies(p, r)
        self.add_rule(Rule::new(
            "implies_transitivity",
            vec![
                RelationPattern::new("implies", vec![Term::var("_p"), Term::var("_q")]),
                RelationPattern::new("implies", vec![Term::var("_q"), Term::var("_r")]),
            ],
            vec![
                RelationPattern::new("implies", vec![Term::var("_p"), Term::var("_r")]),
            ],
        ));

        // Equivalence: implies(p, q), implies(q, p) |- equivalent(p, q)
        self.add_rule(Rule::new(
            "equivalent_from_implies",
            vec![
                RelationPattern::new("implies", vec![Term::var("_p"), Term::var("_q")]),
                RelationPattern::new("implies", vec![Term::var("_q"), Term::var("_p")]),
            ],
            vec![
                RelationPattern::new("equivalent", vec![Term::var("_p"), Term::var("_q")]),
            ],
        ));
    }

    /// After closure, scan all unary properties and derive `implies_observed(P, Q)`
    /// for any pair where every instance of P is also an instance of Q.
    /// This is inductive (ω-rule style): based on all known instances.
    ///
    /// Observed implications are SEPARATE from logical implications:
    /// - `implies_observed(P, Q)` = "on current data, ext(P) ⊆ ext(Q)"
    /// - `implies(P, Q)` = deductive (user-declared or proven)
    /// Only `implies` triggers modus ponens. `implies_observed` is descriptive.
    pub fn derive_property_implications(&mut self) {
        // Collect all properties
        let properties: Vec<Term> = self
            .facts
            .iter()
            .filter(|f| f.name() == "is_property" && f.arity() == 1)
            .map(|f| f.terms()[0].clone())
            .collect();

        if properties.len() < 2 {
            return;
        }

        // For each property, collect its extension (set of elements that have it)
        let mut extensions: Vec<(Term, HashSet<Term>)> = Vec::new();
        for prop in &properties {
            let ext: HashSet<Term> = self
                .facts
                .iter()
                .filter(|f| {
                    f.name() == "has_property_1"
                        && f.arity() == 2
                        && &f.terms()[1] == prop
                })
                .map(|f| f.terms()[0].clone())
                .collect();
            extensions.push((prop.clone(), ext));
        }

        // Check all pairs: if ext(P) ⊆ ext(Q) and ext(P) is non-empty,
        // derive implies_observed(P, Q). NOT implies — observed implications
        // do not trigger modus ponens (they're inductive, not deductive).
        if !self.relation_defs.contains_key("implies_observed") {
            self.define_relation("implies_observed", 2);
        }
        if !self.relation_defs.contains_key("equivalent_observed") {
            self.define_relation("equivalent_observed", 2);
        }

        // Record similarity for all non-trivial property pairs
        if !self.relation_defs.contains_key("similarity") {
            self.define_relation("similarity", 3);
        }

        let mut new_facts = Vec::new();
        let mut seen_pairs: HashSet<(String, String)> = HashSet::new();

        for (p, ext_p) in &extensions {
            for (q, ext_q) in &extensions {
                if p == q {
                    continue;
                }

                // Similarity (record once per unordered pair)
                let p_str = p.to_string();
                let q_str = q.to_string();
                let pair_key = if p_str < q_str {
                    (p_str.clone(), q_str.clone())
                } else {
                    (q_str.clone(), p_str.clone())
                };
                if seen_pairs.insert(pair_key) {
                    if let Some(sim) = Self::extension_similarity(ext_p, ext_q) {
                        // Encode similarity as a ternary relation with a
                        // string-encoded float: similarity(p, q, "0.75")
                        let sim_str = format!("{:.4}", sim);
                        self.define_constant(&sim_str);
                        let sim_fact = Relation::new(
                            "similarity",
                            vec![p.clone(), q.clone(), Term::constant(&sim_str)],
                        );
                        if !self.facts.contains(&sim_fact) {
                            new_facts.push(sim_fact);
                        }
                    }
                    // If None (both empty), no similarity fact recorded
                }

                // Implication (directional)
                if ext_p.is_empty() {
                    continue;
                }
                if ext_p.is_subset(ext_q) {
                    let impl_fact =
                        Relation::binary("implies_observed", p.clone(), q.clone());
                    if !self.facts.contains(&impl_fact) {
                        new_facts.push(impl_fact);
                    }
                    if ext_q.is_subset(ext_p) {
                        let equiv_fact =
                            Relation::binary("equivalent_observed", p.clone(), q.clone());
                        if !self.facts.contains(&equiv_fact) {
                            new_facts.push(equiv_fact);
                        }
                    }
                }
            }
        }

        for fact in new_facts {
            self.facts.insert(fact);
        }
    }

    pub fn set_max_rounds(&mut self, n: usize) {
        self.max_rounds = n;
    }
    pub fn set_max_facts(&mut self, n: usize) {
        self.max_facts = n;
    }

    // ── hypothetical reasoning (proof by contradiction) ────

    /// Run closure under a hypothetical assumption. Returns `true` if the
    /// assumption leads to a contradiction (a fact with the given relation
    /// name is derived).
    ///
    /// The original engine is NOT modified. The assumption is tested on a
    /// clone, and only the contradiction check result is returned.
    ///
    /// Usage pattern for proof by contradiction:
    /// ```ignore
    /// // Define contradiction trigger:
    /// //   member(x, y), neg_member(x, y) |- contradiction()
    /// // Then:
    /// if engine.is_contradictory(assumption, "contradiction") {
    ///     // assumption leads to contradiction → its negation holds
    /// }
    /// ```
    pub fn is_contradictory(
        &self,
        assumption: Relation,
        contradiction_rel: &str,
    ) -> bool {
        let mut trial = self.clone();
        trial.hypothetical_mode = true;
        trial.add_fact(assumption);
        let result = trial.derive_closure();
        result
            .facts
            .iter()
            .any(|f| f.name() == contradiction_rel)
    }

    /// Run closure under multiple hypothetical assumptions.
    /// Returns the full closure result for inspection (not just contradiction check).
    pub fn hypothetical(&self, assumptions: Vec<Relation>) -> ClosureResult {
        let mut trial = self.clone();
        trial.hypothetical_mode = true;
        for a in assumptions {
            trial.add_fact(a);
        }
        trial.derive_closure()
    }

    // ── validation ───────────────────────────────────────────

    /// Validates all facts and rules against declared constants, variables,
    /// and relation schemas.
    ///
    /// Checks:
    /// 1. Every relation used in facts/rules has a declared schema.
    /// 2. Arities match the schema.
    /// 3. Every ground atom in facts is a declared constant.
    /// 4. Every variable in rules is a declared variable.
    ///
    /// Returns `Ok(())` if everything is consistent.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Check facts (skip ground-term validation for pattern facts)
        for fact in &self.facts {
            self.validate_relation_use(fact.name(), fact.arity(), &mut errors);
            if fact.is_ground() {
                for term in fact.terms() {
                    self.validate_ground_term(term, &mut errors);
                }
            }
        }

        // Check rule patterns
        for rule in &self.rules {
            for p in rule.premises() {
                self.validate_relation_use(p.name(), p.terms().len(), &mut errors);
                for term in p.terms() {
                    self.validate_pattern_term(term, &mut errors);
                }
            }
            for c in rule.conclusions() {
                self.validate_relation_use(c.name(), c.terms().len(), &mut errors);
                for term in c.terms() {
                    self.validate_pattern_term(term, &mut errors);
                }
            }
        }

        // Check reflexive/congruent marks
        for name in &self.reflexive_relations {
            if let Some(def) = self.relation_defs.get(name) {
                if def.arity != 2 {
                    errors.push(format!(
                        "relation '{}' is marked reflexive but has arity {} (expected 2)",
                        name, def.arity
                    ));
                }
            } else {
                errors.push(format!(
                    "relation '{}' is marked reflexive but not defined",
                    name
                ));
            }
        }
        for name in &self.congruent_relations {
            if let Some(def) = self.relation_defs.get(name) {
                if def.arity != 2 {
                    errors.push(format!(
                        "relation '{}' is marked congruent but has arity {} (expected 2)",
                        name, def.arity
                    ));
                }
            } else {
                errors.push(format!(
                    "relation '{}' is marked congruent but not defined",
                    name
                ));
            }
        }

        errors.sort();
        errors.dedup();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn validate_relation_use(&self, name: &str, arity: usize, errors: &mut Vec<String>) {
        match self.relation_defs.get(name) {
            None => {
                errors.push(format!("relation '{}' is used but not defined", name));
            }
            Some(def) if def.arity != arity => {
                errors.push(format!(
                    "relation '{}' has arity {} but used with {} terms",
                    name, def.arity, arity
                ));
            }
            _ => {}
        }
    }

    fn validate_ground_term(&self, term: &Term, errors: &mut Vec<String>) {
        match term {
            Term::Var(name) => {
                errors.push(format!(
                    "variable '{}' appears in a fact (facts must be ground)",
                    name
                ));
            }
            Term::App { symbol, args } => {
                if args.is_empty() && !self.constants.contains(symbol) {
                    errors.push(format!("constant '{}' is used but not defined", symbol));
                }
                for arg in args {
                    self.validate_ground_term(arg, errors);
                }
            }
        }
    }

    fn validate_pattern_term(&self, term: &Term, errors: &mut Vec<String>) {
        match term {
            Term::Var(name) => {
                if !self.variables.contains(name) {
                    errors.push(format!(
                        "variable '{}' is used in a rule but not declared",
                        name
                    ));
                }
            }
            Term::App { symbol, args } => {
                if args.is_empty() && !self.constants.contains(symbol) {
                    errors.push(format!(
                        "constant '{}' is used in a rule but not defined",
                        symbol
                    ));
                }
                for arg in args {
                    self.validate_pattern_term(arg, errors);
                }
            }
        }
    }

    // ── closure ──────────────────────────────────────────────

    /// Runs closure derivation until no new facts are produced or limits are hit.
    pub fn derive_closure(&mut self) -> ClosureResult {
        let initial = self.facts.clone();
        let mut rounds = 0;
        let mut fixed_point = false;
        let mut hit_limit = false;
        let mut warnings: Vec<String> = Vec::new();

        // Static analysis: detect expanding axioms
        let mut expanding: HashSet<usize> = HashSet::new();
        for (i, axiom) in self.axioms.iter().enumerate() {
            if detect_expanding(axiom) {
                expanding.insert(i);
                warnings.push(format!(
                    "Axiom \"{}\" is recursive (one side embeds in the other); \
                     instantiation depth is capped at {}",
                    axiom.name(),
                    MAX_TERM_DEPTH,
                ));
            }
        }

        let mut depth_grew_rounds = 0usize;
        let mut prev_max_depth = self
            .facts
            .iter()
            .flat_map(|f| f.terms().iter().map(|t| t.depth()))
            .max()
            .unwrap_or(0);

        // ── Global iteration loop ────────────────────────────
        // Stratum 0 (positive) runs to fixpoint, then negation strata run.
        // If negation strata produce new facts (e.g. AC creating Skolem
        // successors for non-maximal elements), re-run stratum 0 to absorb
        // the new facts, then re-run negation, and so on until global fixpoint.
        'global: loop {

        for _ in 0..self.max_rounds {
            rounds += 1;
            let mut new_facts: HashSet<Relation> = HashSet::new();

            // 1. Apply user-defined / explicit rules (stratum 0 only)
            for rule in &self.rules {
                if rule.has_negation() || rule.has_refutation() {
                    continue; // negated/refutation rules run in later phases
                }
                apply_rule(rule, &self.facts, &mut new_facts);
            }

            // 2. Built-in: reflexivity — R(t, t) for every reflexive relation
            if !self.reflexive_relations.is_empty() {
                let universe = self.collect_universe();
                for rel_name in &self.reflexive_relations {
                    for t in &universe {
                        let fact =
                            Relation::binary(rel_name.as_str(), t.clone(), t.clone());
                        if !self.facts.contains(&fact) {
                            new_facts.insert(fact);
                        }
                    }
                }
            }

            // 3. Built-in: congruence
            if !self.congruent_relations.is_empty() {
                self.apply_congruence(&mut new_facts);
            }

            // 4. Axiom instantiation
            if !self.axioms.is_empty() {
                let universe = self.collect_universe();
                let mut ground_terms: Vec<Term> =
                    universe.iter().filter(|t| t.is_ground()).cloned().collect();
                ground_terms.sort_by_key(|t| t.depth());
                if ground_terms.len() > MAX_AXIOM_UNIVERSE {
                    ground_terms.truncate(MAX_AXIOM_UNIVERSE);
                }

                let mut depth_capped = false;
                for (i, axiom) in self.axioms.iter().enumerate() {
                    let vars = axiom_variables(axiom);
                    let is_expanding = expanding.contains(&i);

                    let depth_limit = if is_expanding {
                        MAX_TERM_DEPTH - 1
                    } else {
                        MAX_AXIOM_SUB_DEPTH
                    };
                    let pool: Vec<Term> = ground_terms
                        .iter()
                        .filter(|t| t.depth() <= depth_limit)
                        .cloned()
                        .collect();

                    for sub in enumerate_substitutions(&vars, &pool) {
                        let lhs = substitute_total(axiom.lhs(), &sub);
                        let rhs = substitute_total(axiom.rhs(), &sub);

                        if lhs.depth() > MAX_TERM_DEPTH || rhs.depth() > MAX_TERM_DEPTH {
                            depth_capped = true;
                            continue;
                        }

                        let fact = Relation::binary(
                            axiom.equiv_relation(),
                            lhs,
                            rhs,
                        );
                        if !self.facts.contains(&fact) {
                            new_facts.insert(fact);
                        }
                    }
                }

                // Depth-growth detection for early halt
                let cur_max_depth = new_facts
                    .iter()
                    .flat_map(|f| f.terms().iter().map(|t| t.depth()))
                    .max()
                    .unwrap_or(0)
                    .max(prev_max_depth);

                if cur_max_depth > prev_max_depth {
                    depth_grew_rounds += 1;
                } else {
                    depth_grew_rounds = 0;
                }
                prev_max_depth = cur_max_depth;

                if depth_grew_rounds >= 3 && depth_capped {
                    warnings.push(format!(
                        "Recursive expansion detected: term depth grew for {} \
                         consecutive rounds (max depth {}); halting early",
                        depth_grew_rounds, cur_max_depth,
                    ));
                    // Still add what we have this round, then break
                    new_facts.retain(|f| !self.facts.contains(f));
                    for fact in new_facts {
                        self.facts.insert(fact);
                        if self.facts.len() >= self.max_facts {
                            hit_limit = true;
                            break;
                        }
                    }
                    break;
                }
            }

            // Remove anything already known
            new_facts.retain(|f| !self.facts.contains(f));

            if new_facts.is_empty() {
                fixed_point = true;
                break;
            }

            for fact in new_facts {
                self.facts.insert(fact);
                if self.facts.len() >= self.max_facts {
                    hit_limit = true;
                    break;
                }
            }

            if hit_limit {
                break;
            }
        }

        // ── Stratum 1+: negated rules ──────────────────────────
        // After stratum 0 reaches fixed point, apply rules with negated premises.
        let negated_rules: Vec<&Rule> = self
            .rules
            .iter()
            .filter(|r| r.has_negation())
            .collect();

        if !negated_rules.is_empty() && !hit_limit {
            let max_strata = negated_rules.iter().map(|r| r.stratum()).max().unwrap_or(1);
            let pre_neg_count = self.facts.len();

            for stratum in 1..=max_strata {
                let stratum_rules: Vec<&&Rule> = negated_rules
                    .iter()
                    .filter(|r| r.stratum() == stratum)
                    .collect();

                if stratum_rules.is_empty() {
                    continue;
                }

                // Run stratum rules to fixed point
                for _ in 0..self.max_rounds {
                    rounds += 1;
                    let mut new_facts: HashSet<Relation> = HashSet::new();

                    for rule in &stratum_rules {
                        apply_rule(rule, &self.facts, &mut new_facts);
                    }

                    new_facts.retain(|f| !self.facts.contains(f));
                    if new_facts.is_empty() {
                        break;
                    }

                    for fact in new_facts {
                        self.facts.insert(fact);
                        if self.facts.len() >= self.max_facts {
                            hit_limit = true;
                            break;
                        }
                    }

                    if hit_limit {
                        break;
                    }
                }
            }

            // If negation strata produced new facts, re-run from stratum 0
            if self.facts.len() > pre_neg_count && !hit_limit {
                fixed_point = false;
                continue 'global;
            }
        }

        break 'global;
        } // end 'global loop

        // ── Refutation phase (proof by contradiction) ───────
        // For each refutation rule: match positive premises, then for each
        // binding, scan all instantiations of the hypothesis. If ALL lead
        // to contradiction in a hypothetical branch, add the conclusion.
        let refutation_rules: Vec<&Rule> = self
            .rules
            .iter()
            .filter(|r| r.has_refutation())
            .collect();

        if !refutation_rules.is_empty() && !hit_limit && !self.hypothetical_mode {
            let mut refutation_derived: Vec<Relation> = Vec::new();

            for rule in &refutation_rules {
                let prem_matches = match_premises(rule.premises(), &self.facts);

                for prem_sub in &prem_matches {
                    // Ground-required check
                    if !rule.ground_required().is_empty() {
                        let ok = rule.ground_required().iter().all(|var| {
                            rule::substitute_partial(&Term::var(var), prem_sub).is_ground()
                        });
                        if !ok {
                            continue;
                        }
                    }

                    // Scan all bindings from refutation scan premises
                    let scan_matches =
                        match_premises(rule.refutation_scan(), &self.facts);

                    if scan_matches.is_empty() {
                        // No scan bindings → vacuously true → conclusion holds
                        for conc in rule.conclusions() {
                            let fact = rule::instantiate_partial(conc, prem_sub);
                            let key = if fact.is_ground() {
                                fact
                            } else {
                                fact.alpha_normalize()
                            };
                            refutation_derived.push(key);
                        }
                        continue;
                    }

                    // For ALL scan bindings, hypothetically add and check contradiction
                    let contradiction_rel = rule.contradiction_rel().unwrap();
                    let mut all_contradict = true;

                    for scan_sub in &scan_matches {
                        // Merge premise + scan substitutions
                        let mut merged = prem_sub.clone();
                        for (k, v) in scan_sub {
                            merged.entry(k.clone()).or_insert_with(|| v.clone());
                        }

                        // Build hypothetical facts
                        let hyp_facts: Vec<Relation> = rule
                            .refutation_hypotheses()
                            .iter()
                            .map(|h| {
                                let f = rule::instantiate_partial(h, &merged);
                                if f.is_ground() { f } else { f.alpha_normalize() }
                            })
                            .collect();

                        // Clone engine and test (suppress refutation in branch)
                        let mut trial = self.clone();
                        trial.hypothetical_mode = true;
                        for h in &hyp_facts {
                            trial.add_fact(h.clone());
                        }
                        let trial_result = trial.derive_closure();

                        let found_contradiction = trial_result
                            .facts
                            .iter()
                            .any(|f| f.name() == contradiction_rel);
                        if !found_contradiction {
                            all_contradict = false;
                            break; // one consistent case → refutation fails
                        }
                    }

                    if all_contradict {
                        for conc in rule.conclusions() {
                            let fact = rule::instantiate_partial(conc, prem_sub);
                            let key = if fact.is_ground() {
                                fact
                            } else {
                                fact.alpha_normalize()
                            };
                            if !self.facts.contains(&key) {
                                warnings.push(format!(
                                    "refutation: {} proven by contradiction via {}",
                                    key, rule.name(),
                                ));
                                refutation_derived.push(key);
                            }
                        }
                    }
                }
            }

            for fact in refutation_derived {
                self.facts.insert(fact);
            }
        }

        // ── ω-rule: inductive promotion ─────────────────────
        // After closure saturates, detect inductive chains and promote
        // to pattern facts. Then re-run closure with the new patterns.
        //
        // Defense: during post-promotion re-run, reject non-ground facts
        // from pure-transfer rules. A pure-transfer rule maps R(x)→S(x)
        // without adding term structure — this would blindly copy a
        // universal claim across relations. Constructive rules (ones that
        // add constants or constructors to the conclusion) are allowed.
        if fixed_point && !hit_limit {
            let promotions = self.detect_inductive_promotions();
            if !promotions.is_empty() {
                for p in &promotions {
                    warnings.push(format!(
                        "ω-rule: promoted {}/1 to pattern fact {}",
                        p.name(),
                        p
                    ));
                    self.facts.insert(p.clone());
                }

                // Identify pure-transfer rules (block their pattern-level output)
                let transfer_rules: HashSet<String> = self
                    .rules
                    .iter()
                    .filter(|r| is_pure_transfer(r))
                    .map(|r| r.name().to_string())
                    .collect();
                let mut transfer_warned: HashSet<String> = HashSet::new();

                // Re-run positive closure with new pattern facts
                for _ in 0..self.max_rounds {
                    rounds += 1;
                    let mut new_facts: HashSet<Relation> = HashSet::new();
                    for rule in &self.rules {
                        if rule.has_negation() {
                            continue;
                        }
                        let mut rule_facts: HashSet<Relation> = HashSet::new();
                        apply_rule(rule, &self.facts, &mut rule_facts);
                        // Defense: block non-ground facts from pure-transfer rules
                        if transfer_rules.contains(rule.name()) {
                            for f in rule_facts {
                                if f.is_ground() && !self.facts.contains(&f) {
                                    new_facts.insert(f);
                                } else if !f.is_ground() {
                                    if transfer_warned.insert(rule.name().to_string()) {
                                        warnings.push(format!(
                                            "ω-defense: blocked pattern facts from \
                                             pure-transfer rule {}",
                                            rule.name(),
                                        ));
                                    }
                                }
                            }
                        } else {
                            for f in rule_facts {
                                if !self.facts.contains(&f) {
                                    new_facts.insert(f);
                                }
                            }
                        }
                    }
                    new_facts.retain(|f| !self.facts.contains(f));
                    if new_facts.is_empty() {
                        break;
                    }
                    for fact in new_facts {
                        self.facts.insert(fact);
                        if self.facts.len() >= self.max_facts {
                            hit_limit = true;
                            break;
                        }
                    }
                    if hit_limit {
                        break;
                    }
                }
            }
        }

        // ── Property implication (inductive, post-closure) ────
        // After all derivation phases, scan property extensions and derive
        // implies(P, Q) where ext(P) ⊆ ext(Q). Then re-run positive closure
        // to propagate modus ponens and transitivity.
        if self.relation_defs.contains_key("is_property") && !hit_limit {
            let pre_impl = self.facts.len();
            self.derive_property_implications();
            if self.facts.len() > pre_impl {
                // Re-run positive closure to propagate implications
                for _ in 0..self.max_rounds {
                    rounds += 1;
                    let mut new_facts: HashSet<Relation> = HashSet::new();
                    for rule in &self.rules {
                        if rule.has_negation() || rule.has_refutation() {
                            continue;
                        }
                        apply_rule(rule, &self.facts, &mut new_facts);
                    }
                    new_facts.retain(|f| !self.facts.contains(f));
                    if new_facts.is_empty() {
                        break;
                    }
                    for fact in new_facts {
                        self.facts.insert(fact);
                        if self.facts.len() >= self.max_facts {
                            break;
                        }
                    }
                }
            }
        }

        let mut all_facts: Vec<Relation> = self.facts.iter().cloned().collect();
        all_facts.sort_by(|a, b| a.to_string().cmp(&b.to_string()));

        let mut derived: Vec<Relation> = self
            .facts
            .iter()
            .filter(|f| !initial.contains(f))
            .cloned()
            .collect();
        derived.sort_by(|a, b| a.to_string().cmp(&b.to_string()));

        ClosureResult {
            facts: all_facts,
            derived,
            rounds,
            saturated: fixed_point && !hit_limit,
            warnings,
        }
    }

    // ── inductive promotion (ω-rule) ──────────────────────

    /// Detect unary relations with inductive rule chains and return
    /// pattern facts for promotion.
    ///
    /// An inductive pattern is:
    /// - A unary relation R with at least one ground base case `R(base)`
    /// - A rule with premise `R(?x)` and conclusion `R(f(?x))` for some
    ///   constructor `f`
    ///
    /// If the closure reached fixpoint (all derivable instances exist),
    /// the relation is promoted to a pattern fact `R(_0)`.
    fn detect_inductive_promotions(&self) -> Vec<Relation> {
        let mut promotions = Vec::new();
        let mut seen_rels: HashSet<&str> = HashSet::new();

        for rule in &self.rules {
            if rule.has_negation() {
                continue;
            }
            if rule.premises().len() != 1 || rule.conclusions().len() != 1 {
                continue;
            }

            let prem = &rule.premises()[0];
            let conc = &rule.conclusions()[0];

            // Same relation, unary
            if prem.name() != conc.name() {
                continue;
            }
            if prem.terms().len() != 1 || conc.terms().len() != 1 {
                continue;
            }

            let rel_name = prem.name();
            if seen_rels.contains(rel_name) {
                continue;
            }

            // Premise must be a single variable
            let prem_var = match &prem.terms()[0] {
                Term::Var(name) => name,
                _ => continue,
            };

            // Conclusion must wrap that variable in a constructor
            match &conc.terms()[0] {
                Term::App { args, .. } => {
                    if !args
                        .iter()
                        .any(|a| matches!(a, Term::Var(n) if n == prem_var))
                    {
                        continue;
                    }
                }
                _ => continue,
            }

            // Found inductive rule for rel_name.
            // Check that a ground base case exists.
            let has_base = self
                .facts
                .iter()
                .any(|f| f.name() == rel_name && f.arity() == 1 && f.is_ground());
            if !has_base {
                continue;
            }

            // Check pattern fact doesn't already exist
            let pattern =
                Relation::new(rel_name, vec![Term::var("_0")]).alpha_normalize();
            if self.facts.contains(&pattern) {
                continue;
            }

            seen_rels.insert(rel_name);
            promotions.push(pattern);
        }

        promotions
    }

    // ── internals ────────────────────────────────────────────

    /// Collects every ground subterm from every fact, plus declared constants.
    fn collect_universe(&self) -> HashSet<Term> {
        let mut terms = HashSet::new();
        for fact in &self.facts {
            for term in fact.terms() {
                term.collect_subterms(&mut terms);
            }
        }
        // Declared constants are part of the universe even without facts
        for name in &self.constants {
            terms.insert(Term::constant(name.as_str()));
        }
        // Filter out non-ground terms (variables from pattern facts)
        terms.retain(|t| t.is_ground());
        terms
    }

    /// For each congruent relation `R` with fact `R(a, b)`, and each compound
    /// term `f(…, a, …)` in the universe, derive `R(f(…a…), f(…b…))`.
    fn apply_congruence(&self, new_facts: &mut HashSet<Relation>) {
        let universe = self.collect_universe();

        for rel_name in &self.congruent_relations {
            let pairs: Vec<(&Term, &Term)> = self
                .facts
                .iter()
                .filter(|f| f.name() == rel_name && f.arity() == 2 && f.is_ground())
                .map(|f| (&f.terms()[0], &f.terms()[1]))
                .collect();

            if pairs.is_empty() {
                continue;
            }

            for term in &universe {
                if let Term::App { symbol, args } = term {
                    for (i, arg) in args.iter().enumerate() {
                        for &(a, b) in &pairs {
                            if arg == a && a != b {
                                let mut new_args = args.clone();
                                new_args[i] = b.clone();
                                let new_term = Term::app(symbol.clone(), new_args);
                                if new_term.depth() <= MAX_TERM_DEPTH {
                                    let rel = Relation::binary(
                                        rel_name.as_str(),
                                        term.clone(),
                                        new_term,
                                    );
                                    if !self.facts.contains(&rel) {
                                        new_facts.insert(rel);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ── Axiom helpers ────────────────────────────────────────────

/// Collect all variable names from a term.
fn collect_var_names(term: &Term, vars: &mut HashSet<String>) {
    match term {
        Term::Var(name) => {
            vars.insert(name.clone());
        }
        Term::App { args, .. } => {
            for arg in args {
                collect_var_names(arg, vars);
            }
        }
    }
}

/// Get sorted variable names from an axiom's lhs and rhs.
fn axiom_variables(axiom: &Axiom) -> Vec<String> {
    let mut vars = HashSet::new();
    collect_var_names(axiom.lhs(), &mut vars);
    collect_var_names(axiom.rhs(), &mut vars);
    let mut v: Vec<String> = vars.into_iter().collect();
    v.sort();
    v
}

/// Generate all possible substitutions mapping variables to terms.
fn enumerate_substitutions(
    vars: &[String],
    terms: &[Term],
) -> Vec<HashMap<String, Term>> {
    if vars.is_empty() {
        return vec![HashMap::new()];
    }
    let rest = enumerate_substitutions(&vars[1..], terms);
    let mut result = Vec::with_capacity(terms.len() * rest.len());
    for term in terms {
        for sub in &rest {
            let mut new_sub = sub.clone();
            new_sub.insert(vars[0].clone(), term.clone());
            result.push(new_sub);
        }
    }
    result
}

/// Substitute all variables in a term (total: unbound vars remain as-is).
fn substitute_total(term: &Term, sub: &HashMap<String, Term>) -> Term {
    match term {
        Term::Var(name) => sub.get(name).cloned().unwrap_or_else(|| term.clone()),
        Term::App { symbol, args } => Term::app(
            symbol.clone(),
            args.iter().map(|a| substitute_total(a, sub)).collect(),
        ),
    }
}

/// Detect if an axiom is "expanding" — one side embeds structurally in the
/// other, causing unbounded term growth during instantiation.
fn detect_expanding(axiom: &Axiom) -> bool {
    let ld = axiom.lhs().depth();
    let rd = axiom.rhs().depth();
    if ld == rd {
        return false;
    }
    let (pattern, host) = if ld < rd {
        (axiom.lhs(), axiom.rhs())
    } else {
        (axiom.rhs(), axiom.lhs())
    };
    if matches!(pattern, Term::Var(_)) {
        return false;
    }
    proper_subterms(host)
        .iter()
        .any(|sub| structural_matches(pattern, sub))
}

/// Collect all proper subterms (children and their descendants, not the term itself).
fn proper_subterms(term: &Term) -> Vec<&Term> {
    let mut out = Vec::new();
    if let Term::App { args, .. } = term {
        for arg in args {
            collect_all_subterms_ref(arg, &mut out);
        }
    }
    out
}

fn collect_all_subterms_ref<'a>(term: &'a Term, out: &mut Vec<&'a Term>) {
    out.push(term);
    if let Term::App { args, .. } = term {
        for arg in args {
            collect_all_subterms_ref(arg, out);
        }
    }
}

/// Check if `pattern` structurally matches `term` (variables match anything).
fn structural_matches(pattern: &Term, term: &Term) -> bool {
    let mut bindings: HashMap<&str, &Term> = HashMap::new();
    structural_matches_inner(pattern, term, &mut bindings)
}

fn structural_matches_inner<'a>(
    pattern: &'a Term,
    term: &'a Term,
    bindings: &mut HashMap<&'a str, &'a Term>,
) -> bool {
    match pattern {
        Term::Var(name) => {
            if let Some(&bound) = bindings.get(name.as_str()) {
                bound == term
            } else {
                bindings.insert(name, term);
                true
            }
        }
        Term::App {
            symbol: ps,
            args: pa,
        } => {
            if let Term::App {
                symbol: ts,
                args: ta,
            } = term
            {
                ps == ts
                    && pa.len() == ta.len()
                    && pa
                        .iter()
                        .zip(ta.iter())
                        .all(|(p, t)| structural_matches_inner(p, t, bindings))
            } else {
                false
            }
        }
    }
}

/// Find all substitutions that simultaneously satisfy every premise.
fn match_premises(
    premises: &[RelationPattern],
    facts: &HashSet<Relation>,
) -> Vec<Substitution> {
    let mut subs: Vec<Substitution> = vec![HashMap::new()];
    let mut var_counter: usize = 0;

    for premise in premises {
        let mut next = Vec::new();
        for sub in &subs {
            for fact in facts {
                if fact.name() != premise.name() || fact.arity() != premise.terms().len() {
                    continue;
                }
                let mut candidate = sub.clone();
                if fact.is_ground() {
                    // Ground fact: use unification (equivalent to match_relation)
                    if rule::unify_relation(premise, fact, &mut candidate) {
                        next.push(candidate);
                    }
                } else {
                    // Pattern fact: rename variables to avoid collision, then unify
                    let renamed = fact.rename_vars_fresh(&mut var_counter);
                    if rule::unify_relation(premise, &renamed, &mut candidate) {
                        next.push(candidate);
                    }
                }
            }
        }
        subs = next;
        if subs.is_empty() {
            break;
        }
    }

    subs
}

/// Check if a rule is a pure variable transfer: single premise R(?x) → single
/// conclusion S(?x) where the conclusion's terms are just variables from the
/// premise without any added constants or constructors.
///
/// Pure transfers blindly copy universal claims across relations. At the
/// pattern level this is dangerous: `nat(_0)` through `nat(x)|-even(x)`
/// produces `even(_0)` which is wrong.
///
/// Constructive rules like `set(x) |- subset(empty, x)` are safe because
/// the conclusion adds structure (the constant `empty`).
fn is_pure_transfer(rule: &Rule) -> bool {
    if rule.premises().len() != 1 || rule.conclusions().len() != 1 {
        return false;
    }
    let prem = &rule.premises()[0];
    let conc = &rule.conclusions()[0];
    // Same relation doesn't count (that's the inductive step, not a transfer)
    if prem.name() == conc.name() {
        return false;
    }
    // Collect premise variable names
    let prem_vars: HashSet<&str> = prem
        .terms()
        .iter()
        .filter_map(|t| match t {
            Term::Var(n) => Some(n.as_str()),
            _ => None,
        })
        .collect();
    // Conclusion terms must ALL be variables from the premise
    conc.terms().iter().all(|t| match t {
        Term::Var(n) => prem_vars.contains(n.as_str()),
        _ => false,
    })
}

/// Apply a single rule against the fact set, collecting new facts.
fn apply_rule(
    rule: &Rule,
    facts: &HashSet<Relation>,
    new_facts: &mut HashSet<Relation>,
) {
    let matches = match_premises(rule.premises(), facts);
    for sub in &matches {
        // Check ground-required variable constraints
        if !rule.ground_required().is_empty() {
            let ok = rule.ground_required().iter().all(|var| {
                rule::substitute_partial(&Term::var(var), sub).is_ground()
            });
            if !ok {
                continue;
            }
        }
        // Check negated premises: each must NOT match any fact
        if rule.has_negation() && !check_negated_absent(rule.negated_premises(), sub, facts) {
            continue;
        }
        for conclusion in rule.conclusions() {
            let fact = rule::instantiate_partial(conclusion, sub);
            let max_d = fact
                .terms()
                .iter()
                .map(|t| t.depth())
                .max()
                .unwrap_or(0);
            if max_d > MAX_TERM_DEPTH {
                continue;
            }
            let key = if fact.is_ground() {
                fact
            } else {
                fact.alpha_normalize()
            };
            if !facts.contains(&key) {
                new_facts.insert(key);
            }
        }
    }
}

/// Check that all negated premises are absent from the fact set.
/// Each negated pattern is instantiated with the current substitution,
/// then checked for absence.
fn check_negated_absent(
    negated: &[RelationPattern],
    sub: &Substitution,
    facts: &HashSet<Relation>,
) -> bool {
    for neg_pattern in negated {
        let instantiated = rule::instantiate_partial(neg_pattern, sub);
        if !instantiated.is_ground() {
            // Non-ground negation: check if ANY fact matches the pattern
            // This implements "not exists" semantics
            let found = facts.iter().any(|fact| {
                if fact.name() != instantiated.name()
                    || fact.arity() != instantiated.arity()
                {
                    return false;
                }
                let mut test_sub = sub.clone();
                rule::unify_relation(
                    &RelationPattern::new(
                        instantiated.name(),
                        instantiated.terms().to_vec(),
                    ),
                    fact,
                    &mut test_sub,
                )
            });
            if found {
                return false; // negated premise IS present → rule blocked
            }
        } else {
            // Ground negation: simple membership check
            if facts.contains(&instantiated) {
                return false; // negated premise IS present → rule blocked
            }
        }
    }
    true // all negated premises are absent → rule can fire
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(s: &str) -> Term {
        Term::constant(s)
    }

    fn equiv(a: Term, b: Term) -> Relation {
        Relation::binary("equiv", a, b)
    }

    // ── Entity and relation definitions ──────────────────────

    #[test]
    fn test_define_constant() {
        let mut engine = ClosureEngine::new();
        let a = engine.define_constant("a");
        assert_eq!(a, Term::constant("a"));
        assert!(engine.constants().contains("a"));
    }

    #[test]
    fn test_define_variable() {
        let mut engine = ClosureEngine::new();
        let x = engine.define_variable("x");
        assert_eq!(x, Term::var("x"));
        assert!(engine.variables().contains("x"));
    }

    #[test]
    fn test_define_relation() {
        let mut engine = ClosureEngine::new();
        engine.define_relation("parent", 2);
        assert_eq!(engine.relation_defs()["parent"].arity(), 2);
    }

    #[test]
    fn test_define_equivalence() {
        let mut engine = ClosureEngine::new();
        engine.define_equivalence("sim");
        assert_eq!(engine.relation_defs()["sim"].arity(), 2);
        // Should have symmetry + transitivity rules
        assert_eq!(engine.rules().len(), 2);
        assert!(engine
            .rules()
            .iter()
            .any(|r| r.name() == "sim_symmetry"));
        assert!(engine
            .rules()
            .iter()
            .any(|r| r.name() == "sim_transitivity"));
    }

    #[test]
    fn test_declared_constants_in_universe() {
        let mut engine = ClosureEngine::new();
        engine.define_equivalence("equiv");
        let a = engine.define_constant("a");
        let b = engine.define_constant("b");
        // No facts yet, but constants are in the universe
        engine.add_fact(equiv(a.clone(), b.clone()));

        let result = engine.derive_closure();
        // Reflexivity should fire for a and b (they're declared constants)
        assert!(result.facts.contains(&equiv(a.clone(), a)));
        assert!(result.facts.contains(&equiv(b.clone(), b)));
    }

    // ── Validation ───────────────────────────────────────────

    #[test]
    fn test_validate_ok() {
        let mut engine = ClosureEngine::new();
        let a = engine.define_constant("a");
        let b = engine.define_constant("b");
        engine.define_equivalence("equiv");
        engine.add_fact(equiv(a, b));
        assert!(engine.validate().is_ok());
    }

    #[test]
    fn test_validate_undeclared_relation() {
        let mut engine = ClosureEngine::new();
        engine.define_constant("a");
        engine.define_constant("b");
        // Don't define "equiv"
        engine.add_fact(equiv(c("a"), c("b")));
        let errs = engine.validate().unwrap_err();
        assert!(errs.iter().any(|e| e.contains("'equiv'") && e.contains("not defined")));
    }

    #[test]
    fn test_validate_undeclared_constant() {
        let mut engine = ClosureEngine::new();
        engine.define_equivalence("equiv");
        // Don't define constants
        engine.add_fact(equiv(c("a"), c("b")));
        let errs = engine.validate().unwrap_err();
        assert!(errs.iter().any(|e| e.contains("'a'")));
        assert!(errs.iter().any(|e| e.contains("'b'")));
    }

    #[test]
    fn test_validate_arity_mismatch() {
        let mut engine = ClosureEngine::new();
        engine.define_constant("a");
        engine.define_constant("b");
        engine.define_constant("c_");
        engine.define_relation("rel", 2);
        engine.add_fact(Relation::new("rel", vec![c("a"), c("b"), c("c_")]));
        let errs = engine.validate().unwrap_err();
        assert!(errs.iter().any(|e| e.contains("arity")));
    }

    #[test]
    fn test_validate_undeclared_variable_in_rule() {
        let mut engine = ClosureEngine::new();
        engine.define_relation("r", 2);
        // Use variable "x" without declaring it
        engine.add_rule(Rule::new(
            "test",
            vec![RelationPattern::new("r", vec![Term::var("x"), Term::var("y")])],
            vec![RelationPattern::new("r", vec![Term::var("y"), Term::var("x")])],
        ));
        let errs = engine.validate().unwrap_err();
        assert!(errs.iter().any(|e| e.contains("'x'")));
        assert!(errs.iter().any(|e| e.contains("'y'")));
    }

    // ── Spec demo ────────────────────────────────────────────

    #[test]
    fn test_basic_equiv_closure() {
        let mut engine = ClosureEngine::with_defaults();
        engine.add_fact(equiv(c("a"), c("b")));
        engine.add_fact(equiv(c("b"), c("c")));

        let result = engine.derive_closure();

        // Transitivity
        assert!(result.facts.contains(&equiv(c("a"), c("c"))));
        // Symmetry
        assert!(result.facts.contains(&equiv(c("c"), c("a"))));
        assert!(result.facts.contains(&equiv(c("b"), c("a"))));
        assert!(result.facts.contains(&equiv(c("c"), c("b"))));
        // Reflexivity
        assert!(result.facts.contains(&equiv(c("a"), c("a"))));
        assert!(result.facts.contains(&equiv(c("b"), c("b"))));
        assert!(result.facts.contains(&equiv(c("c"), c("c"))));

        assert!(result.saturated);
    }

    #[test]
    fn test_closure_reaches_fixed_point() {
        let mut engine = ClosureEngine::with_defaults();
        engine.add_fact(equiv(c("a"), c("b")));

        let result = engine.derive_closure();
        assert!(result.saturated);

        // All 4 equiv facts: (a,b), (b,a), (a,a), (b,b)
        assert_eq!(
            result.facts.iter().filter(|f| f.name() == "equiv").count(),
            4
        );
    }

    // ── Custom equivalence relation ──────────────────────────

    #[test]
    fn test_custom_equivalence() {
        let mut engine = ClosureEngine::new();
        engine.define_equivalence("similar");

        let a = c("a");
        let b = c("b");
        let cc = c("c");
        engine.add_fact(Relation::binary("similar", a.clone(), b.clone()));
        engine.add_fact(Relation::binary("similar", b.clone(), cc.clone()));

        let result = engine.derive_closure();
        assert!(result.facts.contains(&Relation::binary("similar", a.clone(), cc.clone())));
        assert!(result.facts.contains(&Relation::binary("similar", cc, a)));
        assert!(result.saturated);
    }

    // ── Congruence ───────────────────────────────────────────

    #[test]
    fn test_congruence_unary() {
        let mut engine = ClosureEngine::with_defaults();
        let fa = Term::app("f", vec![c("a")]);
        let fb = Term::app("f", vec![c("b")]);

        engine.add_fact(equiv(c("a"), c("b")));
        engine.add_fact(Relation::new("has", vec![fa.clone()]));

        let result = engine.derive_closure();
        assert!(result.facts.contains(&equiv(fa, fb)));
    }

    #[test]
    fn test_congruence_binary() {
        let mut engine = ClosureEngine::with_defaults();
        let a = c("a");
        let b = c("b");
        let gaa = Term::app("g", vec![a.clone(), a.clone()]);
        let gba = Term::app("g", vec![b.clone(), a.clone()]);
        let gab = Term::app("g", vec![a.clone(), b.clone()]);
        let gbb = Term::app("g", vec![b.clone(), b.clone()]);

        engine.add_fact(equiv(a.clone(), b.clone()));
        engine.add_fact(Relation::new("has", vec![gaa.clone()]));

        let result = engine.derive_closure();
        assert!(result.facts.contains(&equiv(gaa.clone(), gba.clone())));
        assert!(result.facts.contains(&equiv(gaa.clone(), gab.clone())));
        assert!(result.facts.contains(&equiv(gaa, gbb)));
    }

    #[test]
    fn test_congruence_for_custom_relation() {
        let mut engine = ClosureEngine::new();
        engine.define_equivalence("sim");

        let fa = Term::app("f", vec![c("a")]);
        let fb = Term::app("f", vec![c("b")]);

        engine.add_fact(Relation::binary("sim", c("a"), c("b")));
        engine.add_fact(Relation::new("has", vec![fa.clone()]));

        let result = engine.derive_closure();
        assert!(result.facts.contains(&Relation::binary("sim", fa, fb)));
    }

    // ── Custom rules ─────────────────────────────────────────

    #[test]
    fn test_custom_rule() {
        let rule = Rule::new(
            "grandparent",
            vec![
                RelationPattern::new("parent", vec![Term::var("x"), Term::var("y")]),
                RelationPattern::new("parent", vec![Term::var("y"), Term::var("z")]),
            ],
            vec![RelationPattern::new(
                "grandparent",
                vec![Term::var("x"), Term::var("z")],
            )],
        );

        let mut engine = ClosureEngine::new();
        engine.add_rule(rule);

        engine.add_fact(Relation::binary("parent", c("alice"), c("bob")));
        engine.add_fact(Relation::binary("parent", c("bob"), c("charlie")));

        let result = engine.derive_closure();
        assert!(result.facts.contains(&Relation::binary(
            "grandparent",
            c("alice"),
            c("charlie")
        )));
        assert!(result.saturated);
    }

    #[test]
    fn test_custom_rule_chain() {
        let r1 = Rule::new(
            "ancestor_base",
            vec![RelationPattern::new(
                "parent",
                vec![Term::var("x"), Term::var("y")],
            )],
            vec![RelationPattern::new(
                "ancestor",
                vec![Term::var("x"), Term::var("y")],
            )],
        );
        let r2 = Rule::new(
            "ancestor_step",
            vec![
                RelationPattern::new("ancestor", vec![Term::var("x"), Term::var("y")]),
                RelationPattern::new("parent", vec![Term::var("y"), Term::var("z")]),
            ],
            vec![RelationPattern::new(
                "ancestor",
                vec![Term::var("x"), Term::var("z")],
            )],
        );

        let mut engine = ClosureEngine::new();
        engine.add_rule(r1);
        engine.add_rule(r2);

        engine.add_fact(Relation::binary("parent", c("a"), c("b")));
        engine.add_fact(Relation::binary("parent", c("b"), c("c")));
        engine.add_fact(Relation::binary("parent", c("c"), c("d")));

        let result = engine.derive_closure();

        assert!(result
            .facts
            .contains(&Relation::binary("ancestor", c("a"), c("b"))));
        assert!(result
            .facts
            .contains(&Relation::binary("ancestor", c("a"), c("c"))));
        assert!(result
            .facts
            .contains(&Relation::binary("ancestor", c("a"), c("d"))));
        assert!(result
            .facts
            .contains(&Relation::binary("ancestor", c("b"), c("d"))));
        assert!(result.saturated);
    }

    // ── Limits ───────────────────────────────────────────────

    #[test]
    fn test_empty_engine() {
        let mut engine = ClosureEngine::new();
        let result = engine.derive_closure();
        assert!(result.facts.is_empty());
        assert!(result.derived.is_empty());
        assert!(result.saturated);
    }

    #[test]
    fn test_max_rounds() {
        let mut engine = ClosureEngine::with_defaults();
        engine.set_max_rounds(1);
        engine.add_fact(equiv(c("a"), c("b")));
        engine.add_fact(equiv(c("b"), c("c")));

        let result = engine.derive_closure();
        assert_eq!(result.rounds, 1);
        assert!(result.facts.contains(&equiv(c("b"), c("a"))));
    }

    // ── Display ──────────────────────────────────────────────

    #[test]
    fn test_result_sorted() {
        let mut engine = ClosureEngine::with_defaults();
        engine.add_fact(equiv(c("b"), c("a")));

        let result = engine.derive_closure();
        for i in 1..result.facts.len() {
            assert!(result.facts[i - 1].to_string() <= result.facts[i].to_string());
        }
    }

    // ── Axiom instantiation ─────────────────────────────────

    #[test]
    fn test_axiom_identity() {
        // Axiom: mul(x, e) = x  (right identity)
        // Use a plain relation (no congruence) to test pure axiom instantiation
        let mut engine = ClosureEngine::new();
        engine.define_relation("eq", 2);
        engine.define_constant("a");
        engine.define_constant("e");

        let x = Term::var("x");
        engine.add_axiom(Axiom::new(
            "right_identity",
            Term::app("mul", vec![x.clone(), Term::constant("e")]),
            x,
            "eq",
        ));

        // Seed facts to get constants into the universe
        engine.add_fact(Relation::binary("eq", c("a"), c("a")));
        engine.add_fact(Relation::binary("eq", c("e"), c("e")));

        let result = engine.derive_closure();

        let mul_a_e = Term::app("mul", vec![c("a"), c("e")]);
        assert!(
            result.facts.contains(&Relation::binary("eq", mul_a_e, c("a"))),
            "should derive eq(mul(a, e), a)"
        );
    }

    #[test]
    fn test_axiom_commutativity() {
        // Axiom: add(x, y) = add(y, x) — no congruence
        let mut engine = ClosureEngine::new();
        engine.define_relation("eq", 2);
        engine.define_constant("a");
        engine.define_constant("b");

        let (x, y) = (Term::var("x"), Term::var("y"));
        engine.add_axiom(Axiom::new(
            "add_comm",
            Term::app("add", vec![x.clone(), y.clone()]),
            Term::app("add", vec![y, x]),
            "eq",
        ));

        engine.add_fact(Relation::binary("eq", c("a"), c("a")));
        engine.add_fact(Relation::binary("eq", c("b"), c("b")));

        let result = engine.derive_closure();

        let add_ab = Term::app("add", vec![c("a"), c("b")]);
        let add_ba = Term::app("add", vec![c("b"), c("a")]);
        assert!(
            result
                .facts
                .contains(&Relation::binary("eq", add_ab, add_ba)),
            "should derive eq(add(a,b), add(b,a))"
        );
    }

    #[test]
    fn test_axiom_no_facts_no_expansion() {
        let mut engine = ClosureEngine::new();
        let x = Term::var("x");
        engine.add_axiom(Axiom::new(
            "id",
            Term::app("f", vec![x.clone()]),
            x,
            "eq",
        ));

        let result = engine.derive_closure();
        assert!(result.derived.is_empty());
        assert!(result.saturated);
    }

    #[test]
    fn test_axiom_expanding_detection() {
        // Axiom: f(x) = f(f(x)) — recursive, should be detected
        let mut engine = ClosureEngine::new();
        engine.define_relation("eq", 2);
        engine.define_constant("a");

        let x = Term::var("x");
        engine.add_axiom(Axiom::new(
            "wrap",
            Term::app("f", vec![x.clone()]),
            Term::app("f", vec![Term::app("f", vec![x])]),
            "eq",
        ));

        engine.add_fact(Relation::binary("eq", c("a"), c("a")));
        engine.set_max_rounds(20);

        let result = engine.derive_closure();

        assert!(
            result.warnings.iter().any(|w| w.contains("recursive")),
            "should warn about recursive axiom, got: {:?}",
            result.warnings,
        );
        // All terms should be within depth limit
        for fact in &result.facts {
            for term in fact.terms() {
                assert!(
                    term.depth() <= MAX_TERM_DEPTH,
                    "term {} has depth {} > MAX_TERM_DEPTH {}",
                    term,
                    term.depth(),
                    MAX_TERM_DEPTH,
                );
            }
        }
    }

    #[test]
    fn test_axiom_non_expanding_no_warning() {
        let mut engine = ClosureEngine::new();
        engine.define_relation("eq", 2);
        engine.define_constant("a");
        engine.define_constant("e");

        let x = Term::var("x");
        engine.add_axiom(Axiom::new(
            "right_id",
            Term::app("mul", vec![x.clone(), Term::constant("e")]),
            x,
            "eq",
        ));

        engine.add_fact(Relation::binary("eq", c("a"), c("a")));
        engine.add_fact(Relation::binary("eq", c("e"), c("e")));

        let result = engine.derive_closure();
        assert!(
            result.warnings.is_empty(),
            "identity axiom should produce no warnings, got: {:?}",
            result.warnings,
        );
    }

    #[test]
    fn test_axiom_with_equivalence_interaction() {
        // Axiom: mul(x, e) = x with symmetry+transitivity (but NOT congruence)
        // If we know eq(a, b), axiom should instantiate for both a and b
        let mut engine = ClosureEngine::new();
        engine.define_relation("eq", 2);
        engine.define_variable("x");
        engine.define_variable("y");
        engine.define_variable("z");
        engine.add_rule(rule::symmetry_for("eq"));
        engine.add_rule(rule::transitivity_for("eq"));
        engine.mark_reflexive("eq");

        engine.define_constant("a");
        engine.define_constant("b");
        engine.define_constant("e");

        let x = Term::var("x");
        engine.add_axiom(Axiom::new(
            "right_id",
            Term::app("mul", vec![x.clone(), Term::constant("e")]),
            x,
            "eq",
        ));

        engine.add_fact(Relation::binary("eq", c("a"), c("b")));

        let result = engine.derive_closure();

        let mul_a_e = Term::app("mul", vec![c("a"), c("e")]);
        let mul_b_e = Term::app("mul", vec![c("b"), c("e")]);
        assert!(result.facts.contains(&Relation::binary("eq", mul_a_e, c("a"))));
        assert!(result.facts.contains(&Relation::binary("eq", mul_b_e, c("b"))));
    }

    #[test]
    fn test_warnings_empty_by_default() {
        let mut engine = ClosureEngine::with_defaults();
        engine.add_fact(equiv(c("a"), c("b")));
        let result = engine.derive_closure();
        assert!(result.warnings.is_empty());
    }

    // ── Propositional logic semantics (research step 1) ─────

    /// Build a propositional logic engine with 2 variables (p, q) and 4 valuations.
    /// Tests that tautologies, contradictions, and contingencies are correctly classified
    /// using pure relational closure — no built-in negation.
    #[test]
    fn test_propositional_logic_tautology_detection() {
        let mut engine = ClosureEngine::new();

        // Declare relations
        engine.define_relation("tv_t", 2); // tv_t(v, f): f is true in valuation v
        engine.define_relation("tv_f", 2); // tv_f(v, f): f is false in valuation v
        engine.define_relation("declared", 1); // declared(f): f is a formula we evaluate
        engine.define_relation("tautology", 1);
        engine.define_relation("contradiction", 1);

        // Declare variables for rules
        engine.define_variable("v");
        engine.define_variable("p");
        engine.define_variable("q");
        engine.define_variable("r");
        engine.define_variable("f");

        // Declare constants: valuations
        let v_tt = engine.define_constant("v_tt");
        let v_tf = engine.define_constant("v_tf");
        let v_ft = engine.define_constant("v_ft");
        let v_ff = engine.define_constant("v_ff");
        // Atomic propositions
        let p = engine.define_constant("p");
        let q = engine.define_constant("q");

        // --- Atomic truth assignments ---
        // v_tt: p=T, q=T
        engine.add_fact(Relation::binary("tv_t", v_tt.clone(), p.clone()));
        engine.add_fact(Relation::binary("tv_t", v_tt.clone(), q.clone()));
        // v_tf: p=T, q=F
        engine.add_fact(Relation::binary("tv_t", v_tf.clone(), p.clone()));
        engine.add_fact(Relation::binary("tv_f", v_tf.clone(), q.clone()));
        // v_ft: p=F, q=T
        engine.add_fact(Relation::binary("tv_f", v_ft.clone(), p.clone()));
        engine.add_fact(Relation::binary("tv_t", v_ft.clone(), q.clone()));
        // v_ff: p=F, q=F
        engine.add_fact(Relation::binary("tv_f", v_ff.clone(), p.clone()));
        engine.add_fact(Relation::binary("tv_f", v_ff.clone(), q.clone()));

        // --- Declared formulas ---
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

        // Modus ponens: (p ∧ (p→q)) → q
        let and_p_imppq = Term::app("and", vec![p.clone(), imp_pq.clone()]);
        let mp = Term::app("imp", vec![and_p_imppq.clone(), q.clone()]);

        for f in &[
            &neg_p, &neg_q, &and_pq, &or_pq, &imp_pq, &imp_pp,
            &or_p_negp, &and_p_negp, &imp_andpq_p, &imp_p_orpq,
            &imp_p_negp, &and_p_imppq, &mp,
        ] {
            engine.add_fact(Relation::new("declared", vec![(*f).clone()]));
        }

        // --- Rules ---
        let vv = Term::var("v");
        let pv = Term::var("p");
        let qv = Term::var("q");
        let fv = Term::var("f");

        // Negation
        engine.add_rule(Rule::new("neg_t",
            vec![
                RelationPattern::new("tv_f", vec![vv.clone(), pv.clone()]),
                RelationPattern::new("declared", vec![Term::app("neg", vec![pv.clone()])]),
            ],
            vec![RelationPattern::new("tv_t", vec![vv.clone(), Term::app("neg", vec![pv.clone()])])],
        ));
        engine.add_rule(Rule::new("neg_f",
            vec![
                RelationPattern::new("tv_t", vec![vv.clone(), pv.clone()]),
                RelationPattern::new("declared", vec![Term::app("neg", vec![pv.clone()])]),
            ],
            vec![RelationPattern::new("tv_f", vec![vv.clone(), Term::app("neg", vec![pv.clone()])])],
        ));

        // Conjunction
        engine.add_rule(Rule::new("and_t",
            vec![
                RelationPattern::new("tv_t", vec![vv.clone(), pv.clone()]),
                RelationPattern::new("tv_t", vec![vv.clone(), qv.clone()]),
                RelationPattern::new("declared", vec![Term::app("and", vec![pv.clone(), qv.clone()])]),
            ],
            vec![RelationPattern::new("tv_t", vec![vv.clone(), Term::app("and", vec![pv.clone(), qv.clone()])])],
        ));
        engine.add_rule(Rule::new("and_f1",
            vec![
                RelationPattern::new("tv_f", vec![vv.clone(), pv.clone()]),
                RelationPattern::new("declared", vec![Term::app("and", vec![pv.clone(), qv.clone()])]),
            ],
            vec![RelationPattern::new("tv_f", vec![vv.clone(), Term::app("and", vec![pv.clone(), qv.clone()])])],
        ));
        engine.add_rule(Rule::new("and_f2",
            vec![
                RelationPattern::new("tv_f", vec![vv.clone(), qv.clone()]),
                RelationPattern::new("declared", vec![Term::app("and", vec![pv.clone(), qv.clone()])]),
            ],
            vec![RelationPattern::new("tv_f", vec![vv.clone(), Term::app("and", vec![pv.clone(), qv.clone()])])],
        ));

        // Disjunction
        engine.add_rule(Rule::new("or_t1",
            vec![
                RelationPattern::new("tv_t", vec![vv.clone(), pv.clone()]),
                RelationPattern::new("declared", vec![Term::app("or", vec![pv.clone(), qv.clone()])]),
            ],
            vec![RelationPattern::new("tv_t", vec![vv.clone(), Term::app("or", vec![pv.clone(), qv.clone()])])],
        ));
        engine.add_rule(Rule::new("or_t2",
            vec![
                RelationPattern::new("tv_t", vec![vv.clone(), qv.clone()]),
                RelationPattern::new("declared", vec![Term::app("or", vec![pv.clone(), qv.clone()])]),
            ],
            vec![RelationPattern::new("tv_t", vec![vv.clone(), Term::app("or", vec![pv.clone(), qv.clone()])])],
        ));
        engine.add_rule(Rule::new("or_f",
            vec![
                RelationPattern::new("tv_f", vec![vv.clone(), pv.clone()]),
                RelationPattern::new("tv_f", vec![vv.clone(), qv.clone()]),
                RelationPattern::new("declared", vec![Term::app("or", vec![pv.clone(), qv.clone()])]),
            ],
            vec![RelationPattern::new("tv_f", vec![vv.clone(), Term::app("or", vec![pv.clone(), qv.clone()])])],
        ));

        // Implication
        engine.add_rule(Rule::new("imp_t1",
            vec![
                RelationPattern::new("tv_f", vec![vv.clone(), pv.clone()]),
                RelationPattern::new("declared", vec![Term::app("imp", vec![pv.clone(), qv.clone()])]),
            ],
            vec![RelationPattern::new("tv_t", vec![vv.clone(), Term::app("imp", vec![pv.clone(), qv.clone()])])],
        ));
        engine.add_rule(Rule::new("imp_t2",
            vec![
                RelationPattern::new("tv_t", vec![vv.clone(), qv.clone()]),
                RelationPattern::new("declared", vec![Term::app("imp", vec![pv.clone(), qv.clone()])]),
            ],
            vec![RelationPattern::new("tv_t", vec![vv.clone(), Term::app("imp", vec![pv.clone(), qv.clone()])])],
        ));
        engine.add_rule(Rule::new("imp_f",
            vec![
                RelationPattern::new("tv_t", vec![vv.clone(), pv.clone()]),
                RelationPattern::new("tv_f", vec![vv.clone(), qv.clone()]),
                RelationPattern::new("declared", vec![Term::app("imp", vec![pv.clone(), qv.clone()])]),
            ],
            vec![RelationPattern::new("tv_f", vec![vv.clone(), Term::app("imp", vec![pv.clone(), qv.clone()])])],
        ));

        // Tautology: true in all 4 valuations
        engine.add_rule(Rule::new("taut",
            vec![
                RelationPattern::new("tv_t", vec![c("v_tt"), fv.clone()]),
                RelationPattern::new("tv_t", vec![c("v_tf"), fv.clone()]),
                RelationPattern::new("tv_t", vec![c("v_ft"), fv.clone()]),
                RelationPattern::new("tv_t", vec![c("v_ff"), fv.clone()]),
            ],
            vec![RelationPattern::new("tautology", vec![fv.clone()])],
        ));

        // Contradiction: false in all 4 valuations
        engine.add_rule(Rule::new("contra",
            vec![
                RelationPattern::new("tv_f", vec![c("v_tt"), fv.clone()]),
                RelationPattern::new("tv_f", vec![c("v_tf"), fv.clone()]),
                RelationPattern::new("tv_f", vec![c("v_ft"), fv.clone()]),
                RelationPattern::new("tv_f", vec![c("v_ff"), fv.clone()]),
            ],
            vec![RelationPattern::new("contradiction", vec![fv.clone()])],
        ));

        // --- Run closure ---
        let result = engine.derive_closure();
        assert!(result.saturated, "closure should saturate");

        // --- Verify tautologies ---
        let taut = |t: &Term| Relation::new("tautology", vec![t.clone()]);
        assert!(result.facts.contains(&taut(&imp_pp)),
            "p → p should be a tautology");
        assert!(result.facts.contains(&taut(&or_p_negp)),
            "p ∨ ¬p (excluded middle) should be a tautology");
        assert!(result.facts.contains(&taut(&imp_andpq_p)),
            "(p ∧ q) → p (simplification) should be a tautology");
        assert!(result.facts.contains(&taut(&imp_p_orpq)),
            "p → (p ∨ q) (addition) should be a tautology");
        assert!(result.facts.contains(&taut(&mp)),
            "(p ∧ (p→q)) → q (modus ponens) should be a tautology");

        // --- Verify contradiction ---
        let contra = |t: &Term| Relation::new("contradiction", vec![t.clone()]);
        assert!(result.facts.contains(&contra(&and_p_negp)),
            "p ∧ ¬p should be a contradiction");

        // --- Verify non-tautologies (no false positives) ---
        assert!(!result.facts.contains(&taut(&imp_pq)),
            "p → q should NOT be a tautology");
        assert!(!result.facts.contains(&taut(&and_pq)),
            "p ∧ q should NOT be a tautology");
        assert!(!result.facts.contains(&taut(&imp_p_negp)),
            "p → ¬p should NOT be a tautology");
    }
}
