# D Merge(Early development)

diff & merge => d_merge json patch-based hkx patcher

## For Tester

The patch page is under development, so there is no need to submit an issue.

![image](https://github.com/user-attachments/assets/1b8f0a0b-8aa2-4bd3-9cba-f75a6ff9095d)

- [Release(For test)](https://github.com/SARDONYX-sard/d-merge/releases)

## Implementation

- [ ] Patch page <- current working(79% Completed)
- [x] Convert page
- [x] settings page

## Patch page detail

The only thing we are considering at this time is support for the Nemesis patch.
(Since I only use the Nemesis patch).

- [x] frontend
- asdsf(Not serialization),
  - [x] Serialization
  - [x] Deserialization
- adsf,
  - [ ] Serialization
  - [x] Deserialization
- info.txt searcher.
- Merge
  - [x] Parallel json patch
  - [x] Fix range add operation of Array
  - [ ] Prioritization and conflict resolution among patches, optimization by
        merging(60%)

## How to debug Nemesis Patches?

1. Generate hkx in Nemesis.
2. Convert the required xml in Nemesis/resource to hkx and then to xml again.
   This will generate xml that meets the d-merge specification.
3. Output the xml generated in step 2 to json for 3.
4. Use serde-hkx tool to output Nemesis hkx → json.
5. Diff the results of 3 and 4.
