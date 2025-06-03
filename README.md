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

  - [ ] Serialization
  - [x] Deserialization
  - [ ] Add Operation
  - [ ] Replace Operation

- hkx templates

  - [x] Change xml to message_pack bin.

- Nemesis Patch
  - [x] Basic parallel merge.
  - [ ] Fix unknown merge race condition

## Debugging Nemesis Patches

1. Generate hkx in Nemesis.
2. Convert the required xml in Nemesis/resource to hkx and then to xml again.
   This will generate xml that meets the d-merge specification.
3. Output the xml generated in step 2 to json for 3.
4. Use serde-hkx tool to output Nemesis hkx → json.
5. Diff the results of 3 and 4.

## Converting page

![image](https://github.com/user-attachments/assets/1b8f0a0b-8aa2-4bd3-9cba-f75a6ff9095d)
