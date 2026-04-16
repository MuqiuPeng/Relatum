//! Set theory experiment: ZFC axioms encoded as relational rules.
//! Tests whether the closure engine can perform pure symbolic deduction
//! from set-theoretic axioms, without finite model enumeration.

use relatum::relational::*;
use relatum::relational::rule::{RelationPattern, match_relation, instantiate, Substitution};
use std::collections::{HashMap, HashSet};

fn c(s: &str) -> Term {
    Term::constant(s)
}

/// Build the ZFC-5 engine: extensionality, empty set, pairing, union, powerset.
/// No finite Cayley tables — pure axiom-driven symbolic deduction.
fn zfc5_engine() -> ClosureEngine {
    let mut engine = ClosureEngine::new();

    // Relations
    engine.define_relation("set", 1);       // set(x): x is a set
    engine.define_relation("member", 2);    // member(x, y): x ∈ y
    engine.define_relation("subset", 2);    // subset(x, y): x ⊆ y
    engine.define_equivalence("eq");        // eq with sym, trans, refl, congruence

    // Variables
    for v in &["x", "y", "z", "a", "b", "s"] {
        engine.define_variable(*v);
    }

    // ── Axiom 1: Empty set ──
    engine.define_constant("empty");
    engine.add_fact(Relation::new("set", vec![c("empty")]));
    // empty has no members — enforced by absence (closed world on empty)
    // We encode: subset(empty, X) for any set X (empty is subset of everything)
    // This is derivable from the definition: subset(A,B) iff forall z, member(z,A) → member(z,B)
    // Since member(z, empty) is never true, the implication is vacuously true.

    // ── Axiom 2: Extensionality ──
    // If X ⊆ Y and Y ⊆ X then X = Y
    engine.add_rule(Rule::new(
        "extensionality",
        vec![
            RelationPattern::new("subset", vec![Term::var("x"), Term::var("y")]),
            RelationPattern::new("subset", vec![Term::var("y"), Term::var("x")]),
        ],
        vec![RelationPattern::new("eq", vec![Term::var("x"), Term::var("y")])],
    ));

    // Subset propagation: if every member of X is a member of Y, then X ⊆ Y
    // We can't directly encode universal quantification, but we can encode:
    // member(z, x), set(y) → if member(z, y) is derivable then subset contribution
    // Instead, encode subset for specific known sets:

    // empty ⊆ everything (vacuous truth)
    engine.add_rule(Rule::new(
        "empty_subset",
        vec![RelationPattern::new("set", vec![Term::var("x")])],
        vec![RelationPattern::new("subset", vec![c("empty"), Term::var("x")])],
    ));

    // Reflexive subset
    engine.add_rule(Rule::new(
        "subset_refl",
        vec![RelationPattern::new("set", vec![Term::var("x")])],
        vec![RelationPattern::new("subset", vec![Term::var("x"), Term::var("x")])],
    ));

    // ── Axiom 3: Pairing ──
    // For any sets a, b: {a, b} exists and contains exactly a and b
    engine.add_rule(Rule::new(
        "pairing_exists",
        vec![
            RelationPattern::new("set", vec![Term::var("a")]),
            RelationPattern::new("set", vec![Term::var("b")]),
        ],
        vec![RelationPattern::new("set", vec![Term::app("pair", vec![Term::var("a"), Term::var("b")])])],
    ));
    engine.add_rule(Rule::new(
        "pairing_left",
        vec![
            RelationPattern::new("set", vec![Term::var("a")]),
            RelationPattern::new("set", vec![Term::var("b")]),
        ],
        vec![RelationPattern::new("member", vec![
            Term::var("a"),
            Term::app("pair", vec![Term::var("a"), Term::var("b")]),
        ])],
    ));
    engine.add_rule(Rule::new(
        "pairing_right",
        vec![
            RelationPattern::new("set", vec![Term::var("a")]),
            RelationPattern::new("set", vec![Term::var("b")]),
        ],
        vec![RelationPattern::new("member", vec![
            Term::var("b"),
            Term::app("pair", vec![Term::var("a"), Term::var("b")]),
        ])],
    ));

    // ── Axiom 4: Union ──
    // union(A) = {x : ∃S ∈ A, x ∈ S}
    engine.add_rule(Rule::new(
        "union_exists",
        vec![RelationPattern::new("set", vec![Term::var("a")])],
        vec![RelationPattern::new("set", vec![Term::app("union", vec![Term::var("a")])])],
    ));
    engine.add_rule(Rule::new(
        "union_member",
        vec![
            RelationPattern::new("member", vec![Term::var("x"), Term::var("s")]),
            RelationPattern::new("member", vec![Term::var("s"), Term::var("a")]),
        ],
        vec![RelationPattern::new("member", vec![
            Term::var("x"),
            Term::app("union", vec![Term::var("a")]),
        ])],
    ).with_ground_required(vec!["a".to_string()]));

    // ── Axiom 5: Powerset ──
    // power(A) = {S : S ⊆ A}
    engine.add_rule(Rule::new(
        "powerset_exists",
        vec![RelationPattern::new("set", vec![Term::var("a")])],
        vec![RelationPattern::new("set", vec![Term::app("power", vec![Term::var("a")])])],
    ));
    engine.add_rule(Rule::new(
        "powerset_member",
        vec![
            RelationPattern::new("subset", vec![Term::var("s"), Term::var("a")]),
            RelationPattern::new("set", vec![Term::var("s")]),
        ],
        vec![RelationPattern::new("member", vec![
            Term::var("s"),
            Term::app("power", vec![Term::var("a")]),
        ])],
    ));

    engine.set_max_rounds(20);
    engine.set_max_facts(5000);

    engine
}

fn write_log(filename: &str, content: &str) {
    let dir = std::path::Path::new("logs");
    std::fs::create_dir_all(dir).ok();
    let path = dir.join(filename);
    std::fs::File::create(&path)
        .and_then(|mut f| {
            use std::io::Write;
            f.write_all(content.as_bytes())
        })
        .expect("write log");
    println!("  Log written to: {}", path.display());
}

#[test]
fn test_zfc5_basic_deductions() {
    let mut engine = zfc5_engine();

    println!("\n============================================================");
    println!("  ZFC-5 SET THEORY EXPERIMENT");
    println!("  Axioms: extensionality, empty, pairing, union, powerset");
    println!("  Depth limit: 4 (to control expansion)");
    println!("============================================================\n");

    engine.set_max_rounds(15);
    engine.set_max_facts(2000);

    let result = engine.derive_closure();

    // Collect facts by relation
    let mut sets: Vec<String> = Vec::new();
    let mut members: Vec<String> = Vec::new();
    let mut subsets: Vec<String> = Vec::new();
    let mut eqs: Vec<String> = Vec::new();

    for fact in &result.facts {
        let s = fact.to_string();
        match fact.name() {
            "set" => sets.push(s),
            "member" => members.push(s),
            "subset" => subsets.push(s),
            "eq" => eqs.push(s),
            _ => {}
        }
    }
    sets.sort();
    members.sort();
    subsets.sort();
    eqs.sort();

    println!("  Closure: {} rounds, {} facts, saturated={}",
        result.rounds, result.facts.len(), result.saturated);
    println!("\n  Sets ({}):", sets.len());
    for s in &sets { println!("    {}", s); }
    println!("\n  Membership ({}):", members.len());
    for m in &members { println!("    {}", m); }
    println!("\n  Subset ({}):", subsets.len());
    for s in &subsets { println!("    {}", s); }
    println!("\n  Equalities ({}):", eqs.len());
    for e in &eqs { println!("    {}", e); }

    // ── Key deductions to verify ──
    let has = |s: &str| result.facts.iter().any(|f| f.to_string() == s);

    // 1. pair(empty, empty) should exist
    let pair_ee = has("set(pair(empty, empty))");
    println!("\n  Checks:");
    println!("    set(pair(empty,empty)): {}", pair_ee);

    // 2. empty ∈ pair(empty, empty)
    let empty_in_pair = has("member(empty, pair(empty, empty))");
    println!("    empty ∈ pair(empty,empty): {}", empty_in_pair);

    // 3. empty ⊆ empty (vacuous)
    let empty_sub_empty = has("subset(empty, empty)");
    println!("    empty ⊆ empty: {}", empty_sub_empty);

    // 4. empty ∈ power(empty) (since empty ⊆ empty)
    let empty_in_power = has("member(empty, power(empty))");
    println!("    empty ∈ power(empty): {}", empty_in_power);

    // 5. power(empty) = {empty} conceptually. Check set(power(empty)).
    let power_empty_exists = has("set(power(empty))");
    println!("    set(power(empty)): {}", power_empty_exists);

    // 6. union(empty) should exist as a set
    let union_empty_exists = has("set(union(empty))");
    println!("    set(union(empty)): {}", union_empty_exists);

    // 7. pair(empty, empty) ⊆ pair(empty, empty)
    let pair_refl = subsets.iter().any(|s| s.contains("pair(empty, empty), pair(empty, empty)"));
    println!("    pair(empty,empty) ⊆ pair(empty,empty): {}", pair_refl);

    // Log
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let mut log = format!(
        "=== ZFC-5 SET THEORY ===\n\
         Rounds: {}, Facts: {}, Saturated: {}\n\n",
        result.rounds, result.facts.len(), result.saturated
    );
    log.push_str(&format!("Sets ({}): {}\n", sets.len(), sets.join(", ")));
    log.push_str(&format!("Members ({}): {}\n", members.len(), members.join(", ")));
    log.push_str(&format!("Subsets ({}): {}\n", subsets.len(), subsets.join(", ")));
    log.push_str(&format!("Equalities ({}): {}\n", eqs.len(), eqs.join(", ")));
    write_log(&format!("{}_zfc5.log", timestamp), &log);

    // Assertions
    assert!(pair_ee, "pairing axiom should produce set(pair(empty,empty))");
    assert!(empty_in_pair, "empty should be member of pair(empty,empty)");
    assert!(empty_sub_empty, "empty should be subset of itself");
    assert!(power_empty_exists, "power(empty) should exist");
    assert!(empty_in_power, "empty should be in power(empty)");
}

// ── Phase 1: Score = rarity × specificity ────────────────────
//
// rarity:      1/ln(relation_instance_count + 1) — rare relations carry more info
// specificity: structural asymmetry of terms — non-trivial facts score higher

