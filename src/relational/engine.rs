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
#[derive(Debug, Clone)]
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
        self.facts.insert(fact);
    }

    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
    }

    /// Adds a universally quantified axiom. During closure, the engine
    /// enumerates all ground substitutions for the axiom's variables and
    /// emits `equiv_relation(subst(lhs), subst(rhs))` facts.
    pub fn add_axiom(&mut self, axiom: Axiom) {
        self.axioms.push(axiom);
    }

    pub fn set_max_rounds(&mut self, n: usize) {
        self.max_rounds = n;
    }
    pub fn set_max_facts(&mut self, n: usize) {
        self.max_facts = n;
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

        // Check facts
        for fact in &self.facts {
            self.validate_relation_use(fact.name(), fact.arity(), &mut errors);
            for term in fact.terms() {
                self.validate_ground_term(term, &mut errors);
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

        for _ in 0..self.max_rounds {
            rounds += 1;
            let mut new_facts: HashSet<Relation> = HashSet::new();

            // 1. Apply user-defined / explicit rules
            for rule in &self.rules {
                let matches = match_premises(rule.premises(), &self.facts);
                for sub in &matches {
                    for conclusion in rule.conclusions() {
                        if let Some(fact) = rule::instantiate(conclusion, sub) {
                            if fact.is_ground() && !self.facts.contains(&fact) {
                                new_facts.insert(fact);
                            }
                        }
                    }
                }
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
                .filter(|f| f.name() == rel_name && f.arity() == 2)
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

    for premise in premises {
        let mut next = Vec::new();
        for sub in &subs {
            for fact in facts {
                if fact.name() != premise.name() || fact.arity() != premise.terms().len() {
                    continue;
                }
                let mut candidate = sub.clone();
                if rule::match_relation(premise, fact, &mut candidate) {
                    next.push(candidate);
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
}
