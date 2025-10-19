//! Term type for Raft consensus algorithm
//!
//! In Raft, time is divided into terms of arbitrary length.
//! Terms are numbered with consecutive integers and act as a logical clock.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents a term in the Raft consensus algorithm.
///
/// Terms are monotonically increasing and are used to detect obsolete information.
/// Each server stores a current term number which increases monotonically over time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Term(u64);

impl Term {
    /// Creates a new term with value 0 (initial term)
    #[must_use]
    pub fn initial() -> Self {
        Self(0)
    }

    /// Creates a new term with the specified value
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the next term (increments by 1)
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }

    /// Returns the inner value
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Term({})", self.0)
    }
}

impl From<u64> for Term {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<Term> for u64 {
    fn from(term: Term) -> Self {
        term.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_term() {
        let term = Term::initial();
        assert_eq!(term.value(), 0);
    }

    #[test]
    fn test_new_term() {
        let term = Term::new(42);
        assert_eq!(term.value(), 42);
    }

    #[test]
    fn test_next_term() {
        let term = Term::new(5);
        let next = term.next();
        assert_eq!(next.value(), 6);
    }

    #[test]
    fn test_term_ordering() {
        let t1 = Term::new(1);
        let t2 = Term::new(2);
        assert!(t1 < t2);
        assert!(t2 > t1);
        assert_eq!(t1, t1);
    }

    #[test]
    fn test_term_display() {
        let term = Term::new(42);
        assert_eq!(format!("{term}"), "Term(42)");
    }

    #[test]
    fn test_term_from_u64() {
        let term: Term = 10_u64.into();
        assert_eq!(term.value(), 10);
    }

    #[test]
    fn test_u64_from_term() {
        let term = Term::new(20);
        let value: u64 = term.into();
        assert_eq!(value, 20);
    }
}
