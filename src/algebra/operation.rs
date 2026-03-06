//! Algebraic operations (operators with arity constraints).

use std::fmt;

/// Stable identifier for an operation within a structure/signature.
/// Allocated by [`crate::algebra::OpRegistry::declare_operation`]; used in [`crate::algebra::Term`] instead of string names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OperationId(pub u32);

impl OperationId {
    #[inline]
    pub const fn id(self) -> u32 {
        self.0
    }
}

impl fmt::Display for OperationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// Arity constraint for an algebraic operation.
///
/// - `Exact(n)` — the operation takes exactly `n` arguments.
/// - `AtLeast(n)` — the operation takes `n` or more arguments (variadic but finite).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Arity {
    /// Fixed number of arguments.
    Exact(usize),
    /// Minimum number of arguments (variadic).
    AtLeast(usize),
}

impl Arity {
    /// Returns `true` if the given argument count satisfies this arity constraint.
    pub fn accepts(&self, count: usize) -> bool {
        match self {
            Arity::Exact(n) => count == *n,
            Arity::AtLeast(n) => count >= *n,
        }
    }
}

impl fmt::Display for Arity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Arity::Exact(n) => write!(f, "{}", n),
            Arity::AtLeast(n) => write!(f, "{}+", n),
        }
    }
}

/// An algebraic operation defined by its name and arity constraint.
///
/// Examples:
/// - `mul` (arity `Exact(2)`): fixed binary multiplication
/// - `add` (arity `AtLeast(2)`): variadic addition
/// - `inv` (arity `Exact(1)`): unary inverse
/// - `e` (arity `Exact(0)`): nullary identity constant
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Operation {
    name: String,
    arity: Arity,
}

impl Operation {
    pub fn new(name: impl Into<String>, arity: usize) -> Self {
        Operation {
            name: name.into(),
            arity: Arity::Exact(arity),
        }
    }

    pub fn with_arity(name: impl Into<String>, arity: Arity) -> Self {
        Operation {
            name: name.into(),
            arity,
        }
    }

    pub fn nullary(name: impl Into<String>) -> Self {
        Self::new(name, 0)
    }

    pub fn unary(name: impl Into<String>) -> Self {
        Self::new(name, 1)
    }

    pub fn binary(name: impl Into<String>) -> Self {
        Self::new(name, 2)
    }

    pub fn at_least(name: impl Into<String>, min: usize) -> Self {
        Self::with_arity(name, Arity::AtLeast(min))
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn arity(&self) -> Arity {
        self.arity
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.name, self.arity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_creation() {
        let mul = Operation::binary("mul");
        assert_eq!(mul.name(), "mul");
        assert_eq!(mul.arity(), Arity::Exact(2));

        let inv = Operation::unary("inv");
        assert_eq!(inv.arity(), Arity::Exact(1));

        let e = Operation::nullary("e");
        assert_eq!(e.arity(), Arity::Exact(0));
    }

    #[test]
    fn test_variadic_operation() {
        let add = Operation::at_least("add", 2);
        assert_eq!(add.name(), "add");
        assert_eq!(add.arity(), Arity::AtLeast(2));
    }

    #[test]
    fn test_arity_accepts() {
        assert!(Arity::Exact(2).accepts(2));
        assert!(!Arity::Exact(2).accepts(1));
        assert!(!Arity::Exact(2).accepts(3));

        assert!(Arity::AtLeast(2).accepts(2));
        assert!(Arity::AtLeast(2).accepts(3));
        assert!(Arity::AtLeast(2).accepts(100));
        assert!(!Arity::AtLeast(2).accepts(1));
        assert!(!Arity::AtLeast(2).accepts(0));

        assert!(Arity::Exact(0).accepts(0));
        assert!(Arity::AtLeast(0).accepts(0));
        assert!(Arity::AtLeast(0).accepts(5));
    }

    #[test]
    fn test_operation_display() {
        assert_eq!(format!("{}", Operation::binary("mul")), "mul/2");
        assert_eq!(format!("{}", Operation::at_least("add", 2)), "add/2+");
        assert_eq!(format!("{}", Operation::nullary("e")), "e/0");
    }

    #[test]
    fn test_operation_equality() {
        let a = Operation::binary("mul");
        let b = Operation::binary("mul");
        let c = Operation::unary("mul");
        let d = Operation::at_least("mul", 2);
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d); // Exact(2) != AtLeast(2)
    }
}
