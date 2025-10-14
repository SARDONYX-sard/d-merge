# Skyrim Animation Mod Test Status

**Project:** Custom patcher development — validation of animation behavior mods for Skyrim SE/AE.
This document records test results for both **Nemesis** and **FNIS**-based mods.

Status Pattern

- ✅: In the tested scenario, no crashes occurred and the animation functioned normally.
- ⚠️: The patch can be applied, but it does not perform as expected.
- ❌: The patch itself cannot be applied. Or it is using syntax that is not yet supported.

## Repository Owner Test environment

Hardware

- CPU: Intel(R) Core(TM) i7-8700 CPU @ 3.20GHz (3.19 GHz)
- DRAM: 32.0 GB (31.8 GB)
- GPU: NVIDIA GeForce GTX 1080 Ti

OS

- Edition: Windows 11
- Version: 24H2
- OS build: 26100.6584

SkyrimSE

- file version: 1.6.1170.0

## Pure Nemesis Mods

- Mods containing only `Nemesis_Engine`. These will likely work if you output hkx files for LE.

| Status | Mod Name                                       | Version | Note                                       |
| ------ | ---------------------------------------------- | ------- | ------------------------------------------ |
| ✅     | Faster Draw                                    | 1.31    |                                            |
| ✅     | FlinchingSSE                                   | 1.4     |                                            |
| ⚠️     | Jump Behavior Overhaul SE                      | 1.5     | Can patch it, but it freezes upon landing. |
| ✅     | Slow Sprint Bug Fix                            | 1.0     |                                            |
| ✅     | Smooth Crouch Transition                       | 1.3     |                                            |
| ✅     | SprintSwim                                     | 1.6     |                                            |
| ✅     | Thu'um - Animated Shouts                       | 1.0     |                                            |
| ✅     | UnblockableAttacks                             | 1.0     |                                            |
| ✅     | Vanguard NPC                                   | 2.2     |                                            |
| ✅     | Weapon Styles - DrawSheathe Animations for IED | 2.1     |                                            |

## Nemesis(+ esp, scripts)

| Status | Mod Name                         | Version   | Note |
| ------ | -------------------------------- | --------- | ---- |
| ✅     | Crouch Sliding                   | 1.0       |      |
| ✅     | Eating Animations And Sounds SE  | 1.9.4     |      |
| ✅     | EVGAT Framework                  | 2.0beta   |      |
| ✅     | Hot Key Skill                    | 0.8       |      |
| ✅     | MaxsuBlockOverhaul MBO           | 0.21alpha |      |
| ✅     | Recovery System                  | 1.01      |      |
| ✅     | SBE True Prone System            | 1.00      |      |
| ✅     | UnderwaterCombat                 | 1.02      |      |
| ✅     | BFCO - Attack Behavior Framework | 3.3.13    |      |

## Nemesis + SKSE plugin

- Nemesis patch with DLL dependencies. This likely won't run in LE.

| Status | Mod Name                           | Version | Note                                               |
| ------ | ---------------------------------- | ------- | -------------------------------------------------- |
| ✅     | ADXP MCO 1.6.0.6 Bug Fixes         | 2.0.6   |                                                    |
| ✅     | Animated Interactions              | 0.3     |                                                    |
| ✅     | Attack - MCO_DXP -SE               | 1.6.06  | Using MCO Universal Support                        |
| ✅     | DMCO_stable                        | 0.9.6   | AE requires separate DLL overwrites and additions. |
| ✅     | Elden Counter MCO-ADXP             | 1.6     | With dTry Plugin Updates                           |
| ✅     | ModernStaggerLock SE/AE            | 1.7     |                                                    |
| ✅     | NPC Block Loop Fix                 | 1.0     |                                                    |
| ✅     | Paraglider -SE                     | 1.5     | Test using NG dll                                  |
| ✅     | Payload Interpreter                | 1.1     |                                                    |
| ✅     | POISE - Stagger Overhaul SE        | 1.02    |                                                    |
| ✅     | Poisebreaker -SE                   | 0.7     |                                                    |
| ✅     | Precision                          | 2.0.4   |                                                    |
| ✅     | Precision Creatures                | 2.4     | Need Precision Creatures damage_patch              |
| ✅     | Project Impact SE                  | 2.72    |                                                    |
| ✅     | SCAR - Skyrim Combos AI Revolution | 1.06b   | With SCAR AE Support(v1.6.1)                       |
| ✅     | True Directional Movement -SE, AE  | 2.2.5   |                                                    |

## FNIS

| Status | Mod Name                                  | Version | Note                                                                               |
| ------ | ----------------------------------------- | ------- | ---------------------------------------------------------------------------------- |
| ✅     | (Super Fast) Immersive Animated Looting   | 2.7     | ofa:                                                                               |
| ✅     | Flying Mod 2.0s                           | 1.1     | b:                                                                                 |
| ⚠️     | FNIS Flyer SE                             | 7.0     | b,s,+: Animation is possible. Movement is completely impossible. (Cause unknown)   |
| ✅     | FNIS Zoo(LE Mod)                          | 5.0.1   | b: We need to convert hkx to SE (otherwise it'll be an A pose).                    |
| ✅     | Immersive Interactions - Animated Actions | 1.78    | b,ofa:                                                                             |
| ✅     | Kinoko Pose(LE Mod)                       | 1.0     | b: We need to convert hkx to SE (otherwise it'll be an A pose).                    |
| ⚠️     | Low-blow human-human killmoves            | 1.3.0   | km: The animation played correctly, but the actual kill move has not been tested.  |
| ✅     | Ride Sharing SE                           | 0.4b    | b:                                                                                 |
| ❌     | XP32 Maximum Skeleton Special Extended    | 5.06    | AAPrefix: The AltAnim syntax is not yet supported. (Planned for conversion to OAR) |

Currently, since the bones themselves are usable in XP32 Maximum Skeleton Special Extended, we can check them in MO2 but do not need to check them in D Merge to use them.
