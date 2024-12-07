use super::default_value;
use crate::vec_utils::extend;
#[cfg(feature = "rayon")]
use rayon::iter::repeat;
use simd_json::borrowed::Value;
#[cfg(not(feature = "rayon"))]
use std::iter::repeat;

pub(super) fn process_array_index<'value>(
    target: &mut Vec<Value<'value>>,
    values: Vec<Value<'value>>,
    index: usize,
) {
    let target_len = target.len();
    if index >= target_len {
        target.resize(index, default_value(&values[0]));
    }
    target.splice(index..index, values);
}

pub(super) fn process_array_from_to<'value>(
    target: &mut Vec<Value<'value>>,
    values: Vec<Value<'value>>,
    range: std::ops::Range<usize>,
) {
    // Push pattern.
    if range.start == target.len() + 1 {
        extend(target, values);
        return;
    }

    let insert_count = range.end - range.start;

    // Insert pattern
    let target_len = target.len();
    let (prefix, suffix) = target.split_at_mut(range.start);

    let mut new_target = Vec::with_capacity(target_len + insert_count);
    new_target.extend_from_slice(prefix);
    extend(&mut new_target, values);
    new_target.extend_from_slice(suffix);
    *target = new_target;
}

/// Same action as shift(Insert `values` before `target`).
pub(super) fn process_array_to<'value>(
    target: &mut Vec<Value<'value>>,
    values: Vec<Value<'value>>,
    end: usize,
) {
    #[cfg(feature = "rayon")]
    let mut new_target: Vec<_> = {
        use rayon::iter::{
            IndexedParallelIterator as _, IntoParallelIterator as _, ParallelIterator as _,
        };
        values.into_par_iter().take(end).collect()
    };
    #[cfg(not(feature = "rayon"))]
    let mut new_target: Vec<_> = values.into_iter().take(end).collect();
    new_target.append(target);
    *target = new_target;
}

/// - When `start` <= `targe.len()`, insert operation from start.
/// - When `start` >= `target.len()`, fill up to `start` with default value, then push operation.
pub(super) fn process_array_from<'value>(
    target: &mut Vec<Value<'value>>,
    values: Vec<Value<'value>>,
    start: usize,
) {
    let target_len = target.len();
    if start >= target_len {
        // Then Grow until start size & Push.
        let pad = repeat(default_value(&values[0])).take(start - target_len);
        extend(target, pad);
        extend(target, values);
        return;
    }

    // Insert
    process_array_from_to(target, values, start..target.len());
}

#[cfg(test)]
mod tests {
    use super::*;
    use simd_json::{json_typed, prelude::*};

    #[test]
    fn test_process_array_index_insert_within_bounds() {
        let mut actual = json_typed!(borrowed, [1, 2]);

        {
            let target = actual.try_as_array_mut().unwrap();
            let values = vec![json_typed!(borrowed, 99)];
            process_array_index(target, values, 1);
        }

        let expected = json_typed!(borrowed, [1, 99, 2]);

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_process_array_index_insert_out_of_bounds() {
        let mut actual = json_typed!(borrowed, [1, 2]);

        {
            let target = actual.try_as_array_mut().unwrap();
            let values = vec![json_typed!(borrowed, 99)];
            process_array_index(target, values, 5);
        }

        let expected = json_typed!(borrowed, [1, 2, 0, 0, 0, 99]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_process_array_from_to_within_bounds() {
        let mut actual = json_typed!(borrowed, [1, 2, 3]);

        {
            let target = actual.try_as_array_mut().unwrap();
            let values = json_typed!(borrowed, [99, 100]).into_array().unwrap();
            process_array_from_to(target, values, 1..2);
        }

        let expected = json_typed!(borrowed, [1, 99, 100, 2, 3]);

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_process_array_from_to_push_pattern() {
        let mut actual = json_typed!(borrowed, [1, 2]);

        {
            let target = actual.try_as_array_mut().unwrap();
            let values = json_typed!(borrowed, [99, 100]).into_array().unwrap();
            process_array_from_to(target, values, 3..5);
        }

        let expected = json_typed!(borrowed, [1, 2, 99, 100]);

        assert_eq!(actual, expected);
    }

    #[test]
    fn process_array_from_should_insert_push_when_out_of_bounds_partially() {
        let mut actual = json_typed!(borrowed, [1, 2]);

        {
            let target = actual.try_as_array_mut().unwrap();
            let values = json_typed!(borrowed, [99, 100]).into_array().unwrap();
            process_array_from_to(target, values, 1..3);
        }

        let expected = json_typed!(borrowed, [1, 99, 100, 2]);

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_process_array_to() {
        let mut actual = json_typed!(borrowed, [1, 2]);

        {
            let target = actual.try_as_array_mut().unwrap();
            let values = json_typed!(borrowed, [99, 100]).into_array().unwrap();
            process_array_to(target, values, 3);
        }

        let expected = json_typed!(borrowed, [99, 100, 1, 2]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_process_array_from_as_insert() {
        let mut actual = json_typed!(borrowed, [1, 2]);

        {
            let target = actual.try_as_array_mut().unwrap();
            let values = json_typed!(borrowed, [99, 100]).into_array().unwrap();
            process_array_from(target, values, 1);
        }

        let expected = json_typed!(borrowed, [1, 99, 100, 2]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_process_array_from_as_push() {
        let mut actual = json_typed!(borrowed, [1, 2]);

        {
            let target = actual.try_as_array_mut().unwrap();
            let values = json_typed!(borrowed, [99, 100]).into_array().unwrap();
            process_array_from(target, values, 3);
        }

        let expected = json_typed!(borrowed, [1, 2, 0, 99, 100]);
        assert_eq!(actual, expected);
    }
}
