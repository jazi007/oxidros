//! Helper traits and utilities.

/// Trait for checking if a container contains a value.
///
/// Used primarily for range checking in parameters.
pub trait Contains {
    /// The type of value to check.
    type T;

    /// Returns true if the container contains the value.
    fn contains(&self, val: Self::T) -> bool;
}

impl Contains for (usize, usize) {
    type T = usize;
    fn contains(&self, val: Self::T) -> bool {
        (self.0..self.1).contains(&val)
    }
}
