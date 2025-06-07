# D Merge(Diff & Merge json based hkx patcher)

<div>
    <a href="https://github.com/SARDONYX-sard/d-merge/actions/workflows/release-gui.yaml">
        <img src="https://github.com/SARDONYX-sard/d-merge/actions/workflows/release-gui.yaml/badge.svg" alt="Release GUI">
    </a>
    <a href="https://github.com/SARDONYX-sard/d-merge/actions/workflows/build-and-test.yaml">
        <img src="https://github.com/SARDONYX-sard/d-merge/actions/workflows/build-and-test.yaml/badge.svg" alt="Build & Test">
    </a>
    <a href="https://opensource.org/licenses/GPL-3.0">
        <img src="https://img.shields.io/badge/License-GPL_v3.0-blue.svg" alt="License">
    </a>

</div>

![patch_page](https://github.com/user-attachments/assets/48b4d85d-ce79-4a46-87de-55a9c7e27436)

## For Tester

The patch page is under development, so there is no need to submit an issue.

- [Early Release](https://github.com/SARDONYX-sard/d-merge/releases)

## Patch Page Progress

This currently works to some extent(Sliding, Paraglider, MCO, DMCO-Dodge, ...), but there seems to be a conflict where the patch changes every time the button is pressed.

The only thing we are considering at this time is support for the Nemesis patch.(Since I only use the Nemesis patch).

- GUI

  - [x] Basic frontend(patch, convert, settings)
  - [ ] Support MO2 mode/Virtual file system mode(auto read settings file)
  - [ ] hkx json/patch editor

- AnimData(`animationdatasinglefile.txt`)

  - [x] Serialization
  - [x] Deserialization
  - [x] Add Operation
  - [ ] Replace Operation

- AnimSetData(`animationsetdatasinglefile.txt`)

  - [x] Serialization
  - [x] Deserialization
  - [ ] Add Operation (Impossible unless the specification of the difference is understood.)
  - [ ] Replace Operation

- hkx templates

  - [x] Change xml to message_pack bin.

- Nemesis Patch
  - [x] Basic parallel merge.
  - [ ] Fix unknown merge race condition

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

### License Propagation

- **`skyrim_anim_parser`**:
  I understood the specification of this `animationdatasinglefile.txt` file from reading pandora. Therefore, I will keep it under GPL-3.0 just in case.

- **`nemesis_merge`**:
  This crate depends on both `skyrim_anim_parser` (GPL-3.0) and a GPL-licensed template(See `resource` dir). Thus, it is required to be **GPL-3.0**.

- **`backend`**:
  As a GUI frontend that depends on `nemesis_merge`, it inherits the **GPL-3.0** license through transitive dependency.

Other utility crates (e.g., `mod_info`, `node_expr`, `json_patch`, etc.) are licensed under **MIT OR Apache-2.0**, but the presence of GPL-licensed dependencies requires that the final binary (the GUI/backend) must be distributed under **GPL-3.0**.

Please ensure that your usage and redistribution of this software complies with the [**GPL-3.0**](./LICENSE) license terms.
