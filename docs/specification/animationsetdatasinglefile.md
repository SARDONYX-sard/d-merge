# `animationsetdatasinglefile.txt`(asdsf) Format Specification

## Project structure

The map has a double structure, with the key coming first.

```txt
project_names len(keys): usize
project_names(keys): Vec<String>(.txt extension)
one project AnimationSetDataList values keys: Vec<str>
AnimationSetDataList: Vec<AnimSetData>
one project AnimationSetDataList values keys: Vec<str>
AnimationSetDataList: Vec<AnimSetData>
...
one project AnimationSetDataList values keys: Vec<str>
AnimationSetDataList: Vec<AnimSetData>
```

## Sample Data

```c
49                                          // <- Project files len
ChickenProjectData\ChickenProject.txt       // <- Project file name[0]: str
HareProjectData\HareProject.txt             // <- Project file name[1]: str
...
WitchlightProjectData\WitchlightProject.txt // <- Project file name[48]: str

// AnimSetDataList["ChickenProjectData\ChickenProject.txt"]
1                                           // <- AnimSetDataList len
FullBody.txt                                // <- AnimSetDataList[0].file name

// AnimSetDataList[0].AnimSetData
V3                                          // <- Version: `"V3"` (always literal)
0                                           // <- triggers len
0                                           // <- conditions len
4                                           // <- attacks len(NOTE: This sample assumes that the Attack is on the chicken)
20                                          // <- anim_infos len

// Attacks
attackStart_L1                              // <- Attack.trigger[0]: str
0                                           // <- Unknown field: i32
1                                           // <- clip names len
AttackLeft1                                 // <- Clip name: str
attackStart_R1                              // <- Attack.trigger[1]: str
0
1
AttackRight1
attackStart_L2                              // <- Attack.trigger[2]: str
0
1
AttackLeft2                                 // <- Attack.trigger[3]: str
attackStart_R2
0
1
AttackRight2                                // <- Attack.trigger[4]: str

// AnimInfos
2725300844                                  // <- AnimInfos[0].dir_path CRC32 encoded `actors` dir path
329189360                                   // <- AnimInfos[0].hashed_file_name (lowercase)
7891816                                     // <- AnimInfos[0].ext `i32::to_le_bytes(b"xkh")` Non-CRC32.
```

## Notes

- `num_triggers`, `num_conditions`, `num_attacks`, and `num_animation_infos` are **optional**.
- If the number is not present or parsing fails (e.g. non-integer), the section is skipped.
- Default values are provided where necessary:
  - `attack_trigger`: `"attackStart"`
  - `clip_names`: `["attackClip"]`
  - `value1`, `value2`: `0`
  - `variable_name`: `""`