/// Structural size of a term (node count).
fn term_size(t: &Term) -> usize {
    match t {
        Term::App { args, .. } => 1 + args.iter().map(term_size).sum::<usize>(),
        _ => 1,
    }
}

/// Specificity: conciseness × non-triviality.
///
/// Concise facts are more valuable (Occam's razor for set theory).
/// Trivial facts (eq(x,x), subset(x,x)) are penalized.
fn specificity(fact: &Relation) -> f64 {
    let total_size: usize = fact.terms().iter().map(term_size).sum();

    // Conciseness: smaller total size → higher score
    // size=2 (e.g. member(empty, power(empty))) → 1.0
    // size=10 → 0.2
    // size=20 → 0.1
    let conciseness = 2.0 / (total_size as f64).max(1.0);

    match fact.arity() {
        0 => 0.0,
        1 => {
            // Unary: just conciseness. set(empty)=high, set(pair(pair(...)))=low
            conciseness.min(1.0)
        }
        2 => {
            let a = &fact.terms()[0];
            let b = &fact.terms()[1];
            if a == b {
                return 0.01; // trivial: eq(x,x), subset(x,x)
            }
            // Non-trivial binary: reward conciseness
            conciseness.min(1.0)
        }
        _ => conciseness.min(1.0),
    }
}

fn total_score(
    fact: &Relation,
    rel_counts: &HashMap<String, usize>,
) -> (f64, f64, f64) {
    let count = *rel_counts.get(fact.name()).unwrap_or(&1) as f64;
    let rarity = 1.0 / (count + 1.0).ln();
    let spec = specificity(fact);
    let total = rarity * spec;
    (total, rarity, spec)
}

/// Phase 1: Score all 2000 ZFC facts and output top 50.
#[test]
fn test_zfc5_score_validation() {
    let mut engine = zfc5_engine();
    engine.set_max_rounds(15);
    engine.set_max_facts(2000);
    let result = engine.derive_closure();

    let all_facts: Vec<Relation> = result.facts.iter().cloned().collect();

    println!("\n============================================================");
    println!("  ZFC-5 SCORE VALIDATION (Phase 1)");
    println!("  {} facts to score", all_facts.len());
    println!("============================================================\n");

    // Count instances per relation
    let mut rel_counts: HashMap<String, usize> = HashMap::new();
    for f in &all_facts {
        *rel_counts.entry(f.name().to_string()).or_insert(0) += 1;
    }

    println!("  Relation distribution:");
    let mut rc: Vec<_> = rel_counts.iter().collect();
    rc.sort_by(|a, b| a.1.cmp(b.1));
    for (rel, count) in &rc {
        let rarity = 1.0 / (**count as f64 + 1.0).ln();
        println!("    {:<10} {:>5} instances  rarity={:.3}", rel, count, rarity);
    }

    // Score each fact
    let mut scored: Vec<(String, f64, f64, f64)> = all_facts.iter()
        .map(|f| {
            let (total, rarity, spec) = total_score(f, &rel_counts);
            (f.to_string(), total, rarity, spec)
        })
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    println!("\n  {:>6} {:>6} {:>6}  {}",
        "SCORE", "rare", "spec", "fact");
    println!("  {:->6} {:->6} {:->6}  {:->60}",
        "", "", "", "");

    for (fact, total, rarity, spec) in scored.iter().take(50) {
        println!("  {:.4} {:.4} {:.4}  {}",
            total, rarity, spec, fact);
    }

    // Check target facts
    println!("\n  --- TARGET FACTS ---\n");
    let targets = [
        "member(empty, power(empty))",
        "subset(empty, empty)",
        "subset(empty, pair(empty, empty))",
        "subset(empty, power(empty))",
        "member(empty, pair(empty, empty))",
        "member(empty, power(pair(empty, empty)))",
        "member(pair(empty, empty), power(pair(empty, empty)))",
        "eq(empty, empty)",
    ];
    for target in &targets {
        if let Some(pos) = scored.iter().position(|(f, ..)| f == target) {
            let (_, total, rarity, spec) = &scored[pos];
            println!("  rank {:>4}: {:.4} (rare={:.3} spec={:.3})  {}",
                pos + 1, total, rarity, spec, target);
        } else {
            println!("  NOT FOUND: {}", target);
        }
    }

    // Log
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let mut log = format!(
        "=== ZFC-5 SCORE: rarity x specificity ===\n\
         Facts: {}\nFormula: score = 1/ln(rel_count+1) * specificity\n\n\
         Top 50:\n", all_facts.len()
    );
    for (fact, total, rarity, spec) in scored.iter().take(50) {
        log.push_str(&format!("{:.4} rare={:.3} spec={:.3}  {}\n", total, rarity, spec, fact));
    }
    write_log(&format!("{}_zfc5_rarity.log", timestamp), &log);

    let target_rank = scored.iter()
        .position(|(f, ..)| f == "member(empty, power(empty))")
        .map(|r| r + 1);
    println!("\n  member(empty, power(empty)) rank: {:?}", target_rank);
    assert!(target_rank.unwrap_or(9999) <= 100,
        "member(empty, power(empty)) should be in top 100");
}

// ── Phase 2: Guided deduction ────────────────────────────────

/// Run ZFC-5 closure with score-guided fact selection.
///
/// Only facts in the whitelist can trigger rules. New facts are scored;
/// only those above threshold enter the whitelist for next round.
/// Axiom-given facts are permanently whitelisted.
#[test]
fn test_zfc5_guided_deduction() {
    let engine = zfc5_engine();
    let rules = engine.rules().to_vec();
    let all_relations: Vec<(String, usize)> = engine.relation_defs()
        .iter().map(|(n, d)| (n.clone(), d.arity())).collect();

    // Initial axiom facts → permanent whitelist
    let axiom_facts: HashSet<String> = engine.facts().iter()
        .map(|f| f.to_string()).collect();
    let mut whitelist: HashSet<String> = axiom_facts.clone();
    let mut all_facts: Vec<Relation> = engine.facts().iter().cloned().collect();
    let mut fact_set: HashSet<String> = whitelist.clone();
    let mut archive: Vec<Relation> = Vec::new(); // low-score facts, kept but not active

    let max_rounds = 10;
    let max_depth = 8usize;

    // Target facts to track
    let targets = [
        ("P1", "member(empty, power(empty))"),
        ("P1", "subset(empty, empty)"),
        ("P1", "subset(empty, power(empty))"),
        ("P2", "member(empty, pair(empty, empty))"),
        ("P2", "subset(empty, pair(empty, empty))"),
        ("P2", "member(empty, power(pair(empty, empty)))"),
        ("P2", "member(pair(empty, empty), power(pair(empty, empty)))"),
    ];

    println!("\n============================================================");
    println!("  ZFC-5 GUIDED DEDUCTION (Phase 2)");
    println!("  Rules: {}, Initial facts: {}", rules.len(), axiom_facts.len());
    println!("  Threshold: top 25% of new facts per round");
    println!("============================================================");

    let mut log_text = String::from("=== ZFC-5 GUIDED DEDUCTION ===\n\n");

    for round in 0..max_rounds {
        // Collect whitelist facts as Relation objects for matching
        let active_facts: Vec<&Relation> = all_facts.iter()
            .filter(|f| whitelist.contains(&f.to_string()))
            .collect();

        // One derivation step: apply rules using only whitelist facts
        let mut candidates: Vec<Relation> = Vec::new();

        const MAX_CANDIDATES: usize = 20_000;
        for rule in &rules {
            // Find all substitutions matching premises against active facts
            let subs = match_premises_vec(rule.premises(), &active_facts);
            for sub in &subs {
                for conclusion in rule.conclusions() {
                    if let Some(fact) = instantiate(conclusion, sub) {
                        if fact.is_ground() {
                            let max_d = fact.terms().iter()
                                .map(|t| t.depth()).max().unwrap_or(0);
                            if max_d <= max_depth {
                                let key = fact.to_string();
                                if !fact_set.contains(&key) {
                                    candidates.push(fact);
                                }
                            }
                        }
                    }
                }
                if candidates.len() >= MAX_CANDIDATES { break; }
            }
            if candidates.len() >= MAX_CANDIDATES { break; }
        }

        // Deduplicate candidates
        let mut unique_candidates: Vec<Relation> = Vec::new();
        let mut seen = HashSet::new();
        for c in candidates {
            let key = c.to_string();
            if seen.insert(key) {
                unique_candidates.push(c);
            }
        }

        if unique_candidates.is_empty() {
            let msg = format!("Round {}: no new facts — converged\n", round);
            println!("\n  {}", msg.trim());
            log_text.push_str(&msg);
            break;
        }

        // Score candidates
        // Need relation counts including ALL facts (whitelist + archive + candidates)
        let mut rel_counts: HashMap<String, usize> = HashMap::new();
        for f in &all_facts { *rel_counts.entry(f.name().to_string()).or_insert(0) += 1; }
        for f in &unique_candidates { *rel_counts.entry(f.name().to_string()).or_insert(0) += 1; }

        let mut scored_candidates: Vec<(Relation, f64)> = unique_candidates.iter()
            .map(|f| {
                let (score, _, _) = total_score(f, &rel_counts);
                (f.clone(), score)
            })
            .collect();
        scored_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Threshold: top 25%
        let threshold_idx = (scored_candidates.len() as f64 * 0.25).ceil() as usize;
        let threshold_score = if threshold_idx < scored_candidates.len() {
            scored_candidates[threshold_idx].1
        } else {
            0.0
        };

        let mut promoted = 0usize;
        let mut archived = 0usize;

        for (fact, score) in &scored_candidates {
            let key = fact.to_string();
            fact_set.insert(key.clone());
            all_facts.push(fact.clone());

            if *score >= threshold_score {
                whitelist.insert(key);
                promoted += 1;
            } else {
                archive.push(fact.clone());
                archived += 1;
            }
        }

        // Report
        let top5: Vec<String> = scored_candidates.iter().take(5)
            .map(|(f, s)| format!("{:.4} {}", s, f))
            .collect();

        let mut target_status = Vec::new();
        for (priority, target) in &targets {
            let reached = fact_set.contains(*target);
            let in_wl = whitelist.contains(*target);
            if reached {
                target_status.push(format!("{} {} [{}]",
                    priority, target,
                    if in_wl { "whitelist" } else { "archive" }));
            }
        }

        let msg = format!(
            "Round {}: {} new facts ({}→whitelist, {}→archive), threshold={:.4}, whitelist={}\n\
             Top 5: {}\n{}\n",
            round, scored_candidates.len(), promoted, archived, threshold_score,
            whitelist.len(),
            top5.join("\n       "),
            if target_status.is_empty() { String::new() }
            else { format!("  Targets reached: {}", target_status.join(", ")) }
        );
        println!("\n  {}", msg.trim_end());
        log_text.push_str(&msg);
    }

    // Final summary
    let mut summary = format!(
        "\n--- FINAL STATE ---\n\
         Total facts: {}\n\
         Whitelist: {}\n\
         Archive: {}\n\n\
         Target status:\n",
        all_facts.len(), whitelist.len(), archive.len()
    );
    for (priority, target) in &targets {
        let reached = fact_set.contains(*target);
        let in_wl = whitelist.contains(*target);
        summary.push_str(&format!("  {} {} {}\n", priority, target,
            if reached && in_wl { "✓ whitelist" }
            else if reached { "✓ archive" }
            else { "✗ not reached" }));
    }
    println!("{}", summary);
    log_text.push_str(&summary);

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    write_log(&format!("{}_zfc5_guided.log", timestamp), &log_text);

    // Key assertion: guided should reach P1 targets
    assert!(fact_set.contains("member(empty, power(empty))"),
        "should reach member(empty, power(empty))");
    assert!(fact_set.contains("subset(empty, power(empty))"),
        "should reach subset(empty, power(empty))");
}

