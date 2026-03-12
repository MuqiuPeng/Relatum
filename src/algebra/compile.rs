//! Compiler: translates algebra-layer types into relational-engine types.
//!
//! The algebra layer uses [`OperationId`]s for type-safe operation references,
//! while the relational engine uses plain strings. This module bridges the two
//! by resolving ids through an [`OpRegistry`].

use super::equation::Equation;
use super::registry::OpRegistry;
use super::structure::Structure;
use super::term::Term as AlgTerm;

use crate::relational::engine::Axiom;
use crate::relational::term::Term as RelTerm;

/// Translate an algebra `Term` (with `OperationId`) into a relational `Term`
/// (with `String` symbols) using the registry to resolve names.
pub fn compile_term(term: &AlgTerm, reg: &OpRegistry) -> RelTerm {
    match term {
        AlgTerm::Var(name) => RelTerm::var(name.clone()),
        AlgTerm::App { op, args } => {
            let name = reg
                .get_operation(*op)
                .map(|o| o.name().to_string())
                .unwrap_or_else(|| format!("#{}", op.id()));
            let rel_args: Vec<RelTerm> = args.iter().map(|a| compile_term(a, reg)).collect();
            RelTerm::app(name, rel_args)
        }
    }
}

/// Translate an algebra `Equation` into a relational `Axiom`.
///
/// The axiom will emit facts into the given `equiv_relation` (e.g. `"eq"`).
pub fn compile_equation(eq: &Equation, reg: &OpRegistry, equiv_relation: &str) -> Axiom {
    let lhs = compile_term(eq.lhs(), reg);
    let rhs = compile_term(eq.rhs(), reg);
    Axiom::new(eq.name(), lhs, rhs, equiv_relation)
}

/// Translate all equations from a `Structure` into relational `Axiom`s.
pub fn compile_structure(
    structure: &Structure,
    reg: &OpRegistry,
    equiv_relation: &str,
) -> Vec<Axiom> {
    structure
        .equations()
        .iter()
        .map(|eq| compile_equation(eq, reg, equiv_relation))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::builders;

    #[test]
    fn test_compile_var() {
        let reg = OpRegistry::new();
        let alg = AlgTerm::var("x");
        let rel = compile_term(&alg, &reg);
        assert_eq!(rel, RelTerm::var("x"));
    }

    #[test]
    fn test_compile_constant() {
        let mut reg = OpRegistry::new();
        let e = reg.declare_operation("e", 0).unwrap();
        let alg = AlgTerm::constant(e);
        let rel = compile_term(&alg, &reg);
        assert_eq!(rel, RelTerm::constant("e"));
    }

    #[test]
    fn test_compile_app() {
        let mut reg = OpRegistry::new();
        let mul = reg.declare_operation("mul", 2).unwrap();
        let e = reg.declare_operation("e", 0).unwrap();

        let alg = AlgTerm::app(mul, vec![AlgTerm::var("x"), AlgTerm::constant(e)]);
        let rel = compile_term(&alg, &reg);
        assert_eq!(
            rel,
            RelTerm::app("mul", vec![RelTerm::var("x"), RelTerm::constant("e")])
        );
    }

    #[test]
    fn test_compile_nested() {
        let mut reg = OpRegistry::new();
        let mul = reg.declare_operation("mul", 2).unwrap();

        let alg = AlgTerm::app(
            mul,
            vec![
                AlgTerm::app(mul, vec![AlgTerm::var("x"), AlgTerm::var("y")]),
                AlgTerm::var("z"),
            ],
        );
        let rel = compile_term(&alg, &reg);
        assert_eq!(
            rel,
            RelTerm::app(
                "mul",
                vec![
                    RelTerm::app("mul", vec![RelTerm::var("x"), RelTerm::var("y")]),
                    RelTerm::var("z"),
                ]
            )
        );
    }

    #[test]
    fn test_compile_equation() {
        let mut reg = OpRegistry::new();
        let mul = reg.declare_operation("mul", 2).unwrap();
        let e = reg.declare_operation("e", 0).unwrap();

        let eq = Equation::new(
            "right_id",
            AlgTerm::app(mul, vec![AlgTerm::var("x"), AlgTerm::constant(e)]),
            AlgTerm::var("x"),
        );
        let axiom = compile_equation(&eq, &reg, "equiv");
        assert_eq!(axiom.name(), "right_id");
        assert_eq!(axiom.equiv_relation(), "equiv");
        assert_eq!(
            *axiom.lhs(),
            RelTerm::app("mul", vec![RelTerm::var("x"), RelTerm::constant("e")])
        );
        assert_eq!(*axiom.rhs(), RelTerm::var("x"));
    }

    #[test]
    fn test_compile_structure() {
        let mut reg = OpRegistry::new();
        let monoid = builders::monoid(&mut reg).unwrap();

        let axioms = compile_structure(&monoid, &reg, "eq");

        // Monoid has 3 equations: mul_associativity, mul_left_identity, mul_right_identity
        assert_eq!(axioms.len(), 3);
        assert!(axioms.iter().any(|a| a.name() == "mul_associativity"));
        assert!(axioms.iter().any(|a| a.name() == "mul_left_identity"));
        assert!(axioms.iter().any(|a| a.name() == "mul_right_identity"));

        // All target the "eq" relation
        for axiom in &axioms {
            assert_eq!(axiom.equiv_relation(), "eq");
        }
    }
}
