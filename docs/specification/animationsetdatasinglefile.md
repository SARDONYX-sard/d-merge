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

- `//` The explain comment. Not actual data.

```c
// Txt projects
49                                          // <- Project files len
ChickenProjectData\ChickenProject.txt       // <- Project file name[0]: str
HareProjectData\HareProject.txt             // <- Project file name[1]: str
...
WitchlightProjectData\WitchlightProject.txt // <- Project file name[48]: str


// Asdsf["ChickenProjectData\ChickenProject.txt"].AnimSetDataList `.txt` keys
1                                           // <- AnimSetDataList len
FullBody.txt                                // <- AnimSetDataList[0].file name
...
// Asdsf["*.txt"].AnimSetDataList `.txt` keys
1                                           // <- AnimSetDataList len
FullBody.txt                                // <- AnimSetDataList[].file name
...


// Asdsf["DefaultMaleData~DefaultMale"]["1HMShield.txt"].AnimSetDataList.AnimSetData
V3                                          // <- Version: "V3" (always literal)


// Triggers
5                                           // <- triggers len
MagicWeap_ForceEquip                        // <- Trigger[0]: str
swimForceEquip                              // <- Trigger[1]: str
WeapEquip                                   // <- Trigger[2]: str
WeapOutLeftReplaceForceEquip                // <- Trigger[3]: str
WeapOutRightReplaceForceEquip               // <- Trigger[4]: str


// Conditions
3                                           // <- conditions len
iLeftHandType                               // <- Condition[0]: str
10                                          // <- Condition[0] value_a: i32
10                                          // <- Condition[0] value_b: i32

iRightHandType                              // <- Condition[1]: str
1                                           // <- Condition[1] value_a: i32
4                                           // <- Condition[1] value_b: i32

iWantMountedWeaponAnims                     // <- Condition[2]: str
0                                           // <- Condition[2] value_a: i32
0                                           // <- Condition[2] value_b: i32


// Attacks
10                                          // <- attacks len
attackPowerStart_Sprint                     // <- Attack.trigger[0]: str
1                                           // <- Unknown field: i32
1                                           // <- clip names len
AttackPowerForwardSprint                    // <- Clip name: str

attackPowerStartBackward                    // <- Attack.trigger[1]: str
0                                           // <- Unknown field: i32
1                                           // <- clip names len
1HM_AttackPowerBwdUncropped                 // <- Clip name: str

attackPowerStartForward                     // <- Attack.trigger[2]: str
0
1
1HM_AttackPowerFwdUncropped                 // <- Clip name: str

attackPowerStartInPlace                     // <- Attack.trigger[3]: str
0
1
1HM_AttackPower_Intro                       // <- Clip name: str

attackPowerStartLeft                        // <- Attack.trigger[4]: str
0
1
1HM_AttackPowerLeftUncropped                // <- Clip name: str

attackPowerStartRight                       // <- Attack.trigger[5]: str
0
1
1HM_AttackPowerRightUncropped               // <- Clip name: str

attackStart                                 // <- Attack.trigger[6]: str
1                                           // <- Unknown field: i32
2                                           // <- clip names len
1HM_AttackLeft                              // <- Clip name: str
1HM_AttackRight                             // <- Clip name: str

attackStartSprint                           // <- Attack.trigger[7]: str
1
1
1HM_AttackForwardSprint                     // <- Clip name: str

bashPowerStart                              // <- Attack.trigger[8]: str
0
1
shd_BashPower                               // <- Clip name: str

bashStart                                   // <- Attack.trigger[9]: str
0
1
Shd_Bash                                    // <- Clip name: str

// AnimInfos
161                                         // <- anim info len
3064642194                                  // <- AnimInfos[0].dir_path CRC32
2909749619                                  // <- AnimInfos[0].hashed_file_name
7891816                                     // <- AnimInfos[0].ext "xkh"

3064642194                                  // <- AnimInfos[1].dir_path CRC32
2458046085                                  // <- AnimInfos[1].hashed_file_name
7891816                                     // <- AnimInfos[1].ext "xkh"

3064642194                                  // <- AnimInfos[2].dir_path CRC32
2816185793                                  // <- AnimInfos[2].hashed_file_name
7891816                                     // <- AnimInfos[2].ext "xkh"

...
```

## Notes

- If the number is not present or parsing fails (e.g. non-integer), the section is skipped.
- Default values are provided where necessary:
  - `attack_trigger`: `"attackStart"`
  - `clip_names`: `["attackClip"]`
  - `value1`, `value2`: `0`
  - `variable_name`: `""`