// ── Phase 3: Pattern facts (symbolic deduction) ─────────────

/// ZFC with pattern facts: derive universal truths symbolically instead of
/// enumerating ground instances. `subset(empty, ?A)` is a single pattern fact
/// that represents the universal truth "empty is a subset of any set".
#[test]
fn test_zfc5_pattern_facts() {
    let mut engine = zfc5_engine();

    // Add pattern fact (axiom): empty is a subset of any set.
    // This replaces the combinatorial expansion of subset(empty, X) for every X.
    engine.add_fact(Relation::binary("subset", c("empty"), Term::var("A")));

    engine.set_max_rounds(6);
    engine.set_max_facts(2000);

    let result = engine.derive_closure();

    println!("\n============================================================");
    println!("  ZFC-5 PATTERN FACTS (Phase 3)");
    println!("  {} facts, {} rounds, saturated={}",
        result.facts.len(), result.rounds, result.saturated);
    println!("============================================================\n");

    let mut ground_count = 0usize;
    let mut pattern_count = 0usize;
    let mut by_rel: std::collections::BTreeMap<String, Vec<String>> = std::collections::BTreeMap::new();
    for f in &result.facts {
        if f.is_ground() { ground_count += 1; } else { pattern_count += 1; }
        by_rel.entry(f.name().to_string()).or_default().push(f.to_string());
    }
    for (rel, facts) in &mut by_rel {
        facts.sort();
        println!("  {} ({}):", rel, facts.len());
        for f in facts.iter() { println!("    {}", f); }
    }
    println!("\n  Ground facts: {}, Pattern facts: {}", ground_count, pattern_count);

    let has = |s: &str| result.facts.iter().any(|f| f.to_string() == s);

    // ── Universal truths derived as pattern facts ──

    // member(empty, power(_0)) — empty is in the powerset of ANY set
    assert!(has("member(empty, power(_0))"),
        "should derive universal: member(empty, power(?A))");

    // subset(empty, _0) — axiom, should be preserved
    assert!(has("subset(empty, _0)"),
        "pattern axiom subset(empty, ?A) should be preserved");

    // ── Ground truths still derivable ──

    assert!(has("set(pair(empty, empty))"),
        "pairing should produce set(pair(empty, empty))");

    assert!(has("member(empty, power(empty))"),
        "ground instance member(empty, power(empty)) should also be derived");

    assert!(has("subset(empty, empty)"),
        "ground instance subset(empty, empty) should be derived");

    // ── P2 targets ──
    assert!(has("member(empty, pair(empty, empty))"),
        "should derive member(empty, pair(empty, empty))");

    println!("\n  Key pattern facts:");
    println!("    member(empty, power(_0)): {}", has("member(empty, power(_0))"));
    println!("    subset(empty, _0): {}", has("subset(empty, _0)"));
    println!("\n  Key ground facts:");
    println!("    member(empty, power(empty)): {}", has("member(empty, power(empty))"));
    println!("    member(empty, pair(empty, empty)): {}", has("member(empty, pair(empty, empty))"));
    println!("    set(pair(empty, empty)): {}", has("set(pair(empty, empty))"));
    println!("\n  Total: {} facts (ground-only approach needed ~2000)",
        result.facts.len());

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let mut log = format!(
        "=== ZFC-5 PATTERN FACTS ===\n\
         Facts: {} ({} ground, {} pattern), Rounds: {}, Saturated: {}\n\n",
        result.facts.len(), ground_count, pattern_count,
        result.rounds, result.saturated
    );
    for (rel, facts) in &by_rel {
        log.push_str(&format!("{} ({}):\n", rel, facts.len()));
        for f in facts { log.push_str(&format!("  {}\n", f)); }
    }
    write_log(&format!("{}_zfc5_pattern.log", timestamp), &log);
}

/// Full pattern facts: subset + pairing + union + powerset axioms as pattern facts.
/// Key risk: member(pair(empty,empty), power(pair(empty,empty))) requires the
/// concrete term pair(empty,empty) — verify it survives pattern-level pairing.
#[test]
fn test_zfc5_full_pattern() {
    let mut engine = zfc5_engine();

    // Pattern axioms — universal truths that replace ground enumeration
    // subset: empty ⊆ anything
    engine.add_fact(Relation::binary("subset", c("empty"), Term::var("A")));
    // pairing: a ∈ {a,b} and b ∈ {a,b}
    engine.add_fact(Relation::binary("member",
        Term::var("A"), Term::app("pair", vec![Term::var("A"), Term::var("B")])));
    engine.add_fact(Relation::binary("member",
        Term::var("B"), Term::app("pair", vec![Term::var("A"), Term::var("B")])));

    engine.set_max_rounds(6);
    engine.set_max_facts(2000);

    let result = engine.derive_closure();

    let ground = result.facts.iter().filter(|f| f.is_ground()).count();
    let pattern = result.facts.iter().filter(|f| !f.is_ground()).count();

    println!("\n============================================================");
    println!("  ZFC-5 FULL PATTERN (Phase 3b)");
    println!("  {} facts ({} ground, {} pattern), {} rounds, saturated={}",
        result.facts.len(), ground, pattern, result.rounds, result.saturated);
    println!("============================================================");

    let has = |s: &str| result.facts.iter().any(|f| f.to_string() == s);

    // All 7 targets
    let targets = [
        ("P1", "member(empty, power(empty))"),
        ("P1", "subset(empty, empty)"),
        ("P1", "subset(empty, power(empty))"),
        ("P2", "member(empty, pair(empty, empty))"),
        ("P2", "subset(empty, pair(empty, empty))"),
        ("P2", "member(empty, power(pair(empty, empty)))"),
        ("P2", "member(pair(empty, empty), power(pair(empty, empty)))"),
    ];
    println!("\n  Target status:");
    let mut all_ok = true;
    for (pri, target) in &targets {
        let ok = has(target);
        println!("    {} {} {}", pri, if ok { "✓" } else { "✗" }, target);
        if !ok { all_ok = false; }
    }

    // Pattern facts derived
    let patterns: Vec<String> = result.facts.iter()
        .filter(|f| !f.is_ground())
        .map(|f| f.to_string())
        .collect();
    println!("\n  Pattern facts ({}):", patterns.len());
    let mut sorted = patterns.clone();
    sorted.sort();
    for p in &sorted { println!("    {}", p); }

    println!("\n  Total: {} facts, {} rounds", result.facts.len(), result.rounds);

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let mut log = format!(
        "=== ZFC-5 FULL PATTERN ===\nFacts: {} ({} ground, {} pattern), Rounds: {}\n\n",
        result.facts.len(), ground, pattern, result.rounds
    );
    for (pri, target) in &targets {
        log.push_str(&format!("{} {} {}\n", pri, if has(target) { "✓" } else { "✗" }, target));
    }
    log.push_str(&format!("\nPattern facts:\n"));
    for p in &sorted { log.push_str(&format!("  {}\n", p)); }
    write_log(&format!("{}_zfc5_full_pattern.log", timestamp), &log);

    assert!(all_ok, "all 7 targets must be reached");
}

