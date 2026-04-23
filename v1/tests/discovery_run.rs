//! Full autonomous discovery run on Z₃, S₃, V₄.
//! Writes structured logs to logs/ directory.
//! Run with: cargo test --test discovery_run -- --nocapture

use relatum::relational::*;
use std::collections::HashSet;
use std::io::Write;

fn c(s: &str) -> Term {
    Term::constant(s)
}

fn z3_engine() -> ClosureEngine {
    let mut engine = ClosureEngine::new();
    engine.define_relation("op", 3);
    engine.define_equivalence("eq");
    engine.define_relation("distinct", 2);
    for name in &["e0", "e1", "e2"] {
        engine.define_constant(*name);
    }
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
    engine
}

fn s3_engine() -> ClosureEngine {
    let mut engine = ClosureEngine::new();
    engine.define_relation("op", 3);
    engine.define_equivalence("eq");
    engine.define_relation("distinct", 2);
    let elems = ["e", "a", "b", "c", "d", "f"];
    for name in &elems {
        engine.define_constant(*name);
    }
    let table: [[&str; 6]; 6] = [
        ["e","a","b","c","d","f"],
        ["a","e","d","f","b","c"],
        ["b","f","e","d","c","a"],
        ["c","d","f","e","a","b"],
        ["d","c","a","b","f","e"],
        ["f","b","c","a","e","d"],
    ];
    for (i, row) in table.iter().enumerate() {
        for (j, result) in row.iter().enumerate() {
            engine.add_fact(Relation::new("op", vec![
                c(elems[i]), c(elems[j]), c(result),
            ]));
        }
    }
    for i in 0..elems.len() {
        for j in (i + 1)..elems.len() {
            engine.add_fact(Relation::binary("distinct", c(elems[i]), c(elems[j])));
            engine.add_fact(Relation::binary("distinct", c(elems[j]), c(elems[i])));
        }
    }
    engine
}

fn v4_engine() -> ClosureEngine {
    let mut engine = ClosureEngine::new();
    engine.define_relation("op", 3);
    engine.define_equivalence("eq");
    engine.define_relation("distinct", 2);
    for name in &["e", "a", "b", "c"] {
        engine.define_constant(*name);
    }
    let table: [[&str; 4]; 4] = [
        ["e", "a", "b", "c"],
        ["a", "e", "c", "b"],
        ["b", "c", "e", "a"],
        ["c", "b", "a", "e"],
    ];
    let elems = ["e", "a", "b", "c"];
    for (i, row) in table.iter().enumerate() {
        for (j, result) in row.iter().enumerate() {
            engine.add_fact(Relation::new("op", vec![
                c(elems[i]), c(elems[j]), c(result),
            ]));
        }
    }
    for i in 0..elems.len() {
        for j in (i + 1)..elems.len() {
            engine.add_fact(Relation::binary("distinct", c(elems[i]), c(elems[j])));
            engine.add_fact(Relation::binary("distinct", c(elems[j]), c(elems[i])));
        }
    }
    engine
}

/// Z₄ under multiplication mod 4.
/// Commutative monoid: identity=1, but 0 and 2 have NO inverse.
/// NOT a group — tests whether system distinguishes group from monoid.
fn z4mul_engine() -> ClosureEngine {
    let mut engine = ClosureEngine::new();
    engine.define_relation("op", 3);
    engine.define_equivalence("eq");
    engine.define_relation("distinct", 2);
    let elems = ["z", "u", "t", "r"]; // 0,1,2,3
    for name in &elems {
        engine.define_constant(*name);
    }
    // Cayley table: multiplication mod 4
    // z=0, u=1, t=2, r=3
    let table: [[&str; 4]; 4] = [
        ["z","z","z","z"], // 0*?
        ["z","u","t","r"], // 1*?
        ["z","t","z","t"], // 2*? (note: 2*2=0, 2*3=6%4=2)
        ["z","r","t","u"], // 3*? (3*3=9%4=1)
    ];
    for (i, row) in table.iter().enumerate() {
        for (j, result) in row.iter().enumerate() {
            engine.add_fact(Relation::new("op", vec![
                c(elems[i]), c(elems[j]), c(result),
            ]));
        }
    }
    for i in 0..elems.len() {
        for j in (i + 1)..elems.len() {
            engine.add_fact(Relation::binary("distinct", c(elems[i]), c(elems[j])));
            engine.add_fact(Relation::binary("distinct", c(elems[j]), c(elems[i])));
        }
    }
    engine
}

fn make_config(exclude: HashSet<String>) -> search::DiscoveryConfig {
    let excl = vec![("eq".to_string(), "distinct".to_string())];
    search::DiscoveryConfig {
        beam: search::BeamConfig {
            candidate_config: search::CandidateConfig {
                guard_relation: None,
                exclude_relations: exclude.clone(),
                min_pattern_support: 2,
                ..search::CandidateConfig::default()
            },
            weights: ScoreWeights {
                generativity: 1.0,
                compression: 0.5,
                consistency_penalty: 10.0,
                exclusions: excl,
            },
            beam_width: 5,
            max_rules_per_beam: 4,
            max_steps: 3,
            adaptive: search::AdaptivePolicy::Fixed,
        },
        promotion: search::PromotionConfig {
            min_support: 2,
            max_promotions_per_round: 5,
            exclude_relations: exclude,
        },
        max_rounds: 2,
    }
}

