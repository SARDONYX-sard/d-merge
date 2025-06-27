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

from d_merge_python import Config, LogLevel, OutPutTarget, Status, behavior_gen


def on_status(status: Status):
    CYAN = "\x1b[36m"
    MAGENTA = "\x1b[35m"
    YELLOW = "\x1b[33m"
    BLUE = "\x1b[34m"
    GREEN_BOLD = "\x1b[32;1m"
    RED_BOLD = "\x1b[31;1m"
    RESET = "\x1b[0m"
    CLEAR_LINE = (
        "\r\x1b[2K"  # Delete the entire line and move to the beginning of the line.
    )

    text = repr(status)
    if "ReadingPatches" in text:
        print(f"{CLEAR_LINE}{CYAN}{text}{RESET}", end="")
    elif "ParsingPatches" in text:
        print(f"{CLEAR_LINE}{MAGENTA}{text}{RESET}", end="")
    elif "ApplyingPatches" in text:
        print(f"{CLEAR_LINE}{YELLOW}{text}{RESET}", end="")
    elif "GeneratingHkxFiles" in text:
        print(f"{CLEAR_LINE}{BLUE}{text}{RESET}", end="")
    elif "Done" in text:
        print(f"{CLEAR_LINE}{GREEN_BOLD}{text}{RESET}")
    elif "Error" in text:
        print(f"{CLEAR_LINE}{RED_BOLD}{text}{RESET}")
    else:
        print(text)


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
        paths = f.read().splitlines()

    import asyncio

    async def run():
        try:
            await behavior_gen(paths, config, status_report=on_status)
            print("✅ behavior_gen_py executed successfully.")
        except Exception as e:
            print(f"❌ Error: {e}")

    asyncio.run(run())


if __name__ == "__main__":
    test_behavior_gen()
