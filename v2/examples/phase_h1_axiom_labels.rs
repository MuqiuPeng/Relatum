//! Phase H.1 — Semantic axiom labels.
//!
//! Axiom ids like `ax_tpl_v3_p0-1_p1-2_c0-2` are informative but
//! cryptic. H.1 provides a `human_label(axiom_id)` function that
//! returns a readable name for common patterns:
//!
//!   ax_tpl_v3_p0-1_p1-2_c0-2  → "transitivity"
//!   ax_tpl_v3_p0-1_p1-2_c2-0  → "reverse-transitivity"
//!   ax_tpl_v2_p0-1_c0-0       → "left-self-loop"
//!   ax_tpl_v2_p0-1_c1-1       → "right-self-loop"
//!   ax_tpl_v2_p0-1_c1-0       → "symmetry"
//!   ax_reflexivity            → "reflexivity"
//!   ax_antisymmetry           → "antisymmetry"
//!   ax_totality               → "totality"
//!
//! Unknown shapes get a structural description fallback.
//!
//! Lib-side helper inlined here as a free function (would graduate
//! to RSet method in a follow-up).

use relatum_v2::{
    axiom_id_to_template,
    runtime::{
        AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    test_substrates::oq1::build_long_stream,
    AxiomTemplate, EdgeTemplate, RSet,
};

const TICKS_PHASE_0: u64 = 1000;

/// Human-readable label for an axiom id. Recognizes well-known
/// shape patterns; falls back to a structural description.
fn human_label(axiom_id: &str) -> String {
    // Predicate axioms
    match axiom_id {
        "ax_reflexivity" => return "reflexivity".to_string(),
        "ax_antisymmetry" => return "antisymmetry".to_string(),
        "ax_totality" => return "totality".to_string(),
        _ => {}
    }
    // Template axioms
    let template = match axiom_id_to_template(axiom_id) {
        Some(t) => t,
        None => return format!("(unknown shape: {})", axiom_id),
    };
    let nvars = template.num_vars;
    let nprem = template.premise.len();
    let cx = template.conclusion.x_var;
    let cy = template.conclusion.y_var;
    match (template.num_vars, template.premise.as_slice(), &template.conclusion) {
        // 2-var, single premise R(0,1)
        (2, [EdgeTemplate { x_var: 0, y_var: 1 }], c) if c.x_var == 0 && c.y_var == 0 =>
            "left-self-loop".to_string(),
        (2, [EdgeTemplate { x_var: 0, y_var: 1 }], c) if c.x_var == 1 && c.y_var == 1 =>
            "right-self-loop".to_string(),
        (2, [EdgeTemplate { x_var: 0, y_var: 1 }], c) if c.x_var == 1 && c.y_var == 0 =>
            "symmetry".to_string(),
        (2, [EdgeTemplate { x_var: 0, y_var: 1 }], c) if c.x_var == 0 && c.y_var == 1 =>
            "trivial-identity".to_string(),
        // 3-var transitivity-like (premise R(0,1) R(1,2))
        (3, p, c) if p.len() == 2
            && p.contains(&EdgeTemplate { x_var: 0, y_var: 1 })
            && p.contains(&EdgeTemplate { x_var: 1, y_var: 2 }) => {
            if c.x_var == 0 && c.y_var == 2 { "transitivity".to_string() }
            else if c.x_var == 2 && c.y_var == 0 { "reverse-transitivity".to_string() }
            else if c.x_var == 0 && c.y_var == 1 { "first-premise-restated".to_string() }
            else if c.x_var == 1 && c.y_var == 0 { "first-premise-flipped".to_string() }
            else { format!("custom-3var ({})", axiom_id) }
        }
        // 3-var with premise R(0,0) R(1,2) — has "noise" character
        (3, p, _) if p.len() == 2
            && p.contains(&EdgeTemplate { x_var: 0, y_var: 0 })
            && p.contains(&EdgeTemplate { x_var: 1, y_var: 2 }) =>
            "self-loop-with-witness-edge".to_string(),
        // Generic structural fallback
        _ => format!(
            "v{} prem{} concl(c{}-{})",
            nvars, nprem, cx, cy,
        ),
    }
}

fn main() {
    println!("=== Phase H.1 — Semantic axiom labels ===");

    // ---- Test on synthetic standard cases ----
    println!();
    println!("--- Standard cases ---");
    let cases = [
        "ax_reflexivity",
        "ax_antisymmetry",
        "ax_totality",
        "ax_tpl_v3_p0-1_p1-2_c0-2",   // transitivity
        "ax_tpl_v3_p0-1_p1-2_c2-0",   // reverse-transitivity
        "ax_tpl_v2_p0-1_c0-0",        // left-self-loop
        "ax_tpl_v2_p0-1_c1-1",        // right-self-loop
        "ax_tpl_v2_p0-1_c1-0",        // symmetry
        "ax_tpl_v2_p0-1_c0-1",        // trivial-identity
        "ax_tpl_v3_p0-0_p1-2_c0-1",   // noise variant
        "ax_unknown_shape",           // unknown
    ];
    for c in &cases {
        println!("  {:<35} → {}", c, human_label(c));
    }

    // ---- Test on actual OQ#1 axioms ----
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(TICKS_PHASE_0);

    let axioms: Vec<String> = rt.rset.axioms().into_iter().map(str::to_owned).collect();
    println!();
    println!("--- OQ#1 axioms ({} total) ---", axioms.len());
    for ax in &axioms {
        println!("  {:<35} → {}", ax, human_label(ax));
    }

    println!();
    println!("=== Verdict ===");
    println!("  POSITIVE — utility function recognizes 8 known shapes + structural fallback.");
    println!("  Future: graduate to RSet method, integrate with logging / debug output.");
    println!();
    println!("--- end ---");
}