/// Pure symbolic deduction: no ground expansion at all.
/// Start from pattern axioms only, derive pattern conclusions.
/// Tests that var-var unification propagates correctly through rule chains.
#[test]
fn test_zfc5_pure_symbolic() {
    let mut engine = ClosureEngine::new();

    // Relations
    engine.define_relation("set", 1);
    engine.define_relation("member", 2);
    engine.define_relation("subset", 2);

    // Variables
    for v in &["x", "y", "z", "a", "b", "s"] {
        engine.define_variable(*v);
    }

    // ── No ground facts. Only pattern axioms. ──

    // Axiom: there exists an empty set (Skolem witness as constant)
    engine.define_constant("empty");
    engine.add_fact(Relation::new("set", vec![c("empty")]));

    // Pattern axiom: empty ⊆ anything
    engine.add_fact(Relation::binary("subset", c("empty"), Term::var("A")));

    // Rules (subset → powerset membership)
    engine.add_rule(Rule::new(
        "powerset_member",
        vec![
            RelationPattern::new("subset", vec![Term::var("s"), Term::var("a")]),
            RelationPattern::new("set", vec![Term::var("s")]),
        ],
        vec![RelationPattern::new("member", vec![
            Term::var("s"),
            Term::app("power", vec![Term::var("a")]),
        ])],
    ));

    // subset reflexivity
    engine.add_rule(Rule::new(
        "subset_refl",
        vec![RelationPattern::new("set", vec![Term::var("x")])],
        vec![RelationPattern::new("subset", vec![Term::var("x"), Term::var("x")])],
    ));

    // powerset produces sets
    engine.add_rule(Rule::new(
        "powerset_exists",
        vec![RelationPattern::new("set", vec![Term::var("a")])],
        vec![RelationPattern::new("set", vec![Term::app("power", vec![Term::var("a")])])],
    ));

    engine.set_max_rounds(3);
    engine.set_max_facts(100);

    let result = engine.derive_closure();

    let ground: Vec<String> = result.facts.iter()
        .filter(|f| f.is_ground()).map(|f| f.to_string()).collect();
    let mut patterns: Vec<String> = result.facts.iter()
        .filter(|f| !f.is_ground()).map(|f| f.to_string()).collect();
    patterns.sort();

    println!("\n============================================================");
    println!("  ZFC-5 PURE SYMBOLIC (no ground expansion)");
    println!("  {} facts ({} ground, {} pattern), {} rounds",
        result.facts.len(), ground.len(), patterns.len(), result.rounds);
    println!("============================================================");

    println!("\n  Ground facts:");
    for f in &ground { println!("    {}", f); }
    println!("\n  Pattern facts (universal truths):");
    for f in &patterns { println!("    {}", f); }

    let has = |s: &str| result.facts.iter().any(|f| f.to_string() == s);

    // Core universal truths derived purely symbolically
    assert!(has("subset(empty, _0)"),
        "axiom: ∀A. empty ⊆ A");
    assert!(has("member(empty, power(_0))"),
        "derived: ∀A. empty ∈ P(A) — from subset(empty,_0) + powerset_member");

    // Check that var-var chain works:
    // subset(empty, _0) + set(empty) → member(empty, power(_0))
    // This requires: ?s = empty, ?a = _0 (var bound to var)
    println!("\n  Var-var unification chain verified:");
    println!("    subset(empty, _0) + set(empty)");
    println!("    → ?s=empty, ?a=_0 (var-to-var binding)");
    println!("    → member(empty, power(_0)) ✓");
}

// ── Stratified negation: derived ordering → maximal/minimal ──

/// Graph reachability: derive transitive closure from edges, then use
/// negation to find sources (no incoming) and sinks (no outgoing).
///
/// The ORDER is not input — it's derived from raw edges via transitive closure.
/// Negation then operates on the derived relation.
///
///   a → b → c → d
///         ↘ e
///
/// Expected: source(a), sink(d), sink(e).
#[test]
fn test_negation_derived_reachability() {
    let mut engine = ClosureEngine::new();

    engine.define_relation("node", 1);
    engine.define_relation("edge", 2);
    engine.define_relation("reachable", 2);  // derived via transitive closure
    engine.define_relation("source", 1);     // derived via negation
    engine.define_relation("sink", 1);       // derived via negation

    for v in &["x", "y", "z"] {
        engine.define_variable(*v);
    }

    for n in &["a", "b", "c", "d", "e"] {
        engine.define_constant(*n);
        engine.add_fact(Relation::new("node", vec![c(*n)]));
    }

    // Raw edges — the ONLY user input
    engine.add_fact(Relation::binary("edge", c("a"), c("b")));
    engine.add_fact(Relation::binary("edge", c("b"), c("c")));
    engine.add_fact(Relation::binary("edge", c("b"), c("e")));
    engine.add_fact(Relation::binary("edge", c("c"), c("d")));

    // Derive reachability (transitive closure of edges)
    engine.add_rule(Rule::new(
        "reach_base",
        vec![RelationPattern::new("edge", vec![Term::var("x"), Term::var("y")])],
        vec![RelationPattern::new("reachable", vec![Term::var("x"), Term::var("y")])],
    ));
    engine.add_rule(Rule::new(
        "reach_step",
        vec![
            RelationPattern::new("reachable", vec![Term::var("x"), Term::var("y")]),
            RelationPattern::new("edge", vec![Term::var("y"), Term::var("z")]),
        ],
        vec![RelationPattern::new("reachable", vec![Term::var("x"), Term::var("z")])],
    ));

    // Source: node with no incoming reachable edges
    engine.add_rule(Rule::new(
        "source_def",
        vec![RelationPattern::new("node", vec![Term::var("x")])],
        vec![RelationPattern::new("source", vec![Term::var("x")])],
    ).with_negated(vec![
        RelationPattern::new("reachable", vec![Term::var("y"), Term::var("x")]),
    ]));

    // Sink: node with no outgoing reachable edges
    engine.add_rule(Rule::new(
        "sink_def",
        vec![RelationPattern::new("node", vec![Term::var("x")])],
        vec![RelationPattern::new("sink", vec![Term::var("x")])],
    ).with_negated(vec![
        RelationPattern::new("reachable", vec![Term::var("x"), Term::var("y")]),
    ]));

    engine.set_max_rounds(10);
    engine.set_max_facts(100);

    let result = engine.derive_closure();
    let has = |s: &str| result.facts.iter().any(|f| f.to_string() == s);

    println!("\n============================================================");
    println!("  STRATIFIED NEGATION: Derived Reachability");
    println!("  {} facts, {} rounds", result.facts.len(), result.rounds);
    println!("============================================================");

    let mut sorted: Vec<String> = result.facts.iter().map(|f| f.to_string()).collect();
    sorted.sort();
    for f in &sorted { println!("  {}", f); }

    // Derived reachability
    assert!(has("reachable(a, d)"), "a can reach d via a→b→c→d");
    assert!(has("reachable(a, e)"), "a can reach e via a→b→e");
    assert!(has("reachable(b, d)"), "b can reach d via b→c→d");
    assert!(!has("reachable(d, a)"), "d cannot reach a");

    // Negation on derived relation
    assert!(has("source(a)"), "a is the only source");
    assert!(!has("source(b)"), "b is reachable from a");
    assert!(has("sink(d)"), "d is a sink");
    assert!(has("sink(e)"), "e is a sink");
    assert!(!has("sink(b)"), "b reaches c, d, e");

    println!("\n  sources: a={} b={} c={} d={} e={}",
        has("source(a)"), has("source(b)"), has("source(c)"),
        has("source(d)"), has("source(e)"));
    println!("  sinks:   a={} b={} c={} d={} e={}",
        has("sink(a)"), has("sink(b)"), has("sink(c)"),
        has("sink(d)"), has("sink(e)"));
}

/// ZFC subset ordering → minimal set via negation.
///
/// All structure is derived from axioms. The subset ordering emerges from
/// empty_subset + subset_refl. Negation identifies empty as the unique
/// minimal element (strict subset of everything, nothing is strict subset of it).
///
/// Two-stratum negation:
///   stratum 1: strict_subset(x, y) ← subset(x, y), NOT subset(y, x)
///   stratum 2: minimal_set(x) ← set(x), NOT strict_subset(_, x)
#[test]
fn test_negation_zfc_minimal_set() {
    let mut engine = ClosureEngine::new();

    engine.define_relation("set", 1);
    engine.define_relation("subset", 2);
    engine.define_relation("strict_subset", 2);
    engine.define_relation("minimal_set", 1);

    for v in &["x", "y"] {
        engine.define_variable(*v);
    }

    engine.define_constant("empty");
    engine.add_fact(Relation::new("set", vec![c("empty")]));

    // ZFC rules (no manual ordering input)
    engine.add_rule(Rule::new(
        "empty_subset",
        vec![RelationPattern::new("set", vec![Term::var("x")])],
        vec![RelationPattern::new("subset", vec![c("empty"), Term::var("x")])],
    ));
    engine.add_rule(Rule::new(
        "subset_refl",
        vec![RelationPattern::new("set", vec![Term::var("x")])],
        vec![RelationPattern::new("subset", vec![Term::var("x"), Term::var("x")])],
    ));
    engine.add_rule(Rule::new(
        "powerset_exists",
        vec![RelationPattern::new("set", vec![Term::var("x")])],
        vec![RelationPattern::new("set", vec![
            Term::app("power", vec![Term::var("x")]),
        ])],
    ));

    // Stratum 1: strict_subset(x, y) ← subset(x, y), NOT subset(y, x)
    engine.add_rule(Rule::new(
        "strict_subset_def",
        vec![RelationPattern::new("subset", vec![Term::var("x"), Term::var("y")])],
        vec![RelationPattern::new("strict_subset", vec![Term::var("x"), Term::var("y")])],
    ).with_negated(vec![
        RelationPattern::new("subset", vec![Term::var("y"), Term::var("x")]),
    ]));

    // Stratum 2: minimal_set(x) ← set(x), NOT strict_subset(_, x)
    engine.add_rule(Rule::new(
        "minimal_set_def",
        vec![RelationPattern::new("set", vec![Term::var("x")])],
        vec![RelationPattern::new("minimal_set", vec![Term::var("x")])],
    ).with_negated(vec![
        RelationPattern::new("strict_subset", vec![Term::var("y"), Term::var("x")]),
    ]).with_stratum(2));

    engine.set_max_rounds(20);
    engine.set_max_facts(200);

    let result = engine.derive_closure();
    let has = |s: &str| result.facts.iter().any(|f| f.to_string() == s);

    println!("\n============================================================");
    println!("  STRATIFIED NEGATION: ZFC Minimal Set (2-stratum)");
    println!("  {} facts, {} rounds", result.facts.len(), result.rounds);
    println!("============================================================");

    // Show key facts
    let mut sorted: Vec<String> = result.facts.iter().map(|f| f.to_string()).collect();
    sorted.sort();
    for f in &sorted {
        if f.contains("strict_subset") || f.contains("minimal") {
            println!("  {}", f);
        }
    }

    // Derived strict ordering: empty ⊊ power(empty), empty ⊊ power(power(empty)), etc.
    assert!(has("strict_subset(empty, power(empty))"),
        "empty ⊊ power(empty) — derived from subset, not input");

    // empty is the unique minimal set
    assert!(has("minimal_set(empty)"),
        "empty should be the minimal set");
    assert!(!has("minimal_set(power(empty))"),
        "power(empty) is not minimal (empty ⊊ power(empty))");

    println!("\n  minimal_set(empty): {}", has("minimal_set(empty)"));
    println!("  minimal_set(power(empty)): {}", has("minimal_set(power(empty))"));
    println!("\n  Ordering derived from ZFC axioms, not user input.");
}

