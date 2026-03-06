//! Builders for common algebraic structures.
//!
//! Each builder returns a fully declared [`Structure`] with the appropriate
//! operations and equational axioms. The structures form a natural hierarchy:
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
//! use relatum::algebra::builders;
//!
//! let group = builders::group().unwrap();
//! assert!(group.validate().is_ok());
//! assert_eq!(group.operations().len(), 3); // mul, inv, e
//! assert_eq!(group.equations().len(), 5);
//! ```

use super::equation::Equation;
use super::operation::OperationId;
use super::structure::{Structure, StructureError};
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

/// Declares `mul/2` and adds associativity.
/// Returns the `mul` OperationId.
fn declare_semigroup_axioms(s: &mut Structure) -> Result<OperationId, StructureError> {
    let mul = s.declare_operation("mul", 2)?;
    let (x, y, z) = (var("x"), var("y"), var("z"));

    // mul(mul(x, y), z) = mul(x, mul(y, z))
    s.add_equation(eq(
        "mul_associativity",
        bin(mul, bin(mul, x.clone(), y.clone()), z.clone()),
        bin(mul, x, bin(mul, y, z)),
    ));

    Ok(mul)
}

/// Declares `e/0` and adds identity axioms for `mul`.
/// Returns the `e` OperationId.
fn declare_monoid_axioms(
    s: &mut Structure,
    mul: OperationId,
) -> Result<OperationId, StructureError> {
    let e = s.declare_operation("e", 0)?;
    let x = var("x");

    // mul(e, x) = x
    s.add_equation(eq(
        "mul_left_identity",
        bin(mul, cnst(e), x.clone()),
        x.clone(),
    ));
    // mul(x, e) = x
    s.add_equation(eq("mul_right_identity", bin(mul, x.clone(), cnst(e)), x));

    Ok(e)
}

/// Declares `inv/1` and adds inverse axioms for `mul` with identity `e`.
/// Returns the `inv` OperationId.
fn declare_group_axioms(
    s: &mut Structure,
    mul: OperationId,
    e: OperationId,
) -> Result<OperationId, StructureError> {
    let inv = s.declare_operation("inv", 1)?;
    let x = var("x");

    // mul(inv(x), x) = e
    s.add_equation(eq(
        "mul_left_inverse",
        bin(mul, un(inv, x.clone()), x.clone()),
        cnst(e),
    ));
    // mul(x, inv(x)) = e
    s.add_equation(eq(
        "mul_right_inverse",
        bin(mul, x.clone(), un(inv, x)),
        cnst(e),
    ));

    Ok(inv)
}

/// Declares `add/2`, `zero/0`, `neg/1` and adds abelian group axioms.
/// Returns `(add, zero, neg)`.
fn declare_additive_abelian_group_axioms(
    s: &mut Structure,
) -> Result<(OperationId, OperationId, OperationId), StructureError> {
    let add = s.declare_operation("add", 2)?;
    let zero = s.declare_operation("zero", 0)?;
    let neg = s.declare_operation("neg", 1)?;
    let (x, y, z) = (var("x"), var("y"), var("z"));

    // associativity: add(add(x,y),z) = add(x,add(y,z))
    s.add_equation(eq(
        "add_associativity",
        bin(add, bin(add, x.clone(), y.clone()), z.clone()),
        bin(add, x.clone(), bin(add, y.clone(), z)),
    ));
    // left identity: add(zero, x) = x
    s.add_equation(eq(
        "add_left_identity",
        bin(add, cnst(zero), x.clone()),
        x.clone(),
    ));
    // right identity: add(x, zero) = x
    s.add_equation(eq(
        "add_right_identity",
        bin(add, x.clone(), cnst(zero)),
        x.clone(),
    ));
    // left inverse: add(neg(x), x) = zero
    s.add_equation(eq(
        "add_left_inverse",
        bin(add, un(neg, x.clone()), x.clone()),
        cnst(zero),
    ));
    // right inverse: add(x, neg(x)) = zero
    s.add_equation(eq(
        "add_right_inverse",
        bin(add, x.clone(), un(neg, x.clone())),
        cnst(zero),
    ));
    // commutativity: add(x, y) = add(y, x)
    s.add_equation(eq(
        "add_commutativity",
        bin(add, x.clone(), y.clone()),
        bin(add, y, x),
    ));

    Ok((add, zero, neg))
}

