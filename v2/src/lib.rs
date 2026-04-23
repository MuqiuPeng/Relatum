//! Relatum v2
//!
//! Core primitive: `R(x, y)` — a binary directed relation with no pre-assigned meaning.
//! All structure (objects, types, meaning) emerges from abstraction over R instances.
//!
//! Ontological commitments: see `docs/constitution.md`.

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct R {
    pub x: String,
    pub y: String,
}

impl R {
    pub fn new(x: impl Into<String>, y: impl Into<String>) -> Self {
        R { x: x.into(), y: y.into() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r_can_be_constructed() {
        let r = R::new("a", "b");
        assert_eq!(r.x, "a");
        assert_eq!(r.y, "b");
    }

    #[test]
    fn r_is_directional() {
        assert_ne!(R::new("a", "b"), R::new("b", "a"));
    }

    #[test]
    fn identity_is_token_based() {
        assert_eq!(R::new("a", "b"), R::new("a", "b"));
    }
}
