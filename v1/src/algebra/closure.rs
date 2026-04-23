//! Equational closure engine.
//!
//! Given an [`OpRegistry`], a set of axioms (from one or more [`Structure`]s),
//! and a set of ground facts, derives new equations by applying:
//!
//! - **Reflexivity / Symmetry / Transitivity** — handled implicitly by union-find
//! - **Congruence** — if `aᵢ = bᵢ` for all i, then `f(a₁…aₙ) = f(b₁…bₙ)`
//! - **Axiom instantiation** — substitute axiom variables with ground terms
//!
//! The expansion is bounded by a maximum number of rounds and a universe size
//! cap to prevent combinatorial explosion.
//!
//! # Example
//!
//! ```
//! use relatum::algebra::{builders, Equation, OpRegistry, Term};
//! use relatum::algebra::closure::ClosureEngine;
//!
//! let mut reg = OpRegistry::new();
//! let monoid = builders::monoid(&mut reg).unwrap();
//! let mul = reg.find_operation_id("mul").unwrap();
//! let e_op = reg.find_operation_id("e").unwrap();
//!
//! let mut engine = ClosureEngine::new(reg);
//! engine.add_structure(&monoid);
//! let a = Term::var("a");
//! engine.add_fact(Equation::new("seed", a.clone(), a.clone()));
//!
//! let result = engine.compute_closure(2);
//!
//! // Axiom instantiation derives: mul(a, e) = a, mul(e, a) = a
//! let mul_a_e = Term::app(mul, vec![a.clone(), Term::constant(e_op)]);
//! assert!(result.equivalence_classes.iter().any(|c| {
//!     c.contains(&a) && c.contains(&mul_a_e)
//! }));
//! ```

use std::collections::{HashMap, HashSet};

use super::equation::Equation;
use super::operation::OperationId;
use super::registry::OpRegistry;
use super::structure::Structure;
use super::term::Term;

/// Maximum terms in the universe before expansion stops.
const MAX_UNIVERSE: usize = 500;

/// Maximum nesting depth of terms admitted to the universe.
const MAX_DEPTH: usize = 8;

// ── Union-Find ──────────────────────────────────────────────

struct UnionFind {
    parent: HashMap<Term, Term>,
    rank: HashMap<Term, usize>,
}

impl UnionFind {
    fn new() -> Self {
        UnionFind {
            parent: HashMap::new(),
            rank: HashMap::new(),
        }
    }

    fn make_set(&mut self, t: Term) {
        self.parent.entry(t).or_insert_with_key(|k| k.clone());
    }

    fn find(&mut self, t: &Term) -> Term {
        let mut root = t.clone();
        loop {
            match self.parent.get(&root) {
                Some(p) if *p != root => root = p.clone(),
                _ => break,
            }
        }
        let mut current = t.clone();
        while current != root {
            if let Some(p) = self.parent.get(&current).cloned() {
                self.parent.insert(current, root.clone());
                current = p;
            } else {
                break;
            }
        }
        root
    }

    fn union(&mut self, a: &Term, b: &Term) -> bool {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra == rb {
            return false;
        }
        let rank_a = self.rank.get(&ra).copied().unwrap_or(0);
        let rank_b = self.rank.get(&rb).copied().unwrap_or(0);
        if rank_a < rank_b {
            self.parent.insert(ra, rb);
        } else {
            self.parent.insert(rb, ra.clone());
            if rank_a == rank_b {
                *self.rank.entry(ra).or_insert(0) += 1;
            }
        }
        true
    }
}

// ── Helpers ─────────────────────────────────────────────────

fn collect_subterms(term: &Term, out: &mut Vec<Term>) {
    out.push(term.clone());
    if let Term::App { args, .. } = term {
        for arg in args {
            collect_subterms(arg, out);
        }
    }
}

fn substitute(term: &Term, sub: &HashMap<String, Term>) -> Term {
    match term {
        Term::Var(name) => sub.get(name).cloned().unwrap_or_else(|| term.clone()),
        Term::App { op, args } => Term::App {
            op: *op,
            args: args.iter().map(|a| substitute(a, sub)).collect(),
        },
    }
}

