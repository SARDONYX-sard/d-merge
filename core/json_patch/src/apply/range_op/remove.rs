use crate::range::Range;
#[cfg(feature = "rayon")]
use rayon::iter::ParallelDrainRange;
use simd_json::borrowed::Value;

/// Remove the `range` portion of `target`.
///
/// # Panics
/// - Panics if `range` is out of bounds.
pub fn handle_remove(target: &mut Vec<Value<'_>>, range: Range) {
    match range {
        Range::Index(index) => {
            target.remove(index);
        }
        Range::FromTo(range) => {
            #[cfg(feature = "rayon")]
            target.par_drain(range);
            #[cfg(not(feature = "rayon"))]
            target.drain(range);
        }
        Range::To(range) => {
            #[cfg(feature = "rayon")]
            target.par_drain(range);
            #[cfg(not(feature = "rayon"))]
            target.drain(range);
        }
        Range::From(range) => {
            #[cfg(feature = "rayon")]
            target.par_drain(range);
            #[cfg(not(feature = "rayon"))]
            target.drain(range);
        }
        Range::Full => target.clear(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simd_json::{base::ValueTryAsArrayMut, json_typed};

    #[test]
    fn test_remove_index_range() {
        let mut target = json_typed!(borrowed, {
            "array_key": [1, 2, 3]
        });

        handle_remove(
            target["array_key"].try_as_array_mut().unwrap(),
            Range::Index(1),
        );

        let expected = json_typed!(borrowed, {
            "array_key": [1, 3]
        });
        assert_eq!(target, expected);
    }

    #[test]
    fn test_remove_to_from_range() {
        let mut target = json_typed!(borrowed, {
            "array_key": [1, 2, 3, 4, 5]
        });

        handle_remove(
            target["array_key"].try_as_array_mut().unwrap(),
            Range::FromTo(1..3),
        );

        let expected = json_typed!(borrowed, {
            "array_key": [1, 4, 5]
        });
        assert_eq!(target, expected);
    }

    #[test]
    fn test_remove_from_range() {
        let mut target = json_typed!(borrowed, {
            "array_key": [1, 2, 3, 4, 5]
        });

        handle_remove(
            target["array_key"].try_as_array_mut().unwrap(),
            Range::From(2..),
        );

        let expected = json_typed!(borrowed, {
            "array_key": [1, 2]
        });
        assert_eq!(target, expected);
    }

    #[test]
    fn test_remove_to_range() {
        let mut target = json_typed!(borrowed, {
            "array_key": [1, 2, 3, 4, 5]
        });

        handle_remove(
            target["array_key"].try_as_array_mut().unwrap(),
            Range::To(..3),
        );

        let expected = json_typed!(borrowed, {
            "array_key": [4, 5]
        });
        assert_eq!(target, expected);
    }

    #[test]
    fn test_remove_full_range() {
        let mut target = json_typed!(borrowed, {
            "array_key": [1, 2, 3]
        });

        handle_remove(target["array_key"].try_as_array_mut().unwrap(), Range::Full);

        let expected = json_typed!(borrowed, {
            "array_key": []
        });
        assert_eq!(target, expected);
    }
}
