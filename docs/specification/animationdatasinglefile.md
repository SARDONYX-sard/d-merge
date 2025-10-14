# `animationdatasinglefile.txt`(adsf) Format Specification

## Project structure

```txt
  project_names len: usize
  project_names: Vec<String>(.txt extension)
  Animation Data
  Motion Data
```

## Display format

Very important line number in `animationdatasinglefile.txt`

```txt
// comment
... skip line display

line number: content
```

## Hands on

```c
  1: 429                                  // <-- project_names line len == project_names_len
  2: ChickenProject.txt                   // <-- project_name start
  3: HareProject.txt
  4: AtronachFlame.txt
  5: AtronachFrostProject.txt
  6: AtronachStormProject.txt
   :...
430: WoodenBow.txt                        // <-- project_name end

// ChickenProject.txt
431: 328                                  // <-- line range (header + anim data block + motion block + empty line 1, hard coding with mod)

// header
432: 1                                    // <-- lead_int (maybe only 1)
433: 3                                    // <-- project_assets_len: usize
435: Behaviors\ChickenBehavior.hkx        // <-- project_asset[0]
436: Characters\ChickenCharacter.hkx      // <-- project_asset[1]
437: Character Assets\skeleton.HKX        // <-- project_asset[2]
439: 1                                    // <-- has motion data (1/0): bool

// clip_anim_blocks

// clip_anim_blocks[0]
440: TurnRight[mirrored]                  // <-- name: &str
441: 18                                   // <-- clip_id: &str
442: 1                                    // <-- play_back_speed: f32
443: 0                                    // <-- crop_start_local_time: f32
444: 0                                    // <-- crop_end_local_time: f32
445: 0                                    // <--  trigger_names_len: usize
// clip_anim_blocks[1]
446: MainIdle                             // <-- name: &str
447: 12                                   // <-- clip_id: &str
448: 1                                    // <-- play_back_speed: f32
449: 0                                    // <-- crop_start_local_time: f32
450: 0                                    // <-- crop_end_local_time: f32
451: 1                                    // <--  trigger_names_len: usize
452: clipEnd:6.65767                      // <--  trigger_name[0]: &str
   :...

// clip_motion_blocks 431 + 328 = 759 line th
760: 219                                  // <--- line range(This means, 219 line is `clip_anim_blocks`)

// clip_anim_blocks[0]
761: 0                                    // <--- clip_id: &str
762: 1.33333                              // <--- duration: f32
763: 1                                    // <--- translation_len: usize
764: 1.33333 0 0 0                        // <--- translations { time: 1.3333, x: 0, y: 0, z: 0 } (no transition)
765: 1                                    // <--- rotation_len: usize
766: 1.33333 0 0 0 1                      // <--- rotations { time: 1.33333, x: 0, y: 0, z: 0, w: 1 } (no rotation)
   :...
```
