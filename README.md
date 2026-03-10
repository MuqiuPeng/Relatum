# Relatum

Relatum is a relational closure engine. It uses composable algebraic theories, proof-producing closure, and shared signatures to explore the boundary of what can be known from what is declared.

You declare relations with properties. You write rules. The engine derives everything that follows — and shows you why.

## Why Not Just Another Rule Engine

Most rule engines work with raw if-then rules. Relatum starts from a different premise: **algebraic properties are the natural vocabulary for describing relational structure**.

When you write `<rel> equiv/2 : equivalence`, you aren't writing three separate rules for reflexivity, symmetry, and transitivity — you're declaring a single structural commitment. The engine knows what equivalence *means*, generates the right inference machinery, and tracks provenance through every derived fact.

This matters because:
- **Composability** — Properties compose. `equivalence` = `reflexive` + `symmetric` + `transitive` + `congruent`. Structures share operations through a common signature.
- **Proof traces** — Every derived fact carries its derivation chain. You can ask "why does `equiv(a, c)` hold?" and get a step-by-step proof tree.
- **Bounded generation** — Compound terms (Skolem witnesses) like `inv(?x)` generate new structure, with depth limits to prevent infinite expansion.

## Quick Example

```
<ele> a, b, c

<rel> equiv/2 : equivalence

equiv(a, b)
equiv(b, c)

derive
```

From 2 seed facts, the engine derives all 9 facts of the complete 3x3 equivalence class, saturating in 2-3 rounds.

```
<ele> alice, bob, carol

<rel> friend/2 : symmetric
<rel> knows/2

<rule> f2k: friend(?x, ?y) |- knows(?x, ?y)
<rule> k_trans: knows(?x, ?y), knows(?y, ?z) |- knows(?x, ?z)

friend(alice, bob)
friend(bob, carol)

derive
```

Symmetric friendship + transitive knowing collapses the network into full connectivity.

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
│   └── engine.rs         #   Semi-naive closure, reflexivity, congruence
├── algebra/              # Theory library (on top of relational core)
│   ├── operation.rs      #   OperationId, Arity — typed operation declarations
│   ├── registry.rs       #   OpRegistry — shared signature namespace
│   ├── term.rs           #   Term with OperationId references
│   ├── equation.rs       #   Named equations (axioms)
│   ├── structure.rs      #   Structure = adopted ops + equational axioms
│   ├── builders.rs       #   Prelude: semigroup, monoid, group, ring
│   ├── closure.rs        #   Equational closure via union-find + congruence
│   └── parser.rs         #   Text syntax → AST
├── node.rs               # Graph primitives (legacy, to be retired)
├── relation.rs           #   ↑
├── network.rs            #   ↑
├── iter.rs               #   ↑
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
| `<Container> x, y` | Declare typed container with members |
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
