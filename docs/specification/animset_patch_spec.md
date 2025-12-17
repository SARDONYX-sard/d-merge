# animset_patch_spec.md

This document specifies the **Animation Set Diff Patch** format used to represent per-line modifications inside animation set files. The key property of this format is that **a diff block may appear around any line**. The block carries both the _applied (modified)_ value and the _original (previous)_ value. This document is formatted for easy machine and human consumption and is split by logical categories to keep diffs readable.

---

## Patch Block Semantics

A patch block always has three markers in this order:

```text
<!-- MOD_CODE ~<category>~ OPEN -->
<modified / applied lines>
<!-- ORIGINAL -->
<original / previous lines>
<!-- CLOSE -->
```

- The lines between `OPEN` and `ORIGINAL` are the **applied (modified)** data — i.e. how the file should look after applying the patch.
- The lines between `ORIGINAL` and `CLOSE` are the **original (previous)** data — i.e. the data that was present before the patch.
- A patch block may wrap a single value line or multiple lines.
- A patch block may appear **before, after, or instead of** the base line — effectively any file position.

---

## Full Example (category-separated, copy-paste ready)

### 1. Version / Length Section

```text
<!-- MOD_CODE ~diff_example~ OPEN -->
V3                             <- version
<!-- ORIGINAL -->
V3                             <- version
<!-- CLOSE -->

<!-- MOD_CODE ~diff_example~ OPEN -->
0                              <- triggers_len
<!-- ORIGINAL -->
0                              <- triggers_len
<!-- CLOSE -->

<!-- MOD_CODE ~diff_example~ OPEN -->
0                              <- conditions_len
<!-- ORIGINAL -->
0                              <- conditions_len
<!-- CLOSE -->

<!-- MOD_CODE ~diff_example~ OPEN -->
4                              <- attacks_len
<!-- ORIGINAL -->
4                              <- attacks_len
<!-- CLOSE -->
```

---

### 2. Attack Blocks

#### Attack[0]

```text
<!-- MOD_CODE ~diff_example~ OPEN -->
attackStart_Attack1            <- attack_trigger[0]
<!-- ORIGINAL -->
attackStart_Attack1            <- attack_trigger[0]
<!-- CLOSE -->

<!-- MOD_CODE ~diff_example~ OPEN -->
0                              <- is_contextual
<!-- ORIGINAL -->
0                              <- is_contextual
<!-- CLOSE -->

<!-- MOD_CODE ~diff_example~ OPEN -->
1                              <- clip_names_len
<!-- ORIGINAL -->
1                              <- clip_names_len
<!-- CLOSE -->

<!-- MOD_CODE ~diff_example~ OPEN -->
Attack1                        <- clip_names[0]
<!-- ORIGINAL -->
Attack1                        <- clip_names[0]
<!-- CLOSE -->
```

#### Attack[1]

```text
<!-- MOD_CODE ~diff_example~ OPEN -->
attackStart_Attack2            <- attack_trigger[1]
<!-- ORIGINAL -->
attackStart_Attack2            <- attack_trigger[1]
<!-- CLOSE -->

0                              <- is_contextual
1                              <- clip_names_len

<!-- MOD_CODE ~diff_example~ OPEN -->
Attack1_Mirrored               <- clip_names[0]
<!-- ORIGINAL -->
Attack1_Mirrored               <- clip_names[0]
<!-- CLOSE -->
```

#### Attack[2]

```text
attackStart_MC_1HMLeft         <- attack_trigger[2]
0                              <- is_contextual
1                              <- clip_names_len

<!-- MOD_CODE ~diff_example~ OPEN -->
MC_1HM_AttackLeft02            <- clip_names[0]
<!-- ORIGINAL -->
MC_1HM_AttackLeft02            <- clip_names[0]
<!-- CLOSE -->
```

#### Attack[3]

```text
attackStart_MC_1HMRight        <- attack_trigger[3]
0                              <- is_contextual
1                              <- clip_names_len

<!-- MOD_CODE ~diff_example~ OPEN -->
MC 1HM AttackRight01           <- clip_names[0]
<!-- ORIGINAL -->
MC 1HM AttackRight01           <- clip_names[0]
<!-- CLOSE -->
```

---

### 3. AnimationInfo Length

```text
<!-- MOD_CODE ~diff_example~ OPEN -->
2                              <- anim_infos_len
<!-- ORIGINAL -->
2                              <- anim_infos_len
<!-- CLOSE -->
```

---

### 4. AnimationInfos

#### AnimInfo[0]

```text
<!-- MOD_CODE ~diff_example~ OPEN -->
3064642194                     <- hashed_path[0]
<!-- ORIGINAL -->
3064642194                     <- hashed_path[0]
<!-- CLOSE -->

<!-- MOD_CODE ~diff_example~ OPEN -->
1047251415                     <- hashed_file_name[0]
<!-- ORIGINAL -->
1047251415                     <- hashed_file_name[0]
<!-- CLOSE -->

<!-- MOD_CODE ~diff_example~ OPEN -->
7891816                        <- ascii_extension[0]
<!-- ORIGINAL -->
7891816                        <- ascii_extension[0]
<!-- CLOSE -->
```

#### AnimInfo[1]

```text
<!-- MOD_CODE ~diff_example~ OPEN -->
3064642194                     <- hashed_path[1]
<!-- ORIGINAL -->
3064642194                     <- hashed_path[1]
<!-- CLOSE -->

<!-- MOD_CODE ~diff_example~ OPEN -->
19150068                       <- hashed_file_name[1]
<!-- ORIGINAL -->
19150068                       <- hashed_file_name[1]
<!-- CLOSE -->

<!-- MOD_CODE ~diff_example~ OPEN -->
7891816                        <- ascii_extension[1]
<!-- ORIGINAL -->
7891816                        <- ascii_extension[1]
<!-- CLOSE -->
```

---

## Notes / Parsing Guidance

- Parsers should treat `OPEN` → `ORIGINAL` content as the **applied/active** value(s) shown in the file; `ORIGINAL` → `CLOSE` as the previous values.
- A block can wrap a single line or multiple consecutive lines. The parser must support reading blocks that start and end between any two lines.
- Blocks cannot be nested; each `OPEN` must match the next `CLOSE`.
- Category labels (the `~tag~` after `MOD_CODE`) are primarily for human organization and tooling; they are optional identifiers, but consistent tags help grouping.
- For human readability, keep grouped categories in separate code blocks to avoid flood of inline diff blocks.

---

## Appendix: small illustrative variation (replace vs add)

```text
<!-- MOD_CODE ~animinfo_replace~ OPEN -->
4000000000
2000000000
7891816
<!-- ORIGINAL -->
3995179646
1440038008
7891816
<!-- CLOSE -->

...followed by existing entry...

3064642194
19150068
7891816

<!-- MOD_CODE ~animinfo_adds~ OPEN -->
4000000003
2000000003
7891816

4000000004
2000000004
7891816

4000000005
2000000005
7891816
<!-- CLOSE -->

3064642194
19150068
7891816
```