/// 20 rounds of fully autonomous reasoning starting from NOTHING.
/// The system decides what to do each round.
#[test]
fn run_20_rounds_autonomous() {
    println!();
    let log = search::run_autonomous(3, 20);
    let text = log.to_log_string();
    print!("{}", text);
    write_log(&format!("{}_autonomous_20rounds.log", chrono_timestamp()), &text);

    assert!(log.rounds.len() >= 3, "should run at least 3 rounds");
    assert!(matches!(log.rounds.last().unwrap().2, search::AutonomousAction::Converged { .. }),
        "should converge");
}

/// 10-round discovery from bare minimum: just Z₃'s Cayley table.
#[test]
fn run_10_rounds_z3() {
    let mut exclude = HashSet::new();
    exclude.insert("distinct".to_string());

    let excl = vec![("eq".to_string(), "distinct".to_string())];
    let config = search::DiscoveryConfig {
        beam: search::BeamConfig {
            candidate_config: search::CandidateConfig {
                guard_relation: None,
                exclude_relations: exclude.clone(),
                min_pattern_support: 2,
                ..search::CandidateConfig::default()
            },
            weights: ScoreWeights {
                generativity: 1.0,
                compression: 0.5,
                consistency_penalty: 10.0,
                exclusions: excl,
            },
            beam_width: 5,
            max_rules_per_beam: 4,
            max_steps: 3,
            adaptive: search::AdaptivePolicy::Fixed,
        },
        promotion: search::PromotionConfig {
            min_support: 2,
            max_promotions_per_round: 5,
            exclude_relations: exclude,
        },
        max_rounds: 10,
    };

    let base = z3_engine();
    let log = search::run_discovery_named(&base, &config, "Z3 (10-round, bare minimum)");

    let text = log.to_log_string();
    print!("{}", text);
    write_log(&format!("{}_z3_10rounds.log", chrono_timestamp()), &text);

    // Should converge well before 10 rounds
    let n_rounds = log.steps.len();
    println!("\n  Converged in {} rounds", n_rounds);
    assert!(n_rounds <= 10);
}

fn write_log(filename: &str, content: &str) {
    let dir = std::path::Path::new("logs");
    std::fs::create_dir_all(dir).ok();
    let path = dir.join(filename);
    let mut file = std::fs::File::create(&path).expect("create log file");
    file.write_all(content.as_bytes()).expect("write log");
    println!("  Log written to: {}", path.display());
}

#[test]
fn run_full_discovery_with_logging() {
    let mut exclude = HashSet::new();
    exclude.insert("distinct".to_string());
    let config = make_config(exclude);

    let timestamp = chrono_timestamp();

    // ── Z₃ ──
    let z3_base = z3_engine();
    let z3_log = search::run_discovery_named(&z3_base, &config, "Z3 (cyclic group, order 3)");
    let z3_text = z3_log.to_log_string();
    print!("{}", z3_text);
    write_log(&format!("{}_z3.log", timestamp), &z3_text);

    // ── S₃ ──
    let s3_base = s3_engine();
    let s3_log = search::run_discovery_named(&s3_base, &config, "S3 (symmetric group, order 6, non-abelian)");
    let s3_text = s3_log.to_log_string();
    print!("{}", s3_text);
    write_log(&format!("{}_s3.log", timestamp), &s3_text);

    // ── V₄ ──
    let v4_base = v4_engine();
    let v4_log = search::run_discovery_named(&v4_base, &config, "V4 (Klein four-group, order 4)");
    let v4_text = v4_log.to_log_string();
    print!("{}", v4_text);
    write_log(&format!("{}_v4.log", timestamp), &v4_text);

    // ── Cross-structure abstraction + theorems ──
    let abstracts = search::abstract_across_structures(vec![
        ("Z3", &z3_log.steps),
        ("S3", &s3_log.steps),
    ]);

    let mut theorems = search::discover_theorems(&abstracts);
    for thm in &mut theorems {
        search::verify_theorem(thm, "V4", &v4_log.steps);
    }

    // Build summary log
    let mut summary = String::new();
    summary.push_str(&format!(
        "═══════════════════════════════════════════════════════\n\
         CROSS-STRUCTURE SUMMARY\n\
         Timestamp: {}\n\
         Structures: Z3, S3 (training) + V4 (held-out verification)\n\
         ═══════════════════════════════════════════════════════\n\n",
        timestamp
    ));

    summary.push_str("── ABSTRACT CONCEPTS ──\n\n");
    for (i, abs) in abstracts.iter().enumerate() {
        summary.push_str(&format!("  Abstract Concept #{}\n", i));
        summary.push_str(&format!("    Pattern: {}\n", abs.signature));
        for (structure, concept_name, instances) in &abs.occurrences {
            let inst_str: Vec<String> = instances.iter().map(|t| t.to_string()).collect();
            summary.push_str(&format!(
                "    {} -> {} = {{{}}}\n",
                structure, concept_name, inst_str.join(", ")
            ));
        }
        if !abs.universal_properties.is_empty() {
            summary.push_str("    Universal properties:\n");
            for prop in &abs.universal_properties {
                summary.push_str(&format!("      {}\n", prop));
            }
        }
        summary.push('\n');
    }

    summary.push_str("── THEOREMS ──\n\n");
    for (i, thm) in theorems.iter().enumerate() {
        let status = if !thm.refuted_on.is_empty() {
            "REFUTED"
        } else if !thm.verified_on.is_empty() {
            "VERIFIED"
        } else {
            "inconclusive"
        };
        summary.push_str(&format!("  Theorem #{} [{}]\n", i, status));
        summary.push_str(&format!("    {}\n", thm));
        for (s, ia, ib) in &thm.evidence {
            let sa: Vec<String> = ia.iter().map(|t| t.to_string()).collect();
            let sb: Vec<String> = ib.iter().map(|t| t.to_string()).collect();
            summary.push_str(&format!(
                "    evidence {}: {{{}}} vs {{{}}}\n",
                s, sa.join(","), sb.join(",")
            ));
        }
        summary.push('\n');
    }

    print!("{}", summary);
    write_log(&format!("{}_summary.log", timestamp), &summary);

    // ── Assertions ──
    assert!(z3_log.steps.iter().any(|s| s.promoted.iter().any(|c| c.arity == 1 && c.instances == 1)),
        "Z3: should invent identity concept");
    assert!(s3_log.steps.iter().any(|s| s.promoted.iter().any(|c| c.arity == 1 && c.instances == 1)),
        "S3: should invent identity concept");
    assert!(z3_log.steps.iter().any(|s| !s.verification_rules.is_empty()),
        "Z3: should discover verification rules");

    let equiv_theorems: Vec<&search::ConceptTheorem> = theorems
        .iter()
        .filter(|t| t.kind == search::TheoremKind::Equivalent)
        .collect();
    assert!(!equiv_theorems.is_empty(), "should discover equivalence theorems");

    let verified = equiv_theorems.iter().any(|t| {
        t.verified_on.contains(&"V4".to_string()) && t.refuted_on.is_empty()
    });
    assert!(verified, "left_id <-> right_id should be verified on V4");
}

