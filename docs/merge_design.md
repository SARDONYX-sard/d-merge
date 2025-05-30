# Merge Memo

```txt
op: [[1..10], [15..20]]
value: [[`dummy` * 9], [`dummy2` * 4]]

issue: prev: 1..10, next: 2..10 => diff key

key: 1..10, value: ArrayMap[`dummy` * 9]

1/3 par_iter cache arrays
Dashmap<OpRange, ArrayMap>

insert, then lock guard -> check range
Apply Array: HashMap.iter -> each check range

nemesis_xml: parse
   [1, 2, 3, 4]: slide(1) op: add
                  [6, 7, 8, 9, 10]: slide(1) op: add

                              [10, , 11, 12, 13, 14, 15]: 2  op: add
                                                    [15, 16, 17, 18, 19, 20]: 2  op: add

      [2, 3, 4, 5]: 3 op: remove

let mut map = HashMapWrapper {
        key: { Op, [1..10] }, value: (value: [], priority: 1)
        key: { Op, [2..5]  }, value: (value: [], priority: 2)
        key: { Op, [10..15] }, value: [], (value: [], priority: 3)
};

2/3 range battle(cannot par_iter)
map.sort_by_priority();

3/3. apply last vector(cannot par_iter)
let new_value = ArrayMap::new();  // array[max_range]

// add
for (key, value) in map  // priority
    new_value = value[range];

// remove
-> new_value.into_vec() -> [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]
```