/// Declares `one/0` and adds multiplicative monoid axioms for `mul`.
/// Returns the `one` OperationId.
fn declare_multiplicative_monoid_axioms(
    s: &mut Structure,
    mul: OperationId,
) -> Result<OperationId, StructureError> {
    let one = s.declare_operation("one", 0)?;
    let x = var("x");

    // mul(mul(x,y),z) = mul(x,mul(y,z))
    let (y, z) = (var("y"), var("z"));
    s.add_equation(eq(
        "mul_associativity",
        bin(mul, bin(mul, x.clone(), y.clone()), z.clone()),
        bin(mul, x.clone(), bin(mul, y, z)),
    ));
    // mul(one, x) = x
    s.add_equation(eq(
        "mul_left_identity",
        bin(mul, cnst(one), x.clone()),
        x.clone(),
    ));
    // mul(x, one) = x
    s.add_equation(eq("mul_right_identity", bin(mul, x.clone(), cnst(one)), x));

    Ok(one)
}

/// Adds distributivity axioms for `mul` over `add`.
fn declare_distributivity_axioms(s: &mut Structure, mul: OperationId, add: OperationId) {
    let (x, y, z) = (var("x"), var("y"), var("z"));

    // mul(x, add(y, z)) = add(mul(x, y), mul(x, z))
    s.add_equation(eq(
        "left_distributivity",
        bin(mul, x.clone(), bin(add, y.clone(), z.clone())),
        bin(
            add,
            bin(mul, x.clone(), y.clone()),
            bin(mul, x.clone(), z.clone()),
        ),
    ));
    // mul(add(x, y), z) = add(mul(x, z), mul(y, z))
    s.add_equation(eq(
        "right_distributivity",
        bin(mul, bin(add, x.clone(), y.clone()), z.clone()),
        bin(add, bin(mul, x, z.clone()), bin(mul, y, z)),
    ));
}

// ── public builders ─────────────────────────────────────────

/// A semigroup: a set with an associative binary operation `mul`.
///
/// Operations: `mul/2`
/// Equations: associativity (1)
pub fn semigroup() -> Result<Structure, StructureError> {
    let mut s = Structure::new("Semigroup");
    declare_semigroup_axioms(&mut s)?;
    Ok(s)
}

/// A monoid: a semigroup with an identity element `e`.
///
/// Operations: `mul/2`, `e/0`
/// Equations: associativity, left/right identity (3)
pub fn monoid() -> Result<Structure, StructureError> {
    let mut s = Structure::new("Monoid");
    let mul = declare_semigroup_axioms(&mut s)?;
    declare_monoid_axioms(&mut s, mul)?;
    Ok(s)
}

/// A group: a monoid with an inverse operation `inv`.
///
/// Operations: `mul/2`, `e/0`, `inv/1`
/// Equations: associativity, left/right identity, left/right inverse (5)
pub fn group() -> Result<Structure, StructureError> {
    let mut s = Structure::new("Group");
    let mul = declare_semigroup_axioms(&mut s)?;
    let e = declare_monoid_axioms(&mut s, mul)?;
    declare_group_axioms(&mut s, mul, e)?;
    Ok(s)
}

