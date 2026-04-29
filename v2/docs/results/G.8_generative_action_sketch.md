# G.8 — ActionKind::ApplyGenerativeRule (design sketch)

**Status**: ✓ done (design only, no lib code)
**Format**: Architectural sketch; companion to G.5 (creation drive).

## Goal

Specify the runtime integration surface that would let the scheduler trigger generative axiom application — analog of B.5.1's wiring of Beta-1 (`DiscoverAxiomShapeFamilies`) into the runtime.

Without this slice, even with G.5's drive, there's no execution path from "system wants to mint" to "system actually mints".

## Five integration points

### 1. ActionKind variant

```rust
// in src/runtime/action.rs
pub enum ActionKind {
    // ... existing variants
    /// G.8 / ADR 0069 — apply a registered generative recipe
    /// to a seed identifier (or pair, for multi-arity), producing
    /// fresh identifiers and materializing them as R edges.
    ApplyGenerativeRule {
        recipe_id: String,    // e.g., "successor", "addition"
        seeds: Vec<String>,   // 1 or 2 ids depending on arity
        bound: usize,         // K_MAX cap from G.5
    },
}
```

### 2. FrontierKind variant

```rust
// in src/runtime/frontier.rs
pub enum FrontierKind {
    // ... existing variants
    /// G.8 — candidate generative-recipe invocation.
    /// Refreshed by the runtime when structural_growth_drive
    /// signal exceeds threshold (G.5).
    GenerativeRuleCandidate {
        recipe_id: String,
        seeds: Vec<String>,
        estimated_value: f64,  // counterfactual prediction-error reduction
    },
}
```

### 3. Refresh method

```rust
// in src/runtime/autonomous.rs
impl AutonomousRuntime {
    /// G.8 — analog of refresh_shape_family_candidates from B.5.1.
    pub fn refresh_generative_candidates(&mut self) {
        // For each registered recipe (R(GENERATIVE_AXIOM_MARKER, _)):
        //   Sample N candidate seed pairs from current data ids
        //   Estimate each pair's counterfactual prediction-error reduction
        //   Above threshold → push as FrontierKind::GenerativeRuleCandidate
    }
}
```

### 4. execute_action arm

```rust
// in src/runtime/autonomous.rs
ActionKind::ApplyGenerativeRule { recipe_id, seeds, bound } => {
    let recipe = self.lookup_generative_recipe(&recipe_id)?;
    let mut minted_count = 0;
    for seed in &seeds {
        if minted_count >= *bound { break; }
        let new_id = recipe.apply(seed);
        if !self.rset.contains_id(&new_id) {
            // materialize per recipe spec: 1 edge unary, 2 edges binary
            for edge in recipe.materialize_edges(seed, &new_id) {
                self.rset.add(edge);
            }
            self.rset.add(R::new(GENERATIVE_DERIVED_MARKER, &new_id));
            minted_count += 1;
        }
    }
    EpisodeResult::Minted(minted_count)
}
```

### 5. Persistence round-trip

```rust
// in src/runtime/persistence.rs
fn action_kind_to_str(kind: &ActionKind) -> String {
    match kind {
        // ... existing arms
        ActionKind::ApplyGenerativeRule { recipe_id, seeds, bound } => {
            format!("ApplyGenerativeRule:{}|{}|{}",
                    recipe_id, seeds.join(","), bound)
        }
    }
}

fn parse_action_kind(s: &str) -> Option<ActionKind> {
    if let Some(rest) = s.strip_prefix("ApplyGenerativeRule:") {
        let parts: Vec<&str> = rest.splitn(3, '|').collect();
        if parts.len() == 3 {
            return Some(ActionKind::ApplyGenerativeRule {
                recipe_id: parts[0].to_string(),
                seeds: parts[1].split(',').map(str::to_owned).collect(),
                bound: parts[2].parse().ok()?,
            });
        }
    }
    None
}
```

## Recipe registry (new construct)

Recipes need a registry analog to axioms. ADR 0069 specified `R(GENERATIVE_AXIOM_MARKER, recipe_id)` but didn't formalize storage:

```rust
// in src/markers.rs
pub const GENERATIVE_AXIOM_MARKER: &str = "__generative_axiom__";

pub const GENERATIVE_DERIVED_MARKER: &str = "__generative_derived__";
```

