//! Inference rules and pattern matching.
//!
//! A [`Rule`] consists of premise patterns and conclusion patterns. The engine
//! finds all substitutions that satisfy the premises against the current fact
//! set, then instantiates the conclusions to derive new facts.

use super::relation::Relation;
use super::term::Term;
use std::collections::HashMap;
use std::fmt;

/// A variable binding: maps pattern variable names to ground terms.
pub type Substitution = HashMap<String, Term>;

/// A pattern that matches relations. Variables in `terms` act as wildcards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationPattern {
    name: String,
    terms: Vec<Term>,
}

impl RelationPattern {
    pub fn new(name: impl Into<String>, terms: Vec<Term>) -> Self {
        RelationPattern {
            name: name.into(),
            terms,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn terms(&self) -> &[Term] {
        &self.terms
    }
}

impl fmt::Display for RelationPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(", self.name)?;
        for (i, t) in self.terms.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", t)?;
        }
        write!(f, ")")
    }
}

/// An inference rule: if all premises match, derive the conclusions.
#[derive(Debug, Clone)]
pub struct Rule {
    name: String,
    premises: Vec<RelationPattern>,
    conclusions: Vec<RelationPattern>,
    /// Variables that must resolve to ground terms for the rule to fire.
    /// Empty means no constraint (all substitutions accepted).
    ground_required: Vec<String>,
    /// Negated premises: these patterns must NOT match any fact for the rule to fire.
    negated_premises: Vec<RelationPattern>,
    /// Stratum level for stratified evaluation.
    /// Stratum 0 = positive only. Higher strata run after lower strata saturate.
    stratum: usize,
    /// Refutation: prove by contradiction. For each binding from positive premises,
    /// scan all bindings of `refutation_scan` premises. For each scan binding,
    /// hypothetically add `refutation_hypotheses` and check if `contradiction_rel`
    /// is derived. If ALL scan bindings lead to contradiction → conclusion holds.
    refutation_scan: Vec<RelationPattern>,
    refutation_hypotheses: Vec<RelationPattern>,
    contradiction_rel: Option<String>,
}

impl Rule {
    pub fn new(
        name: impl Into<String>,
        premises: Vec<RelationPattern>,
        conclusions: Vec<RelationPattern>,
    ) -> Self {
        Rule {
            name: name.into(),
            premises,
            conclusions,
            ground_required: Vec::new(),
            negated_premises: Vec::new(),
            stratum: 0,
            refutation_scan: Vec::new(),
            refutation_hypotheses: Vec::new(),
            contradiction_rel: None,
        }
    }

    /// Mark variables that must resolve to ground terms for this rule to fire.
    pub fn with_ground_required(mut self, vars: Vec<String>) -> Self {
        self.ground_required = vars;
        self
    }

    /// Add negated premises: these patterns must NOT match any fact.
    /// Automatically sets stratum to 1 if still at 0.
    pub fn with_negated(mut self, negated: Vec<RelationPattern>) -> Self {
        self.negated_premises = negated;
        if self.stratum == 0 {
            self.stratum = 1;
        }
        self
    }

    /// Set the stratum level explicitly.
    pub fn with_stratum(mut self, stratum: usize) -> Self {
        self.stratum = stratum;
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn premises(&self) -> &[RelationPattern] {
        &self.premises
    }

    pub fn conclusions(&self) -> &[RelationPattern] {
        &self.conclusions
    }

    pub fn ground_required(&self) -> &[String] {
        &self.ground_required
    }

    pub fn negated_premises(&self) -> &[RelationPattern] {
        &self.negated_premises
    }

    pub fn stratum(&self) -> usize {
        self.stratum
    }

    pub fn has_negation(&self) -> bool {
        !self.negated_premises.is_empty()
    }

    /// Declare proof by contradiction: for each binding from positive premises,
    /// scan all bindings from `scan` premises. For each, hypothetically add
    /// `hypotheses` facts and check if `contradiction` relation is derived.
    /// If ALL scan bindings lead to contradiction, the conclusion holds.
    pub fn with_refutation(
        mut self,
        scan: Vec<RelationPattern>,
        hypotheses: Vec<RelationPattern>,
        contradiction: impl Into<String>,
    ) -> Self {
        self.refutation_scan = scan;
        self.refutation_hypotheses = hypotheses;
        self.contradiction_rel = Some(contradiction.into());
        self
    }

    pub fn has_refutation(&self) -> bool {
        self.contradiction_rel.is_some()
    }

    pub fn refutation_scan(&self) -> &[RelationPattern] {
        &self.refutation_scan
    }

    pub fn refutation_hypotheses(&self) -> &[RelationPattern] {
        &self.refutation_hypotheses
    }

    pub fn contradiction_rel(&self) -> Option<&str> {
        self.contradiction_rel.as_deref()
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]", self.name)?;
        if !self.premises.is_empty() {
            for p in &self.premises {
                write!(f, "\n  {}", p)?;
            }
            write!(f, "\n  ────────")?;
        }
        for c in &self.conclusions {
            write!(f, "\n  {}", c)?;
        }
        Ok(())
    }
}

