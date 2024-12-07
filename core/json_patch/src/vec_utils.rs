#[cfg(feature = "rayon")]
use rayon::prelude::*;

/// Extend a collection with the contents of an iterator.
///
/// # Reason for this function's existence
/// Add more layers of functions to use `#[feature]` branch processing.
#[cfg(not(feature = "rayon"))]
#[inline]
pub(crate) fn extend<T, I>(target: &mut Vec<T>, iter: I)
where
    I: IntoIterator<Item = T>,
    T: Send,
{
    target.extend(iter);
}

/// Extend a collection with the contents of an iterator.
///
/// # Reason for this function's existence
/// Add more layers of functions to use `#[feature]` branch processing.
#[cfg(feature = "rayon")]
#[inline]
pub(crate) fn extend<T, I>(target: &mut Vec<T>, iter: I)
where
    // NOTE: Do not set `ParallelBridge` to a trait boundary as it is not guaranteed to be in sequence.
    // - https://docs.rs/rayon/1.10.0/rayon/iter/trait.ParallelBridge.html
    I: IntoParallelIterator<Item = T>,
    T: Send,
{
    target.par_extend(iter.into_par_iter());
}
