# d_merge_cli

Generates Nemesis/FNIS behavior patches for Skyrim SE/LE.

---

## Overview

There are two usage patterns depending on whether you use a mod manager with a
virtual filesystem (e.g. Mod Organizer 2) or manage mods directly.

### VFS mode (Mod Organizer 2)

`info mods --vfs` returns only the mod code (e.g. `colisc`), not a full path.
You must look up the Skyrim data directory from the registry and assemble the
full path yourself before writing the ini.

```txt
info skyrim-dir  →  info mods --vfs  →  (assemble paths + write ini)  →  patch
```

### Manual mode

`info mods` returns absolute or relative paths directly as the mod ID.
No path assembly is needed.

```txt
info mods  →  (write ini)  →  patch
```

---

## Quick start (Python)

### VFS mode

```python
import subprocess
import json
from pathlib import Path

BIN          = "d_merge_cli"
GLOB         = "D:/GAME/ModOrganizer Skyrim SE/mods/*"
OUTPUT_DIR   = "./output"
RESOURCE_DIR = "./assets/templates"


def run(*args: str, capture: bool = False) -> subprocess.CompletedProcess:
    return subprocess.run(
        [BIN, *args],
        capture_output=capture,
        text=True,
        check=True,
    )


# 1. Resolve Skyrim SE data directory from registry
skyrim_data = Path(run(
    "info", "skyrim-dir", "--runtime", "SkyrimSE",
    capture=True,
).stdout.strip())
print(f"Skyrim data dir: {skyrim_data}")

# 2. List mods through VFS — ids are mod codes only (e.g. "colisc")
mods_json = run(
    "info", "mods", "--glob", GLOB, "--vfs",
    capture=True,
).stdout
mods: list[dict] = json.loads(mods_json)
print(f"Found {len(mods)} mods")

# 3. The ID formats for VFS and manual are different. Since VFS can only accept mod_code, we'll construct it here.
#    Nemesis: <skyrim_data>\Nemesis_Engine\mod\<id>
#    FNIS:    namespace only — path resolution is handled internally
nemesis_base = skyrim_data / "Nemesis_Engine" / "mod"

nemesis_paths = [
    str(nemesis_base / m["id"])
    for m in mods if m["mod_type"] == "nemesis"
]
fnis_ids = [m["id"] for m in mods if m["mod_type"] == "fnis"]

nemesis_ini = Path("nemesis_ids.ini")
fnis_ini    = Path("fnis_ids.ini")
nemesis_ini.write_text("; Auto-generated\n" + "\n".join(nemesis_paths) + "\n")
fnis_ini.write_text("; Auto-generated\n"    + "\n".join(fnis_ids)      + "\n")

# 4. Generate behavior patches
run(
    "patch",
    "--nemesis-ini",  str(nemesis_ini),
    "--fnis-ini",     str(fnis_ini),
    "--output-dir",   OUTPUT_DIR,
    "--resource-dir", RESOURCE_DIR,
    "--skyrim-data-glob", skyrim_data, # If you're using the fnis mod, you must always have this installed or you won't be able to explore.
)
print(f"Done — behaviors written to {OUTPUT_DIR}")
```

### Manual mode

```python
import subprocess
import json
from pathlib import Path

BIN          = "d_merge_cli"
GLOB         = "D:/MO2/mods/*"
OUTPUT_DIR   = "./output"
RESOURCE_DIR = "./assets/templates"


def run(*args: str, capture: bool = False) -> subprocess.CompletedProcess:
    return subprocess.run(
        [BIN, *args],
        capture_output=capture,
        text=True,
        check=True,
    )


# 1. List mods — ids are already full paths in manual mode
mods_json = run(
    "info", "mods", "--glob", GLOB,
    capture=True,
).stdout
mods: list[dict] = json.loads(mods_json)
print(f"Found {len(mods)} mods")

# 2. Write ini files directly (no path assembly needed)
nemesis_ids = [m["id"] for m in mods if m["mod_type"] == "nemesis"]
fnis_ids    = [m["id"] for m in mods if m["mod_type"] == "fnis"]

nemesis_ini = Path("nemesis_ids.ini")
fnis_ini    = Path("fnis_ids.ini")
nemesis_ini.write_text("; Auto-generated\n" + "\n".join(nemesis_ids) + "\n")
fnis_ini.write_text("; Auto-generated\n"    + "\n".join(fnis_ids)    + "\n")

# 3. Generate behavior patches
run(
    "patch",
    "--nemesis-ini",  str(nemesis_ini),
    "--fnis-ini",     str(fnis_ini),
    "--output-dir",   OUTPUT_DIR,
    "--resource-dir", RESOURCE_DIR,
    "--skyrim-data-glob", GLOB, # If you're using the fnis mod, you must always have this installed or you won't be able to explore.
)
print(f"Done — behaviors written to {OUTPUT_DIR}")
```

---

## ini file format

### nemesis_ids.ini

Whether or not the directory is under VFS does not matter for the `patch` command.
When running within the MO2 virtualization environment, the path will always be
`<data dir>\Nemesis_Engine\mod\<mod_code>`.

```ini
; Nemesis mod IDs — order determines patch priority (top = highest)
D:\STEAM\steamapps\common\Skyrim Special Edition\Data\Nemesis_Engine\mod\slide
D:\STEAM\steamapps\common\Skyrim Special Edition\Data\Nemesis_Engine\mod\dmco
D:\STEAM\steamapps\common\Skyrim Special Edition\Data\Nemesis_Engine\mod\dwulkr
D:\STEAM\steamapps\common\Skyrim Special Edition\Data\Nemesis_Engine\mod\flinch
D:\STEAM\steamapps\common\Skyrim Special Edition\Data\Nemesis_Engine\mod\poise
D:\STEAM\steamapps\common\Skyrim Special Edition\Data\Nemesis_Engine\mod\para
D:\STEAM\steamapps\common\Skyrim Special Edition\Data\Nemesis_Engine\mod\scar
D:\STEAM\steamapps\common\Skyrim Special Edition\Data\Nemesis_Engine\mod\draw
```

### fnis_ids.ini

FNIS specifies the namespace located directly under the `animations` directory
within the `meshes` directory. Path resolution is handled internally.

Use the `--skyrim-data-glob` option to specify the search range during patching

```ini
; FNIS mod IDs (namespaces)
FNISBase
FNISCreatureVersion
FNISZoo
P1FlyingRing
XPMSE
backgrab
backgrabnosneak
backstabnosneak
frontgrab
newfightcb1
```