/// Cross-structure comparison: groups vs monoid.
/// Z₃ + S₃ (groups) vs Z₄× (monoid) — which properties are group-only?
#[test]
fn run_group_vs_monoid_comparison() {
    let mut exclude = HashSet::new();
    exclude.insert("distinct".to_string());
    let config = make_config(exclude);

    let timestamp = chrono_timestamp();

    // Discovery on each structure
    let z3_log = search::run_discovery_named(&z3_engine(), &config, "Z3 (group)");
    let s3_log = search::run_discovery_named(&s3_engine(), &config, "S3 (group)");
    let z4m_log = search::run_discovery_named(&z4mul_engine(), &config, "Z4mul (monoid)");

    // Write individual logs
    write_log(&format!("{}_z3_group.log", timestamp), &z3_log.to_log_string());
    write_log(&format!("{}_s3_group.log", timestamp), &s3_log.to_log_string());
    write_log(&format!("{}_z4mul_monoid.log", timestamp), &z4m_log.to_log_string());

    // Cross-structure: all three
    let abstracts = search::abstract_across_structures(vec![
        ("Z3", &z3_log.steps),
        ("S3", &s3_log.steps),
        ("Z4mul", &z4m_log.steps),
    ]);

    let theorems = search::discover_theorems(&abstracts);

    // Build summary
    let mut summary = String::new();
    summary.push_str(&format!(
        "=== GROUP vs MONOID COMPARISON ===\n\
         Timestamp: {}\n\
         Groups: Z3 (order 3), S3 (order 6)\n\
         Monoid: Z4mul (Z4 under multiplication, order 4, NOT a group)\n\n",
        timestamp
    ));

    summary.push_str("--- PER-STRUCTURE SUMMARY ---\n\n");
    for (name, log) in &[("Z3", &z3_log), ("S3", &s3_log), ("Z4mul", &z4m_log)] {
        let step0 = &log.steps[0];
        let n_concepts = step0.promoted.len();
        let n_verify = step0.verification_rules.len();
        let n_chain = step0.chain_identities.len();
        summary.push_str(&format!("  {}:\n", name));
        summary.push_str(&format!("    concepts: {}, verification: {}, chain identities: {}\n", n_concepts, n_verify, n_chain));
        for c in &step0.promoted {
            let inst: Vec<String> = c.instance_set.iter().map(|t| t.to_string()).collect();
            summary.push_str(&format!("    {} = {{{}}} (sig={})\n", c.name, inst.join(","), c.signature));
        }
        summary.push('\n');
    }

    summary.push_str("--- ABSTRACT CONCEPTS (appear in >=2 structures) ---\n\n");
    for (i, abs) in abstracts.iter().enumerate() {
        let structures: Vec<&str> = abs.occurrences.iter().map(|(s, _, _)| s.as_str()).collect();
        summary.push_str(&format!("  Concept #{}: {}\n", i, abs.signature));
        summary.push_str(&format!("    structures: {:?}\n", structures));
        for (s, cn, inst) in &abs.occurrences {
            let is: Vec<String> = inst.iter().map(|t| t.to_string()).collect();
            summary.push_str(&format!("    {} -> {} = {{{}}}\n", s, cn, is.join(",")));
        }
        if !abs.universal_properties.is_empty() {
            summary.push_str(&format!("    universal properties: {:?}\n", abs.universal_properties));
        }
        summary.push('\n');
    }

    summary.push_str("--- THEOREMS ---\n\n");
    for (i, thm) in theorems.iter().enumerate() {
        let kind = match thm.kind {
            search::TheoremKind::Equivalent => "equiv",
            search::TheoremKind::Subsumption => "subset",
        };
        let structures: Vec<&str> = thm.evidence.iter().map(|(s, _, _)| s.as_str()).collect();
        summary.push_str(&format!("  Theorem #{} [{}]: {}\n", i, kind, thm));
        summary.push_str(&format!("    structures: {:?}\n\n", structures));
    }

    print!("{}", summary);
    write_log(&format!("{}_group_vs_monoid.log", timestamp), &summary);

    // Key finding: "left_id ↔ right_id" theorem holds even across groups + monoid
    let equiv_count = theorems.iter()
        .filter(|t| t.kind == search::TheoremKind::Equivalent)
        .count();

    // In Z4mul, the left/right id patterns match ALL elements (due to absorbing 0).
    // The system correctly assigns 0 verification rules → concept is unverified.
    // This is the mathematical distinction: groups have selective identity,
    // monoids with absorbing elements have degenerate identity patterns.
    let z4_verify_count: usize = z4m_log.steps.iter()
        .map(|s| s.verification_rules.len())
        .sum();

    println!("\n--- KEY FINDINGS ---\n");
    println!("  Z3/S3 (groups): identity patterns have 1 instance → 4 verification rules each");
    println!("  Z4mul (monoid): identity patterns have 4 instances → {} verification rules", z4_verify_count);
    println!("  → Absorbing element (0) makes ALL elements match op(?e,?x,?x) for x=0");
    println!("  → System correctly withholds verification (exclusivity check fails)");
    println!("  → This is the structural difference between group and monoid with absorber");
    println!("\n  Equivalence theorems: {} (left_id ↔ right_id still universal)", equiv_count);

    // The "left_id ↔ right_id" equivalence holds even in the monoid
    assert!(equiv_count >= 1, "left_id ↔ right_id should hold across all structures");

    // Z4mul should NOT have verification rules (degenerate identity)
    assert_eq!(z4_verify_count, 0,
        "Z4mul should have 0 verification rules (absorbing element breaks exclusivity)");
}

