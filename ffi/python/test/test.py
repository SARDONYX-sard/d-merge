"""
python10~13

$/d-merge/ffi/python
>>>

python -m venv venv
venv/Scripts/activate
pip install -U pip maturin
maturin develop
python ./test/test.py
"""

from d_merge_python import (
    Config,
    LogLevel,
    OutPutTarget,
    PatchMaps,
    Status,
    behavior_gen,
)


import time

start_time = None


def on_status(status: Status):
    global start_time

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

    text = str(status)

    display_text = elapsed_str + text

    if "[1/5]" in text:
        print(f"{CLEAR_LINE}{CYAN}{display_text}{RESET}", end="")
    elif "[2/5]" in text:
        print(f"{CLEAR_LINE}{MAGENTA}{display_text}{RESET}", end="")
    elif "[3/5]" in text:
        print(f"{CLEAR_LINE}{YELLOW}{display_text}{RESET}", end="")
    elif "[4/5]" in text:
        print(f"{CLEAR_LINE}{BLUE}{display_text}{RESET}", end="")
    elif "[5/5]" in text:
        print(f"{CLEAR_LINE}{GREEN_BOLD}{display_text}{RESET}")
    elif "Error" in text:
        print(f"{CLEAR_LINE}{RED_BOLD}{display_text}{RESET}")
    else:
        print(display_text)


def test_behavior_gen():
    # Path:
    # Relative path from the location of `venv` dir.
    # NOTE: Not from the location of test.py.

    config = Config(
        resource_dir="../../resource/assets/templates",
        output_dir="test/out",
        output_target=OutPutTarget.SkyrimSe,
        cast_ragdoll_event=True,
        output_patch_json=False,
        output_merged_json=False,
        output_merged_xml=False,
        log_level=LogLevel.Trace,
        log_path="./python3_ffi_test.log",
    )

    # Nemesis patch ids
    with open("../../dummy/ids.txt", "r", encoding="utf-8") as f:
        nemesis_paths = f.read().splitlines()
        nemesis_entries = {path: idx for idx, path in enumerate(nemesis_paths)}
    patches = PatchMaps(nemesis_entries, fnis_entries={})

    import asyncio

    async def run():
        try:
            await behavior_gen(patches, config, status_report=on_status)
            print("✅ behavior_gen_py executed successfully.")
        except Exception as e:
            print(f"❌ Error: {e}")

    asyncio.run(run())


if __name__ == "__main__":
    test_behavior_gen()
