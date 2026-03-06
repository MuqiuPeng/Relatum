//! Builders for common algebraic structures.
//!
//! Each builder takes a shared [`OpRegistry`], declares the necessary operations
//! in it, and returns a [`Structure`] containing only the equational axioms.
//! Because all builders share the same registry, operations with the same
//! name and arity are automatically deduplicated, enabling cross-structure
//! interaction.
//!
//! ```text
//! Semigroup  ⊂  Monoid  ⊂  Group
//!                              ↓
//!                Ring (additive abelian group + multiplicative monoid + distributivity)
//! ```
//!
//! # Example
//!
//! ```
//! use relatum::algebra::{builders, OpRegistry};
//!
//! let mut reg = OpRegistry::new();
//! let group = builders::group(&mut reg).unwrap();
//! assert!(group.validate(&reg).is_ok());
//! assert_eq!(group.referenced_operations().len(), 3); // mul, inv, e
//! assert_eq!(group.equations().len(), 5);
//! ```

use super::equation::Equation;
use super::operation::OperationId;
use super::registry::{OpRegistry, RegistryError};
use super::structure::Structure;
use super::term::Term;

// ── helpers ──────────────────────────────────────────────────

fn var(name: &str) -> Term {
    Term::var(name)
}

fn bin(op: OperationId, a: Term, b: Term) -> Term {
    Term::app(op, vec![a, b])
}

fn un(op: OperationId, a: Term) -> Term {
    Term::app(op, vec![a])
}

fn cnst(op: OperationId) -> Term {
    Term::constant(op)
}

fn eq(name: &str, lhs: Term, rhs: Term) -> Equation {
    Equation::new(name, lhs, rhs)
}

// ── internal: declare axiom groups on a mutable Structure ───

/// Declares `mul/2` in the registry and adds associativity to the structure.
fn declare_semigroup_axioms(
    s: &mut Structure,
    reg: &mut OpRegistry,
) -> Result<OperationId, RegistryError> {
    let mul = reg.declare_operation("mul", 2)?;
    let (x, y, z) = (var("x"), var("y"), var("z"));

    s.add_equation(
        eq(
            "mul_associativity",
            bin(mul, bin(mul, x.clone(), y.clone()), z.clone()),
            bin(mul, x, bin(mul, y, z)),
        )
        .with_category("associativity"),
    );

    Ok(mul)
}

/// Declares `e/0` in the registry and adds identity axioms for `mul`.
fn declare_monoid_axioms(
    s: &mut Structure,
    reg: &mut OpRegistry,
    mul: OperationId,
) -> Result<OperationId, RegistryError> {
    let e = reg.declare_operation("e", 0)?;
    let x = var("x");

    s.add_equation(
        eq(
            "mul_left_identity",
            bin(mul, cnst(e), x.clone()),
            x.clone(),
        )
        .with_category("identity"),
    );
    s.add_equation(
        eq("mul_right_identity", bin(mul, x.clone(), cnst(e)), x).with_category("identity"),
    );

    Ok(e)
}

/// Declares `inv/1` in the registry and adds inverse axioms for `mul` with identity `e`.
fn declare_group_axioms(
    s: &mut Structure,
    reg: &mut OpRegistry,
    mul: OperationId,
    e: OperationId,
) -> Result<OperationId, RegistryError> {
    let inv = reg.declare_operation("inv", 1)?;
    let x = var("x");

    s.add_equation(
        eq(
            "mul_left_inverse",
            bin(mul, un(inv, x.clone()), x.clone()),
            cnst(e),
        )
        .with_category("inverse"),
    );
    s.add_equation(
        eq(
            "mul_right_inverse",
            bin(mul, x.clone(), un(inv, x)),
            cnst(e),
        )
        .with_category("inverse"),
    );

    Ok(inv)
}

/// Declares `add/2`, `zero/0`, `neg/1` in the registry and adds abelian group axioms.
fn declare_additive_abelian_group_axioms(
    s: &mut Structure,
    reg: &mut OpRegistry,
) -> Result<(OperationId, OperationId, OperationId), RegistryError> {
    let add = reg.declare_operation("add", 2)?;
    let zero = reg.declare_operation("zero", 0)?;
    let neg = reg.declare_operation("neg", 1)?;
    let (x, y, z) = (var("x"), var("y"), var("z"));

    let cat = "additive_group";

    s.add_equation(
        eq(
            "add_associativity",
            bin(add, bin(add, x.clone(), y.clone()), z.clone()),
            bin(add, x.clone(), bin(add, y.clone(), z)),
        )
        .with_category(cat),
    );
    s.add_equation(
        eq(
            "add_left_identity",
            bin(add, cnst(zero), x.clone()),
            x.clone(),
        )
        .with_category(cat),
    );
    s.add_equation(
        eq(
            "add_right_identity",
            bin(add, x.clone(), cnst(zero)),
            x.clone(),
        )
        .with_category(cat),
    );
    s.add_equation(
        eq(
            "add_left_inverse",
            bin(add, un(neg, x.clone()), x.clone()),
            cnst(zero),
        )
        .with_category(cat),
    );
    s.add_equation(
        eq(
            "add_right_inverse",
            bin(add, x.clone(), un(neg, x.clone())),
            cnst(zero),
        )
        .with_category(cat),
    );
    s.add_equation(
        eq(
            "add_commutativity",
            bin(add, x.clone(), y.clone()),
            bin(add, y, x),
        )
        .with_category(cat),
    );

    Ok((add, zero, neg))
}

