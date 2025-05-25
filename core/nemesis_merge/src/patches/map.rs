use dashmap::mapref::one::Ref;
use dashmap::DashMap;
use simd_json::borrowed::Value;

#[derive(Debug, Clone, Default)]
pub struct Node<'a> {
    /// when removed, then None,
    value: Option<Value<'a>>,
    // insert target index, priority
    target_index: usize,
    priority: usize,
}

/// key: array index, value: any & insert target + priority
#[derive(Debug, Clone, Default)]
pub struct ArrayMap<'a> {
    inner: DashMap<usize, Node<'a>>,
    last_index: usize,
}

impl<'a> ArrayMap<'a> {
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: DashMap::new(),
            last_index: 0,
        }
    }

    pub fn insert(&mut self, value: Value<'a>) -> Option<Node<'a>> {
        let ret = self.inner.insert(
            self.last_index,
            Node {
                value: Some(value),
                target_index: self.last_index,
                priority: 0,
            },
        );
        self.last_index += 1;

        ret
    }

    pub fn insert_after(&mut self, target_index: usize, priority: usize, value: Value<'a>) {
        if let Some(pair) = self.inner.get(&target_index) {
            let (_, node) = pair.pair();

            // Low priority
            if priority < node.priority {
                return;
            }
        };

        self.inner.insert(
            self.last_index,
            Node {
                value: Some(value),
                target_index,
                priority,
            },
        );
        self.last_index += 1;
    }

    pub fn remove(&self, key: usize) -> Option<(usize, Node<'a>)> {
        self.inner.remove(&key)
    }

    pub fn replace(&self, target_index: usize, priority: usize, value: Value<'a>) {
        if let Some(pair) = self.inner.get(&target_index) {
            let (_, node) = pair.pair();

            // Low priority
            if priority < node.priority {
                return;
            }
        };

        self.inner.insert(
            target_index,
            Node {
                value: Some(value),
                target_index,
                priority,
            },
        );
    }

    pub fn get(&self, key: usize) -> Option<Ref<'_, usize, Node<'a>>> {
        self.inner.get(&key)
    }

    // array of map.keys()
    pub fn sorted_keys(&self) -> Vec<usize> {
        let mut entries: Vec<(usize, usize, usize)> = self
            .inner
            .iter()
            .map(|r| {
                let (key, node) = r.pair();
                (*key, node.target_index, node.priority)
            })
            .collect();

        // sort by target_index, priority
        entries.sort_by_key(|&(_, target_index, priority)| (target_index, priority));

        // key
        entries.into_iter().map(|(key, _, _)| key).collect()
    }

    pub fn into_vec(self) -> Vec<Value<'a>> {
        self.into()
    }
}

impl<'a> From<&mut Vec<Value<'a>>> for ArrayMap<'a> {
    #[inline]
    fn from(value: &mut Vec<Value<'a>>) -> Self {
        let mut map = ArrayMap::new();

        let value = core::mem::take(value);
        for (i, v) in value.into_iter().enumerate() {
            map.insert_after(i, 0, v);
        }
        map
    }
}

impl<'a> From<Vec<Value<'a>>> for ArrayMap<'a> {
    #[inline]
    fn from(value: Vec<Value<'a>>) -> Self {
        let mut map = ArrayMap::new();

        for (i, v) in value.into_iter().enumerate() {
            map.insert_after(i, 0, v);
        }
        map
    }
}

impl<'a> From<ArrayMap<'a>> for Vec<Value<'a>> {
    #[inline]
    fn from(value: ArrayMap<'a>) -> Self {
        let mut output = Self::with_capacity(value.inner.len());

        let sorted_indexes = value.sorted_keys();

        for index in sorted_indexes {
            let Some((_, node)) = value.remove(index) else {
                continue;
            };
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
    fn test_sort_map_add() {
        let mut map = super::ArrayMap::new();

        map.insert(1_usize.into());
        map.insert(2_usize.into());
        map.insert(3_usize.into());
        map.insert(4_usize.into());
        assert_eq!(map.sorted_keys(), vec![0, 1, 2, 3]);

        // mods
        map.insert_after(1, 1, 5_usize.into()); // [1] = 5  (priority 1)
        map.insert_after(1, 3, 15_usize.into()); // [1] = 15(priority 3)

        assert_eq!(map.sorted_keys(), vec![0, 1, 4, 5, 2, 3]);
        assert_eq!(map.into_vec(), [1, 2, 5, 15, 3, 4]);
    }

    #[test]
    fn test_sort_map_remove() {
        let mut map = super::ArrayMap::new();

        map.insert(1_usize.into());
        map.insert(2_usize.into());
        map.insert(3_usize.into());
        map.insert(4_usize.into());
        assert_eq!(map.sorted_keys(), vec![0, 1, 2, 3]);

        // mods
        map.remove(1);
        assert_eq!(map.sorted_keys(), vec![0, 2, 3]);
    }

    #[test]
    fn tes_sort_map_replace() {
        // op: Replace, range: 0..1, 3..5, patch:[1, 2], [2, 3]
        let mut map = super::ArrayMap::new();

        map.insert(1_usize.into());
        map.insert(2_usize.into());
        map.insert(3_usize.into());
        map.insert(4_usize.into());
        assert_eq!(map.sorted_keys(), vec![0, 1, 2, 3]);

        // mods
        map.replace(1, 1, 5_usize.into()); // [1] = 5  (priority 1)
        assert_eq!(map.sorted_keys(), vec![0, 1, 2, 3]);
        assert_eq!(map.clone().into_vec(), [1, 5, 3, 4]);

        map.replace(1, 3, 15_usize.into()); // [1] = 15(priority 3)
        assert_eq!(map.sorted_keys(), vec![0, 1, 2, 3]);
        assert_eq!(map.into_vec(), [1, 15, 3, 4]);
    }
}
