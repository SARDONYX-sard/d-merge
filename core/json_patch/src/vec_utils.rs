// Conditional import of rayon traits
#[cfg(feature = "rayon")]
use rayon::prelude::*;

/// Trait to abstract over conditional extension behavior based on feature flags.
///
/// This allows calling `.smart_extend(...)` on containers like `Vec<T>`,
/// using `extend` when the `rayon` feature is **disabled**, or `par_extend` when **enabled**.
///
/// # Example
///
/// ```rust
/// let mut vec = Vec::new();
/// vec.smart_extend(vec![1, 2, 3]);
/// ```
pub trait SmartExtend<T> {
    /// Extend the container using an iterator, which could be sequential or parallel
    /// depending on whether the `rayon` feature is enabled.
    fn smart_extend<I>(&mut self, iter: I)
    where
        I: SmartIntoIter<Item = T>;
}

/// A trait that abstracts over `IntoIterator` and `IntoParallelIterator`.
///
/// The implementation returns either a standard iterator or a parallel iterator
/// depending on whether the `rayon` feature is enabled.
pub trait SmartIntoIter {
    /// The item type produced by the iterator.
    type Item;

    /// The iterator type, either sequential or parallel depending on feature.
    #[cfg(not(feature = "rayon"))]
    type Iter: Iterator<Item = Self::Item>;

    #[cfg(feature = "rayon")]
    type Iter: ParallelIterator<Item = Self::Item>;

    /// Convert self into the appropriate iterator type.
    fn smart_iter(self) -> Self::Iter;
}

// === Vec<T> Implementation ===

impl<T: Send> SmartExtend<T> for Vec<T> {
    fn smart_extend<I>(&mut self, iter: I)
    where
        I: SmartIntoIter<Item = T>,
    {
        #[cfg(not(feature = "rayon"))]
        {
            self.extend(iter.smart_iter());
        }

        #[cfg(feature = "rayon")]
        {
            self.par_extend(iter.smart_iter());
        }
    }
}

// === SmartIntoIter Implementation (sequential) ===

#[cfg(not(feature = "rayon"))]
impl<I, T> SmartIntoIter for I
where
    I: IntoIterator<Item = T>,
{
    type Item = T;
    type Iter = I::IntoIter;

    fn smart_iter(self) -> Self::Iter {
        self.into_iter()
    }
}

// === SmartIntoIter Implementation (parallel) ===

#[cfg(feature = "rayon")]
impl<I, T> SmartIntoIter for I
where
    I: IntoParallelIterator<Item = T>,
    T: Send,
{
    type Item = T;
    type Iter = I::Iter;

    fn smart_iter(self) -> Self::Iter {
        self.into_par_iter()
    }
}