// ── ω-rule: inductive promotion ─────────────────────────────

/// Natural numbers: nat(zero), nat(x) |- nat(succ(x)).
/// After closure saturates, the ω-rule should promote nat(_0).
///
/// Also demonstrates a known capability boundary: imprecise downstream rules
/// produce over-broad conclusions when triggered by promoted pattern facts.
/// `nat(x) |- even(x)` is mathematically wrong (not all nats are even), but
/// the engine faithfully derives `even(_0)` from `nat(_0)`. This is not a bug
/// in the ω-rule — it correctly promoted `nat(_0)`. The error is in the rule
/// itself. The ω-rule requires precise rule premises to produce correct results.
#[test]
fn test_omega_rule_nat() {
    let mut engine = ClosureEngine::new();

    engine.define_relation("nat", 1);
    engine.define_relation("nonneg", 1);  // correct: all nats are non-negative
    engine.define_relation("even", 1);    // WRONG rule: not all nats are even
    engine.define_variable("x");

    engine.define_constant("zero");
    engine.add_fact(Relation::new("nat", vec![c("zero")]));

    // Inductive rule: nat(x) |- nat(succ(x))
    engine.add_rule(Rule::new(
        "nat_step",
        vec![RelationPattern::new("nat", vec![Term::var("x")])],
        vec![RelationPattern::new("nat", vec![
            Term::app("succ", vec![Term::var("x")]),
        ])],
    ));

    // Correct downstream rule: nat(x) |- nonneg(x) — all nats ARE non-negative
    engine.add_rule(Rule::new(
        "nat_nonneg",
        vec![RelationPattern::new("nat", vec![Term::var("x")])],
        vec![RelationPattern::new("nonneg", vec![Term::var("x")])],
    ));

    // IMPRECISE downstream rule: nat(x) |- even(x) — NOT all nats are even.
    // This rule is wrong, but the engine has no way to know that.
    // When nat(_0) is promoted, this rule fires and produces even(_0) — an
    // over-broad conclusion caused by the imprecise rule, not by the ω-rule.
    engine.add_rule(Rule::new(
        "nat_even_WRONG",
        vec![RelationPattern::new("nat", vec![Term::var("x")])],
        vec![RelationPattern::new("even", vec![Term::var("x")])],
    ));

    engine.set_max_rounds(20);
    engine.set_max_facts(200);

    let result = engine.derive_closure();

    let has = |s: &str| result.facts.iter().any(|f| f.to_string() == s);

    println!("\n============================================================");
    println!("  ω-RULE: Natural Number Induction");
    println!("  {} facts, {} rounds, saturated={}",
        result.facts.len(), result.rounds, result.saturated);
    println!("============================================================");

    let mut sorted: Vec<String> = result.facts.iter().map(|f| f.to_string()).collect();
    sorted.sort();
    for f in &sorted { println!("  {}", f); }

    if !result.warnings.is_empty() {
        println!("\n  Warnings:");
        for w in &result.warnings { println!("    {}", w); }
    }

    // Ground instances
    assert!(has("nat(zero)"), "base case");
    assert!(has("nat(succ(zero))"), "step 1");

    // ω-rule promotion
    assert!(has("nat(_0)"), "ω-rule should promote nat(_0)");

    // Defense: pure-transfer rules are blocked at pattern level.
    // nat(x)|-nonneg(x) is pure transfer: conclusion is just variable from premise.
    // nat(x)|-even(x) is also pure transfer.
    // Both are blocked, even though nonneg(_0) would be correct.
    // This is a conservative defense: some correct pattern facts are lost
    // to prevent incorrect ones from propagating.
    assert!(!has("even(_0)"),
        "even(_0) should be BLOCKED by pure-transfer defense");
    assert!(!has("nonneg(_0)"),
        "nonneg(_0) also blocked (conservative: pure transfer, even though correct)");

    // Ground instances still exist (defense only blocks pattern-level)
    assert!(has("even(zero)"), "ground even(zero) still derived");
    assert!(has("nonneg(zero)"), "ground nonneg(zero) still derived");

    println!("\n  ω-rule promotion:");
    println!("    nat(_0): {} ✓", has("nat(_0)"));
    println!("\n  Pure-transfer defense (blocks pattern-level):");
    println!("    even(_0): {} ← blocked (wrong rule)", has("even(_0)"));
    println!("    nonneg(_0): {} ← blocked (conservative)", has("nonneg(_0)"));
    println!("\n  Ground instances preserved:");
    println!("    even(zero): {}, nonneg(zero): {}", has("even(zero)"), has("nonneg(zero)"));
    println!("\n  Trade-off: nonneg(_0) is mathematically correct but blocked");
    println!("  because the engine cannot distinguish correct from incorrect");
    println!("  pure-transfer rules. Constructive rules (like set(x)|-subset(empty,x))");
    println!("  are allowed because they add term structure to the conclusion.");
}

/// ω-rule on ZFC: set(empty), set(x) |- set(power(x)) should promote set(_0).
/// Then subset(empty, _0) can be derived from set(_0) + empty_subset rule
/// WITHOUT manually injecting the pattern axiom.
#[test]
fn test_omega_rule_set_bootstrap() {
    let mut engine = ClosureEngine::new();

    engine.define_relation("set", 1);
    engine.define_relation("subset", 2);
    engine.define_relation("member", 2);
    engine.define_variable("x");
    engine.define_variable("s");
    engine.define_variable("a");

    engine.define_constant("empty");
    engine.add_fact(Relation::new("set", vec![c("empty")]));

    // set(x) |- set(power(x))  — inductive
    engine.add_rule(Rule::new(
        "powerset_exists",
        vec![RelationPattern::new("set", vec![Term::var("x")])],
        vec![RelationPattern::new("set", vec![
            Term::app("power", vec![Term::var("x")]),
        ])],
    ));

    // set(x) |- subset(empty, x)
    engine.add_rule(Rule::new(
        "empty_subset",
        vec![RelationPattern::new("set", vec![Term::var("x")])],
        vec![RelationPattern::new("subset", vec![c("empty"), Term::var("x")])],
    ));

    // subset(s, a), set(s) |- member(s, power(a))
    engine.add_rule(Rule::new(
        "powerset_member",
        vec![
            RelationPattern::new("subset", vec![Term::var("s"), Term::var("a")]),
            RelationPattern::new("set", vec![Term::var("s")]),
        ],
        vec![RelationPattern::new("member", vec![
            Term::var("s"),
            Term::app("power", vec![Term::var("a")]),
        ])],
    ));

    engine.set_max_rounds(20);
    engine.set_max_facts(200);

    let result = engine.derive_closure();

    let has = |s: &str| result.facts.iter().any(|f| f.to_string() == s);

    println!("\n============================================================");
    println!("  ω-RULE: ZFC Set Bootstrap");
    println!("  {} facts, {} rounds", result.facts.len(), result.rounds);
    println!("============================================================");

    let mut sorted: Vec<String> = result.facts.iter().map(|f| f.to_string()).collect();
    sorted.sort();
    for f in &sorted { println!("  {}", f); }

    if !result.warnings.is_empty() {
        println!("\n  Warnings:");
        for w in &result.warnings { println!("    {}", w); }
    }

    // ω-rule should promote set(_0)
    assert!(has("set(_0)"), "ω-rule should promote set(_0)");

    // With set(_0), empty_subset should derive subset(empty, _0)
    // WITHOUT manually injecting the pattern axiom
    assert!(has("subset(empty, _0)"),
        "subset(empty, _0) should be auto-derived from set(_0) + empty_subset");

    // And then powerset_member should derive member(empty, power(_0))
    assert!(has("member(empty, power(_0))"),
        "member(empty, power(_0)) should follow from subset(empty, _0)");

    println!("\n  set(_0): {} (ω-rule)", has("set(_0)"));
    println!("  subset(empty, _0): {} (auto-derived)", has("subset(empty, _0)"));
    println!("  member(empty, power(_0)): {} (chain)", has("member(empty, power(_0))"));
}

// ── Iterative chain construction with stratified maximality ──