// ── Built-in rule constructors ────────────────────────────────

/// `R(x, y) ⊢ R(y, x)` for an arbitrary binary relation `R`.
pub fn symmetry_for(rel: &str) -> Rule {
    let (x, y) = (Term::var("x"), Term::var("y"));
    Rule::new(
        format!("{rel}_symmetry"),
        vec![RelationPattern::new(rel, vec![x.clone(), y.clone()])],
        vec![RelationPattern::new(rel, vec![y, x])],
    )
}

/// `R(x, y), R(y, z) ⊢ R(x, z)` for an arbitrary binary relation `R`.
pub fn transitivity_for(rel: &str) -> Rule {
    let (x, y, z) = (Term::var("x"), Term::var("y"), Term::var("z"));
    Rule::new(
        format!("{rel}_transitivity"),
        vec![
            RelationPattern::new(rel, vec![x.clone(), y.clone()]),
            RelationPattern::new(rel, vec![y, z.clone()]),
        ],
        vec![RelationPattern::new(rel, vec![x, z])],
    )
}

/// Shortcut: `equiv(x, y) ⊢ equiv(y, x)`
pub fn symmetry() -> Rule {
    symmetry_for("equiv")
}

/// Shortcut: `equiv(x, y), equiv(y, z) ⊢ equiv(x, z)`
pub fn transitivity() -> Rule {
    transitivity_for("equiv")
}

// ── Pattern matching ─────────────────────────────────────────

/// Attempt to match a pattern term against a ground term, extending `sub`.
///
/// - `Var(x)` matches any term; if `x` is already bound, the term must equal
///   the existing binding.
/// - `App { symbol, args }` must match structurally.
pub fn match_term(pattern: &Term, ground: &Term, sub: &mut Substitution) -> bool {
    match pattern {
        Term::Var(name) => {
            if let Some(bound) = sub.get(name) {
                bound == ground
            } else {
                sub.insert(name.clone(), ground.clone());
                true
            }
        }
        Term::App { symbol, args } => match ground {
            Term::App {
                symbol: gs,
                args: ga,
            } => {
                symbol == gs
                    && args.len() == ga.len()
                    && args
                        .iter()
                        .zip(ga.iter())
                        .all(|(p, g)| match_term(p, g, sub))
            }
            _ => false,
        },
    }
}

/// Match a relation pattern against a ground relation.
pub fn match_relation(pattern: &RelationPattern, fact: &Relation, sub: &mut Substitution) -> bool {
    pattern.name() == fact.name()
        && pattern.terms().len() == fact.terms().len()
        && pattern
            .terms()
            .iter()
            .zip(fact.terms())
            .all(|(p, g)| match_term(p, g, sub))
}

/// Apply a substitution to a term. Returns `None` if any variable is unbound.
pub fn substitute_term(term: &Term, sub: &Substitution) -> Option<Term> {
    match term {
        Term::Var(name) => sub.get(name).cloned(),
        Term::App { symbol, args } => {
            let new_args: Option<Vec<Term>> =
                args.iter().map(|a| substitute_term(a, sub)).collect();
            new_args.map(|a| Term::app(symbol.clone(), a))
        }
    }
}

/// Instantiate a relation pattern with a substitution.
pub fn instantiate(pattern: &RelationPattern, sub: &Substitution) -> Option<Relation> {
    let terms: Option<Vec<Term>> = pattern
        .terms()
        .iter()
        .map(|t| substitute_term(t, sub))
        .collect();
    terms.map(|ts| Relation::new(pattern.name(), ts))
}

// ── Bidirectional unification (for pattern facts) ───────────

/// Resolve a variable through the substitution chain.
fn resolve(term: &Term, sub: &Substitution, fuel: usize) -> Term {
    if fuel == 0 {
        return term.clone();
    }
    match term {
        Term::Var(name) => match sub.get(name) {
            Some(bound) if bound != term => resolve(bound, sub, fuel - 1),
            _ => term.clone(),
        },
        _ => term.clone(),
    }
}