/// A ring: an abelian group under addition, a monoid under multiplication,
/// with multiplication distributing over addition.
///
/// Operations: `add/2`, `zero/0`, `neg/1`, `mul/2`, `one/0`
/// Equations: additive abelian group (6) + multiplicative monoid (3)
///            + distributivity (2) = 11
pub fn ring() -> Result<Structure, StructureError> {
    let mut s = Structure::new("Ring");

    // additive abelian group
    let (add, _zero, _neg) = declare_additive_abelian_group_axioms(&mut s)?;

    // multiplicative monoid
    let mul = s.declare_operation("mul", 2)?;
    let _one = declare_multiplicative_monoid_axioms(&mut s, mul)?;

    // distributivity
    declare_distributivity_axioms(&mut s, mul, add);

    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semigroup() {
        let s = semigroup().unwrap();
        assert!(s.validate().is_ok());
        assert_eq!(s.name(), "Semigroup");
        assert_eq!(s.operations().len(), 1); // mul
        assert_eq!(s.equations().len(), 1); // associativity
        assert!(s.find_operation("mul").is_some());
    }

    #[test]
    fn test_monoid() {
        let s = monoid().unwrap();
        assert!(s.validate().is_ok());
        assert_eq!(s.name(), "Monoid");
        assert_eq!(s.operations().len(), 2); // mul, e
        assert_eq!(s.equations().len(), 3); // assoc + 2 identity
        assert!(s.find_operation("e").is_some());
    }

    #[test]
    fn test_group() {
        let s = group().unwrap();
        assert!(s.validate().is_ok());
        assert_eq!(s.name(), "Group");
        assert_eq!(s.operations().len(), 3); // mul, e, inv
        assert_eq!(s.equations().len(), 5); // assoc + 2 identity + 2 inverse
        assert!(s.find_operation("inv").is_some());
    }

    #[test]
    fn test_ring() {
        let s = ring().unwrap();
        assert!(s.validate().is_ok());
        assert_eq!(s.name(), "Ring");
        assert_eq!(s.operations().len(), 5); // add, zero, neg, mul, one
        assert_eq!(s.equations().len(), 11); // 6 + 3 + 2
        assert!(s.find_operation("add").is_some());
        assert!(s.find_operation("zero").is_some());
        assert!(s.find_operation("neg").is_some());
        assert!(s.find_operation("mul").is_some());
        assert!(s.find_operation("one").is_some());
    }

    #[test]
    fn test_hierarchy_operation_counts() {
        let sg = semigroup().unwrap();
        let mo = monoid().unwrap();
        let gr = group().unwrap();
        let ri = ring().unwrap();

        // Each level adds operations
        assert!(sg.operations().len() < mo.operations().len());
        assert!(mo.operations().len() < gr.operations().len());
        assert!(gr.operations().len() < ri.operations().len());

        // Each level adds equations
        assert!(sg.equations().len() < mo.equations().len());
        assert!(mo.equations().len() < gr.equations().len());
        assert!(gr.equations().len() < ri.equations().len());
    }

    #[test]
    fn test_group_equation_names() {
        let g = group().unwrap();
        let names: Vec<&str> = g.equations().iter().map(|e| e.name()).collect();
        assert!(names.contains(&"mul_associativity"));
        assert!(names.contains(&"mul_left_identity"));
        assert!(names.contains(&"mul_right_identity"));
        assert!(names.contains(&"mul_left_inverse"));
        assert!(names.contains(&"mul_right_inverse"));
    }

    #[test]
    fn test_ring_equation_names() {
        let r = ring().unwrap();
        let names: Vec<&str> = r.equations().iter().map(|e| e.name()).collect();
        // additive group
        assert!(names.contains(&"add_associativity"));
        assert!(names.contains(&"add_commutativity"));
        assert!(names.contains(&"add_left_identity"));
        assert!(names.contains(&"add_right_identity"));
        assert!(names.contains(&"add_left_inverse"));
        assert!(names.contains(&"add_right_inverse"));
        // multiplicative monoid
        assert!(names.contains(&"mul_associativity"));
        assert!(names.contains(&"mul_left_identity"));
        assert!(names.contains(&"mul_right_identity"));
        // distributivity
        assert!(names.contains(&"left_distributivity"));
        assert!(names.contains(&"right_distributivity"));
    }
}
