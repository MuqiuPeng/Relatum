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
    // CWA says sk^7(a) is maximal because no lt(sk^7(a), ?y) was derived.
    // But is that genuine? Test: does assuming lt(sk^7(a), sk^8(a)) lead
    // to contradiction?

    // Add irreflexivity contradiction rule to the engine
    engine.define_relation("contradiction", 0);
    engine.add_rule(Rule::new(
        "irrefl_contradiction",
        vec![RelationPattern::new("lt", vec![Term::var("x"), Term::var("x")])],
        vec![RelationPattern::new("contradiction", vec![])],
    ));

    // Build sk^7(a) and sk^8(a) terms
    let mut sk7 = c("a");
    for _ in 0..7 {
        sk7 = Term::app("sk", vec![sk7]);
    }
    let sk8 = Term::app("sk", vec![sk7.clone()]);

    // Hypothetical: can sk^7(a) have a successor without contradiction?
    let hypothesis = Relation::binary("lt", sk7.clone(), sk8.clone());
    let genuinely_maximal = engine.is_contradictory(hypothesis.clone(), "contradiction");

    println!("\n  Contradiction check on depth-boundary \"maximal\" element:");
    println!("    Assume lt(sk^7(a), sk^8(a)): contradiction = {}", genuinely_maximal);

    // sk^7(a) < sk^8(a) does NOT lead to contradiction — the chain CAN continue.
    // Therefore sk^7(a) is NOT genuinely maximal. CWA was wrong.
    assert!(!genuinely_maximal,
        "sk^7(a) is NOT genuinely maximal — extending the chain causes no contradiction");

    // Compare with a genuinely maximal element in a finite order.
    // In {a < b < c}, c IS genuinely maximal: every extension contradicts irreflexivity.
    println!("    → CWA maximality is a depth-limit artifact (confirmed)");
    println!("    → Proof by contradiction distinguishes real from fake maximality");
}

// ── Proof by contradiction ──────────────────────────────────

/// Prove by contradiction: in a finite strict linear order, assuming
/// "every element has a successor" leads to a cycle, which contradicts
/// the irreflexivity of strict ordering.
///
/// Poset: a < b < c (finite, 3 elements).
/// Assume: every element has a strict successor (no maximal element).
/// Then c must have a successor — but the only elements are {a, b, c},
/// and c < a would create a cycle. With transitivity, c < a < b < c → c < c.
/// But lt is irreflexive → contradiction.
#[test]
fn test_proof_by_contradiction() {
    let mut engine = ClosureEngine::new();

    engine.define_relation("element", 1);
    engine.define_relation("lt", 2);
    engine.define_relation("has_successor", 1);
    engine.define_relation("contradiction", 0);  // 0-ary: flag

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

    // Contradiction: lt(x, x) is impossible (irreflexivity)
    engine.add_rule(Rule::new(
        "irrefl_contradiction",
        vec![RelationPattern::new("lt", vec![Term::var("x"), Term::var("x")])],
        vec![RelationPattern::new("contradiction", vec![])],
    ));

    engine.set_max_rounds(10);
    engine.set_max_facts(100);

    println!("\n============================================================");
    println!("  PROOF BY CONTRADICTION: Finite Order Has Maximal Element");
    println!("============================================================");

    // First: verify the base engine has no contradiction
    let base_result = engine.derive_closure();
    let has_base = |s: &str| base_result.facts.iter().any(|f| f.to_string() == s);
    assert!(!has_base("contradiction()"), "base should have no contradiction");
    println!("\n  Base: {} facts, no contradiction ✓", base_result.facts.len());

    // Hypothesis: "every element has a successor" → c must go somewhere
    // The only candidates are a, b, c. Since a < b < c are given, adding
    // c < a (the only new option) should create a cycle → contradiction.
    let assumption = Relation::binary("lt", c("c"), c("a"));
    let contradicts = engine.is_contradictory(assumption.clone(), "contradiction");

    println!("  Assume lt(c, a): contradiction = {}", contradicts);
    assert!(contradicts, "lt(c, a) + transitivity → lt(c, c) → contradiction");

    // Also test: c < b
    let contradicts_b = engine.is_contradictory(
        Relation::binary("lt", c("c"), c("b")), "contradiction");
    println!("  Assume lt(c, b): contradiction = {}", contradicts_b);
    assert!(contradicts_b, "lt(c, b) + lt(b, c) → lt(c, c) → contradiction");

    // Also test: c < c directly
    let contradicts_self = engine.is_contradictory(
        Relation::binary("lt", c("c"), c("c")), "contradiction");
    println!("  Assume lt(c, c): contradiction = {}", contradicts_self);
    assert!(contradicts_self, "lt(c, c) directly contradicts irreflexivity");

    // Conclusion: c cannot have any successor in {a, b, c}
    // → c is maximal (proven by exhaustive contradiction, not by closed-world negation)
    println!("\n  All possible successors for c lead to contradiction.");
    println!("  Therefore c is maximal — proven by contradiction, not by negation.");
    println!("  This is stronger than closed-world negation: it proves");
    println!("  non-existence rather than absence-from-derivation.");
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