/// Unify two terms bidirectionally, extending the substitution.
///
/// Unlike [`match_term`], both sides can contain variables. Fact variables
/// should be pre-renamed (see [`Relation::rename_vars_fresh`]) to avoid
/// collision with pattern variables.
pub fn unify_term(a: &Term, b: &Term, sub: &mut Substitution) -> bool {
    let ar = resolve(a, sub, 64);
    let br = resolve(b, sub, 64);
    match (&ar, &br) {
        (Term::Var(name), _) => {
            if ar == br {
                return true;
            }
            sub.insert(name.clone(), br);
            true
        }
        (_, Term::Var(name)) => {
            sub.insert(name.clone(), ar);
            true
        }
        (
            Term::App {
                symbol: s1,
                args: a1,
            },
            Term::App {
                symbol: s2,
                args: a2,
            },
        ) => {
            s1 == s2
                && a1.len() == a2.len()
                && a1
                    .iter()
                    .zip(a2.iter())
                    .all(|(x, y)| unify_term(x, y, sub))
        }
    }
}

/// Unify a relation pattern against a (possibly non-ground) fact.
pub fn unify_relation(
    pattern: &RelationPattern,
    fact: &Relation,
    sub: &mut Substitution,
) -> bool {
    pattern.name() == fact.name()
        && pattern.terms().len() == fact.terms().len()
        && pattern
            .terms()
            .iter()
            .zip(fact.terms())
            .all(|(p, g)| unify_term(p, g, sub))
}

/// Apply substitution partially: substitute bound variables, leave unbound
/// variables as-is. Follows variable chains transitively.
pub fn substitute_partial(term: &Term, sub: &Substitution) -> Term {
    _substitute_partial(term, sub, 64)
}

fn _substitute_partial(term: &Term, sub: &Substitution, fuel: usize) -> Term {
    if fuel == 0 {
        return term.clone();
    }
    match term {
        Term::Var(name) => match sub.get(name) {
            Some(bound) if bound != term => _substitute_partial(bound, sub, fuel - 1),
            _ => term.clone(),
        },
        Term::App { symbol, args } => Term::app(
            symbol.clone(),
            args.iter()
                .map(|a| _substitute_partial(a, sub, fuel))
                .collect(),
        ),
    }
}

