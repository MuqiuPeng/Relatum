//! Algebraic operations (operators with fixed arity).

use std::fmt;

/// An algebraic operation defined by its name and arity.
///
/// Examples:
/// - `mul` (arity 2): binary multiplication
/// - `inv` (arity 1): unary inverse
/// - `e` (arity 0): nullary identity constant
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Operation {
    name: String,
    arity: usize,
}

impl Operation {
    pub fn new(name: impl Into<String>, arity: usize) -> Self {
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

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn arity(&self) -> usize {
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
        assert_eq!(mul.arity(), 2);

        let inv = Operation::unary("inv");
        assert_eq!(inv.arity(), 1);

        let e = Operation::nullary("e");
        assert_eq!(e.arity(), 0);
    }

    #[test]
    fn test_operation_display() {
        let mul = Operation::binary("mul");
        assert_eq!(format!("{}", mul), "mul/2");
    }

    #[test]
    fn test_operation_equality() {
        let a = Operation::binary("mul");
        let b = Operation::binary("mul");
        let c = Operation::unary("mul");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