fn axiom_variables(eq: &Equation) -> Vec<String> {
    let mut vars: HashSet<String> = HashSet::new();
    collect_var_names(eq.lhs(), &mut vars);
    collect_var_names(eq.rhs(), &mut vars);
    let mut v: Vec<String> = vars.into_iter().collect();
    v.sort();
    v
}

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

fn enumerate_substitutions(vars: &[String], terms: &[Term]) -> Vec<HashMap<String, Term>> {
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

fn detect_expanding(eq: &Equation) -> bool {
    let ld = eq.lhs().depth();
    let rd = eq.rhs().depth();
    if ld == rd {
        return false;
    }
    let (pattern, host) = if ld < rd {
        (eq.lhs(), eq.rhs())
    } else {
        (eq.rhs(), eq.lhs())
    };
    if pattern.is_var() {
        return false;
    }
    proper_subterms(host)
        .iter()
        .any(|sub| pattern_matches(pattern, sub))
}

fn proper_subterms(term: &Term) -> Vec<&Term> {
    let mut out = Vec::new();
    if let Term::App { args, .. } = term {
        for arg in args {
            collect_all_subterms(arg, &mut out);
        }
    }
    out
}

fn collect_all_subterms<'a>(term: &'a Term, out: &mut Vec<&'a Term>) {
    out.push(term);
    if let Term::App { args, .. } = term {
        for arg in args {
            collect_all_subterms(arg, out);
        }
    }
}

fn pattern_matches(pattern: &Term, term: &Term) -> bool {
    let mut bindings: HashMap<&str, &Term> = HashMap::new();
    pattern_matches_inner(pattern, term, &mut bindings)
}

fn pattern_matches_inner<'a>(
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
            op: pop,
            args: pargs,
        } => {
            if let Term::App {
                op: top,
                args: targs,
            } = term
            {
                if pop != top || pargs.len() != targs.len() {
                    return false;
                }
                pargs
                    .iter()
                    .zip(targs.iter())
                    .all(|(p, t)| pattern_matches_inner(p, t, bindings))
            } else {
                false
            }
        }
    }
}

fn extract_classes(uf: &mut UnionFind, universe: &HashSet<Term>) -> Vec<Vec<Term>> {
    let mut groups: HashMap<Term, Vec<Term>> = HashMap::new();
    for term in universe {
        let root = uf.find(term);
        groups.entry(root).or_default().push(term.clone());
    }
    let mut classes: Vec<Vec<Term>> = groups
        .into_values()
        .filter(|class| class.len() > 1)
        .collect();
    for class in &mut classes {
        class.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    }
    classes.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    classes
}

// ── Public API ──────────────────────────────────────────────

/// A group of derived equations sharing the same derivation rule.
#[derive(Debug, Clone)]
pub struct DerivedCategory {
    /// Short identifier for the rule (e.g. `"congruence"`, `"mul_associativity"`).
    pub rule: String,
    /// Human-readable description of why these equations hold.
    pub description: String,
    /// Equations derived by this rule.
    pub equations: Vec<Equation>,
}

/// Result of a closure computation.
#[derive(Debug, Clone)]
pub struct ClosureResult {
    /// All derived equations, flat (union of all categories).
    pub derived_equations: Vec<Equation>,
    /// Derived equations grouped by derivation rule.
    pub categories: Vec<DerivedCategory>,
    /// Equivalence classes of terms (only classes with more than one member).
    pub equivalence_classes: Vec<Vec<Term>>,
    /// Number of expansion rounds used.
    pub steps_used: usize,
    /// Warnings about recursive / expanding axioms or depth capping.
    pub warnings: Vec<String>,
}

/// Engine for computing equational closure over ground facts.
///
/// # Algorithm
///
/// 1. Seed the term universe with all subterms from the user-provided facts.
/// 2. Merge each fact pair in the union-find.
/// 3. In each round (up to `max_rounds`):
///    - **Congruence**: for every pair of `f(a₁…aₙ)` and `f(b₁…bₙ)` in the
///      universe with `aᵢ ≡ bᵢ`, merge them.
///    - **Axiom instantiation**: for each axiom (from all added structures),
///      substitute its variables with every combination of ground terms from
///      the universe, then merge the resulting lhs ≡ rhs. New subterms are
///      added to the universe (up to [`MAX_UNIVERSE`]).
/// 4. Stop when a round produces no new merges, or `max_rounds` is reached.
/// 5. Extract equivalence classes from the union-find.
pub struct ClosureEngine {
    registry: OpRegistry,
    axioms: Vec<Equation>,
    facts: Vec<Equation>,
}