```rust
// in src/lib.rs
impl RSet {
    /// Register a generative recipe by id. Recipe code lives outside
    /// rset (Rust function); rset stores only the registry edge.
    /// G.8 / ADR 0069.
    pub fn register_generative_recipe(&mut self, recipe_id: &str) -> bool {
        self.add(R::new(GENERATIVE_AXIOM_MARKER, recipe_id))
    }

    pub fn generative_recipes(&self) -> Vec<&str> {
        self.left_of(GENERATIVE_AXIOM_MARKER).iter().map(|r| r.y.as_str()).collect()
    }
}
```

The Rust function table is a runtime data structure (HashMap<String, fn(&[&str]) -> String>) populated at scheduler init. This is the same compile-time-vs-runtime split that drives use (drive impls live in code; registration goes in rset).

## Scheduler integration

In `RuleBasedScheduler::execute_for_kind` (mirrors B.5.1):

```rust
FrontierKind::GenerativeRuleCandidate { .. } => {
    // accept in Expand mode when drive arbitration favors creation
    if matches!(self.mode, Mode::Expand) && drive_favors_creation(ctx) {
        return Some(item);
    }
    None
}
```

## End-to-end execution path

1. Tick `t`: scheduler enters Expand mode
2. Drive composite: `structural_growth_drive` signal high
3. `refresh_generative_candidates` populates frontier with candidate seed pairs ranked by estimated value
4. Scheduler picks top-1 from `GenerativeRuleCandidate` items
5. Dispatches `ActionKind::ApplyGenerativeRule { recipe_id, seeds, bound }`
6. `execute_action` mints up to K new ids, adds edges
7. Tick `t+1`: prediction_error re-evaluated; drive saturation may follow

## Open questions (deferred)

### Q1: How are recipes "registered"?

Compile-time table in `runtime`? Plugin trait? ADR 0069 left this open; G.8 needs a concrete answer.

**Tentative**: a `GenerativeRegistry: HashMap<String, Box<dyn GenerativeRecipe>>` initialized at runtime construction. Default registry contains `successor`, `addition`. Custom recipes added via `runtime.register_recipe(id, impl)`.

### Q2: Counterfactual evaluation cost

`refresh_generative_candidates` needs to estimate value before action. Computing value = simulating the rset state post-minting + measuring prediction-error change. Expensive.

**Tentative**: cheap proxy = "is the seed in a position with high incoming edge density but low outgoing?" — high-density positions tend to benefit most from extension. Analytical heuristic, no simulation.

### Q3: Recipe selection bias

If multiple recipes are registered, which gets picked first? Successor before addition? Random?

**Tentative**: by drive-shaped score. The drive can target SPECIFIC recipes (signal modulated by recipe). Successor preferred when chain depth shallow; addition preferred when chain established.

## Why this slice is design-only

Implementation needs:
- 5 lib modifications (action.rs, frontier.rs, autonomous.rs, persistence.rs, lib.rs)
- New scheduler routing (scheduler_rule.rs)
- Default recipe registry initialization
- Comprehensive tests (idempotency, persistence round-trip, drive arbitration tested over 200+ ticks)
- Integration with G.5's drive (which is itself only designed, not built)

Estimate: 2-3 medium slices to land. G.8 = the spec; G.8.1 = recipe registry; G.8.2 = action wiring; G.8.3 = scheduler routing & tests.

## What this slice produced

1. Surface specification — 5 integration points (ActionKind, FrontierKind, refresh_*, execute_action arm, persistence)
2. Marker proposal: `GENERATIVE_AXIOM_MARKER`, `GENERATIVE_DERIVED_MARKER`
3. Recipe registry sketch (Rust-side function table + rset-side registry edges)
4. End-to-end execution path (drive → frontier → scheduler → action → mint)
5. 3 named open questions with tentative resolutions

## Future implications

- Together G.5 + G.8 = the autonomy bridge. Until both ship, G-series remains user-invoked rather than scheduler-driven.
- B.5.1's wiring of Beta-1 took ~1 medium slice. G.8 is similar shape but with the additional complication of the recipe registry.
- Once landed, the runtime can autonomously mint identifiers when prediction-error pressure justifies it — the strongest possible empirical demonstration of "v2 自动拓展" at the identifier layer.
