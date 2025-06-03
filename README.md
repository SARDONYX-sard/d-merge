# D Merge(Early development)

diff & merge => d_merge json patch-based hkx patcher

![image](https://github.com/user-attachments/assets/1b8f0a0b-8aa2-4bd3-9cba-f75a6ff9095d)

## For Tester

The patch page is under development, so there is no need to submit an issue.

- [Release](https://github.com/SARDONYX-sard/d-merge/releases)

## Patch Page Progress

The only thing we are considering at this time is support for the Nemesis patch.(Since I only use the Nemesis patch).

- GUI

  - [x] Basic frontend(patch, convert, settings)
  - [ ] Support MO2 mode/Virtual file system mode(auto read settings file)
  - [ ] hkx json/patch editor

- AnimData(`animationdatasinglefile.txt`)

  - [x] Serialization
  - [x] Deserialization
  - [ ] Add Operation
  - [x] Replace Operation

- AnimSetData(`animationsetdatasinglefile.txt`)

  - [ ] Serialization
  - [x] Deserialization
  - [ ] Add Operation
  - [ ] Replace Operation

- hkx templates

  - [x] Change xml to message_pack bin.

- Nemesis Patch
  - [ ] Fix unknown merge race condition

## Debugging Nemesis Patches

1. Generate hkx in Nemesis.
2. Convert the required xml in Nemesis/resource to hkx and then to xml again.
   This will generate xml that meets the d-merge specification.
3. Output the xml generated in step 2 to json for 3.
4. Use serde-hkx tool to output Nemesis hkx → json.
5. Diff the results of 3 and 4.