impl ClosureEngine {
    pub fn new(registry: OpRegistry) -> Self {
        ClosureEngine {
            registry,
            axioms: Vec::new(),
            facts: Vec::new(),
        }
    }

    /// Adds all axioms from a structure.
    pub fn add_structure(&mut self, structure: &Structure) {
        self.axioms.extend(structure.equations().iter().cloned());
    }

    /// Adds a single axiom directly.
    pub fn add_axiom(&mut self, eq: Equation) {
        self.axioms.push(eq);
    }

    /// Adds a ground fact (equation between concrete terms).
    pub fn add_fact(&mut self, eq: Equation) {
        self.facts.push(eq);
    }

    /// Returns a reference to the underlying registry.
    pub fn registry(&self) -> &OpRegistry {
        &self.registry
    }

    pub fn compute_closure(&self, max_rounds: usize) -> ClosureResult {
        let mut uf = UnionFind::new();
        let mut universe: HashSet<Term> = HashSet::new();
        let mut derived: Vec<Equation> = Vec::new();
        let mut derive_count: usize = 0;
        let mut warnings: Vec<String> = Vec::new();

        let mut cat_eqs: HashMap<String, Vec<Equation>> = HashMap::new();

        let axiom_cat: HashMap<&str, &str> = self
            .axioms
            .iter()
            .map(|ax| {
                let cat = ax.category();
                (ax.name(), if cat.is_empty() { ax.name() } else { cat })
            })
            .collect();

        // 0. Static analysis: detect expanding axioms
        let mut expanding: HashSet<&str> = HashSet::new();
        for axiom in &self.axioms {
            if detect_expanding(axiom) {
                expanding.insert(axiom.name());
                warnings.push(format!(
                    "Axiom \"{}\" is recursive (one side embeds in the other); \
                     instantiation depth is capped at {}",
                    axiom.name(),
                    MAX_DEPTH,
                ));
            }
        }

        // 1. Seed universe from facts
        for fact in &self.facts {
            add_term(&mut universe, &mut uf, fact.lhs());
            add_term(&mut universe, &mut uf, fact.rhs());
        }

        // 2. Merge initial facts
        for fact in &self.facts {
            uf.union(fact.lhs(), fact.rhs());
        }

        // 3. Iterative expansion
        let mut steps_used = 0;
        let mut depth_grew_rounds = 0usize;
        let mut prev_max_depth = universe.iter().map(|t| t.depth()).max().unwrap_or(0);

        for _ in 0..max_rounds {
            steps_used += 1;
            let mut changed = false;

            // 3a. Congruence closure
            let merges = apply_congruence(&mut uf, &universe);
            for (a, b) in merges {
                derive_count += 1;
                let eq = Equation::new(format!("congruence_{derive_count}"), a, b);
                cat_eqs
                    .entry("congruence".to_string())
                    .or_default()
                    .push(eq.clone());
                derived.push(eq);
                changed = true;
            }

            // 3b. Axiom instantiation
            let terms: Vec<Term> = universe.iter().cloned().collect();
            let mut depth_capped_this_round = false;
            for axiom in &self.axioms {
                let vars = axiom_variables(axiom);
                let axiom_name = axiom.name();
                let is_expanding = expanding.contains(axiom_name);
                let cat_key = axiom_cat
                    .get(axiom_name)
                    .copied()
                    .unwrap_or(axiom_name)
                    .to_string();

                let pool: Vec<&Term> = if is_expanding {
                    terms.iter().filter(|t| t.depth() < MAX_DEPTH - 1).collect()
                } else {
                    terms.iter().collect()
                };
                let pool_terms: Vec<Term> = pool.into_iter().cloned().collect();

                for sub in enumerate_substitutions(&vars, &pool_terms) {
                    let lhs = substitute(axiom.lhs(), &sub);
                    let rhs = substitute(axiom.rhs(), &sub);

                    if lhs.depth() > MAX_DEPTH || rhs.depth() > MAX_DEPTH {
                        depth_capped_this_round = true;
                        continue;
                    }

                    add_term(&mut universe, &mut uf, &lhs);
                    add_term(&mut universe, &mut uf, &rhs);

                    if uf.union(&lhs, &rhs) {
                        derive_count += 1;
                        let eq =
                            Equation::new(format!("{}_{derive_count}", axiom_name), lhs, rhs);
                        cat_eqs
                            .entry(cat_key.clone())
                            .or_default()
                            .push(eq.clone());
                        derived.push(eq);
                        changed = true;
                    }
                }
            }

            // 3c. Depth-growth detection
            let cur_max_depth = universe.iter().map(|t| t.depth()).max().unwrap_or(0);
            if cur_max_depth > prev_max_depth {
                depth_grew_rounds += 1;
            } else {
                depth_grew_rounds = 0;
            }
            prev_max_depth = cur_max_depth;

            if depth_grew_rounds >= 3 && depth_capped_this_round {
                warnings.push(format!(
                    "Recursive expansion detected: term depth grew for {} consecutive rounds \
                     (max depth {}); halting early",
                    depth_grew_rounds, cur_max_depth,
                ));
                break;
            }

            if !changed {
                break;
            }
        }

        // 4. Build categories
        let categories = self.build_categories(cat_eqs);

        // 5. Extract equivalence classes
        let classes = extract_classes(&mut uf, &universe);

        ClosureResult {
            derived_equations: derived,
            categories,
            equivalence_classes: classes,
            steps_used,
            warnings,
        }
    }
}

