# Relatum

A relational closure engine — declare elements, relations with algebraic properties, and custom inference rules, then let the engine derive everything that follows.

## What It Does

Relatum takes a small set of declared facts and relation properties (reflexive, symmetric, transitive, congruent, equivalence) and computes the full closure. You write:

```
<ele> a, b, c

<rel> equiv/2 : equivalence

equiv(a, b)
equiv(b, c)

derive
```

And get all 9 facts of the complete equivalence class, with full proof traces.

## Features

**Core Engine**
- Algebraic relation properties: `reflexive`, `symmetric`, `transitive`, `congruent`, `equivalence`
- Custom inference rules: `<rule> name: premise(?x, ?y) |- conclusion(?x, ?y)`
- Membership constraints via typed containers: `<Person> alice, bob`
- Compound terms & Skolem witnesses: `inv(?x)`, `pair(?a, ?b)` with depth limiting
- Semi-naive evaluation with provenance tracking
- `prove` command for targeted proof trees

**Notebook UI** (`www/index.html`)
- Jupyter-style cells (code + markdown) with per-cell execution
- Multi-tab editor with independent KB per tab
- Workspace file manager (localStorage or external folder via File System Access API)
- Syntax highlighting, autocomplete, keyboard shortcuts
- Knowledge Base panel with fact provenance trees
- 8 built-in examples (equivalence, partial order, custom rules, congruence, typed relations, Skolem terms, multi-relation, error handling)
- Slide-out documentation panel

## Project Structure

```
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── node.rs            # Node type (element identifiers)
│   ├── relation.rs        # Relation schema
│   ├── network.rs         # Relation network (storage)
│   ├── iter.rs            # Iterators
│   ├── algebra/           # Algebraic structure builders
│   └── relational/        # Closure engine
│       ├── term.rs        # Term model (Atom/Compound/Var)
│       ├── relation.rs    # Relation + arity + properties
│       ├── rule.rs        # Inference rules
│       └── engine.rs      # Semi-naive closure computation
└── www/
    ├── index.html         # Single-file web app (DSL + UI + engine)
    └── examples/          # .relnb notebook files
```

## Usage

**Web UI** — Open `www/index.html` in a browser. No build step, no server, no dependencies.

**Rust library**:
```bash
cargo test
```

## DSL Quick Reference

| Syntax | Description |
|---|---|
| `<ele> a, b, c` | Declare elements |
| `<Container> x, y` | Declare typed container with members |
| `<rel> R/2 : props` | Declare relation with arity and properties |
| `<rule> name: P(?x) \|- Q(?x)` | Custom inference rule |
| `R(a, b)` | Assert a fact |
| `derive` | Compute closure and show all facts |
| `prove R(a, b)` | Show proof tree for a specific fact |
| `depth N` | Limit compound term nesting depth |

## License

MIT
