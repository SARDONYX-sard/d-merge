# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.5] - 2025-10-30
### :sparkles: New Features
- [`bf44e4b`](https://github.com/SARDONYX-sard/d-merge/commit/bf44e4b2c67f33ec4cc3ebb9753533600e9d9329) - add template keys (for `Draugr MCO-DXP` *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*

### :bug: Bug Fixes
- [`23e850a`](https://github.com/SARDONYX-sard/d-merge/commit/23e850a0fc5b9da0e2a003ff25091e143129422c) - **asdsf**: fix one patch diff deserializer *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`f99791c`](https://github.com/SARDONYX-sard/d-merge/commit/f99791c5278b251ea8de698190ef4d2730612cca) - fix wrong template key *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`6451c35`](https://github.com/SARDONYX-sard/d-merge/commit/6451c359071369df5740d489110a1a9466b1e8d1) - fix duplicate eventName check to avoid breaking vanilla Skyrim behavior *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*

### :zap: Performance Improvements
- [`24fbeb9`](https://github.com/SARDONYX-sard/d-merge/commit/24fbeb9e0adc8ad4675d127cf152e56e86e44b1b) - **json**: stop redundant heap alloc *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*


## [0.2.4] - 2025-10-28
### :sparkles: New Features
- [`ebc4152`](https://github.com/SARDONYX-sard/d-merge/commit/ebc4152b00d9cc549944d1f47c85354c358bfbe4) - add landing page *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`39ee35f`](https://github.com/SARDONYX-sard/d-merge/commit/39ee35f465d6efbb32da0b9fdd987dabf8e391dd) - **i18n**: apply i18n to landing page *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*

### :bug: Bug Fixes
- [`01d18cb`](https://github.com/SARDONYX-sard/d-merge/commit/01d18cb9f41b77fa0ff8abcdc3a1f1d602fae6b2) - **frontend**: fix ordering *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*

### :white_check_mark: Tests
- [`5e52993`](https://github.com/SARDONYX-sard/d-merge/commit/5e52993678e6df1dae81811a57f6826cee446217) - remove custom matcher *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*


## [0.2.3] - 2025-10-23
### :sparkles: New Features
- [`01a5a16`](https://github.com/SARDONYX-sard/d-merge/commit/01a5a161a4686a5bc0d663d2e7c3f654e9766c66) - **hkanno**: cache preview & active tab *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*


## [0.2.0] - 2025-10-21
### :sparkles: New Features
- [`c781a4b`](https://github.com/SARDONYX-sard/d-merge/commit/c781a4b8433a7232a9fb7680b8ba6b67ebce6838) - implement `hkanno` fn *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`f1b9a69`](https://github.com/SARDONYX-sard/d-merge/commit/f1b9a6940b0dd86240d7a45ad27d301c5a854b97) - implement hkanno page *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`0a4d6d6`](https://github.com/SARDONYX-sard/d-merge/commit/0a4d6d647aac8b8fa2d210ea4d9516cb1078460d) - hkanno lsp *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`271c418`](https://github.com/SARDONYX-sard/d-merge/commit/271c4183ff843155c89268875b77e4878487e362) - **frontend**: add signatureProvider *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`9b95e18`](https://github.com/SARDONYX-sard/d-merge/commit/9b95e18664d03d10c7fed9543cef52c5ba35490f) - implement strict parser *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`12ac785`](https://github.com/SARDONYX-sard/d-merge/commit/12ac785a543cd572573016abb1754463948b2835) - change syntax highlight *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`c788905`](https://github.com/SARDONYX-sard/d-merge/commit/c78890528eaf025ca33fda8d2e2161177e6dab44) - **hkanno**: support pie event completion *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`d5c4dde`](https://github.com/SARDONYX-sard/d-merge/commit/d5c4dde4c5d77a585f734deecfb9e9213e00976d) - use a new parser for the signature provider *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`10c5dea`](https://github.com/SARDONYX-sard/d-merge/commit/10c5dea8d7877ba04fbca7a4215750554f137092) - new hover *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`f07a47c`](https://github.com/SARDONYX-sard/d-merge/commit/f07a47cca754aa64fae03b2ee013c8c3ca510983) - **preview**: implement sync preview *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`f993800`](https://github.com/SARDONYX-sard/d-merge/commit/f993800451be739e2b8dbfb20b6f53ce814d808a) - **frontend**: implement hkanno state cache *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`8c7eec1`](https://github.com/SARDONYX-sard/d-merge/commit/8c7eec12249512606ca04bc66ab3d07eb4687d53) - change diagnostic level *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*

### :bug: Bug Fixes
- [`ee5c6d1`](https://github.com/SARDONYX-sard/d-merge/commit/ee5c6d10f88d51093710897d574a3b12e160db15) - **frontend**: fix react compiler *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`f2330ff`](https://github.com/SARDONYX-sard/d-merge/commit/f2330ffce2d2f1b40c117976e9518778eb5ea911) - fix diagnostics *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`7c44b26`](https://github.com/SARDONYX-sard/d-merge/commit/7c44b26354f8a01ac2c7bac2e0cdc729766443ea) - **hkanno**: fix diagnostic *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`7dc1415`](https://github.com/SARDONYX-sard/d-merge/commit/7dc14158cc263798677ce5ea5f256aa89fb25ec0) - **frontend**: fix patch execution err *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*

### :recycle: Refactors
- [`b80c35f`](https://github.com/SARDONYX-sard/d-merge/commit/b80c35f6d10f913e8f7ef1619db0bb159ba85a14) - use var *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`609e280`](https://github.com/SARDONYX-sard/d-merge/commit/609e2809d63fe79668e989ba5bde77c14537171e) - remove unused key *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*


## [0.1.2] - 2025-10-17
### :sparkles: New Features
- [`8ce8e3a`](https://github.com/SARDONYX-sard/d-merge/commit/8ce8e3ab772dd729ef0a30160cf80e21489874ff) - create new issue template *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`3593738`](https://github.com/SARDONYX-sard/d-merge/commit/35937380f9c84d0337f86c8a580d03c20223f074) - change os name *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`08ae28b`](https://github.com/SARDONYX-sard/d-merge/commit/08ae28bc5b9a4531e1cd26f19a55ad63388260f1) - add file version fn *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*

### :bug: Bug Fixes
- [`eae2d34`](https://github.com/SARDONYX-sard/d-merge/commit/eae2d34179634b14b97f4012e50268d6ca2c434c) - fix import *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*

### :recycle: Refactors
- [`3811efa`](https://github.com/SARDONYX-sard/d-merge/commit/3811efa02beb1324aaa2c631e4532a88389506c4) - rename arg name *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*

### :wrench: Chores
- [`6aa3469`](https://github.com/SARDONYX-sard/d-merge/commit/6aa3469a0dfd4362a1bc1789e87d87d6f3f74ef3) - fix issue template *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`db34832`](https://github.com/SARDONYX-sard/d-merge/commit/db3483292a4691d5e957682396d487a43008f265) - **issue**: add edition *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`2b3ce12`](https://github.com/SARDONYX-sard/d-merge/commit/2b3ce12bf8d913af0e61a12d164e16653baa0bb7) - **issue**: rename *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*


## [0.1.1] - 2025-10-14
### :sparkles: New Features
- [`3f4c99f`](https://github.com/SARDONYX-sard/d-merge/commit/3f4c99f482fb8179429c54f9f4da609e74f1e203) - **adsf**: support anim data header *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`d4ffda3`](https://github.com/SARDONYX-sard/d-merge/commit/d4ffda31668bbb9483997b71af59a4ee94ddcc5d) - **seq**: change the replace operation in seq to remove elements if there are few *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`bd2dc43`](https://github.com/SARDONYX-sard/d-merge/commit/bd2dc4310feefd58e28d32d1d74b30188c8fdaf1) - new visualizer *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*

### :bug: Bug Fixes
- [`3ee3ab6`](https://github.com/SARDONYX-sard/d-merge/commit/3ee3ab6e73ce1af958eac658d498e00406d633a2) - **adsf**: fix `delete this line` range bug *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`d3f4745`](https://github.com/SARDONYX-sard/d-merge/commit/d3f4745a60cfbe8c2dbe48dde0d72f65f935609d) - **adsf**: fix `delete this line` range bug *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`fa6c4b4`](https://github.com/SARDONYX-sard/d-merge/commit/fa6c4b437dfcbcde878f0dbccb6302555a160cc8) - fix json patch seq visualizer *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*

### :zap: Performance Improvements
- [`791ef12`](https://github.com/SARDONYX-sard/d-merge/commit/791ef126344daded38370260bad62d83d3fc5758) - use rayon *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*

### :recycle: Refactors
- [`cb088e0`](https://github.com/SARDONYX-sard/d-merge/commit/cb088e02380aeba659bae684d968f0584426d760) - use `JoinSet` *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*

### :white_check_mark: Tests
- [`b0c3020`](https://github.com/SARDONYX-sard/d-merge/commit/b0c302025689b81cb5aba2d1d90df6ab1df58ae8) - use ini *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*

### :wrench: Chores
- [`5a9e332`](https://github.com/SARDONYX-sard/d-merge/commit/5a9e332c3f9cce09c80cc623746a2943e9034790) - update adsf template *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*


## [0.1.0] - 2025-10-13
### :sparkles: New Features
- [`68f8cf6`](https://github.com/SARDONYX-sard/d-merge/commit/68f8cf6704f219907532052815c5583fc28c6c68) - add copy button *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`db935ae`](https://github.com/SARDONYX-sard/d-merge/commit/db935aefab7e4919a26682b0ead25249b23d89d8) - **egui**: add notify *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`a4ad2fd`](https://github.com/SARDONYX-sard/d-merge/commit/a4ad2fd1a5b29af764eb0a87faaea4b92f9ce63d) - **egui**: add notify *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`b17cd99`](https://github.com/SARDONYX-sard/d-merge/commit/b17cd995c064e34aaa9e7d732aa383cbd5b645d4) - **status**: change it so that a reporter also submits a report at the start of the status *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`0c93aff`](https://github.com/SARDONYX-sard/d-merge/commit/0c93aff01457995e42b3cb45323f024e36cac536) - **patch**: add `SeqPush` & remove `Discrete` *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`c3f15bc`](https://github.com/SARDONYX-sard/d-merge/commit/c3f15bc0278ad8e13e7b74d188a0841d0b80674b) - **json_patch**: change json format *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*

### :bug: Bug Fixes
- [`374705d`](https://github.com/SARDONYX-sard/d-merge/commit/374705d71fec7619d4ef3b503295fd3e4af21962) - fix settings i18n parser *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`978197d`](https://github.com/SARDONYX-sard/d-merge/commit/978197d4c778cceeee05aecf1aa9b71e9692a7f9) - **mod_info**: fix link *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*

### :recycle: Refactors
- [`8672afa`](https://github.com/SARDONYX-sard/d-merge/commit/8672afa9134759f71917006b468806246cc871cf) - use `rename_all` *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`f9395e1`](https://github.com/SARDONYX-sard/d-merge/commit/f9395e137b3d30fa995f89e44a5b2d044e113bf3) - refactor code *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`2271933`](https://github.com/SARDONYX-sard/d-merge/commit/2271933a9e842f2eb60fc26f8cf6d75e73a7e286) - **fnis**: organize to fn *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`f384a3e`](https://github.com/SARDONYX-sard/d-merge/commit/f384a3ee8a64694adc89f92d7361944a9781306c) - eliminate duplicate management of template_key *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*

[0.1.0]: https://github.com/SARDONYX-sard/d-merge/compare/0.1.0-beta.3...0.1.0
[0.1.1]: https://github.com/SARDONYX-sard/d-merge/compare/0.1.0...0.1.1
[0.1.2]: https://github.com/SARDONYX-sard/d-merge/compare/0.1.1...0.1.2
[0.2.0]: https://github.com/SARDONYX-sard/d-merge/compare/0.1.2...0.2.0
[0.2.3]: https://github.com/SARDONYX-sard/d-merge/compare/0.2.2...0.2.3
[0.2.4]: https://github.com/SARDONYX-sard/d-merge/compare/0.2.3...0.2.4
[0.2.5]: https://github.com/SARDONYX-sard/d-merge/compare/0.2.4...0.2.5