impl ClosureEngine {
    fn build_categories(&self, cat_eqs: HashMap<String, Vec<Equation>>) -> Vec<DerivedCategory> {
        let mut cats: Vec<DerivedCategory> = Vec::new();

        let mut cat_axioms: HashMap<&str, Vec<String>> = HashMap::new();
        for ax in &self.axioms {
            let cat = ax.category();
            let key = if cat.is_empty() { ax.name() } else { cat };
            cat_axioms
                .entry(key)
                .or_default()
                .push(self.registry.format_equation(ax));
        }

        let known_desc: HashMap<&str, &str> = HashMap::from([
            (
                "congruence",
                "Congruence closure: if ai = bi for all i, then f(a1..an) = f(b1..bn)",
            ),
            ("associativity", "Associativity: (x * y) * z = x * (y * z)"),
            ("identity", "Identity: e * x = x, x * e = x"),
            ("inverse", "Inverse: x^{-1} * x = e, x * x^{-1} = e"),
            (
                "additive_group",
                "Additive abelian group: associativity, commutativity, identity, and inverse for addition",
            ),
            (
                "multiplicative_monoid",
                "Multiplicative monoid: associativity and identity for multiplication",
            ),
            (
                "distributivity",
                "Distributivity: multiplication distributes over addition",
            ),
        ]);

        for (rule, equations) in cat_eqs {
            let description = if let Some(&desc) = known_desc.get(rule.as_str()) {
                let mut s = desc.to_string();
                if let Some(axiom_strs) = cat_axioms.get(rule.as_str()) {
                    s.push_str(" — ");
                    s.push_str(&axiom_strs.join(", "));
                }
                s
            } else if let Some(axiom_strs) = cat_axioms.get(rule.as_str()) {
                format!("Instantiation of {}: {}", rule, axiom_strs.join(", "))
            } else {
                format!("Derived from rule: {}", rule)
            };

            cats.push(DerivedCategory {
                rule,
                description,
                equations,
            });
        }

        cats.sort_by(|a, b| {
            let key = |c: &DerivedCategory| {
                if c.rule == "congruence" {
                    (0, c.rule.clone())
                } else {
                    (1, c.rule.clone())
                }
            };
            key(a).cmp(&key(b))
        });

        cats
    }
}

fn add_term(universe: &mut HashSet<Term>, uf: &mut UnionFind, term: &Term) {
    if universe.len() >= MAX_UNIVERSE {
        return;
    }
    let mut subs = Vec::new();
    collect_subterms(term, &mut subs);
    for t in subs {
        if universe.len() >= MAX_UNIVERSE {
            break;
        }
        if universe.insert(t.clone()) {
            uf.make_set(t);
        }
    }
}