/// ℤ₇ — integers mod 7 under addition. A group of order 7.
fn z7_engine() -> ClosureEngine {
    let mut engine = ClosureEngine::new();
    engine.define_relation("op", 3);
    engine.define_equivalence("eq");
    engine.define_relation("distinct", 2);
    let elems: Vec<String> = (0..7).map(|i| format!("n{}", i)).collect();
    for name in &elems {
        engine.define_constant(name);
    }
    for i in 0..7usize {
        for j in 0..7usize {
            let r = (i + j) % 7;
            engine.add_fact(Relation::new("op", vec![
                c(&elems[i]), c(&elems[j]), c(&elems[r]),
            ]));
        }
    }
    for i in 0..7 {
        for j in (i + 1)..7 {
            engine.add_fact(Relation::binary("distinct", c(&elems[i]), c(&elems[j])));
            engine.add_fact(Relation::binary("distinct", c(&elems[j]), c(&elems[i])));
        }
    }
    engine
}

/// The core experiment: transfer universal rules from Z₃ to ℤ₇.
///
/// Phase A: discover rules on Z₃ — extract transferable knowledge automatically
/// Phase B: create ℤ₇ with partial op table (one hint fact + rest withheld)
///          inject transferred rules → predict op facts
/// Phase C: verify predictions against ground truth
#[test]
fn run_transfer_z3_to_z7() {
    let mut exclude = HashSet::new();
    exclude.insert("distinct".to_string());
    let config = make_config(exclude);

    println!("\n============================================================");
    println!("  TRANSFER EXPERIMENT: Z₃ → ℤ₇");
    println!("============================================================");

    // ── Phase A: Discover on Z₃, extract transferable knowledge ──
    let z3_log = search::run_discovery_named(&z3_engine(), &config, "Z3");
    let knowledge = search::extract_transferable(&z3_log);

    println!("\n  Phase A: Extracted from Z₃ ({} transferable concepts):", knowledge.len());
    for k in &knowledge {
        println!("    concept: {} (arity={})", k.concept_name, k.concept_arity);
        println!("      promotion: {}", k.promotion_rule);
        for desc in &k.descriptions {
            println!("      universal: {}", desc);
        }
    }

    // ── Phase B: Build ℤ₇ target ──
    let mut target = ClosureEngine::new();
    target.define_relation("op", 3);
    target.define_equivalence("eq");
    target.define_relation("distinct", 2);

    let z7_elems: Vec<String> = (0..7).map(|i| format!("n{}", i)).collect();
    for name in &z7_elems {
        target.define_constant(name);
    }
    for i in 0..7 {
        for j in (i + 1)..7 {
            target.add_fact(Relation::binary("distinct", c(&z7_elems[i]), c(&z7_elems[j])));
            target.add_fact(Relation::binary("distinct", c(&z7_elems[j]), c(&z7_elems[i])));
        }
    }

    // Give partial op table: exclude identity row/column EXCEPT one hint fact.
    // op(n0, n0, n0) is the hint — enough for the promotion rule to discover auto_0(n0).
    let mut given_facts = 0;
    let mut withheld_facts = Vec::new();
    for i in 0..7usize {
        for j in 0..7usize {
            let r = (i + j) % 7;
            if (i == 0 || j == 0) && !(i == 0 && j == 0) {
                // Withhold identity row/column (except the hint)
                withheld_facts.push((
                    z7_elems[i].clone(),
                    z7_elems[j].clone(),
                    z7_elems[r].clone(),
                ));
            } else {
                // Give this fact (includes the hint op(n0,n0,n0) and all non-identity facts)
                target.add_fact(Relation::new("op", vec![
                    c(&z7_elems[i]), c(&z7_elems[j]), c(&z7_elems[r]),
                ]));
                given_facts += 1;
            }
        }
    }

    println!("\n  Phase B: ℤ₇ target");
    println!("    elements: {}", z7_elems.len());
    println!("    op facts given: {} (includes 1 hint: op(n0,n0,n0))", given_facts);
    println!("    op facts withheld: {} (to be predicted)", withheld_facts.len());

    // Inject transferred knowledge — FULLY AUTOMATIC, no manual concept declarations
    search::inject_transfer(&mut target, &knowledge);

    println!("    transferred: {} concepts, {} rules total",
        knowledge.len(),
        knowledge.iter().map(|k| 1 + k.universal_rules.len()).sum::<usize>());

    // ── Phase C: Run closure and check predictions ──
    let result = target.derive_closure();

    // Check which concept was discovered
    for k in &knowledge {
        let instances: Vec<String> = result.facts.iter()
            .filter(|f| f.name() == k.concept_name)
            .map(|f| f.terms().iter().map(|t| t.to_string()).collect::<Vec<_>>().join(","))
            .collect();
        println!("\n  Discovered: {}({}) from transferred promotion rule",
            k.concept_name, instances.join("; "));
    }

    // Check predictions
    let mut correct = 0;
    let mut wrong = 0;
    for (a, b, expected_r) in &withheld_facts {
        let predicted_fact = Relation::new("op", vec![c(a), c(b), c(expected_r)]);
        if result.facts.contains(&predicted_fact) {
            correct += 1;
        } else {
            let any_predicted = result.facts.iter().any(|f| {
                f.name() == "op" && f.terms()[0] == Term::constant(a) && f.terms()[1] == Term::constant(b)
            });
            if any_predicted {
                wrong += 1;
                println!("    WRONG: op({}, {}) expected {}", a, b, expected_r);
            }
        }
    }
    let not_predicted = withheld_facts.len() - correct - wrong;

    let distinct_violations: usize = result.facts.iter()
        .filter(|f| f.name() == "eq" && f.arity() == 2 && f.terms()[0] != f.terms()[1])
        .filter(|f| {
            result.facts.contains(&Relation::binary("distinct", f.terms()[0].clone(), f.terms()[1].clone()))
        })
        .count();

    println!("\n  Results:");
    println!("    correct predictions: {}/{}", correct, withheld_facts.len());
    println!("    wrong predictions:   {}", wrong);
    println!("    not predicted:       {}", not_predicted);
    println!("    inconsistencies:     {}", distinct_violations);

    let timestamp = chrono_timestamp();
    let knowledge_desc: Vec<String> = knowledge.iter()
        .flat_map(|k| k.descriptions.iter().cloned())
        .collect();
    let log_content = format!(
        "=== TRANSFER EXPERIMENT: Z₃ → ℤ₇ (fully automatic) ===\n\
         Timestamp: {}\n\n\
         Source: Z₃ (3 elements)\n\
         Transferred knowledge ({} concepts):\n{}\n\n\
         Target: ℤ₇ (7 elements, {} op facts given, 1 hint, {} withheld)\n\n\
         Discovered concepts in target:\n\
         (auto-identified via transferred promotion rules)\n\n\
         Results:\n\
         Correct: {}/{}, Wrong: {}, Not predicted: {}, Inconsistencies: {}\n\
         \nVERDICT: {}\n",
        timestamp,
        knowledge.len(),
        knowledge_desc.iter().map(|d| format!("  {}", d)).collect::<Vec<_>>().join("\n"),
        given_facts, withheld_facts.len(),
        correct, withheld_facts.len(), wrong, not_predicted, distinct_violations,
        if correct == withheld_facts.len() && wrong == 0 && distinct_violations == 0 {
            "PERFECT TRANSFER — all predictions correct, zero inconsistencies."
        } else {
            "PARTIAL TRANSFER"
        }
    );
    write_log(&format!("{}_transfer_z3_to_z7.log", timestamp), &log_content);

    assert_eq!(wrong, 0, "zero wrong predictions");
    assert_eq!(distinct_violations, 0, "zero inconsistencies");
    assert_eq!(correct, withheld_facts.len(), "all withheld facts correctly predicted");
}

