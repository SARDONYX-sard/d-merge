"""
python10~13

$/d-merge/ffi/python
>>>

python -m venv venv;pip install -U pip maturin
venv/Scripts/activate;
maturin develop;python ./test/test.py

cargo run --bin stub_gen
"""

import asyncio
import time

from d_merge_python import (
    Config,
    OutPutTarget,
    PatchMaps,
    PatchStatus,
    behavior_gen,
    change_log_level,
    logger_init,
)


def test_behavior_gen():
    config = Config(
        resource_dir="../../resource/assets/templates",
        cast_ragdoll_event=True,
        output_dir="test/out",
        output_target=OutPutTarget.SkyrimSE,
        output_patch_json=False,
        output_merged_json=False,
        output_merged_xml=False,
        skyrim_data_dir_glob="../dummy/fnis_test_mods/*",
    )

    logger_init("./test/logs", "d_merge_python.log")
    change_log_level("debug")

    # Nemesis patch ids
    with open("../../dummy/ids.ini", "r", encoding="utf-8") as f:
        nemesis_paths = [
            line.strip() for line in f if line.strip() and not line.startswith(";")
        ]
        nemesis_entries = {path: idx for idx, path in enumerate(nemesis_paths)}
    patches = PatchMaps()
    patches.nemesis_entries = nemesis_entries

    async def run():
        try:
            await behavior_gen(patches, config, status_fn=make_on_status())
            print("✅ behavior_gen_py executed successfully.")
        except Exception as e:
            print(f"❌ Error: {e}")

    asyncio.run(run())


def make_on_status():
    start_time = None

    def on_status(status: PatchStatus):
        nonlocal start_time

        if start_time is None:
            start_time = time.time()

        elapsed = time.time() - start_time
        elapsed_str = f"{elapsed:.1f}s: "

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
                    f"{CLEAR_LINE}{GREEN_BOLD}{elapsed_str}✅{status}{RESET}",
                    flush=True,
                )
                return
            if isinstance(status, PatchStatus.Error):
                print(
                    f"{CLEAR_LINE}{RED_BOLD}{elapsed_str}❌ {status}{RESET}", flush=True
                )
                return

            for class_or_tuple, color in COLOR_MAP.items():
                if isinstance(status, class_or_tuple):
                    print(
                        f"{CLEAR_LINE}{color}{elapsed_str}{status}{RESET}",
                        end="",
                        flush=True,
                    )
                    return

            print(f"{CLEAR_LINE}{elapsed_str}{status}", flush=True)

        print_status(status, elapsed_str)

    return on_status


if __name__ == "__main__":
    test_behavior_gen()
