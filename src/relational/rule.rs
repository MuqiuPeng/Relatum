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
        }
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
}