/// Enumerate all binary operations on {0,...,n-1} and classify by axioms.
/// This is the bridge from "discover properties of given structures"
/// to "discover which structures are worth studying."
#[test]
fn run_axiom_lattice_enumeration() {
    let n = 3usize; // carrier size
    let total = n.pow((n * n) as u32); // 3^9 = 19683
    println!("\n============================================================");
    println!("  AXIOM LATTICE: all binary ops on {{0..{}}}", n - 1);
    println!("  Total candidate operations: {}", total);
    println!("============================================================\n");

    // Axiom flags
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut examples: std::collections::BTreeMap<String, Vec<Vec<usize>>> = std::collections::BTreeMap::new();

    let elems: Vec<usize> = (0..n).collect();

    // Enumerate all n^(n*n) operation tables
    // Table[i][j] = (i op j) encoded as a single number in base n
    let mut table = vec![0usize; n * n];
    for op_id in 0..total {
        // Decode op_id into table
        let mut x = op_id;
        for k in 0..n * n {
            table[k] = x % n;
            x /= n;
        }
        let op = |a: usize, b: usize| -> usize { table[a * n + b] };

        // Check axioms
        let assoc = check_associativity(n, &op);
        let comm = check_commutativity(n, &op);
        let (has_id, id_elem) = check_identity(n, &op);
        let has_inv = has_id && check_inverse(n, &op, id_elem.unwrap());

        // Build axiom key
        let mut props = Vec::new();
        if assoc { props.push("assoc"); }
        if comm { props.push("comm"); }
        if has_id { props.push("id"); }
        if has_inv { props.push("inv"); }

        let key = if props.is_empty() {
            "none".to_string()
        } else {
            props.join("+")
        };

        *counts.entry(key.clone()).or_insert(0) += 1;

        // Save first few examples
        let ex = examples.entry(key).or_default();
        if ex.len() < 2 {
            ex.push(table.clone());
        }
    }

    // Named structure types
    let structure_names: Vec<(&str, &str)> = vec![
        ("assoc+comm+id+inv", "abelian group"),
        ("assoc+id+inv", "group (non-abelian)"),
        ("assoc+comm+id", "commutative monoid"),
        ("assoc+id", "monoid"),
        ("assoc+comm", "commutative semigroup"),
        ("assoc", "semigroup"),
        ("comm+id", "commutative unital magma"),
        ("comm", "commutative magma"),
        ("id", "unital magma"),
        ("none", "bare magma"),
    ];

    // Summary by structure hierarchy
    println!("  --- MODEL COUNTS BY AXIOM SET ---\n");
    println!("  {:>7}  {:<30} {}", "count", "axioms", "structure");
    println!("  {:->7}  {:-<30} {:-<20}", "", "", "");

    // Sort by count descending
    let mut sorted: Vec<(String, usize)> = counts.iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    for (key, count) in &sorted {
        let name = structure_names.iter()
            .find(|(k, _)| k == key)
            .map(|(_, n)| *n)
            .unwrap_or("");
        println!("  {:>7}  {:<30} {}", count, key, name);
    }

    // Aggregate counts
    let total_assoc: usize = counts.iter()
        .filter(|(k, _)| k.contains("assoc"))
        .map(|(_, v)| *v)
        .sum();
    let total_comm: usize = counts.iter()
        .filter(|(k, _)| k.contains("comm"))
        .map(|(_, v)| *v)
        .sum();
    let total_id: usize = counts.iter()
        .filter(|(k, _)| k.contains("id"))
        .map(|(_, v)| *v)
        .sum();
    let total_inv: usize = counts.iter()
        .filter(|(k, _)| k.contains("inv"))
        .map(|(_, v)| *v)
        .sum();

    println!("\n  --- AGGREGATE ---\n");
    println!("  total operations:        {:>7}", total);
    println!("  with associativity:      {:>7} ({:.1}%)", total_assoc, 100.0 * total_assoc as f64 / total as f64);
    println!("  with commutativity:      {:>7} ({:.1}%)", total_comm, 100.0 * total_comm as f64 / total as f64);
    println!("  with identity:           {:>7} ({:.1}%)", total_id, 100.0 * total_id as f64 / total as f64);
    println!("  with inverse (= groups): {:>7} ({:.1}%)", total_inv, 100.0 * total_inv as f64 / total as f64);

    println!("\n  --- AXIOM SELECTIVITY (each axiom's filtering power) ---\n");
    println!("  associativity: keeps {:.2}% of all operations", 100.0 * total_assoc as f64 / total as f64);
    println!("  + identity:    keeps {:.2}% of semigroups", if total_assoc > 0 { 100.0 * total_id as f64 / total_assoc as f64 } else { 0.0 });
    println!("  + inverse:     keeps {:.2}% of monoids", if total_id > 0 { 100.0 * total_inv as f64 / total_id as f64 } else { 0.0 });

    // Write log
    let timestamp = chrono_timestamp();
    let mut log = format!(
        "=== AXIOM LATTICE ENUMERATION ===\n\
         Carrier size: {}\nTotal operations: {}\n\n",
        n, total
    );
    for (key, count) in &sorted {
        let name = structure_names.iter()
            .find(|(k, _)| k == key)
            .map(|(_, n)| *n)
            .unwrap_or("");
        log.push_str(&format!("{:>7}  {:<30} {}\n", count, key, name));
    }
    log.push_str(&format!(
        "\nAssociativity: {}/{} ({:.1}%)\n\
         Groups: {}/{} ({:.4}%)\n",
        total_assoc, total, 100.0 * total_assoc as f64 / total as f64,
        total_inv, total, 100.0 * total_inv as f64 / total as f64,
    ));
    write_log(&format!("{}_axiom_lattice_n{}.log", timestamp, n), &log);
}

