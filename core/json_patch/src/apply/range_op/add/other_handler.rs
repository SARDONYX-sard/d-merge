use super::{default_value, extend};
use simd_json::borrowed::Value;
use std::iter::repeat;

pub(super) fn process_index<'value>(
    target: &mut Vec<Value<'value>>,
    other: Value<'value>,
    index: usize,
) {
    let target_len = target.len();
    if index >= target_len {
        let pad = repeat(default_value(&other)).take(index - target_len);
        extend(target, pad);
    }
    target.insert(index, other);
}

pub(super) fn process_from_to<'value>(
    target: &mut Vec<Value<'value>>,
    other: Value<'value>,
    range: std::ops::Range<usize>,
) {
    for index in range {
        // FIXME: It may not be efficient to perform resizing every time.
        if index > target.len() {
            target.resize(index, default_value(&other));
        }
        target.insert(index, other.clone());
    }
}

pub(super) fn process_to<'value>(
    target: &mut Vec<Value<'value>>,
    other: Value<'value>,
    end: usize,
) {
    process_from_to(target, other, 0..end);
}

pub(super) fn process_from<'value>(
    target: &mut Vec<Value<'value>>,
    other: Value<'value>,
    start: usize,
) {
    let target_len = target.len();
    let range = if start > target_len {
        start..start + 1
    } else {
        start..target_len
    };

    process_from_to(target, other, range);
}

#[cfg(test)]
mod tests {
    use super::*;
    use simd_json::json_typed;

    fn value_from_str(s: &str) -> Value {
        json_typed!(borrowed, s)
    }

    #[test]
    fn test_process_index_in_bounds() {
        let mut target = vec![
            value_from_str("1"),
            value_from_str("2"),
            value_from_str("3"),
        ];
        process_index(&mut target, value_from_str("4"), 1);
        assert_eq!(
            target,
            vec![
                value_from_str("1"),
                value_from_str("4"),
                value_from_str("2"),
                value_from_str("3"),
            ]
        );
    }

    #[test]
    fn test_process_index_out_of_bounds() {
        let mut target = vec![value_from_str("1")];
        process_index(&mut target, value_from_str("2"), 3);
        assert_eq!(
            target,
            vec![
                value_from_str("1"),
                default_value(&value_from_str("2")),
                default_value(&value_from_str("2")),
                value_from_str("2"),
            ]
        );
    }

    #[test]
    fn test_process_from_to_add_range() {
        let mut target = vec![
            value_from_str("1"),
            value_from_str("2"),
            value_from_str("3"),
        ];
        process_from_to(&mut target, value_from_str("9"), 1..3);
        assert_eq!(
            target,
            vec![
                value_from_str("1"),
                value_from_str("9"),
                value_from_str("9"),
                value_from_str("2"),
                value_from_str("3"),
            ]
        );
    }

    #[test]
    fn test_process_from_to_out_of_bounds() {
        let mut target = vec![value_from_str("1")];
        process_from_to(&mut target, value_from_str("9"), 2..4);
        assert_eq!(
            target,
            vec![
                value_from_str("1"),
                default_value(&value_from_str("9")),
                value_from_str("9"),
                value_from_str("9"),
            ]
        );
    }

    #[test]
    fn test_process_to_prefix_elements() {
        let mut target = vec![value_from_str("1")];

        let value = value_from_str("2");
        process_to(&mut target, value.clone(), 3);
        assert_eq!(
            target,
            vec![
                value_from_str("2"),
                value_from_str("2"),
                value_from_str("2"),
                value_from_str("1"),
            ]
        );
    }

    #[test]
    fn test_process_from_append_elements() {
        let mut target = vec![value_from_str("1")];
        let value = value_from_str("2");
        process_from(&mut target, value.clone(), 2);
        assert_eq!(
            target,
            vec![value_from_str("1"), default_value(&value), value]
        );
    }

    #[test]
    fn test_process_from() {
        let mut target = vec![value_from_str("1"), value_from_str("2")];
        let value = value_from_str("9");
        process_from(&mut target, value.clone(), 1);
        assert_eq!(
            target,
            vec![value_from_str("1"), value, value_from_str("2")]
        );
    }
}
