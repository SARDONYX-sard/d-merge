# D Merge(Diff & Merge) hkx patcher

<div align="center">
  <a href="https://github.com/SARDONYX-sard/d-merge/releases">
    <img src="./gui/backend/tauri/icons/icon.svg" alt="D Merge"/>
  </a>

  <!-- Release Badges -->
  <p>
    <a href="https://github.com/SARDONYX-sard/d-merge/releases/latest">
      <img src="https://img.shields.io/github/v/release/SARDONYX-sard/d-merge?style=flat-square" alt="Latest Release">
    </a>
    <a href="https://github.com/SARDONYX-sard/d-merge/releases">
      <img src="https://img.shields.io/github/downloads/SARDONYX-sard/d-merge/total?style=flat-square" alt="Total Downloads">
    </a>
    <a href="https://github.com/SARDONYX-sard/d-merge/actions/workflows/release-gui.yaml">
      <img src="https://github.com/SARDONYX-sard/d-merge/actions/workflows/release-gui.yaml/badge.svg?style=flat-square" alt="Release GUI Status">
    </a>
    <a href="https://opensource.org/licenses/GPL-3.0">
      <img src="https://img.shields.io/badge/License-GPLv3-blue.svg?style=flat-square" alt="License: GPL v3">
    </a>
    <a href="https://github.com/SARDONYX-sard/d-merge/stargazers">
      <img src="https://img.shields.io/github/stars/SARDONYX-sard/d-merge?style=social" alt="GitHub Stars">
    </a>
  </p>

  <!-- Development Badges -->
  <p>
    <a href="https://github.com/SARDONYX-sard/d-merge/actions/workflows/build-and-test.yaml">
      <img src="https://github.com/SARDONYX-sard/d-merge/actions/workflows/build-and-test.yaml/badge.svg?style=flat-square" alt="Build & Test Status">
    </a>
    <a href="https://github.com/SARDONYX-sard/d-merge/issues">
      <img src="https://img.shields.io/github/issues/SARDONYX-sard/d-merge?style=flat-square" alt="Open Issues">
    </a>
    <a href="https://github.com/SARDONYX-sard/d-merge/pulls">
      <img src="https://img.shields.io/github/issues-pr/SARDONYX-sard/d-merge?style=flat-square" alt="Open PRs">
    </a>
    <a href="https://github.com/SARDONYX-sard/d-merge/commits/main">
      <img src="https://img.shields.io/github/last-commit/SARDONYX-sard/d-merge?style=flat-square" alt="Last Commit">
    </a>
    <a href="https://github.com/SARDONYX-sard/d-merge/graphs/contributors">
      <img src="https://img.shields.io/github/contributors/SARDONYX-sard/d-merge?style=flat-square" alt="Contributors">
    </a>
    <a href="https://github.com/SARDONYX-sard/d-merge">
      <img src="https://img.shields.io/github/languages/top/SARDONYX-sard/d-merge?style=flat-square" alt="Top Language">
    </a>
    <a href="https://github.com/SARDONYX-sard/d-merge">
      <img src="https://img.shields.io/github/languages/code-size/SARDONYX-sard/d-merge?style=flat-square" alt="Code Size">
    </a>
  </p>
</div>

## Release

There have been reports that the Tauri-based GUI may not work in virtual environments such as MO2.

In such cases, we recommend using the egui version build.

