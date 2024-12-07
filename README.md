# D Merge(Early development stage)

diff & merge => d_merge json patch-based hkx patch

- Current serde_hkx ver. 0.5.0

## For Tester

The patch page is under development, so there is no need to submit an issue.

Convert page problem

- For the look and feel, Write d_merge issue
- Conversion feature bugs should be written in serde_hkx issues

![image](https://github.com/user-attachments/assets/1b8f0a0b-8aa2-4bd3-9cba-f75a6ff9095d)

- [Release(For test)](https://github.com/SARDONYX-sard/d-merge/releases)

## Implementation

- [x] Convert page
- [ ] Patch page <- current working
- [x] settings page

## Patch page detail

The only thing we are considering at this time is support for the Nemesis patch.
(Since I only use the Nemesis patch).

- [x] frontend
- asdsf(Not serialization),
  - [ ] Serialization
  - [x] Deserialization
- adsf,
  - [ ] Serialization
  - [x] Deserialization
- info.txt searcher.
- Merge
  - [x] Parallel json patch
  - [x] Fix range add operation of Array
  - [ ] Prioritization and conflict resolution among patches, optimization by
        merging

## Extra build

- json, yaml support
- hkx conversion log with tracing

## `Package.json` notes

Comments cannot be written in `Package.json`, so write them here.

Note that the following version must be fixed or it will not work for some
reason. (eslint in particular has many deprecated libraries at 8, but it can't
be helped)

- Biome:
  - version: 1.9.3
  - VS Code extension: 2.2.3
- eslint: ^8

- mui/x-data-grid, when changing from `7.22.2` to `7.23.1`, the `setState` in
  `handleRowSelectionModelChange` is now

  `Cannot update a component () while rendering a different component ()` and
  therefore do not use "7.23.1".

- `React19` is a new ver. stabilized on 2024/12/5, so `notistack`,
  `@monaco-editor/react` warns. In that case, use `npm i --force`.