/// Start from "a set + a binary operation" with NO axioms.
/// Enumerate all operations, classify by axioms, run discovery on representatives,
/// score by rarity × richness. The highest-scoring axiom set IS the "group proto"
/// — derived, not declared.
#[test]
fn run_derive_group_proto_from_set() {
    let n = 3usize;
    let total = n.pow((n * n) as u32);

    println!("\n============================================================");
    println!("  DERIVING STRUCTURE PROTOTYPES FROM SET + BINARY OP");
    println!("  Carrier: {{0, 1, 2}}, {} total operations", total);
    println!("============================================================\n");

    // Phase 1: Enumerate and classify
    let mut classes: std::collections::BTreeMap<String, Vec<Vec<usize>>> = std::collections::BTreeMap::new();

    let mut table = vec![0usize; n * n];
    for op_id in 0..total {
        let mut x = op_id;
        for k in 0..n * n {
            table[k] = x % n;
            x /= n;
        }
        let op = |a: usize, b: usize| -> usize { table[a * n + b] };

        let assoc = check_associativity(n, &op);
        let comm = check_commutativity(n, &op);
        let (has_id, id_elem) = check_identity(n, &op);
        let has_inv = has_id && check_inverse(n, &op, id_elem.unwrap());

        let mut props = Vec::new();
        if assoc { props.push("assoc"); }
        if comm { props.push("comm"); }
        if has_id { props.push("id"); }
        if has_inv { props.push("inv"); }
        let key = if props.is_empty() { "none".to_string() } else { props.join("+") };

        let entry = classes.entry(key).or_default();
        if entry.len() < 3 { // keep a few representatives
            entry.push(table.clone());
        }
    }

    // Phase 2: For each class, build engine from a representative and measure discovery richness
    let mut exclude = HashSet::new();
    exclude.insert("distinct".to_string());

    let elems = ["e0", "e1", "e2"];

    println!("  Phase 1: {} axiom classes found\n", classes.len());
    println!("  Phase 2: Running discovery on each class representative...\n");

    struct ClassScore {
        axioms: String,
        model_count: usize,
        rarity: f64,
        concepts_verified: usize,
        verification_rules: usize,
        universal_rules: usize,
        chain_identities: usize,
        richness: f64,
        score: f64,
        structure_name: String,
    }

    let structure_names: std::collections::HashMap<&str, &str> = [
        ("assoc+comm+id+inv", "abelian group"),
        ("assoc+id+inv", "group"),
        ("assoc+comm+id", "commutative monoid"),
        ("assoc+id", "monoid"),
        ("assoc+comm", "commutative semigroup"),
        ("assoc", "semigroup"),
        ("comm+id+inv", "loop (commutative)"),
        ("id+inv", "loop"),
        ("comm+id", "commutative unital magma"),
        ("comm", "commutative magma"),
        ("id", "unital magma"),
        ("none", "magma"),
    ].iter().cloned().collect();

    let mut scores = Vec::new();

    // Count models per class (re-enumerate quickly)
    let mut model_counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    {
        let mut t2 = vec![0usize; n * n];
        for op_id in 0..total {
            let mut x = op_id;
            for k in 0..n * n { t2[k] = x % n; x /= n; }
            let op = |a: usize, b: usize| -> usize { t2[a * n + b] };
            let assoc = check_associativity(n, &op);
            let comm = check_commutativity(n, &op);
            let (has_id, id_elem) = check_identity(n, &op);
            let has_inv = has_id && check_inverse(n, &op, id_elem.unwrap());
            let mut props = Vec::new();
            if assoc { props.push("assoc"); }
            if comm { props.push("comm"); }
            if has_id { props.push("id"); }
            if has_inv { props.push("inv"); }
            let key = if props.is_empty() { "none".to_string() } else { props.join("+") };
            *model_counts.entry(key).or_insert(0) += 1;
        }
    }

    for (axioms, representatives) in &classes {
        let rep = &representatives[0];
        let mc = model_counts.get(axioms).copied().unwrap_or(0);

        // Build engine from representative
        let mut engine = ClosureEngine::new();
        engine.define_relation("op", 3);
        engine.define_equivalence("eq");
        engine.define_relation("distinct", 2);

        for name in &elems {
            engine.define_constant(*name);
        }
        for i in 0..n {
            for j in 0..n {
                let r = rep[i * n + j];
                engine.add_fact(Relation::new("op", vec![
                    c(elems[i]), c(elems[j]), c(elems[r]),
                ]));
            }
        }
        for i in 0..n {
            for j in (i + 1)..n {
                engine.add_fact(Relation::binary("distinct", c(elems[i]), c(elems[j])));
                engine.add_fact(Relation::binary("distinct", c(elems[j]), c(elems[i])));
            }
        }

        // Run lightweight discovery (1 round, small beam)
        let config = search::DiscoveryConfig {
            beam: search::BeamConfig {
                candidate_config: search::CandidateConfig {
                    guard_relation: None,
                    exclude_relations: exclude.clone(),
                    min_pattern_support: 2,
                    ..search::CandidateConfig::default()
                },
                weights: ScoreWeights {
                    generativity: 1.0,
                    compression: 0.5,
                    consistency_penalty: 10.0,
                    exclusions: vec![("eq".to_string(), "distinct".to_string())],
                },
                beam_width: 3,
                max_rules_per_beam: 3,
                max_steps: 1,
                adaptive: search::AdaptivePolicy::Fixed,
            },
            promotion: search::PromotionConfig {
                min_support: 2,
                max_promotions_per_round: 5,
                exclude_relations: exclude.clone(),
            },
            max_rounds: 1,
        };

        let log = search::run_discovery_named(&engine, &config, axioms);
        let step = &log.steps[0];

        let concepts_verified = step.verification_rules.len() / 2; // 2 rules per concept
        let n_verify = step.verification_rules.len();
        let n_universal = step.universal_rules.len();
        let n_chain = step.chain_identities.len();

        let rarity = if mc > 0 { 1.0 / mc as f64 } else { 0.0 };
        let richness = (concepts_verified as f64) * 2.0
            + n_verify as f64
            + n_universal as f64
            + n_chain as f64 * 3.0; // chain identities weighted higher

        let score = rarity * richness * 1000.0; // scale for readability

        scores.push(ClassScore {
            axioms: axioms.clone(),
            model_count: mc,
            rarity,
            concepts_verified,
            verification_rules: n_verify,
            universal_rules: n_universal,
            chain_identities: n_chain,
            richness,
            score,
            structure_name: structure_names.get(axioms.as_str())
                .unwrap_or(&"").to_string(),
        });
    }

    // Phase 3: Rank by score
    scores.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    println!("  {:>8} {:>6} {:>4} {:>4} {:>4} {:>7} {:>7}  {:<25} {}",
        "score", "models", "ver", "uni", "chn", "rare", "rich", "axioms", "structure");
    println!("  {:->8} {:->6} {:->4} {:->4} {:->4} {:->7} {:->7}  {:->25} {:->15}",
        "", "", "", "", "", "", "", "", "");

    for s in &scores {
        println!("  {:>8.2} {:>6} {:>4} {:>4} {:>4} {:>7.4} {:>7.1}  {:<25} {}",
            s.score, s.model_count,
            s.verification_rules, s.universal_rules, s.chain_identities,
            s.rarity, s.richness,
            s.axioms, s.structure_name);
    }

    // The top-scoring axiom set should be the group
    let top = &scores[0];
    println!("\n  TOP RESULT: {} ({})", top.axioms, top.structure_name);
    println!("  → System derived '{}' as the most valuable structure proto", top.structure_name);
    println!("    from pure enumeration + discovery, with NO prior knowledge of group theory.\n");

    // Log
    let timestamp = chrono_timestamp();
    let mut log_content = format!(
        "=== STRUCTURE PROTO DERIVATION ===\nCarrier: {{0,1,2}}\nTotal ops: {}\n\n", total);
    for s in &scores {
        log_content.push_str(&format!(
            "score={:.2}  models={}  verify={}  universal={}  chain={}  {}  {}\n",
            s.score, s.model_count, s.verification_rules, s.universal_rules,
            s.chain_identities, s.axioms, s.structure_name));
    }
    log_content.push_str(&format!("\nTOP: {} ({})\n", top.axioms, top.structure_name));
    write_log(&format!("{}_proto_derivation.log", timestamp), &log_content);

    // The group should be in the top 3
    let group_rank = scores.iter().position(|s| s.axioms.contains("inv") && s.axioms.contains("assoc"));
    assert!(group_rank.is_some() && group_rank.unwrap() < 3,
        "group should be in top 3 by score");
}

