//! WASM bindings for the discovery engine.
//!
//! Exposes a single function `discover` that accepts a JSON configuration
//! and returns the full discovery log as JSON.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::relational::engine::ClosureEngine;
use crate::relational::relation::Relation;
// rule types used indirectly through search module
use crate::relational::search;
use crate::relational::score::ScoreWeights;
use crate::relational::term::Term;

// ── Input types ─────────────────────────────────────────────

#[derive(Deserialize)]
struct DiscoverInput {
    /// Name of the structure (for log header).
    name: String,
    /// Element names.
    elements: Vec<String>,
    /// Operation table: list of [a, b, result] triples.
    op_table: Vec<[String; 3]>,
    /// Maximum discovery rounds (default 2).
    #[serde(default = "default_rounds")]
    rounds: usize,
    /// Minimum pattern support (default 2).
    #[serde(default = "default_support")]
    min_support: usize,
}

fn default_rounds() -> usize { 2 }
fn default_support() -> usize { 2 }

// ── Output types ────────────────────────────────────────────

#[derive(Serialize)]
struct DiscoverOutput {
    structure_name: String,
    initial: InitialState,
    rounds: Vec<RoundOutput>,
}

#[derive(Serialize)]
struct InitialState {
    elements: Vec<String>,
    op_facts: usize,
    distinct_facts: usize,
    relations: Vec<String>,
}

#[derive(Serialize)]
struct RoundOutput {
    round: usize,
    concepts: Vec<ConceptOutput>,
    verification_rules: Vec<String>,
    chain_identities: Vec<ChainOutput>,
    beam: Option<BeamOutput>,
    total_facts: usize,
    total_relations: usize,
}

#[derive(Serialize)]
struct ConceptOutput {
    name: String,
    signature: String,
    instances: Vec<String>,
    support: usize,
}

#[derive(Serialize)]
struct ChainOutput {
    path_a: String,
    path_b: String,
}

#[derive(Serialize)]
struct BeamOutput {
    score: f64,
    derived: usize,
    inconsistencies: usize,
    rules: Vec<String>,
    injected: bool,
}

// ── WASM entry point ────────────────────────────────────────

#[wasm_bindgen]
pub fn discover(input_json: &str) -> Result<String, JsValue> {
    let input: DiscoverInput = serde_json::from_str(input_json)
        .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?;

    let mut engine = ClosureEngine::new();

    // Relations
    engine.define_relation("op", 3);
    engine.define_equivalence("eq");
    engine.define_relation("distinct", 2);

    // Elements
    for name in &input.elements {
        engine.define_constant(name);
    }

    // Op table
    for triple in &input.op_table {
        engine.add_fact(Relation::new("op", vec![
            Term::constant(&triple[0]),
            Term::constant(&triple[1]),
            Term::constant(&triple[2]),
        ]));
    }

    // Pairwise distinct
    for i in 0..input.elements.len() {
        for j in (i + 1)..input.elements.len() {
            engine.add_fact(Relation::binary(
                "distinct",
                Term::constant(&input.elements[i]),
                Term::constant(&input.elements[j]),
            ));
            engine.add_fact(Relation::binary(
                "distinct",
                Term::constant(&input.elements[j]),
                Term::constant(&input.elements[i]),
            ));
        }
    }

    // Discovery config
    let mut exclude = HashSet::new();
    exclude.insert("distinct".to_string());

    let config = search::DiscoveryConfig {
        beam: search::BeamConfig {
            candidate_config: search::CandidateConfig {
                guard_relation: None,
                exclude_relations: exclude.clone(),
                min_pattern_support: input.min_support,
                ..search::CandidateConfig::default()
            },
            weights: ScoreWeights {
                generativity: 1.0,
                compression: 0.5,
                consistency_penalty: 10.0,
                exclusions: vec![("eq".to_string(), "distinct".to_string())],
                purity_decay: 0.0,
            },
            beam_width: 5,
            max_rules_per_beam: 4,
            max_steps: 3,
            adaptive: search::AdaptivePolicy::Fixed,
        },
        promotion: search::PromotionConfig {
            min_support: input.min_support,
            max_promotions_per_round: 5,
            exclude_relations: exclude,
        },
        max_rounds: input.rounds,
    };

    let log = search::run_discovery_named(&engine, &config, &input.name);

    // Convert to output
    let initial = InitialState {
        elements: input.elements.clone(),
        op_facts: input.op_table.len(),
        distinct_facts: input.elements.len() * (input.elements.len() - 1),
        relations: log.initial_relations.iter()
            .map(|(n, a)| format!("{}/{}", n, a))
            .collect(),
    };

    let rounds: Vec<RoundOutput> = log.steps.iter().map(|step| {
        let concepts: Vec<ConceptOutput> = step.promoted.iter().map(|c| {
            ConceptOutput {
                name: c.name.clone(),
                signature: format!("{}", c.signature),
                instances: c.instance_set.iter().map(|t| t.to_string()).collect(),
                support: c.support,
            }
        }).collect();

        let chain_identities: Vec<ChainOutput> = step.chain_identities.iter().map(|ci| {
            ChainOutput {
                path_a: ci.path_a.clone(),
                path_b: ci.path_b.clone(),
            }
        }).collect();

        let beam = step.beam_best.as_ref().map(|b| {
            let injected = b.score > 0.0 && b.profile.derived_facts > 0;
            BeamOutput {
                score: b.score,
                derived: b.profile.derived_facts,
                inconsistencies: b.profile.inconsistencies,
                rules: b.rule_names.clone(),
                injected,
            }
        });

        RoundOutput {
            round: step.round,
            concepts,
            verification_rules: step.verification_rules.clone(),
            chain_identities,
            beam,
            total_facts: step.total_facts,
            total_relations: step.total_relations,
        }
    }).collect();

    let output = DiscoverOutput {
        structure_name: input.name,
        initial,
        rounds,
    };

    serde_json::to_string_pretty(&output)
        .map_err(|e| JsValue::from_str(&format!("JSON serialize error: {}", e)))
}