/// Declares `one/0` in the registry and adds multiplicative monoid axioms for `mul`.
fn declare_multiplicative_monoid_axioms(
    s: &mut Structure,
    reg: &mut OpRegistry,
    mul: OperationId,
) -> Result<OperationId, RegistryError> {
    let one = reg.declare_operation("one", 0)?;
    let x = var("x");

    let cat = "multiplicative_monoid";

    let (y, z) = (var("y"), var("z"));
    s.add_equation(
        eq(
            "mul_associativity",
            bin(mul, bin(mul, x.clone(), y.clone()), z.clone()),
            bin(mul, x.clone(), bin(mul, y, z)),
        )
        .with_category(cat),
    );
    s.add_equation(
        eq(
            "mul_left_identity",
            bin(mul, cnst(one), x.clone()),
            x.clone(),
        )
        .with_category(cat),
    );
    s.add_equation(
        eq("mul_right_identity", bin(mul, x.clone(), cnst(one)), x).with_category(cat),
    );

    Ok(one)
}

/// Adds distributivity axioms for `mul` over `add`.
fn declare_distributivity_axioms(s: &mut Structure, mul: OperationId, add: OperationId) {
    let (x, y, z) = (var("x"), var("y"), var("z"));

    let cat = "distributivity";

    s.add_equation(
        eq(
            "left_distributivity",
            bin(mul, x.clone(), bin(add, y.clone(), z.clone())),
            bin(
                add,
                bin(mul, x.clone(), y.clone()),
                bin(mul, x.clone(), z.clone()),
            ),
        )
        .with_category(cat),
    );
    s.add_equation(
        eq(
            "right_distributivity",
            bin(mul, bin(add, x.clone(), y.clone()), z.clone()),
            bin(add, bin(mul, x, z.clone()), bin(mul, y, z)),
        )
        .with_category(cat),
    );
}

// ── public builders ─────────────────────────────────────────

/// A semigroup: a set with an associative binary operation `mul`.
///
/// Operations: `mul/2`
/// Equations: associativity (1)
pub fn semigroup(reg: &mut OpRegistry) -> Result<Structure, RegistryError> {
    let mut s = Structure::new("Semigroup");
    declare_semigroup_axioms(&mut s, reg)?;
    Ok(s)
}

/// A monoid: a semigroup with an identity element `e`.
///
/// Operations: `mul/2`, `e/0`
/// Equations: associativity, left/right identity (3)
pub fn monoid(reg: &mut OpRegistry) -> Result<Structure, RegistryError> {
    let mut s = Structure::new("Monoid");
    let mul = declare_semigroup_axioms(&mut s, reg)?;
    declare_monoid_axioms(&mut s, reg, mul)?;
    Ok(s)
}

/// A group: a monoid with an inverse operation `inv`.
///
/// Operations: `mul/2`, `e/0`, `inv/1`
/// Equations: associativity, left/right identity, left/right inverse (5)
pub fn group(reg: &mut OpRegistry) -> Result<Structure, RegistryError> {
    let mut s = Structure::new("Group");
    let mul = declare_semigroup_axioms(&mut s, reg)?;
    let e = declare_monoid_axioms(&mut s, reg, mul)?;
    declare_group_axioms(&mut s, reg, mul, e)?;
    Ok(s)
}