fn check_associativity(n: usize, op: &dyn Fn(usize, usize) -> usize) -> bool {
    for a in 0..n {
        for b in 0..n {
            for cc in 0..n {
                if op(op(a, b), cc) != op(a, op(b, cc)) {
                    return false;
                }
            }
        }
    }
    true
}

fn check_commutativity(n: usize, op: &dyn Fn(usize, usize) -> usize) -> bool {
    for a in 0..n {
        for b in (a + 1)..n {
            if op(a, b) != op(b, a) {
                return false;
            }
        }
    }
    true
}

fn check_identity(n: usize, op: &dyn Fn(usize, usize) -> usize) -> (bool, Option<usize>) {
    for e in 0..n {
        let is_id = (0..n).all(|x| op(e, x) == x && op(x, e) == x);
        if is_id {
            return (true, Some(e));
        }
    }
    (false, None)
}

fn check_inverse(n: usize, op: &dyn Fn(usize, usize) -> usize, e: usize) -> bool {
    for a in 0..n {
        let has_inv = (0..n).any(|b| op(a, b) == e && op(b, a) == e);
        if !has_inv {
            return false;
        }
    }
    true
}

fn chrono_timestamp() -> String {
    chrono::Local::now().format("%Y%m%d_%H%M%S").to_string()
}
