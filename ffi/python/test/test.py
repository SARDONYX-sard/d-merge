"""
- python10~13

- 1 liner
cd ./ffi/python;python -m venv venv;pip install -U pip maturin;venv/Scripts/activate;cargo run -p d_merge_python --bin stub_gen;maturin develop;python ./test/test.py

- step run
cd ./ffi/python;python -m venv venv;pip install -U pip maturin;venv/Scripts/activate;
cargo run -p d_merge_python --bin stub_gen;maturin develop;

python ./test/test.py
"""

import json
from pathlib import Path
import asyncio
import time

from d_merge_python import (
    behavior_gen,
    Config,
    DebugOptions,
    HackOptions,
    load_mods_info,
    logger_init,
    ModInfo,
    ModType,
    OutPutTarget,
    PatchMaps,
    PatchStatus,
    is_dangerous_remove,
    remove_meshes_dir_all,
)


def test_behavior_gen():
    skyrim_data_dir_glob = "D:\\GAME\\ModOrganizer Skyrim SE\\mods\\*"
    output_dir = "test/out"

    config = Config(
        resource_dir="../../resource/assets/templates",
        output_dir=output_dir,
        output_target=OutPutTarget.SkyrimSE,
        hack_options=HackOptions(
            cast_ragdoll_event=True,
            bone_weight_outside_hkparam=True,
        ),
        debug=DebugOptions(
            # output_patch_json=True,
            # output_merged_json=True,
            # output_merged_xml=True,
        ),
        skyrim_data_dir_glob=skyrim_data_dir_glob,
        generate_fnis_esp=True,
    )

    logger_init("./test/logs", "d_merge_python.log", 5, "debug")

    if is_dangerous_remove(output_dir, skyrim_data_dir_glob):
        remove_meshes_dir_all(output_dir)

    patches = to_patches(load_mods_info(skyrim_data_dir_glob, False))
    dump_patches("./test/out/patches.json", patches)

    async def run():
        try:
            await behavior_gen(patches, config, status_fn=make_on_status())
        except Exception as e:
            print(f"Error: {e}")

    asyncio.run(run())


def to_patches(mod_info: list[ModInfo]) -> PatchMaps:
    seen: set[str] = set()
    fnis_entries = dict()
    nemesis_entries = dict()

    for priority, mod in enumerate(mod_info):
        key = Path(mod.id).name  # same as file_name()

        if key in seen:
            continue

        seen.add(key)

        match mod.mod_type:
            case ModType.Fnis:
                fnis_entries[mod.id] = priority
            case ModType.Nemesis | ModType.NemesisExt:
                nemesis_entries[mod.id] = priority

    return PatchMaps(fnis_entries=fnis_entries, nemesis_entries=nemesis_entries)


def dump_patches(path: str, patches: PatchMaps) -> None:
    p = Path(path)
    p.parent.mkdir(parents=True, exist_ok=True)

    data = {
        "fnis_entries": patches.fnis_entries,
        "nemesis_entries": patches.nemesis_entries,
    }

    with p.open("w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, ensure_ascii=False)


def make_on_status():
    start_time = None

    def on_status(status: PatchStatus):
        nonlocal start_time

        if start_time is None:
            start_time = time.time()

        elapsed = time.time() - start_time
        elapsed_str = f"{elapsed:.1f}s"

        CYAN = "\x1b[36m"
        MAGENTA = "\x1b[35m"
        YELLOW = "\x1b[33m"
        BLUE = "\x1b[34m"
        GREEN_BOLD = "\x1b[32;1m"
        RED_BOLD = "\x1b[31;1m"
        RESET = "\x1b[0m"
        CLEAR_LINE = "\r\x1b[2K"

        COLOR_MAP: dict[type, str] = {
            PatchStatus.GeneratingFnisPatches: CYAN,
            PatchStatus.ReadingPatches: MAGENTA,
            PatchStatus.ParsingPatches: YELLOW,
            PatchStatus.ApplyingPatches: BLUE,
            PatchStatus.GeneratingHkxFiles: GREEN_BOLD,
        }

        def print_status(status: PatchStatus, elapsed_str: str):
            if isinstance(status, PatchStatus.Done):
                print(
                    f"{CLEAR_LINE}{GREEN_BOLD}{status}({elapsed_str}){RESET}",
                    flush=True,
                )
                return
            if isinstance(status, PatchStatus.Error):
                print(
                    f"{CLEAR_LINE}{RED_BOLD}{status}({elapsed_str}){RESET}", flush=True
                )
                return

            for class_or_tuple, color in COLOR_MAP.items():
                if isinstance(status, class_or_tuple):
                    print(
                        f"{CLEAR_LINE}{color}{status}({elapsed_str}){RESET}",
                        end="",
                        flush=True,
                    )
                    return

            print(f"{CLEAR_LINE}{elapsed_str}{status}", flush=True)

        print_status(status, elapsed_str)

    return on_status


if __name__ == "__main__":
    test_behavior_gen()
