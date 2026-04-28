//! Axiom-id encoder/decoder primitives. Each axiom template family
//! has a deterministic string id derived from its canonical form, and
//! the inverse parse function. ADR 0030 (basic templates), ADR 0047
//! (equality + disjunctive families). The id is stable across runs and
//! RSets — it depends only on the canonicalized template structure.

use crate::canonicalize_template;
use crate::{
    AxiomTemplate, DisjunctiveAxiomTemplate, EdgeTemplate, EqualityAxiomTemplate,
};

/// ADR 0030: compute the deterministic axiom id for a template.
/// Canonical form `[R(0,1), R(1,2)] ⇒ R(0,2)` (transitivity) becomes
/// `ax_tpl_v3_p0-1_p1-2_c0-2`. Stable across runs and RSets.
pub fn axiom_template_id(template: &AxiomTemplate) -> String {
    let canon = canonicalize_template(template.clone());
    let mut parts: Vec<String> = Vec::new();
    parts.push(format!("ax_tpl_v{}", canon.num_vars));
    for e in &canon.premise {
        parts.push(format!("p{}-{}", e.x_var, e.y_var));
    }
    parts.push(format!("c{}-{}", canon.conclusion.x_var, canon.conclusion.y_var));
    parts.join("_")
}

/// ADR 0030: parse a template axiom id back into a template. Returns
/// `None` if the id is a predicate axiom (reflexivity / antisymmetry)
/// or otherwise not a template form.
pub fn axiom_id_to_template(id: &str) -> Option<AxiomTemplate> {
    let rest = id.strip_prefix("ax_tpl_v")?;
    let mut parts = rest.split('_');
    let num_vars: usize = parts.next()?.parse().ok()?;
    let mut premise: Vec<EdgeTemplate> = Vec::new();
    let mut conclusion: Option<EdgeTemplate> = None;
    for p in parts {
        if let Some(body) = p.strip_prefix('p') {
            let (x, y) = split_edge_part(body)?;
            premise.push(EdgeTemplate { x_var: x, y_var: y });
        } else if let Some(body) = p.strip_prefix('c') {
            let (x, y) = split_edge_part(body)?;
            conclusion = Some(EdgeTemplate { x_var: x, y_var: y });
        } else {
            return None;
        }
    }
    Some(AxiomTemplate {
        num_vars,
        premise,
        conclusion: conclusion?,
    })
}

fn split_edge_part(s: &str) -> Option<(usize, usize)> {
    let mut it = s.split('-');
    let x: usize = it.next()?.parse().ok()?;
    let y: usize = it.next()?.parse().ok()?;
    if it.next().is_some() {
        return None;
    }
    Some((x, y))
}

/// ADR 0047: deterministic id for an equality-conclusion template.
/// Format: `ax_eq_v{n}_p{x}-{y}_..._eq{a}-{b}`.
pub fn equality_axiom_id(template: &EqualityAxiomTemplate) -> String {
    let mut parts: Vec<String> = Vec::new();
    parts.push(format!("ax_eq_v{}", template.num_vars));
    let mut premise_sorted = template.premise.clone();
    premise_sorted.sort_by(|a, b| (a.x_var, a.y_var).cmp(&(b.x_var, b.y_var)));
    for e in &premise_sorted {
        parts.push(format!("p{}-{}", e.x_var, e.y_var));
    }
    // Equality is order-independent: normalize (a, b) to (min, max).
    let (a, b) = (
        template.equal_vars.0.min(template.equal_vars.1),
        template.equal_vars.0.max(template.equal_vars.1),
    );
    parts.push(format!("eq{}-{}", a, b));
    parts.join("_")
}

/// ADR 0047: deterministic id for a disjunctive-conclusion template.
/// Format: `ax_disj_v{n}_p{x}-{y}_..._d{cx}-{cy}_...`.
pub fn disjunctive_axiom_id(template: &DisjunctiveAxiomTemplate) -> String {
    let mut parts: Vec<String> = Vec::new();
    parts.push(format!("ax_disj_v{}", template.num_vars));
    let mut premise_sorted = template.premise.clone();
    premise_sorted.sort_by(|a, b| (a.x_var, a.y_var).cmp(&(b.x_var, b.y_var)));
    for e in &premise_sorted {
        parts.push(format!("p{}-{}", e.x_var, e.y_var));
    }
    let mut concls_sorted = template.conclusions.clone();
    concls_sorted.sort_by(|a, b| (a.x_var, a.y_var).cmp(&(b.x_var, b.y_var)));
    for c in &concls_sorted {
        parts.push(format!("d{}-{}", c.x_var, c.y_var));
    }
    parts.join("_")
}

/// ADR 0047: parse an equality axiom id. Returns None if not the
/// equality form.
pub fn equality_id_to_template(id: &str) -> Option<EqualityAxiomTemplate> {
    let rest = id.strip_prefix("ax_eq_v")?;
    let mut parts = rest.split('_');
    let num_vars: usize = parts.next()?.parse().ok()?;
    let mut premise: Vec<EdgeTemplate> = Vec::new();
    let mut equal_vars: Option<(usize, usize)> = None;
    for p in parts {
        if let Some(body) = p.strip_prefix('p') {
            let (x, y) = split_edge_part(body)?;
            premise.push(EdgeTemplate { x_var: x, y_var: y });
        } else if let Some(body) = p.strip_prefix("eq") {
            equal_vars = Some(split_edge_part(body)?);
        } else {
            return None;
        }
    }
    Some(EqualityAxiomTemplate {
        num_vars,
        premise,
        equal_vars: equal_vars?,
    })
}

/// ADR 0047: parse a disjunctive axiom id. Returns None otherwise.
pub fn disjunctive_id_to_template(id: &str) -> Option<DisjunctiveAxiomTemplate> {
    let rest = id.strip_prefix("ax_disj_v")?;
    let mut parts = rest.split('_');
    let num_vars: usize = parts.next()?.parse().ok()?;
    let mut premise: Vec<EdgeTemplate> = Vec::new();
    let mut conclusions: Vec<EdgeTemplate> = Vec::new();
    for p in parts {
        if let Some(body) = p.strip_prefix('p') {
            let (x, y) = split_edge_part(body)?;
            premise.push(EdgeTemplate { x_var: x, y_var: y });
        } else if let Some(body) = p.strip_prefix('d') {
            let (x, y) = split_edge_part(body)?;
            conclusions.push(EdgeTemplate { x_var: x, y_var: y });
        } else {
            return None;
        }
    }
    if conclusions.is_empty() {
        return None;
    }
    Some(DisjunctiveAxiomTemplate {
        num_vars,
        premise,
        conclusions,
    })
}