fn apply_congruence(uf: &mut UnionFind, universe: &HashSet<Term>) -> Vec<(Term, Term)> {
    let mut merges = Vec::new();

    let mut by_op: HashMap<OperationId, Vec<&Term>> = HashMap::new();
    for term in universe {
        if let Term::App { op, .. } = term {
            by_op.entry(*op).or_default().push(term);
        }
    }

    for terms in by_op.values() {
        for i in 0..terms.len() {
            for j in (i + 1)..terms.len() {
                if let (Term::App { args: ai, .. }, Term::App { args: aj, .. }) =
                    (terms[i], terms[j])
                {
                    if ai.len() != aj.len() {
                        continue;
                    }
                    let all_equiv = ai
                        .iter()
                        .zip(aj.iter())
                        .all(|(a, b)| uf.find(a) == uf.find(b));
                    if all_equiv && uf.union(terms[i], terms[j]) {
                        merges.push((terms[i].clone(), terms[j].clone()));
                    }
                }
            }
        }
    }

    merges
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::builders;

    // ── basic inference rules ───────────────────────────────

    #[test]
    fn test_reflexivity_no_nontrivial_class() {
        let mut engine = ClosureEngine::new(OpRegistry::new());
        engine.add_fact(Equation::new("f1", Term::var("a"), Term::var("a")));
        let result = engine.compute_closure(5);
        assert!(result.equivalence_classes.is_empty());
    }

    #[test]
    fn test_symmetry() {
        let mut engine = ClosureEngine::new(OpRegistry::new());
        engine.add_fact(Equation::new("f1", Term::var("a"), Term::var("b")));
        let result = engine.compute_closure(5);
        assert!(result
            .equivalence_classes
            .iter()
            .any(|c| { c.contains(&Term::var("a")) && c.contains(&Term::var("b")) }));
    }

    #[test]
    fn test_transitivity() {
        let mut engine = ClosureEngine::new(OpRegistry::new());
        engine.add_fact(Equation::new("f1", Term::var("a"), Term::var("b")));
        engine.add_fact(Equation::new("f2", Term::var("b"), Term::var("c")));
        let result = engine.compute_closure(5);
        assert!(result.equivalence_classes.iter().any(|c| {
            c.contains(&Term::var("a"))
                && c.contains(&Term::var("b"))
                && c.contains(&Term::var("c"))
        }));
    }

    #[test]
    fn test_congruence() {
        let mut reg = OpRegistry::new();
        let f = reg.declare_operation("f", 2).unwrap();

        let (a, b, c) = (Term::var("a"), Term::var("b"), Term::var("c"));
        let fa = Term::app(f, vec![a.clone(), c.clone()]);
        let fb = Term::app(f, vec![b.clone(), c.clone()]);

        let mut engine = ClosureEngine::new(reg);
        engine.add_fact(Equation::new("eq_ab", a.clone(), b.clone()));
        engine.add_fact(Equation::new("fa_is_p", fa.clone(), Term::var("p")));
        engine.add_fact(Equation::new("fb_is_q", fb.clone(), Term::var("q")));

        let result = engine.compute_closure(5);

        assert!(result
            .equivalence_classes
            .iter()
            .any(|c| { c.contains(&Term::var("p")) && c.contains(&Term::var("q")) }));
    }

    // ── axiom instantiation ─────────────────────────────────

    #[test]
    fn test_monoid_identity_instantiation() {
        let mut reg = OpRegistry::new();
        let monoid = builders::monoid(&mut reg).unwrap();
        let mul = reg.find_operation_id("mul").unwrap();
        let e_op = reg.find_operation_id("e").unwrap();

        let a = Term::var("a");
        let mul_a_e = Term::app(mul, vec![a.clone(), Term::constant(e_op)]);
        let mul_e_a = Term::app(mul, vec![Term::constant(e_op), a.clone()]);

        let mut engine = ClosureEngine::new(reg);
        engine.add_structure(&monoid);
        engine.add_fact(Equation::new("seed", a.clone(), a.clone()));

        let result = engine.compute_closure(2);

        assert!(result
            .equivalence_classes
            .iter()
            .any(|c| { c.contains(&a) && c.contains(&mul_a_e) && c.contains(&mul_e_a) }));
    }

    #[test]
    fn test_monoid_associativity_instantiation() {
        let mut reg = OpRegistry::new();
        let monoid = builders::monoid(&mut reg).unwrap();
        let mul = reg.find_operation_id("mul").unwrap();

        let (a, b) = (Term::var("a"), Term::var("b"));

        let mut engine = ClosureEngine::new(reg);
        engine.add_structure(&monoid);
        engine.add_fact(Equation::new("seed_a", a.clone(), a.clone()));
        engine.add_fact(Equation::new("seed_b", b.clone(), b.clone()));

        let result = engine.compute_closure(2);

        let lhs = Term::app(
            mul,
            vec![Term::app(mul, vec![a.clone(), a.clone()]), b.clone()],
        );
        let rhs = Term::app(
            mul,
            vec![a.clone(), Term::app(mul, vec![a.clone(), b.clone()])],
        );
        assert!(result
            .equivalence_classes
            .iter()
            .any(|c| { c.contains(&lhs) && c.contains(&rhs) }));
    }

    #[test]
    fn test_empty_facts_no_expansion() {
        let mut reg = OpRegistry::new();
        let monoid = builders::monoid(&mut reg).unwrap();
        let mut engine = ClosureEngine::new(reg);
        engine.add_structure(&monoid);
        let result = engine.compute_closure(10);
        assert!(result.derived_equations.is_empty());
        assert!(result.equivalence_classes.is_empty());
        assert_eq!(result.steps_used, 1);
    }

    #[test]
    fn test_fixed_point_terminates() {
        let mut engine = ClosureEngine::new(OpRegistry::new());
        engine.add_fact(Equation::new("f1", Term::var("a"), Term::var("b")));
        let result = engine.compute_closure(100);
        assert_eq!(result.steps_used, 1);
    }

    #[test]
    fn test_derived_equations_populated() {
        let mut reg = OpRegistry::new();
        let monoid = builders::monoid(&mut reg).unwrap();
        let mut engine = ClosureEngine::new(reg);
        engine.add_structure(&monoid);
        engine.add_fact(Equation::new("seed", Term::var("a"), Term::var("a")));

        let result = engine.compute_closure(2);
        assert!(!result.derived_equations.is_empty());
    }

    // ── categorization ────────────────────────────────────────

    #[test]
    fn test_categories_congruence() {
        let mut reg = OpRegistry::new();
        let f = reg.declare_operation("f", 1).unwrap();

        let (a, b) = (Term::var("a"), Term::var("b"));
        let mut engine = ClosureEngine::new(reg);
        engine.add_fact(Equation::new("eq_ab", a.clone(), b.clone()));
        engine.add_fact(Equation::new("fa", Term::app(f, vec![a]), Term::var("p")));
        engine.add_fact(Equation::new("fb", Term::app(f, vec![b]), Term::var("q")));

        let result = engine.compute_closure(5);

        let cong = result.categories.iter().find(|c| c.rule == "congruence");
        assert!(cong.is_some(), "should have a congruence category");
        let cong = cong.unwrap();
        assert!(!cong.equations.is_empty());
        assert!(cong.description.contains("Congruence"));
    }

    #[test]
    fn test_categories_axiom_instantiation() {
        let mut reg = OpRegistry::new();
        let monoid = builders::monoid(&mut reg).unwrap();
        let mut engine = ClosureEngine::new(reg);
        engine.add_structure(&monoid);
        engine.add_fact(Equation::new("seed", Term::var("a"), Term::var("a")));

        let result = engine.compute_closure(2);

        let identity_cat = result.categories.iter().find(|c| c.rule == "identity");
        assert!(identity_cat.is_some(), "should have an identity category");

        let cat = identity_cat.unwrap();
        assert!(
            cat.description.contains("Identity"),
            "description should mention Identity: {}",
            cat.description
        );
        assert!(!cat.equations.is_empty());

        let assoc_cat = result
            .categories
            .iter()
            .find(|c| c.rule == "associativity");
        assert!(assoc_cat.is_some(), "should have an associativity category");
    }

    #[test]
    fn test_categories_flat_equals_sum() {
        let mut reg = OpRegistry::new();
        let monoid = builders::monoid(&mut reg).unwrap();
        let mut engine = ClosureEngine::new(reg);
        engine.add_structure(&monoid);
        engine.add_fact(Equation::new("seed", Term::var("a"), Term::var("a")));

        let result = engine.compute_closure(2);

        let cat_total: usize = result.categories.iter().map(|c| c.equations.len()).sum();
        assert_eq!(
            cat_total,
            result.derived_equations.len(),
            "categories should partition all derived equations"
        );
    }

    #[test]
    fn test_categories_empty_when_no_derivation() {
        let engine = ClosureEngine::new(OpRegistry::new());
        let result = engine.compute_closure(5);
        assert!(result.categories.is_empty());
    }

    #[test]
    fn test_categories_sorted_congruence_first() {
        let mut reg = OpRegistry::new();
        let f = reg.declare_operation("f", 1).unwrap();

        let mut s = Structure::new("S");
        let x = Term::var("x");
        s.add_equation(Equation::new("id", Term::app(f, vec![x.clone()]), x));

        let (a, b) = (Term::var("a"), Term::var("b"));
        let mut engine = ClosureEngine::new(reg);
        engine.add_structure(&s);
        engine.add_fact(Equation::new("eq", a.clone(), b.clone()));
        engine.add_fact(Equation::new("fa", Term::app(f, vec![a]), Term::var("p")));
        engine.add_fact(Equation::new("fb", Term::app(f, vec![b]), Term::var("q")));

        let result = engine.compute_closure(5);

        if result.categories.len() >= 2 {
            assert_eq!(
                result.categories[0].rule, "congruence",
                "congruence category should come first"
            );
        }
    }

    // ── recursive expansion detection ─────────────────────────

    #[test]
    fn test_expanding_axiom_detected() {
        let mut reg = OpRegistry::new();
        let f = reg.declare_operation("f", 1).unwrap();
        let x = Term::var("x");

        let mut engine = ClosureEngine::new(reg);
        engine.add_axiom(Equation::new(
            "wrap",
            Term::app(f, vec![x.clone()]),
            Term::app(f, vec![Term::app(f, vec![x])]),
        ));
        engine.add_fact(Equation::new("seed", Term::var("a"), Term::var("a")));

        let result = engine.compute_closure(20);

        assert!(
            result.warnings.iter().any(|w| w.contains("recursive")),
            "should warn about recursive axiom, got: {:?}",
            result.warnings,
        );
        assert!(
            result
                .equivalence_classes
                .iter()
                .all(|c| c.iter().all(|t| t.depth() <= MAX_DEPTH)),
            "no term should exceed MAX_DEPTH",
        );
    }

    #[test]
    fn test_non_expanding_axiom_no_warning() {
        let mut reg = OpRegistry::new();
        let monoid = builders::monoid(&mut reg).unwrap();
        let mut engine = ClosureEngine::new(reg);
        engine.add_structure(&monoid);
        engine.add_fact(Equation::new("seed", Term::var("a"), Term::var("a")));

        let result = engine.compute_closure(2);

        assert!(
            result.warnings.is_empty(),
            "monoid should produce no recursive warnings, got: {:?}",
            result.warnings,
        );
    }

    #[test]
    fn test_depth_capped_terms_excluded() {
        let mut reg = OpRegistry::new();
        let g = reg.declare_operation("g", 1).unwrap();
        let x = Term::var("x");

        let mut engine = ClosureEngine::new(reg);
        engine.add_axiom(Equation::new(
            "nest",
            Term::app(g, vec![x.clone()]),
            Term::app(g, vec![Term::app(g, vec![x])]),
        ));
        engine.add_fact(Equation::new("seed", Term::var("a"), Term::var("a")));

        let result = engine.compute_closure(50);

        for class in &result.equivalence_classes {
            for term in class {
                assert!(
                    term.depth() <= MAX_DEPTH,
                    "term {:?} has depth {} > MAX_DEPTH {}",
                    term,
                    term.depth(),
                    MAX_DEPTH,
                );
            }
        }
    }

    // ── multi-structure interaction ──────────────────────────

    #[test]
    fn test_multi_structure_shared_operations() {
        let mut reg = OpRegistry::new();
        let grp = builders::group(&mut reg).unwrap();
        let rng = builders::ring(&mut reg).unwrap();

        let mut engine = ClosureEngine::new(reg);
        engine.add_structure(&grp);
        engine.add_structure(&rng);

        // Both group and ring axioms are active
        assert!(engine.axioms.len() > grp.equations().len());
    }
}
