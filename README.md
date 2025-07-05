# D Merge(Diff & Merge) hkx patcher

<div align="center">
  <a href="https://github.com/SARDONYX-sard/d-merge/releases">
    <img src="./gui/backend/icons/icon.svg" alt="D Merge"/>
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

- [Early Release](https://github.com/SARDONYX-sard/d-merge/releases)

## Patch Page Progress

This currently works to some extent(Sliding, Paraglider, MCO, DMCO-Dodge, ...), but there seems to be a conflict where the patch changes every time the button is pressed.

The only thing we are considering at this time is support for the Nemesis patch.(Since I only use the Nemesis patch).

- GUI

  - [x] Basic frontend(patch, convert, settings)
  - [x] Support MO2 mode/Virtual file system mode(auto read settings file) when use auto detect(Current: windows only)
  - [ ] hkx json/patch editor
  - [ ] In the case of vfs, use mod_code as the ID (if the ID is duplicated, the UI will bug out, but this will allow you to transfer your environment to others).

- AnimData(`animationdatasinglefile.txt`)

  - [x] Serialization
  - [x] Deserialization
  - [ ] txt project header patch
  - [ ] anim header patch
  - [x] Add Operation
  - [x] Replace/Remove Operation
  - [ ] Conflict resolver

- AnimSetData(`animationsetdatasinglefile.txt`)

  - [x] Serialization
  - [x] Deserialization
  - [ ] Add Operation (Impossible unless the specification of the difference is understood.)
  - [ ] Replace/Remove Operation(Same issue)

- hkx templates

  - [x] Change xml to message_pack bin.

- Nemesis Patch
  - [x] Basic parallel merge.
  - [ ] Fix unknown merge race condition

![patch_page](https://github.com/user-attachments/assets/a601c347-10f1-459e-bb70-ecbee5f82590)

## Licenses

This project includes multiple crates with different licenses. The overall license of the `backend` crate is **GPL-3.0**, due to transitive dependencies on GPL-licensed components.

- [GPL-3.0](./LICENSE)
- [MIT](./LICENSES/LICENSE-MIT)
- [Apache2.0](./LICENSES/LICENSE-APACHE)

### License Tree

```txt
gui/backend (GPL-3.0)
├── nemesis_merge (GPL-3.0)
│   ├── skyrim_anim_parser (GPL-3.0)
│   ├── nemesis_xml (MIT OR Apache-2.0)
│   ├── skyrim_crc (MIT OR Apache-2.0)
│   └── json_patch (MIT OR Apache-2.0)
├── mod_info (MIT OR Apache-2.0)
└── node_expr (MIT OR Apache-2.0)
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
