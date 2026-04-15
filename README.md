# Relatum

A relation-first reasoning engine for autonomous mathematical discovery, supporting both **finite-model inductive discovery** and **symbolic axiom-level deductive derivation** within a single relational closure framework.

---

## Current Research Results

**Algebraic Discovery (Inductive Direction)**

Starting from exhaustive enumeration of all 19,683 binary operations on a 3-element carrier, Relatum autonomously partitions the model space by axiom class, performs closure-based concept extraction, and identifies the abelian group structure as the optimal region through dual-signal alignment of model-space rarity and closure-space richness. A formal equivalence proof establishes that the discovered rule bundle `{D1, D2, D3}` is logically equivalent to the group axiom subset `{G2, G3}`, holding for all groups including infinite ones. Transfer to a held-out Z₄ structure yields 6/6 correct predictions.

**Set-Theoretic Derivation (Deductive Direction)**

With variable-variable unification at the pattern layer, the engine derives universally quantified relational statements directly from axiomatic input — without finite-model enumeration. From two input propositions, the system produces:

```
member(empty, power(_0))   —  ∀A. ∅ ∈ 𝒫(A)
subset(empty, _0)          —  ∀A. ∅ ⊆ A
member(_0, pair(_0, _1))   —  ∀a,b. a ∈ {a,b}
member(_1, pair(_0, _1))   —  ∀a,b. b ∈ {a,b}
```

as first-order consequences of ZFC axioms, in 0.00s, with 11 total facts.

---

## Navigation

| What you want | Where to look |
|---|---|
| System architecture | This README (below) |
| Formal equivalence proof | [`docs/formal-equivalence.md`](docs/formal-equivalence.md) |
| Dual-signal analysis | [`docs/dual-signal-analysis.md`](docs/dual-signal-analysis.md) |
| Research conclusions | [`docs/conclusions-2026-04-15.md`](docs/conclusions-2026-04-15.md) |
| Experiment logs | [`logs/archive/`](logs/archive/) |

---

## Architecture

Relatum has two layers. The relational engine is the foundation; the algebra layer is a theory library built on top of it.

```
┌─────────────────────────────────────────────┐
│  Algebra Layer (theory library)             │
│  ┌─────────┐ ┌─────────┐ ┌──────────────┐  │
│  │Structure│ │Builders │ │OpRegistry    │  │
│  │(theory) │ │(prelude)│ │(shared sig.) │  │
│  └─────────┘ └─────────┘ └──────────────┘  │
├─────────────────────────────────────────────┤
│  Relational Core                            │
│  ┌────┐ ┌────┐ ┌─────┐ ┌───────┐ ┌──────┐ │
│  │Term│ │Fact│ │Rule │ │Engine │ │Prove │ │
│  └────┘ └────┘ └─────┘ └───────┘ └──────┘ │
└─────────────────────────────────────────────┘
```

**Relational Core** — Terms (atom / compound / variable), facts (ground relations), rules (pattern-match + substitute), semi-naive closure with provenance tracking.

**Algebra Layer** — Structures are bundles of relation schemas + equational axioms. An `OpRegistry` provides a shared signature namespace so that different structures can reference the same operations. Builders provide common theories: semigroup, monoid, group, ring.

The key design decision: **equations are relations**. An equation `a = b` is just a binary equivalence relation. Algebraic axioms are inference rules. A "structure" is a named collection of schemas and rules that can be fed to the relational engine. This means the algebra layer adds no new execution semantics — it's purely a declarative packaging layer.

## Project Structure

```
src/
├── relational/           # Core closure engine
│   ├── term.rs           #   Term = Atom | Compound(symbol, args) | Var
│   ├── relation.rs       #   Relation = named tuple of terms
│   ├── rule.rs           #   Rule = premises |- conclusions; pattern matching
│   └── engine.rs         #   Semi-naive closure, reflexivity, congruence, axiom instantiation
├── algebra/              # Theory library (on top of relational core)
│   ├── operation.rs      #   OperationId, Arity — typed operation declarations
│   ├── registry.rs       #   OpRegistry — shared signature namespace
│   ├── term.rs           #   Term with OperationId references
│   ├── equation.rs       #   Named equations (axioms)
│   ├── structure.rs      #   Structure = adopted ops + equational axioms
│   ├── builders.rs       #   Prelude: semigroup, monoid, group, ring
│   ├── closure.rs        #   Equational closure via union-find + congruence
│   ├── compile.rs        #   Algebra → relational term/axiom compiler
│   └── parser.rs         #   Text syntax → AST
└── lib.rs                # Crate root
www/
├── index.html            # Single-file web app: DSL parser + closure engine + notebook UI
└── examples/             # .relnb notebook files (8 examples)
```

## Usage

**Web UI** — Open `www/index.html` in any browser. No build step, no server, no dependencies. The entire engine runs client-side.

**Rust library**:
```bash
cargo test
```

## DSL Reference

### Declarations

| Syntax | Meaning |
|---|---|
| `<ele> a, b, c` | Declare elements |
| `<Container> x, y` | Declare typed members (Container must be declared as element first) |
| `<rel> R/2` | Declare relation with arity |
| `<rel> R/2 : props` | Declare relation with algebraic properties |
| `<rule> name: P(?x) \|- Q(?x)` | Custom inference rule |

### Properties

| Property | Effect |
|---|---|
| `reflexive` | Generates `R(t, t)` for all known terms |
| `symmetric` | `R(a, b)` implies `R(b, a)` |
| `transitive` | `R(a, b)` + `R(b, c)` implies `R(a, c)` |
| `congruent` | `R(a, b)` propagates substitution across all facts |
| `equivalence` | All four above |

### Commands

| Command | Effect |
|---|---|
| `R(a, b)` | Assert a fact |
| `derive` | Compute closure, show all derived facts |
| `prove R(a, b)` | Show proof tree for a specific fact |
| `depth N` | Limit compound term nesting depth |

### Compound Terms

Rules can generate new terms: `<rule> has_inv: member(?x) |- group(?x, inv(?x), e)`

This creates Skolem witnesses — structured terms like `inv(a)`, `inv(inv(a))` — bounded by the `depth` directive.

## Notebook UI

The web interface is a Jupyter-style notebook with:

- **Code cells** — Write DSL, run with Shift+Enter, see derived facts inline
- **Markdown cells** — Documentation alongside code, rendered on blur
- **Multi-tab** — Each tab has independent cells, KB, undo stack, execution state
- **Workspace** — File manager backed by localStorage or an external folder (File System Access API)
- **Knowledge Base panel** — Browse all elements, relations, rules, facts; click facts for proof trees
- **8 built-in examples** — Equivalence, partial order, custom rules, congruence, typed relations, Skolem witnesses, multi-relation interaction, error handling

## License

MIT