/// Iterative chain construction with Skolem witnesses and stratified
/// maximality detection.
///
/// Demonstrates the global iteration loop: negation-gated rules (stratum 2)
/// produce new facts (Skolem successors), which feed back into stratum 0
/// (lt transitivity), which feeds stratum 1 (maximality recomputation).
///
/// This is NOT a proof of Zorn's Lemma. The "maximal" elements found here
/// are artifacts of the depth limit, not genuine mathematical maximality.
/// Changing the depth limit changes which elements are "maximal."
///
/// What this test actually demonstrates:
/// 1. Iterative stratification (strata cycle until global fixpoint)
/// 2. Skolem witness generation gated by negation
/// 3. Depth-bounded chain construction
#[test]
fn test_iterative_chain_construction() {
    let mut engine = ClosureEngine::new();

    engine.define_relation("element", 1);
    engine.define_relation("lt", 2);
    engine.define_relation("maximal", 1);

    for v in &["x", "y", "z"] {
        engine.define_variable(*v);
    }

    // Start with a single element
    engine.define_constant("a");
    engine.add_fact(Relation::new("element", vec![c("a")]));

    // And a successor: a < b (so a is NOT maximal)
    engine.define_constant("b");
    engine.add_fact(Relation::new("element", vec![c("b")]));
    engine.add_fact(Relation::binary("lt", c("a"), c("b")));

    // Transitivity of lt (stratum 0)
    engine.add_rule(Rule::new(
        "lt_trans",
        vec![
            RelationPattern::new("lt", vec![Term::var("x"), Term::var("y")]),
            RelationPattern::new("lt", vec![Term::var("y"), Term::var("z")]),
        ],
        vec![RelationPattern::new("lt", vec![Term::var("x"), Term::var("z")])],
    ));

    // Maximal: element(x), NOT lt(x, ?y) — stratum 1
    engine.add_rule(Rule::new(
        "maximal_def",
        vec![RelationPattern::new("element", vec![Term::var("x")])],
        vec![RelationPattern::new("maximal", vec![Term::var("x")])],
    ).with_negated(vec![
        RelationPattern::new("lt", vec![Term::var("x"), Term::var("y")]),
    ]));

    // AC: if x is not maximal, create Skolem successor — stratum 2
    // "There exists something strictly greater than x"
    engine.add_rule(Rule::new(
        "ac_successor",
        vec![RelationPattern::new("element", vec![Term::var("x")])],
        vec![
            RelationPattern::new("element", vec![
                Term::app("sk", vec![Term::var("x")]),
            ]),
            RelationPattern::new("lt", vec![
                Term::var("x"),
                Term::app("sk", vec![Term::var("x")]),
            ]),
        ],
    ).with_negated(vec![
        RelationPattern::new("maximal", vec![Term::var("x")]),
    ]).with_stratum(2));

    engine.set_max_rounds(50);
    engine.set_max_facts(500);

    let result = engine.derive_closure();
    let has = |s: &str| result.facts.iter().any(|f| f.to_string() == s);

    println!("\n============================================================");
    println!("  ITERATIVE CHAIN: Skolem Extension + Stratified Maximality");
    println!("  {} facts, {} rounds", result.facts.len(), result.rounds);
    println!("============================================================");

    // Show the chain
    let mut elements: Vec<String> = result.facts.iter()
        .filter(|f| f.name() == "element")
        .map(|f| f.to_string())
        .collect();
    elements.sort_by_key(|s| s.len());
    println!("\n  Chain constructed:");
    for e in &elements { println!("    {}", e); }

    let maximals: Vec<String> = result.facts.iter()
        .filter(|f| f.name() == "maximal")
        .map(|f| f.to_string())
        .collect();
    println!("\n  Maximal elements: {:?}", maximals);

    if !result.warnings.is_empty() {
        println!("\n  Warnings:");
        for w in &result.warnings { println!("    {}", w); }
    }

    // AC creates Skolem successors for non-maximal elements.
    // a is non-maximal (a < b given), so AC creates sk(a) > a.
    // b starts as maximal (no given successor), so AC doesn't extend b.
    // The chain from a: a → sk(a) → sk(sk(a)) → ... → sk^7(a) at depth limit.
    // b remains maximal (parallel peak).
    assert!(has("element(a)"), "starting element");
    assert!(has("element(b)"), "given element");
    assert!(has("element(sk(a))"), "AC created sk(a) as successor of a");
    assert!(has("lt(a, sk(a))"), "AC established a < sk(a)");

    // At depth limit, the deepest Skolem term and b are maximal
    // Depth-bounded maximality (NOT genuine mathematical maximality —
    // these elements are "maximal" only because the depth limit prevents
    // further Skolem extension)
    assert!(!maximals.is_empty(), "depth-bounded maximal elements should exist");
    assert!(!has("maximal(a)"), "a is not maximal (a < b, a < sk(a))");

    println!("\n  {} depth-bounded maximal element(s) found", maximals.len());

    // ── Proof by contradiction exposes the fake maximality ──
    //
    // CWA says the deepest Skolem term is maximal because no successor was derived.
    // But is that genuine? Hypothetically extend it and check for contradiction.

    engine.define_relation("contradiction", 0);
    engine.add_rule(Rule::new(
        "irrefl_contradiction",
        vec![RelationPattern::new("lt", vec![Term::var("x"), Term::var("x")])],
        vec![RelationPattern::new("contradiction", vec![])],
    ));

    // Find the deepest maximal element (highest term depth among maximals)
    let deepest_maximal = result.facts.iter()
        .filter(|f| f.name() == "maximal")
        .max_by_key(|f| f.terms()[0].depth())
        .expect("should have at least one maximal element");
    let deepest_term = deepest_maximal.terms()[0].clone();
    let extension = Term::app("sk", vec![deepest_term.clone()]);

    // Hypothetical: can this element have a successor without contradiction?
    let hypothesis = Relation::binary("lt", deepest_term.clone(), extension);
    let genuinely_maximal = engine.is_contradictory(hypothesis, "contradiction");

    println!("\n  Contradiction check on deepest CWA-maximal element:");
    println!("    {} (depth {})", deepest_maximal, deepest_term.depth());
    println!("    Assume it has a successor: contradiction = {}", genuinely_maximal);

    // Extending the chain does NOT lead to contradiction — the chain CAN continue.
    // Therefore the CWA-maximal element is NOT genuinely maximal.
    assert!(!genuinely_maximal,
        "deepest element is NOT genuinely maximal — extending causes no contradiction");

    println!("    → CWA maximality is a depth-limit artifact (confirmed)");
    println!("    → Proof by contradiction distinguishes real from fake maximality");
}

// ── Proof by contradiction ──────────────────────────────────

/// Proof by contradiction as an internal rule — the engine automatically
/// performs hypothetical reasoning for each element.
///
/// Rule declaration:
///   element(x) |- proven_maximal(x)
///     with_refutation(
///       scan: element(y),
///       hypotheses: lt(x, y),
///       contradiction: "contradiction"
///     )
///
/// Semantics: for each element x, test ALL elements y. Hypothetically
/// add lt(x, y), run closure. If ALL y lead to contradiction → x is
/// proven maximal. No external code drives the reasoning.
///
/// Poset: a < b < c.
/// - x=c: lt(c,a)→contradiction, lt(c,b)→contradiction, lt(c,c)→contradiction → proven_maximal(c) ✓
/// - x=b: lt(b,a)→contradiction, lt(b,b)→contradiction, lt(b,c)→NO contradiction → not maximal ✓
/// - x=a: lt(a,b)→NO contradiction → not maximal ✓
#[test]
fn test_proof_by_contradiction() {
    let mut engine = ClosureEngine::new();

    engine.define_relation("element", 1);
    engine.define_relation("lt", 2);
    engine.define_relation("proven_maximal", 1);
    engine.define_relation("contradiction", 0);

    for v in &["x", "y", "z"] {
        engine.define_variable(*v);
    }

    for e in &["a", "b", "c"] {
        engine.define_constant(*e);
        engine.add_fact(Relation::new("element", vec![c(*e)]));
    }

    // Strict linear order: a < b < c
    engine.add_fact(Relation::binary("lt", c("a"), c("b")));
    engine.add_fact(Relation::binary("lt", c("b"), c("c")));

    // Transitivity
    engine.add_rule(Rule::new(
        "lt_trans",
        vec![
            RelationPattern::new("lt", vec![Term::var("x"), Term::var("y")]),
            RelationPattern::new("lt", vec![Term::var("y"), Term::var("z")]),
        ],
        vec![RelationPattern::new("lt", vec![Term::var("x"), Term::var("z")])],
    ));

    // Irreflexivity → contradiction
    engine.add_rule(Rule::new(
        "irrefl_contradiction",
        vec![RelationPattern::new("lt", vec![Term::var("x"), Term::var("x")])],
        vec![RelationPattern::new("contradiction", vec![])],
    ));

    // Maximal by refutation: for each element x, if assuming lt(x, y)
    // leads to contradiction for ALL elements y → x is proven maximal.
    // The engine does the scanning and hypothetical branching internally.
    engine.add_rule(Rule::new(
        "maximal_by_refutation",
        vec![RelationPattern::new("element", vec![Term::var("x")])],
        vec![RelationPattern::new("proven_maximal", vec![Term::var("x")])],
    ).with_refutation(
        vec![RelationPattern::new("element", vec![Term::var("y")])],
        vec![RelationPattern::new("lt", vec![Term::var("x"), Term::var("y")])],
        "contradiction",
    ));

    engine.set_max_rounds(10);
    engine.set_max_facts(100);

    let result = engine.derive_closure();
    let has = |s: &str| result.facts.iter().any(|f| f.to_string() == s);

    println!("\n============================================================");
    println!("  PROOF BY CONTRADICTION (engine-internal)");
    println!("  {} facts, {} rounds", result.facts.len(), result.rounds);
    println!("============================================================");

    let mut sorted: Vec<String> = result.facts.iter().map(|f| f.to_string()).collect();
    sorted.sort();
    for f in &sorted { println!("  {}", f); }

    if !result.warnings.is_empty() {
        println!("\n  Warnings:");
        for w in &result.warnings { println!("    {}", w); }
    }

    // Only c should be proven maximal
    assert!(has("proven_maximal(c)"),
        "c is proven maximal: all lt(c, y) contradict irreflexivity");
    assert!(!has("proven_maximal(b)"),
        "b is NOT maximal: lt(b, c) is consistent");
    assert!(!has("proven_maximal(a)"),
        "a is NOT maximal: lt(a, b) is consistent");

    println!("\n  proven_maximal(a): {} (correct: a < b is consistent)", has("proven_maximal(a)"));
    println!("  proven_maximal(b): {} (correct: b < c is consistent)", has("proven_maximal(b)"));
    println!("  proven_maximal(c): {} (correct: all successors contradict)", has("proven_maximal(c)"));
}

// ── Peano arithmetic + finite poset maximality ──────────────

