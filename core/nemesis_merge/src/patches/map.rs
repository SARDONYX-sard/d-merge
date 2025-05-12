use dashmap::mapref::one::Ref;
use dashmap::DashMap;
use simd_json::borrowed::Value;

struct Node<'a> {
    /// when removed, then None,
    value: Option<Value<'a>>,
    // insert target index, priority
    target_index: usize,
    priority: usize,
}

/// key: array index, value: any & insert target + priority
#[repr(transparent)]
pub struct Map<'a>(pub DashMap<usize, Node<'a>>);

impl<'a> Map<'a> {
    #[inline]
    pub fn new() -> Self {
        Self(DashMap::new())
    }

    pub fn insert_after(&self, key: usize, value: Value<'a>, target_index: usize, priority: usize) {
        self.0.insert(
            key,
            Node {
                value: Some(value),
                target_index,
                priority,
            },
        );
    }

    pub fn remove(&self, key: usize) -> Option<(usize, Node<'a>)> {
        self.0.remove(&key)
    }

    pub fn get(&self, key: usize) -> Option<Ref<'_, usize, Node<'a>>> {
        self.0.get(&key)
    }

    // array of map.keys()
    pub fn sort(&self) -> Vec<usize> {
        let mut output = Vec::with_capacity(self.0.len());

        for (index, pair) in self.0.iter().enumerate() {
            let (&key, node) = pair.pair();
            let Node {
                target_index,
                priority,
                ..
            } = node;

            match output.get_mut(*target_index) {
                Some(prev_index) => {
                    let prev_priority = self.0.get(prev_index).unwrap().priority;
                    if prev_priority > *priority {
                        output.insert(*target_index, key);
                    }
                }
                None => {
                    output.insert(index, key);
                }
            }

            output[target_index + 1] = index;
        }

        output
    }
}

impl<'a> From<&mut Vec<Value<'a>>> for Map<'a> {
    fn from(value: &mut Vec<Value<'a>>) -> Self {
        let map = Map::new();

        let value = core::mem::take(value);
        for (i, v) in value.into_iter().enumerate() {
            map.insert_after(i, v, i, 0);
        }
        map
    }
}

impl<'a> From<Map<'a>> for Vec<Value<'a>> {
    fn from(value: Map<'a>) -> Self {
        let mut output = Vec::with_capacity(value.0.len());

        let sorted_indexes = value.sort();
        for index in sorted_indexes {
            let (_, node) = value.remove(index).unwrap();
            if let Some(value) = node.value {
                output.push(value);
            };
        }

        output
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_sort_map() {
        let map = super::Map::new();
        map.insert_after(0, 1_usize.into(), 0, 0);
        map.insert_after(1, 2_usize.into(), 1, 0);
        map.insert_after(2, 3_usize.into(), 2, 0);
        map.insert_after(3, 4_usize.into(), 3, 0);

        // mods
        map.insert_after(4, 5_usize.into(), 1, 1); // 4 => 1(priority 1)
        map.insert_after(5, 5_usize.into(), 1, 3); // 5 => 1(priority 3)

        assert_eq!(map.sort(), vec![0, 1, 5, 4, 2, 3]);
    }
}