/// A ring: an abelian group under addition, a monoid under multiplication,
/// with multiplication distributing over addition.
///
/// Operations: `add/2`, `zero/0`, `neg/1`, `mul/2`, `one/0`
/// Equations: additive abelian group (6) + multiplicative monoid (3)
///            + distributivity (2) = 11
pub fn ring(reg: &mut OpRegistry) -> Result<Structure, RegistryError> {
    let mut s = Structure::new("Ring");

    let (add, _zero, _neg) = declare_additive_abelian_group_axioms(&mut s, reg)?;

    let mul = reg.declare_operation("mul", 2)?;
    let _one = declare_multiplicative_monoid_axioms(&mut s, reg, mul)?;

    declare_distributivity_axioms(&mut s, mul, add);

    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semigroup() {
        let mut reg = OpRegistry::new();
        let s = semigroup(&mut reg).unwrap();
        assert!(s.validate(&reg).is_ok());
        assert_eq!(s.name(), "Semigroup");
        assert_eq!(s.referenced_operations().len(), 1);
        assert_eq!(s.equations().len(), 1);
        assert!(reg.find_operation("mul").is_some());
    }

    #[test]
    fn test_monoid() {
        let mut reg = OpRegistry::new();
        let s = monoid(&mut reg).unwrap();
        assert!(s.validate(&reg).is_ok());
        assert_eq!(s.name(), "Monoid");
        assert_eq!(s.referenced_operations().len(), 2);
        assert_eq!(s.equations().len(), 3);
        assert!(reg.find_operation("e").is_some());
    }

    #[test]
    fn test_group() {
        let mut reg = OpRegistry::new();
        let s = group(&mut reg).unwrap();
        assert!(s.validate(&reg).is_ok());
        assert_eq!(s.name(), "Group");
        assert_eq!(s.referenced_operations().len(), 3);
        assert_eq!(s.equations().len(), 5);
        assert!(reg.find_operation("inv").is_some());
    }

    #[test]
    fn test_ring() {
        let mut reg = OpRegistry::new();
        let s = ring(&mut reg).unwrap();
        assert!(s.validate(&reg).is_ok());
        assert_eq!(s.name(), "Ring");
        assert_eq!(s.referenced_operations().len(), 5);
        assert_eq!(s.equations().len(), 11);
        assert!(reg.find_operation("add").is_some());
        assert!(reg.find_operation("zero").is_some());
        assert!(reg.find_operation("neg").is_some());
        assert!(reg.find_operation("mul").is_some());
        assert!(reg.find_operation("one").is_some());
    }

    #[test]
    fn test_hierarchy_operation_counts() {
        let mut reg = OpRegistry::new();
        let sg = semigroup(&mut reg).unwrap();
        let mo = monoid(&mut reg).unwrap();
        let gr = group(&mut reg).unwrap();
        let ri = ring(&mut reg).unwrap();

        assert!(sg.referenced_operations().len() < mo.referenced_operations().len());
        assert!(mo.referenced_operations().len() < gr.referenced_operations().len());
        assert!(gr.referenced_operations().len() < ri.referenced_operations().len());

        assert!(sg.equations().len() < mo.equations().len());
        assert!(mo.equations().len() < gr.equations().len());
        assert!(gr.equations().len() < ri.equations().len());
    }

    #[test]
    fn test_group_equation_names() {
        let mut reg = OpRegistry::new();
        let g = group(&mut reg).unwrap();
        let names: Vec<&str> = g.equations().iter().map(|e| e.name()).collect();
        assert!(names.contains(&"mul_associativity"));
        assert!(names.contains(&"mul_left_identity"));
        assert!(names.contains(&"mul_right_identity"));
        assert!(names.contains(&"mul_left_inverse"));
        assert!(names.contains(&"mul_right_inverse"));
    }

    #[test]
    fn test_ring_equation_names() {
        let mut reg = OpRegistry::new();
        let r = ring(&mut reg).unwrap();
        let names: Vec<&str> = r.equations().iter().map(|e| e.name()).collect();
        assert!(names.contains(&"add_associativity"));
        assert!(names.contains(&"add_commutativity"));
        assert!(names.contains(&"add_left_identity"));
        assert!(names.contains(&"add_right_identity"));
        assert!(names.contains(&"add_left_inverse"));
        assert!(names.contains(&"add_right_inverse"));
        assert!(names.contains(&"mul_associativity"));
        assert!(names.contains(&"mul_left_identity"));
        assert!(names.contains(&"mul_right_identity"));
        assert!(names.contains(&"left_distributivity"));
        assert!(names.contains(&"right_distributivity"));
    }

    #[test]
    fn test_shared_registry_cross_structure() {
        let mut reg = OpRegistry::new();
        let grp = group(&mut reg).unwrap();
        let rng = ring(&mut reg).unwrap();

        // "mul" is shared between group and ring — same OperationId
        let mul_from_group = grp
            .referenced_operations()
            .iter()
            .find(|&&id| reg.get_operation(id).unwrap().name() == "mul")
            .copied()
            .unwrap();
        let mul_from_ring = rng
            .referenced_operations()
            .iter()
            .find(|&&id| reg.get_operation(id).unwrap().name() == "mul")
            .copied()
            .unwrap();
        assert_eq!(mul_from_group, mul_from_ring);
    }

    #[test]
    fn test_semigroup_and_monoid_share_mul() {
        let mut reg = OpRegistry::new();
        let sg = semigroup(&mut reg).unwrap();
        let mo = monoid(&mut reg).unwrap();

        let sg_mul = sg.referenced_operations()[0]; // only op is mul
        let mo_mul = mo
            .referenced_operations()
            .iter()
            .find(|&&id| reg.get_operation(id).unwrap().name() == "mul")
            .copied()
            .unwrap();
        assert_eq!(sg_mul, mo_mul, "semigroup and monoid should share the same mul OperationId");
    }
}
