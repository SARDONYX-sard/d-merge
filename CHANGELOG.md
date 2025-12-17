# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2025-12-17
### :sparkles: New Features
- [`f8a23e8`](https://github.com/SARDONYX-sard/d-merge/commit/f8a23e8b01e7148797e49f904aae5c3f4b0fa7e1) - **FNIS**: add a feature that automatically converts added animations to hkx files compatible with the target *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`00918d9`](https://github.com/SARDONYX-sard/d-merge/commit/00918d9abf14b70bb4d6c4b6db5b47339e8645ae) - skip tag file(unsupported yet) *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`8480f85`](https://github.com/SARDONYX-sard/d-merge/commit/8480f85cf77cc49dc16b7298e21de66a3740e397) - **FNIS**: add behavior conversion *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`eed98c3`](https://github.com/SARDONYX-sard/d-merge/commit/eed98c317e1070924d8d75d9bb82c3419cf905ae) - **FNIS**: implement a feature to convert alt anim to OAR *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`f1c3504`](https://github.com/SARDONYX-sard/d-merge/commit/f1c350404dc657efe9258a683843b772c4a73011) - **FNIS**: change to warn behavior path *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`4a713f5`](https://github.com/SARDONYX-sard/d-merge/commit/4a713f5b2cc191924eba1fc1d968f8afffdc68a7) - **FNIS**: no io parallel *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`57ccf09`](https://github.com/SARDONYX-sard/d-merge/commit/57ccf09a76b35124f8119a043350aa1509db0a06) - **FNIS**: use async *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`7854821`](https://github.com/SARDONYX-sard/d-merge/commit/785482117a619707a3574cefcd94ea5ed0018f45) - **egui**: more transparent *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`c75ae88`](https://github.com/SARDONYX-sard/d-merge/commit/c75ae8886b1af702475c2e9aa45d18875a481537) - implement new asdsf nemesis patch deserializer *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`9d0a761`](https://github.com/SARDONYX-sard/d-merge/commit/9d0a761f5aec9b8281699446214050771716fe83) - **asdsf**: support `$crc32[]$` macro *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*

### :bug: Bug Fixes
- [`24aa4b2`](https://github.com/SARDONYX-sard/d-merge/commit/24aa4b2d77de03a779124b5f2f93cc3add5c8a2e) - **FNIS**: fix ptr size check *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`990cfd5`](https://github.com/SARDONYX-sard/d-merge/commit/990cfd528bfec6ffbc471a362ff728ca6c623bb2) - fix parent dir path *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`3d49720`](https://github.com/SARDONYX-sard/d-merge/commit/3d49720a93211206d20cfa360b901723ca0c7e6b) - **FNIS**: fix create_dir_all *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`6d99af1`](https://github.com/SARDONYX-sard/d-merge/commit/6d99af12951cf1f25e4008b2941fd3de37bfe2d6) - **FNIS**: fix header check *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`f94a079`](https://github.com/SARDONYX-sard/d-merge/commit/f94a079da26a76e96c08751cf92547a09ef02f6b) - **FNIS alt anim**: forgot copy *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`3a7d75a`](https://github.com/SARDONYX-sard/d-merge/commit/3a7d75a67b63aa4db5f17e36afe1d2a5ca5cd263) - **FNIS**: add create parent dir *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`caee30d`](https://github.com/SARDONYX-sard/d-merge/commit/caee30dfc6e1cbd3fd6a49e624859bb4b0b74a6a) - **schema**: fix schema *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`07fc7a3`](https://github.com/SARDONYX-sard/d-merge/commit/07fc7a32874dbac8eeea46e03b03ef2f9b67ffa7) - **FNIS**: add `default` *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`7228ae7`](https://github.com/SARDONYX-sard/d-merge/commit/7228ae7a67f23f39e5870bec9ac43af18f8b018e) - **FNIS**: fix config json *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`9488c0c`](https://github.com/SARDONYX-sard/d-merge/commit/9488c0cb3fdf263315f797e753dbab40aa31fd57) - **FNIS**: fix path *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`cf06863`](https://github.com/SARDONYX-sard/d-merge/commit/cf06863db5ce31e41110391d39d1a2dda58edf1d) - **FNIS**: try to fix conversion error *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`9aa29b5`](https://github.com/SARDONYX-sard/d-merge/commit/9aa29b594f9ca755e92e7826b60e8ac2cc0b9b8f) - **egui**: fix the behavior of floating bottom buttons *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`58fe746`](https://github.com/SARDONYX-sard/d-merge/commit/58fe7468ce5ef6f95044022e039863e24165c2ab) - **frontend-vim**: fix broken CDN link *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`37996d2`](https://github.com/SARDONYX-sard/d-merge/commit/37996d2908dba687aa062273e18d496fd7b74447) - **asdsf**: fix path *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`a758e4c`](https://github.com/SARDONYX-sard/d-merge/commit/a758e4cb59b291652a51111f15924f9cc2a754d6) - **fnis**: add skip code *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`abb304b`](https://github.com/SARDONYX-sard/d-merge/commit/abb304bb9a1a401e92225ddddffffd9ce5bef2bd) - **asdsf**: fix patch handling and serialization for anim sets *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*

### :zap: Performance Improvements
- [`cdd74df`](https://github.com/SARDONYX-sard/d-merge/commit/cdd74dfd6cca74e4606e69b72377f7c3e7b5a57f) - stop redundant alloc *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*

### :recycle: Refactors
- [`846b565`](https://github.com/SARDONYX-sard/d-merge/commit/846b565f8fe0e592cbdd590b6273890013f8f284) - use `serde_hkx` low level api *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`df65204`](https://github.com/SARDONYX-sard/d-merge/commit/df652045e5c26b43875368e9735ec7b16ff1c607) - remove redundant scope *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`70d324c`](https://github.com/SARDONYX-sard/d-merge/commit/70d324caf801f7417125348f907a9478779ceedc) - **FNIS**: separate process *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`4dda3fc`](https://github.com/SARDONYX-sard/d-merge/commit/4dda3fc7a98e7a867f76faee9e5d7f536aa7a7c6) - remove redundant comment *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`48e61aa`](https://github.com/SARDONYX-sard/d-merge/commit/48e61aaf5e7cc4781cf26a824dc10d1db4444f0b) - **dependencies**: remove redundant features in `Cargo.toml` *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`1fb793f`](https://github.com/SARDONYX-sard/d-merge/commit/1fb793f4916228c017df85264c66e457f0f29f16) - **FNIS**: change `unstable conversion` feature *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*

### :white_check_mark: Tests
- [`4121211`](https://github.com/SARDONYX-sard/d-merge/commit/4121211d57b69b766497e6a3f6c81c8218c64da1) - **adsf**: add missing line *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*

### :wrench: Chores
- [`c625052`](https://github.com/SARDONYX-sard/d-merge/commit/c6250520b563f38122778c11cb38c3a10e614e9e) - **bug-report**: add % *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`dc628d0`](https://github.com/SARDONYX-sard/d-merge/commit/dc628d0ece528471faadbf071b512c46b0c1c5ce) - **FNIS**: add info log *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*
- [`81520a4`](https://github.com/SARDONYX-sard/d-merge/commit/81520a448cbc81cc332f58c63a5ec44efcf5f5a4) - use `par_extend` *(commit by [@SARDONYX-sard](https://github.com/SARDONYX-sard))*


## [0.2.6] - 2025-11-12
### :sparkles: New Features
- [`5f38d2f`](https://github.com/SARDONYX-sard/d-merge/commit/5f38d2f1844adf86938400b93d01a5102bc1970d) - **egui**: add issue report button *(commit by @SARDONYX-sard)*
- [`ffd9c2e`](https://github.com/SARDONYX-sard/d-merge/commit/ffd9c2e53d799f6a654a58fe79becb57a7e79c7d) - **ffi_python**: enhance API *(commit by @SARDONYX-sard)*
- [`4097987`](https://github.com/SARDONYX-sard/d-merge/commit/4097987e71d3e80fd4c19baba581463b9635694c) - **ffi**: include stub generator *(commit by @SARDONYX-sard)*
- [`7931313`](https://github.com/SARDONYX-sard/d-merge/commit/79313131e2c5ea149a7d0bb159bb75d7ec709b47) - **hkanno**: add color pair *(commit by @SARDONYX-sard)*
- [`71afe67`](https://github.com/SARDONYX-sard/d-merge/commit/71afe672557b4dae1b4066e1c99a9310af1b6e51) - **egui**: add `ModType` column *(commit by @SARDONYX-sard)*
- [`ac9a138`](https://github.com/SARDONYX-sard/d-merge/commit/ac9a13844b666bec609dd979624d177215245cfc) - **egui**: support ancestor dialog *(commit by @SARDONYX-sard)*
- [`f002bce`](https://github.com/SARDONYX-sard/d-merge/commit/f002bce6e9a33d991e59262999f43863cecacafc) - **egui**: add `auto detect button` *(commit by @SARDONYX-sard)*
- [`edcd20f`](https://github.com/SARDONYX-sard/d-merge/commit/edcd20fe487f7e00d037e10f7b515cedc118b8ee) - support any encode file reading *(commit by @SARDONYX-sard)*
- [`2e31724`](https://github.com/SARDONYX-sard/d-merge/commit/2e317249f43617fde54b7c170d77e1cf60d5bd01) - support any encode file reading *(commit by @SARDONYX-sard)*
- [`1e52885`](https://github.com/SARDONYX-sard/d-merge/commit/1e52885ccc99636e7296fd47eba501eb92d0efe8) - **fnis**: support caseless anim_type *(commit by @SARDONYX-sard)*
- [`524f6c8`](https://github.com/SARDONYX-sard/d-merge/commit/524f6c8fcc711c13d901daa62379d31a6fa07539) - **FNIS**: support first anim_var *(commit by @SARDONYX-sard)*

### :bug: Bug Fixes
- [`ede0bde`](https://github.com/SARDONYX-sard/d-merge/commit/ede0bde2347bb4c2ed05d7bedb9f636ef7e4b7d8) - **egui**: fix issue link *(commit by @SARDONYX-sard)*
- [`e300844`](https://github.com/SARDONYX-sard/d-merge/commit/e300844f2e256526f9b314f5cb068fe87deaea79) - **mod_info**: fix ord *(commit by @SARDONYX-sard)*
- [`2d0ca83`](https://github.com/SARDONYX-sard/d-merge/commit/2d0ca8396b54c03ac0e1e44cb045787472f65f9a) - **FNIS furniture**: fix index *(commit by @SARDONYX-sard)*
- [`4042a01`](https://github.com/SARDONYX-sard/d-merge/commit/4042a01879c22514734e1b2d8e67218da7352c4e) - **FNIS furniture**: fix index *(commit by @SARDONYX-sard)*
- [`e50e3d2`](https://github.com/SARDONYX-sard/d-merge/commit/e50e3d2a24d784249c6d86aafb13c041c3c51a3f) - **egui**: fix mod list update timing *(commit by @SARDONYX-sard)*
- [`fb7e787`](https://github.com/SARDONYX-sard/d-merge/commit/fb7e787a6caec4177812aba7c1f8500ad2ead76a) - fix wrong patch *(commit by @SARDONYX-sard)*
- [`e718664`](https://github.com/SARDONYX-sard/d-merge/commit/e718664cf539bf3aeb556b36c9f275814eb1faa7) - **FNIS_furniture**: add missing furniture root gen *(commit by @SARDONYX-sard)*
- [`45b6678`](https://github.com/SARDONYX-sard/d-merge/commit/45b6678223279ee228da95ba821dfbcd0f309547) - **FNIS**: fix type *(commit by @SARDONYX-sard)*
- [`54a9234`](https://github.com/SARDONYX-sard/d-merge/commit/54a9234429e3415284de21cbd7fc56d9b0b940bb) - **FNIS furniture**: fix state id *(commit by @SARDONYX-sard)*
- [`867aeae`](https://github.com/SARDONYX-sard/d-merge/commit/867aeae4cd805d4169dd95eaf36a821aaae2d897) - **FNIS furniture**: fix transition info *(commit by @SARDONYX-sard)*
- [`17bce23`](https://github.com/SARDONYX-sard/d-merge/commit/17bce232b4afe444e1ffa5cc3eaee7707dbe1951) - **FNIS furniture**: fix event ID *(commit by @SARDONYX-sard)*

### :recycle: Refactors
- [`c519ca3`](https://github.com/SARDONYX-sard/d-merge/commit/c519ca3f2b9b4b93c22912738ebb637efea7c825) - **egui**: use const var *(commit by @SARDONYX-sard)*

### :white_check_mark: Tests
- [`f57ddfb`](https://github.com/SARDONYX-sard/d-merge/commit/f57ddfbe6e09f73f8e6f9cbdebf3c34d5180d96c) - fix doc test *(commit by @SARDONYX-sard)*

### :wrench: Chores
- [`900d44a`](https://github.com/SARDONYX-sard/d-merge/commit/900d44abf19e3c6c3dfa2386a70cf6d1c72e22e2) - fix typo *(commit by @SARDONYX-sard)*
- [`59a1763`](https://github.com/SARDONYX-sard/d-merge/commit/59a1763a0742ff6e829ff678f08f6f807bfc309b) - fix typo *(commit by @SARDONYX-sard)*
- [`58a2a26`](https://github.com/SARDONYX-sard/d-merge/commit/58a2a2662cd218fe0c4f9c642fcf79be96ed4ce4) - **i18n**: fix typo *(commit by @SARDONYX-sard)*
- [`5e8c3b3`](https://github.com/SARDONYX-sard/d-merge/commit/5e8c3b39a9487b7768bdc16b95e63d0eac567e1a) - **egui**: change table alpha color *(commit by @SARDONYX-sard)*


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
[0.3.0]: https://github.com/SARDONYX-sard/d-merge/compare/0.2.6...0.3.0