- [Click assets](https://github.com/SARDONYX-sard/d-merge/releases)

## Intended Use of Automatic Settings File Loading

The load order configuration file is automatically loaded each time the application runs.

It is written to <skyrim data dir>/.d_merge/d_merge_settings.json immediately before the application closes.

By utilizing this behavior as follows, settings will be automatically loaded and updated whenever you switch profiles.

```txt
D:/MO2/mods/
├── male_profile_dir/
│   └── .d_merge/
│       └── d_merge_settings.json <- output_dir: D:/MO2/mods/male_profile_dir
│
└── female_profile_dir/
    └── .d_merge/
        └── d_merge_settings.json <- output_dir: D:/MO2/mods/female_profile_dir
```

## Patch Page Progress

- [Mods Test status](./docs/test_status.md)

- GUI
  - [x] Basic frontend(Patch/Convert/Settings: Reports indicate that tauri ver. MO2 cannot be launched. Cause unknown.)
  - [x] Support Virtual fs mode(Use `egui` ver.)
  - [x] Auto read settings file(Use `egui` ver.)
  - [ ] hkx json/patch editor

- AnimData(`animationdatasinglefile.txt`)
  - [x] Serialization
  - [x] Deserialization
  - Patch(Add/Remove/Replace)
    - [x] Txt project header
    - [x] Anim data header
    - [x] Clip Motion Block
    - [x] Anim Data Block

- AnimSetData(`animationsetdatasinglefile.txt`)
  - [x] Serialization
  - [x] Deserialization
  - Patch(Add/Remove/Replace)
    - [x] Version
    - [x] Triggers
    - [x] Conditions
    - [ ] Attacks(Add only. TODO: Remove/Replace)
    - [x] AnimInfos

- hkx templates
  - [x] Change xml to message_pack bin.

- Nemesis Patch
  - [x] Basic parallel merge.
  - [x] Fix unknown merge race condition(The cause was a deadlock in applying patches.)

- FNIS Patch

| Status | Feature                         | Abbreviation(s) | Notes                                                                          |
| ------ | ------------------------------- | --------------- | ------------------------------------------------------------------------------ |
| ✅     | Basic                           | b               | Creature also supported (tested with FNISZoo converted to SE)                  |
| ✅     | Sequenced Animations            | s, so           | Creature also supported                                                        |
| ✅     | Arm Offset Animations           | ofa             | Tested with(Immersive Animated Looting v2.7)                                   |
| ❌     | Furniture Animations            | fu, fuo         | I couldn't find any mods that could be tested.                                 |
| ⚠️     | Paired Animations and KillMoves | pa, km          | KillMove: only animation checked, not tested in actual kill. Paired: untested. |
| ❌     | Chair Animations                | ch              | I couldn't find any mods that could be tested.                                 |
| ❌     | Alternate Animations            | AAprefix        | I'm debating whether to convert it to OAR.                                     |

![patch_page](https://github.com/user-attachments/assets/a601c347-10f1-459e-bb70-ecbee5f82590)

## Build

requirements: Rust1.91

```shell
cargo build -p d_merge_egui --profile release-no-lto # Simple GUI by egui
```

requirements: Rust1.91, Node.js LTS

```shell
npm run build # Rich GUI by tauri
```

## Licenses

This project includes multiple crates with different licenses. The overall license of the `backend` crate is **GPL-3.0**, due to transitive dependencies on GPL-licensed components.

- [GPL-3.0](./LICENSE)
- [MIT](./LICENSES/LICENSE-MIT)
- [Apache2.0](./LICENSES/LICENSE-APACHE)

### License Tree

```txt
gui (GPL-3.0)
├── nemesis_merge (GPL-3.0)
│   ├── json_patch (MIT OR Apache-2.0)
│   ├── nemesis_xml (MIT OR Apache-2.0)
│   ├── skyrim_anim_parser (GPL-3.0)
│   └── skyrim_crc (MIT OR Apache-2.0)
├── node_expr (MIT OR Apache-2.0)
├── mod_info (MIT OR Apache-2.0)
├── serde_hkx_for_gui (MIT OR Apache-2.0)
└── tracing_rotation (MIT OR Apache-2.0)
```

#### License Propagation

- **`skyrim_anim_parser`**:
  I understood the specification of this `animationdatasinglefile.txt` file from reading pandora. Therefore, I will keep it under GPL-3.0 just in case.

- **`nemesis_merge`**:
  This crate depends on both `skyrim_anim_parser` (GPL-3.0) and a GPL-licensed template(See `resource` dir). Thus, it is required to be **GPL-3.0**.

- **`backend`**:
  As a GUI frontend that depends on `nemesis_merge`, it inherits the **GPL-3.0** license through transitive dependency.

Other utility crates (e.g., `mod_info`, `node_expr`, `json_patch`, etc.) are licensed under **MIT OR Apache-2.0**, but the presence of GPL-licensed dependencies requires that the final binary (the GUI/backend) must be distributed under **GPL-3.0**.

Please ensure that your usage and redistribution of this software complies with the [**GPL-3.0**](./LICENSE) license terms.

### Deps NOTES

- "zod": "^3.25.67"

- Do not include react or react-dom in frontend dependencies. Next.js appears to include its own React implementation, and adding it manually will cause vitest's React to throw errors. [See](https://t.co/1Oi722pfbb)

- [babel-plugin-react-compiler](https://www.npmjs.com/package/babel-plugin-react-compiler?activeTab=versions)

- Due to a bug in turbopack at the time of Next.js 16 that prevents it from loading alias JSON files, use relative paths.
