# 📄 `animationsetdatasinglefile.txt`(asdsf) Format Specification

## 🧾 Notation Style (W3C EBNF)

This grammar uses the W3C XML specification EBNF style:

- `::=` means "is defined as".
- Terminals (literal values) are enclosed in double quotes (`"..."`).
- Repetition:
  - `(...)+` means one or more repetitions.
  - `(...)*` means zero or more repetitions.
- Optional:
  - `[...]` indicates optional elements.
- Comments are written using `(* comment *)`.
- Terminals must match exactly unless otherwise noted.

---

## 📄 EBNF Grammar

```ebnf
(* XML-style EBNF for animationsetdatasinglefile.txt *)

file ::= txt_projects anim_set_list

txt_projects ::= project_names_len project_name+
project_names_len ::= number newline
project_name ::= string ".txt" newline

anim_set_list ::= anim_set_data*

anim_set_data ::=
    [ file_names_len file_names ]
    version
    triggers_len triggers
    conditions_len conditions
    attacks_len attacks
    anim_infos_len anim_infos

file_names_len ::= number newline
file_names ::= file_name+
file_name ::= string newline

version ::= "V3" newline

triggers_len ::= number newline
triggers ::= string newline+

conditions_len ::= number newline
conditions ::= condition+
condition ::=
    variable_name value_a value_b
variable_name ::= string newline
value_a ::= integer newline
value_b ::= integer newline

attacks_len ::= number newline
attacks ::= attack+
attack ::=
    attack_trigger unknown clip_names_len clip_names
attack_trigger ::= string newline
unknown ::= ("0" | "1") newline
clip_names_len ::= number newline
clip_names ::= string newline+

anim_infos_len ::= number newline
anim_infos ::= anim_info+
anim_info ::=
    hashed_path hashed_file_name ascii_extension
hashed_path ::= number newline
hashed_file_name ::= number newline
ascii_extension ::= number newline

(* Basic terminals *)
number ::= digit+
integer ::= ["-"] digit+
string ::= printable_non_newline+
digit ::= "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
newline ::= "\n"
printable_non_newline ::= any printable character except newline
```

---

## 🧪 Sample Data

```txt
1
FullBody.txt

V3
0
0
4
attackStart_L1
0
1
AttackLeft1
attackStart_R1
0
1
AttackRight1
attackStart_L2
0
1
AttackLeft2
attackStart_R2
0
1
AttackRight2
```

---

## Explanation (Line-by-Line)

### 🔹 Project Definition

```txt
1
FullBody.txt
```

- Project count: `1`
- Project file name: `FullBody.txt`

---

### 🔹 ProjectAnimSetData Block

```txt
V3
0
0
4
```

- Version: `"V3"` (always literal)
- `0` triggers
- `0` conditions
- `4` attack entries

---

### 🔹 Attack Entries

```txt
attackStart_L1
0
1
AttackLeft1
```

- Attack trigger: `attackStart_L1`
- Unknown field: `0`
- Number of clips: `1`
- Clip name: `AttackLeft1`

(Repeated for L2, R1, R2)

---

### 🔹 Animation Info Block (Example)

```tx
3064642194
0
7891816
```

- `3064642194`: CRC32 encoded path (vanilla actor directory path)
- `0`: encoded file name (likely lowercase)
- `7891816`: ascii of `xkh` (extension `hkx` reversed. Non-CRC32)

---

## 📘 Notes

- `num_triggers`, `num_conditions`, `num_attacks`, and `num_animation_infos` are **optional**.
- If the number is not present or parsing fails (e.g. non-integer), the section is skipped.
- Default values are provided where necessary:
  - `attack_trigger`: `"attackStart"`
  - `clip_names`: `["attackClip"]`
  - `value1`, `value2`: `0`
  - `variable_name`: `""`