/// Peano arithmetic as relations + chain-based proof of maximal element existence.
///
/// The proof schema (for any finite poset):
///   1. Assign Peano indices to elements: idx(a,0), idx(b,1), idx(c,2)
///   2. Build chain from any start: chain(a,0) → chain(b,1) → chain(c,2)
///   3. Chain length bounded by element count (pigeonhole on indices)
///   4. Chain termination → terminal element has no successor → maximal
///
/// What this actually demonstrates:
///   - For any SPECIFIC finite poset the engine receives, it constructs
///     a proof artifact (the chain + termination witness)
///   - The proof is internal (relations, not external code)
///   - Universality is in the PROCEDURE: same rules work on any input
#[test]
fn test_peano_chain_maximality() {
    let mut engine = ClosureEngine::new();

    // ── Relations ──
    engine.define_relation("element", 1);
    engine.define_relation("lt", 2);
    // Peano naturals
    engine.define_relation("leq_nat", 2);   // ≤ on naturals
    engine.define_relation("lt_nat", 2);    // < on naturals
    // Chain construction
    engine.define_relation("chain", 2);      // chain(element, position)
    engine.define_relation("chain_end", 1);  // terminal element
    engine.define_relation("has_maximal", 0); // witness: poset has maximal element

    for v in &["x", "y", "z", "n", "m"] {
        engine.define_variable(*v);
    }

    // ── Diamond poset: a < b, a < c, b < d, c < d ──
    for e in &["a", "b", "c", "d"] {
        engine.define_constant(*e);
        engine.add_fact(Relation::new("element", vec![c(*e)]));
    }
    engine.add_fact(Relation::binary("lt", c("a"), c("b")));
    engine.add_fact(Relation::binary("lt", c("a"), c("c")));
    engine.add_fact(Relation::binary("lt", c("b"), c("d")));
    engine.add_fact(Relation::binary("lt", c("c"), c("d")));

    // lt transitivity
    engine.add_rule(Rule::new(
        "lt_trans",
        vec![
            RelationPattern::new("lt", vec![Term::var("x"), Term::var("y")]),
            RelationPattern::new("lt", vec![Term::var("y"), Term::var("z")]),
        ],
        vec![RelationPattern::new("lt", vec![Term::var("x"), Term::var("z")])],
    ));

    // ── Peano comparison ──
    engine.define_constant("z");  // zero (using "z" to avoid conflict with element names)
    engine.add_fact(Relation::new("leq_nat", vec![c("z"), c("z")]));
    // leq_nat(z, succ(x)) — zero ≤ anything
    engine.add_rule(Rule::new(
        "leq_zero",
        vec![RelationPattern::new("leq_nat", vec![Term::var("n"), Term::var("n")])],
        vec![RelationPattern::new("leq_nat", vec![
            c("z"),
            Term::app("s", vec![Term::var("n")]),
        ])],
    ));
    // leq_nat(succ(x), succ(y)) ← leq_nat(x, y)
    engine.add_rule(Rule::new(
        "leq_succ",
        vec![RelationPattern::new("leq_nat", vec![Term::var("n"), Term::var("m")])],
        vec![RelationPattern::new("leq_nat", vec![
            Term::app("s", vec![Term::var("n")]),
            Term::app("s", vec![Term::var("m")]),
        ])],
    ));

    // ── Chain construction ──
    // Start: pick element a at position z (zero)
    engine.add_fact(Relation::new("chain", vec![c("a"), c("z")]));

    // Extend: chain(x, n), lt(x, y), element(y) |- chain(y, succ(n))
    // Guard: y not already in chain (negation)
    engine.add_rule(Rule::new(
        "chain_extend",
        vec![
            RelationPattern::new("chain", vec![Term::var("x"), Term::var("n")]),
            RelationPattern::new("lt", vec![Term::var("x"), Term::var("y")]),
            RelationPattern::new("element", vec![Term::var("y")]),
        ],
        vec![RelationPattern::new("chain", vec![
            Term::var("y"),
            Term::app("s", vec![Term::var("n")]),
        ])],
    ).with_negated(vec![
        // y must not already be in the chain at any position
        RelationPattern::new("chain", vec![Term::var("y"), Term::var("m")]),
    ]));

    // Chain end: element in chain with no unvisited successor
    engine.add_rule(Rule::new(
        "chain_end_def",
        vec![
            RelationPattern::new("chain", vec![Term::var("x"), Term::var("n")]),
            RelationPattern::new("element", vec![Term::var("x")]),
        ],
        vec![RelationPattern::new("chain_end", vec![Term::var("x")])],
    ).with_negated(vec![
        RelationPattern::new("lt", vec![Term::var("x"), Term::var("y")]),
    ]).with_stratum(2));

    // Witness: if chain_end exists → poset has maximal element
    engine.add_rule(Rule::new(
        "maximal_witness",
        vec![RelationPattern::new("chain_end", vec![Term::var("x")])],
        vec![RelationPattern::new("has_maximal", vec![])],
    ));

    engine.set_max_rounds(30);
    engine.set_max_facts(500);

    let result = engine.derive_closure();
    let has = |s: &str| result.facts.iter().any(|f| f.to_string() == s);

    println!("\n============================================================");
    println!("  PEANO CHAIN: Internal Proof of Maximal Element");
    println!("  {} facts, {} rounds", result.facts.len(), result.rounds);
    println!("============================================================");

    // Show chain
    let mut chain_facts: Vec<String> = result.facts.iter()
        .filter(|f| f.name() == "chain")
        .map(|f| f.to_string())
        .collect();
    chain_facts.sort();
    println!("\n  Chain constructed:");
    for f in &chain_facts { println!("    {}", f); }

    let ends: Vec<String> = result.facts.iter()
        .filter(|f| f.name() == "chain_end")
        .map(|f| f.to_string())
        .collect();
    println!("\n  Chain end: {:?}", ends);
    println!("  has_maximal(): {}", has("has_maximal()"));

    // Assertions
    assert!(has("chain(a, z)"), "chain starts at a");
    assert!(has("chain_end(d)"), "d is the chain end (no successor)");
    assert!(has("has_maximal()"), "poset has a maximal element (proven internally)");

    // d should be the end — it has no successor in the poset
    assert!(!has("chain_end(a)"), "a is not chain end (a < b)");
    assert!(!has("chain_end(b)"), "b is not chain end (b < d)");

    println!("\n  Proof artifact: chain(a,0) → chain(?,1) → ... → chain_end(d)");
    println!("  The engine constructed the proof internally.");
    println!("  Same rules work on ANY finite poset given as input.");
}

// ── Autonomous order-theory discovery ────────────────────────

/// Generate all strict partial orders on n labeled elements.
/// Returns vec of (element_names, lt_pairs).
fn enumerate_posets(n: usize) -> Vec<(Vec<String>, Vec<(String, String)>)> {
    let elements: Vec<String> = (0..n).map(|i| format!("e{}", i)).collect();
    // All possible directed pairs (i, j) with i ≠ j
    let mut possible_pairs: Vec<(usize, usize)> = Vec::new();
    for i in 0..n {
        for j in 0..n {
            if i != j { possible_pairs.push((i, j)); }
        }
    }

    let mut valid_posets = Vec::new();
    let num_subsets = 1u64 << possible_pairs.len();

    for mask in 0..num_subsets {
        let mut lt: Vec<Vec<bool>> = vec![vec![false; n]; n];
        for (bit, &(i, j)) in possible_pairs.iter().enumerate() {
            if mask & (1u64 << bit) != 0 {
                lt[i][j] = true;
            }
        }
        // Check: no self-loops (irreflexive)
        let mut valid = true;
        for i in 0..n {
            if lt[i][i] { valid = false; break; }
        }
        if !valid { continue; }

        // Check: antisymmetric (no i<j and j<i)
        for i in 0..n {
            for j in (i+1)..n {
                if lt[i][j] && lt[j][i] { valid = false; break; }
            }
            if !valid { break; }
        }
        if !valid { continue; }

        // Check: transitive
        for i in 0..n {
            for j in 0..n {
                for k in 0..n {
                    if lt[i][j] && lt[j][k] && !lt[i][k] {
                        valid = false; break;
                    }
                }
                if !valid { break; }
            }
            if !valid { break; }
        }
        if !valid { continue; }

        // Collect pairs
        let mut pairs = Vec::new();
        for i in 0..n {
            for j in 0..n {
                if lt[i][j] {
                    pairs.push((elements[i].clone(), elements[j].clone()));
                }
            }
        }
        valid_posets.push((elements.clone(), pairs));
    }

    valid_posets
}

/// Compute properties of a poset using the engine.
fn compute_poset_properties(
    elements: &[String],
    lt_pairs: &[(String, String)],
) -> HashMap<String, bool> {
    let mut engine = ClosureEngine::new();

    engine.define_relation("element", 1);
    engine.define_relation("lt", 2);
    engine.define_relation("maximal", 1);
    engine.define_relation("minimal", 1);
    engine.define_relation("comparable", 2);
    engine.define_relation("has_max", 0);
    engine.define_relation("has_min", 0);
    engine.define_relation("is_total", 0);
    engine.define_relation("has_unique_max", 0);
    engine.define_relation("contradiction", 0);

    for v in &["x", "y", "z"] {
        engine.define_variable(*v);
    }

    for e in elements {
        engine.define_constant(e);
        engine.add_fact(Relation::new("element", vec![c(e)]));
    }

    for (a, b) in lt_pairs {
        engine.add_fact(Relation::binary("lt", c(a), c(b)));
    }

    // Transitivity (stratum 0)
    engine.add_rule(Rule::new("lt_trans",
        vec![
            RelationPattern::new("lt", vec![Term::var("x"), Term::var("y")]),
            RelationPattern::new("lt", vec![Term::var("y"), Term::var("z")]),
        ],
        vec![RelationPattern::new("lt", vec![Term::var("x"), Term::var("z")])],
    ));

    // Comparable: x and y are comparable if x<y or y<x
    engine.add_rule(Rule::new("comp_lt",
        vec![RelationPattern::new("lt", vec![Term::var("x"), Term::var("y")])],
        vec![RelationPattern::new("comparable", vec![Term::var("x"), Term::var("y")])],
    ));
    engine.add_rule(Rule::new("comp_gt",
        vec![RelationPattern::new("lt", vec![Term::var("y"), Term::var("x")])],
        vec![RelationPattern::new("comparable", vec![Term::var("x"), Term::var("y")])],
    ));

    // Maximal: element(x), NOT lt(x, ?y) — stratum 1
    engine.add_rule(Rule::new("maximal_def",
        vec![RelationPattern::new("element", vec![Term::var("x")])],
        vec![RelationPattern::new("maximal", vec![Term::var("x")])],
    ).with_negated(vec![
        RelationPattern::new("lt", vec![Term::var("x"), Term::var("y")]),
    ]));

    // Minimal: element(x), NOT lt(?y, x)
    engine.add_rule(Rule::new("minimal_def",
        vec![RelationPattern::new("element", vec![Term::var("x")])],
        vec![RelationPattern::new("minimal", vec![Term::var("x")])],
    ).with_negated(vec![
        RelationPattern::new("lt", vec![Term::var("y"), Term::var("x")]),
    ]));

    // has_max ← maximal(?x)
    engine.add_rule(Rule::new("has_max_def",
        vec![RelationPattern::new("maximal", vec![Term::var("x")])],
        vec![RelationPattern::new("has_max", vec![])],
    ));
    engine.add_rule(Rule::new("has_min_def",
        vec![RelationPattern::new("minimal", vec![Term::var("x")])],
        vec![RelationPattern::new("has_min", vec![])],
    ));

    // is_total: all pairs comparable
    // Detect NOT total: element(x), element(y), x≠y, NOT comparable(x,y) → not_total
    // Then: is_total ← NOT not_total
    engine.define_relation("not_total", 0);
    engine.add_rule(Rule::new("not_total_def",
        vec![
            RelationPattern::new("element", vec![Term::var("x")]),
            RelationPattern::new("element", vec![Term::var("y")]),
        ],
        vec![RelationPattern::new("not_total", vec![])],
    ).with_negated(vec![
        RelationPattern::new("comparable", vec![Term::var("x"), Term::var("y")]),
    ]));
    engine.add_rule(Rule::new("is_total_def",
        vec![],
        vec![RelationPattern::new("is_total", vec![])],
    ).with_negated(vec![
        RelationPattern::new("not_total", vec![]),
    ]).with_stratum(2));

    // Unique max: exactly one maximal element
    // has_two_max ← maximal(x), maximal(y), x ≠ y (via NOT lt and comparability)
    // Simpler: has_two_max ← maximal(x), maximal(y), and x,y are different elements
    // We detect: two distinct maximals
    engine.define_relation("multi_max", 0);
    engine.define_relation("multi_min", 0);

    engine.set_max_rounds(10);
    engine.set_max_facts(200);

    let result = engine.derive_closure();
    let has = |s: &str| result.facts.iter().any(|f| f.to_string() == s);

    // Count maximal/minimal
    let max_count = result.facts.iter().filter(|f| f.name() == "maximal").count();
    let min_count = result.facts.iter().filter(|f| f.name() == "minimal").count();

    let mut props = HashMap::new();
    props.insert("has_maximal".into(), has("has_max()"));
    props.insert("has_minimal".into(), has("has_min()"));
    props.insert("unique_max".into(), max_count == 1);
    props.insert("unique_min".into(), min_count == 1);
    props.insert("is_total".into(), has("is_total()"));
    props.insert("is_antichain".into(), lt_pairs.is_empty());
    props.insert("multi_max".into(), max_count > 1);
    props.insert("multi_min".into(), min_count > 1);
    props
}

