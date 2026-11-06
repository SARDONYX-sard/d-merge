# D Merge(Diff & Merge) hkx patcher

<div align="center">
  <a href="https://github.com/SARDONYX-sard/d-merge/releases">
    <img src="./crates/gui/gui_shared/assets/icon.svg" alt="D Merge"/>
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
  </p>

  <!-- Development Badges -->
  <p>
    <a href="https://github.com/SARDONYX-sard/d-merge/actions/workflows/build-and-test.yaml">
      <img src="https://github.com/SARDONYX-sard/d-merge/actions/workflows/build-and-test.yaml/badge.svg?style=flat-square" alt="Build & Test Status">
    </a>
  </p>
</div>

## Release

- [Click assets](https://github.com/SARDONYX-sard/d-merge/releases)

## Intended Use of Automatic Settings File Loading

The load order configuration file is automatically loaded each time the application runs.

It is written to <skyrim data dir>/.d_merge/d_merge_settings.json immediately before the application closes.

By utilizing this behavior as follows, settings will be automatically loaded and updated whenever you switch profiles.

<details><summary>MO2 Directory tree</summary><div>

```txt
D:/MO2/mods/
├── d_merge_windows_egui/
│       ├── .d_merge/
│       │    └── translation.json
│       ├── assets/                        <- templates files dir
│       ├── interface/                     <- dummy dir
│       └── d_merge.exe               <- main exe
│
├── male_profile_dir/
│       ├── meshes/
│       └── .d_merge/
│             └── d_merge_settings.json    <- (output_dir: D:/MO2/mods/male_profile_dir)
│
└── female_profile_dir/
           ├── meshes/
           └── .d_merge/
                  └── d_merge_settings.json <- (output_dir: D:/MO2/mods/female_profile_dir)
```

</div></details>

## Patch Page Progress

- [Mods Test status](./docs/test_status.md)

- GUI
  - [x] Basic frontend(Patch / Convert / hkanno editor)
  - [x] Support Virtual fs mode
  - [x] Auto read settings file
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
    - [x] Attacks
    - [x] AnimInfos(Only the directory and file name support the `$crc32[some/path]` macro syntax.)

- hkx templates
  - [x] Change xml to message_pack bin.

- Nemesis Patch
  - [x] Parallel merge.

- FNIS Patch

| Status | Feature                         | Abbreviation(s) | Notes                                                         |
| ------ | ------------------------------- | --------------- | ------------------------------------------------------------- |
| ✅     | Basic                           | b               | Creature also supported (tested with FNISZoo converted to SE) |
| ✅     | Sequenced Animations            | s, so           | Creature also supported                                       |
| ✅     | Arm Offset Animations           | ofa             | Tested with (Immersive Animated Looting v2.7)                 |
| ✅     | Furniture Animations            | fu, fuo         | Tested with (Fort Knox v1.4.3)                                |
| ✅     | Paired Animations and KillMoves | pa, km          | Tested with (Animated Wedding Scene v4.0.2)                   |
| ✅     | Chair Animations                | ch              | Tested with (EatingSleepingDrinking v3.2.0)                   |
| ✅     | Alternate Animations            | AAprefix        | Automatically convert to OAR. Need: OAR and fnis_aa.          |

![patch_page](https://github.com/user-attachments/assets/a601c347-10f1-459e-bb70-ecbee5f82590)

## Build

requirements: Rust1.91

```shell
cargo build -p d_merge --profile release-no-lto
```

## Licenses

This project includes multiple crates with different licenses. The distributed binary (`d_merge`) is licensed under **GPL-3.0-or-later**, due to its dependencies on GPL-licensed components.

- [GPL-3.0-or-later](./LICENSE)
- [MIT](./LICENSES/LICENSE-MIT)
- [Apache2.0](./LICENSES/LICENSE-APACHE)

### License Tree

Workspace crates only (`crates/core/*`, `crates/gui/*`). `fast_hash` (MIT OR Apache-2.0) is used by most of them and omitted.

```txt
d_merge (GPL-3.0-or-later)                          <- crates/gui/d_merge: the app
│
├── behavior_merge (GPL-3.0-or-later)               <- crates/core: patch merge engine
│   ├── behavior_merge_types (GPL-3.0-or-later)
│   ├── fnis_patch_gen (GPL-3.0-or-later)
│   │   ├── auto_charset (MIT OR Apache-2.0)
│   │   ├── fnis_list (GPL-3.0-or-later)
│   │   ├── skyrim_crc (MIT OR Apache-2.0)
│   │   └── skyrim_esp (MIT OR Apache-2.0)
│   ├── json_patch (MIT OR Apache-2.0)
│   ├── jwalk_glob (MIT OR Apache-2.0)
│   ├── nemesis_xml (MIT OR Apache-2.0)
│   └── skyrim_anim_data (GPL-3.0-or-later)
│       └── skyrim_crc (MIT OR Apache-2.0)
│
├── gui_shared (MIT OR Apache-2.0)                   <- crates/gui: GUI parts
│   ├── gh_issue_link (MIT OR Apache-2.0)
│   ├── gui_shared_derive (MIT OR Apache-2.0)
│   ├── mod_info (MIT OR Apache-2.0)
│   └── skyrim_data_dir (MIT OR Apache-2.0)
├── page_convert (GPL-3.0-or-later)
│   └── serde_hkx_for_gui (MIT OR Apache-2.0)
│       └── node_expr (MIT OR Apache-2.0)
├── page_patch_editor (GPL-3.0-or-later)
│   ├── behavior_merge_types (GPL-3.0-or-later)
│   ├── egui_code_editor (MIT)
│   ├── nemesis_diff (MIT OR Apache-2.0)
│   │   └── nemesis_xml (MIT OR Apache-2.0)
│   └── skyrim_crc (MIT OR Apache-2.0)
├── page_hkanno (MIT OR Apache-2.0)
│   ├── egui_code_editor (MIT)
│   │   └── egui_widgets (MIT)
│   ├── hkanno_lsp (MIT OR Apache-2.0)
│   └── serde_hkx_for_gui (MIT OR Apache-2.0)
├── egui_widgets (MIT)
└── tracing_rotation (MIT OR Apache-2.0)
```

`xtask` (MIT OR Apache-2.0) is a development tool (`cargo x help`) and is not distributed.

#### License Propagation

- **`skyrim_anim_data`**:
  I understood the specification of this `animationdatasinglefile.txt` file from reading pandora. Therefore, I will keep it under GPL-3.0-or-later just in case.

- **`fnis_patch_gen`**:
  Based on the logic of Pandora-Behaviour-Engine-Plus (GPL-3.0-or-later) and depends on `skyrim_anim_data`.

- **`behavior_merge_types`**:
  Holds the patch types of `skyrim_anim_data` shared by `behavior_merge` and `fnis_patch_gen`.

- **`behavior_merge`**:
  This crate depends on `skyrim_anim_data`, `fnis_patch_gen` (GPL-3.0-or-later) and a GPL-licensed template(See `resource` dir). Thus, it is required to be **GPL-3.0-or-later**.

- **`d_merge`** and the GUI crates without a license of their own (`gui_shared`, `page_*`):
  As the GUI that depends on `behavior_merge`, they inherit **GPL-3.0-or-later** through transitive dependency.

Other utility crates (e.g., `mod_info`, `node_expr`, `json_patch`, `egui_code_editor`, etc.) are licensed under **MIT OR Apache-2.0** (or **MIT**), but the presence of GPL-licensed dependencies requires that the final binary (`d_merge`) must be distributed under **GPL-3.0-or-later**.

Please ensure that your usage and redistribution of this software complies with the [**GPL-3.0-or-later**](./LICENSE) license terms.

## egui version on The Windows Subsystem for Linux (WSL)

Notes

- For some reason, WSL crashes unless we set the window to width: 640, height: 320.
- With the above settings and wgpu features enabled, it seems we can adjust the window size after a short while. (If we change the size immediately after startup, it will crash.)

Deps

```shell
sudo apt update

sudo apt install -y \
  libfontconfig1 \ # font rendering (system fallback fonts)
  libwayland-client0 libwayland-cursor0 libwayland-egl1 \ # Wayland EGL window backend
  libxkbcommon0 libxkbcommon-x11-0 \ # keyboard/input handling (winit dependency)
  libegl1 libgl1 libgles2 libglx-mesa0 mesa-utils \ # OpenGL / EGL graphics stack
  xclip xsel \ # clipboard integration (X11 fallback)
  zenity # file dialog fallback (rfd backend)
```