/// Instantiate a relation pattern with partial substitution.
/// Unlike [`instantiate`], never fails — unbound variables remain in the result.
pub fn instantiate_partial(pattern: &RelationPattern, sub: &Substitution) -> Relation {
    let terms: Vec<Term> = pattern
        .terms()
        .iter()
        .map(|t| substitute_partial(t, sub))
        .collect();
    Relation::new(pattern.name(), terms)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(s: &str) -> Term {
        Term::constant(s)
    }
    fn v(s: &str) -> Term {
        Term::var(s)
    }

    #[test]
    fn test_match_var() {
        let mut sub = Substitution::new();
        assert!(match_term(&v("x"), &c("a"), &mut sub));
        assert_eq!(sub["x"], c("a"));
    }

    #[test]
    fn test_match_var_consistent() {
        let mut sub = Substitution::new();
        sub.insert("x".into(), c("a"));
        assert!(match_term(&v("x"), &c("a"), &mut sub));
        assert!(!match_term(&v("x"), &c("b"), &mut sub));
    }

    #[test]
    fn test_match_app() {
        let pattern = Term::app("f", vec![v("x")]);
        let ground = Term::app("f", vec![c("a")]);
        let mut sub = Substitution::new();
        assert!(match_term(&pattern, &ground, &mut sub));
        assert_eq!(sub["x"], c("a"));
    }

    #[test]
    fn test_match_app_mismatch() {
        let pattern = Term::app("f", vec![v("x")]);
        let ground = Term::app("g", vec![c("a")]);
        let mut sub = Substitution::new();
        assert!(!match_term(&pattern, &ground, &mut sub));
    }

    #[test]
    fn test_match_relation() {
        let pattern = RelationPattern::new("equiv", vec![v("x"), v("y")]);
        let fact = Relation::binary("equiv", c("a"), c("b"));
        let mut sub = Substitution::new();
        assert!(match_relation(&pattern, &fact, &mut sub));
        assert_eq!(sub["x"], c("a"));
        assert_eq!(sub["y"], c("b"));
    }

    #[test]
    fn test_substitute() {
        let mut sub = Substitution::new();
        sub.insert("x".into(), c("a"));
        sub.insert("y".into(), c("b"));

        let t = Term::app("f", vec![v("x"), v("y")]);
        let result = substitute_term(&t, &sub).unwrap();
        assert_eq!(result, Term::app("f", vec![c("a"), c("b")]));
    }

    #[test]
    fn test_substitute_unbound() {
        let sub = Substitution::new();
        let t = v("x");
        assert!(substitute_term(&t, &sub).is_none());
    }

    #[test]
    fn test_instantiate_pattern() {
        let pattern = RelationPattern::new("equiv", vec![v("x"), v("y")]);
        let mut sub = Substitution::new();
        sub.insert("x".into(), c("a"));
        sub.insert("y".into(), c("b"));

        let fact = instantiate(&pattern, &sub).unwrap();
        assert_eq!(fact, Relation::binary("equiv", c("a"), c("b")));
    }

    #[test]
    fn test_symmetry_rule_display() {
        let rule = symmetry();
        let s = rule.to_string();
        assert!(s.contains("symmetry"));
        assert!(s.contains("equiv(x, y)"));
        assert!(s.contains("equiv(y, x)"));
    }

    #[test]
    fn test_transitivity_rule() {
        let rule = transitivity();
        assert_eq!(rule.premises().len(), 2);
        assert_eq!(rule.conclusions().len(), 1);
    }

    // ── Unification tests ──────────────────────────────────

    #[test]
    fn test_unify_var_ground() {
        let mut sub = Substitution::new();
        assert!(unify_term(&v("x"), &c("a"), &mut sub));
        assert_eq!(sub["x"], c("a"));
    }

    #[test]
    fn test_unify_ground_var() {
        // fact-side variable binds to pattern structure
        let mut sub = Substitution::new();
        assert!(unify_term(&c("a"), &v("X"), &mut sub));
        assert_eq!(sub["X"], c("a"));
    }

    #[test]
    fn test_unify_var_var() {
        let mut sub = Substitution::new();
        assert!(unify_term(&v("x"), &v("Y"), &mut sub));
        assert_eq!(sub["x"], v("Y"));
    }

    #[test]
    fn test_unify_app_with_fact_var() {
        // pattern App vs fact Var — the fact var should bind
        let pattern = Term::app("f", vec![c("a")]);
        let mut sub = Substitution::new();
        assert!(unify_term(&pattern, &v("X"), &mut sub));
        assert_eq!(sub["X"], Term::app("f", vec![c("a")]));
    }

    #[test]
    fn test_unify_consistency() {
        let mut sub = Substitution::new();
        sub.insert("x".into(), c("a"));
        assert!(unify_term(&v("x"), &c("a"), &mut sub));
        assert!(!unify_term(&v("x"), &c("b"), &mut sub));
    }

    #[test]
    fn test_unify_transitive_chain() {
        let mut sub = Substitution::new();
        // x -> Y, then Y -> a
        assert!(unify_term(&v("x"), &v("Y"), &mut sub));
        assert!(unify_term(&v("Y"), &c("a"), &mut sub));
        // x should resolve to a
        let resolved = substitute_partial(&v("x"), &sub);
        assert_eq!(resolved, c("a"));
    }

    #[test]
    fn test_substitute_partial_unbound() {
        let sub = Substitution::new();
        // unbound var stays as-is
        assert_eq!(substitute_partial(&v("x"), &sub), v("x"));
    }

    #[test]
    fn test_substitute_partial_nested() {
        let mut sub = Substitution::new();
        sub.insert("x".into(), c("a"));
        // y stays unbound
        let term = Term::app("f", vec![v("x"), v("y")]);
        let result = substitute_partial(&term, &sub);
        assert_eq!(result, Term::app("f", vec![c("a"), v("y")]));
    }

    #[test]
    fn test_instantiate_partial_with_free_vars() {
        let pattern = RelationPattern::new("member", vec![v("s"), Term::app("power", vec![v("a")])]);
        let mut sub = Substitution::new();
        sub.insert("s".into(), c("empty"));
        // a is unbound — stays as variable
        let fact = instantiate_partial(&pattern, &sub);
        assert_eq!(fact.name(), "member");
        assert_eq!(fact.terms()[0], c("empty"));
        assert_eq!(fact.terms()[1], Term::app("power", vec![v("a")]));
        assert!(!fact.is_ground());
    }
}