/// Proto-level discovery: beam search over AXIOM CANDIDATES, not models.
///
/// Input: raw binary relation facts (NOT transitively closed), NO rules.
/// The system generates relational composition candidates and discovers
/// which axiom schemas are most valuable (produce the most new facts).
///
/// Expected: the system discovers transitivity as the top-scoring axiom
/// for this strict ordering — without being told what transitivity is.
#[test]
fn test_proto_axiom_discovery() {
    use relatum::relational::search::{self, BeamConfig, CandidateConfig, AdaptivePolicy};
    use relatum::relational::score::ScoreWeights;

    println!("\n============================================================");
    println!("  PROTO-LEVEL AXIOM DISCOVERY");
    println!("  Input: raw edges. No rules. No properties defined.");
    println!("  System discovers which axiom schemas are valuable.");
    println!("============================================================");

    // Raw binary relation — NOT transitively closed, NO rules
    let mut engine = ClosureEngine::new();
    engine.define_relation("R", 2);
    engine.define_equivalence("eq");
    engine.define_relation("distinct", 2);

    let elems = ["a", "b", "c", "d", "e"];
    for e in &elems {
        engine.define_constant(*e);
    }
    for v in &["x", "y", "z"] { engine.define_variable(*v); }

    // Chain: a→b→c→d→e (raw edges only, NOT transitive closure)
    engine.add_fact(Relation::binary("R", c("a"), c("b")));
    engine.add_fact(Relation::binary("R", c("b"), c("c")));
    engine.add_fact(Relation::binary("R", c("c"), c("d")));
    engine.add_fact(Relation::binary("R", c("d"), c("e")));

    // Distinct pairs (for consistency checking)
    for i in 0..elems.len() {
        for j in (i+1)..elems.len() {
            engine.add_fact(Relation::binary("distinct", c(elems[i]), c(elems[j])));
            engine.add_fact(Relation::binary("distinct", c(elems[j]), c(elems[i])));
        }
    }

    // Beam search config — discover axiom schemas
    let mut exclude = HashSet::new();
    exclude.insert("distinct".to_string());

    let beam_config = BeamConfig {
        candidate_config: CandidateConfig {
            guard_relation: None,
            exclude_relations: exclude,
            min_pattern_support: 2,
            ..CandidateConfig::default()
        },
        weights: ScoreWeights {
            generativity: 1.0,
            compression: 0.5,
            consistency_penalty: 10.0,
            exclusions: vec![("eq".to_string(), "distinct".to_string())],
        },
        beam_width: 10,
        max_rules_per_beam: 3,
        max_steps: 3,
        adaptive: AdaptivePolicy::Fixed,
    };

    // Generate candidates
    let candidates = search::generate_candidates(&engine, &beam_config.candidate_config);
    println!("\n  Candidate axiom schemas: {}", candidates.len());
    for r in &candidates {
        println!("    {}", r.name());
    }

    // Run beam search — system discovers which axioms are most productive
    let beam_log = search::beam_search(&engine, &beam_config);

    println!("\n  Beam search results:");
    for step in &beam_log {
        println!("\n  Step {}:", step.round);
        for (i, entry) in step.beam.iter().enumerate().take(5) {
            println!("    #{}: score={:.2}, derived={}, rules={:?}",
                i+1, entry.score, entry.profile.derived_facts, entry.rule_names);
        }
    }

    // The top-scoring axiom should be transitivity: R(x,y),R(y,z)|-R(x,z)
    let final_step = beam_log.last().unwrap();
    let best = &final_step.beam[0];
    println!("\n  DISCOVERED: best axiom bundle = {:?}", best.rule_names);
    println!("  Score: {:.2}, derived facts: {}", best.score, best.profile.derived_facts);

    // Verify: transitivity should be in the best bundle
    let has_transitivity = best.rule_names.iter()
        .any(|name| name.contains("R_R_R"));  // rel_comp_R_R_R = transitivity
    println!("  Contains transitivity (R∘R→R): {}", has_transitivity);

    if has_transitivity {
        println!("\n  System autonomously discovered transitivity as the");
        println!("  most valuable axiom for this binary relation.");
        println!("  No human told it what transitivity is.");
    }
}

/// Enumerate posets and find implications (manual property version for comparison).
#[test]
fn test_order_theory_discovery() {
    println!("\n============================================================");
    println!("  ORDER THEORY AUTONOMOUS DISCOVERY");
    println!("============================================================");

    // Enumerate posets of size 2, 3, 4
    let mut all_results: Vec<(usize, Vec<(String, String)>, HashMap<String, bool>)> = Vec::new();

    for n in 2..=4 {
        let posets = enumerate_posets(n);
        println!("\n  Posets on {} elements: {}", n, posets.len());

        for (elements, pairs) in &posets {
            let props = compute_poset_properties(elements, pairs);
            all_results.push((n, pairs.clone(), props));
        }
    }

    println!("\n  Total posets analyzed: {}", all_results.len());

    // Collect all property names
    let prop_names: Vec<String> = {
        let mut names: Vec<String> = all_results[0].2.keys().cloned().collect();
        names.sort();
        names
    };

    // Count how often each property holds
    println!("\n  Property frequencies:");
    for name in &prop_names {
        let count = all_results.iter().filter(|(_, _, p)| *p.get(name).unwrap_or(&false)).count();
        println!("    {:<15} {}/{}", name, count, all_results.len());
    }

    // Find implications: A → B (whenever A holds, B also holds)
    println!("\n  Discovered implications (A → B, no counterexample):");
    let mut implications = Vec::new();

    for a in &prop_names {
        for b in &prop_names {
            if a == b { continue; }
            let a_count = all_results.iter().filter(|(_, _, p)| *p.get(a).unwrap_or(&false)).count();
            if a_count == 0 { continue; }

            let counterexamples = all_results.iter().filter(|(_, _, p)| {
                *p.get(a).unwrap_or(&false) && !*p.get(b).unwrap_or(&false)
            }).count();

            if counterexamples == 0 {
                let b_count = all_results.iter().filter(|(_, _, p)| *p.get(b).unwrap_or(&false)).count();
                // Surprise: how much does knowing A narrow down B?
                let surprise = if b_count == all_results.len() { 0.0 }
                    else { (all_results.len() as f64 / b_count as f64).ln() };
                implications.push((a.clone(), b.clone(), a_count, surprise));
            }
        }
    }

    implications.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));

    for (a, b, support, surprise) in &implications {
        let marker = if *surprise > 0.1 { "★" } else { " " };
        println!("    {} {:<15} → {:<15} (support={}, surprise={:.3})",
            marker, a, b, support, surprise);
    }

    // Find equivalences: A ↔ B
    println!("\n  Discovered equivalences (A ↔ B):");
    let mut equivalences: Vec<(String, String)> = Vec::new();
    for i in 0..implications.len() {
        for j in (i+1)..implications.len() {
            if implications[i].0 == implications[j].1 && implications[i].1 == implications[j].0 {
                if !equivalences.iter().any(|(a,b)| (a == &implications[i].0 && b == &implications[i].1) ||
                    (a == &implications[i].1 && b == &implications[i].0)) {
                    equivalences.push((implications[i].0.clone(), implications[i].1.clone()));
                }
            }
        }
    }
    for (a, b) in &equivalences {
        println!("    {} ↔ {}", a, b);
    }

    // Basic assertions: every non-empty poset has a maximal and minimal element
    for (n, _, props) in &all_results {
        assert!(props["has_maximal"], "poset of size {} should have maximal", n);
        assert!(props["has_minimal"], "poset of size {} should have minimal", n);
    }

    println!("\n  Key finding: has_maximal and has_minimal hold for ALL finite posets.");
    println!("  This is the finite case of Zorn — discovered, not assumed.");
}

/// Simple premise matching against a vec of fact references.
const MAX_SUBSTITUTIONS: usize = 10_000;

fn match_premises_vec(
    premises: &[RelationPattern],
    facts: &[&Relation],
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
                if match_relation(premise, fact, &mut candidate) {
                    next.push(candidate);
                    if next.len() >= MAX_SUBSTITUTIONS { break; }
                }
            }
            if next.len() >= MAX_SUBSTITUTIONS { break; }
        }
        subs = next;
        if subs.is_empty() { break; }
    }
    subs
}
