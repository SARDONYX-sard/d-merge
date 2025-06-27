from typing import List, Optional, Awaitable, Callable
from enum import Enum

class OutPutTarget(Enum):
    """
    Specifies the Skyrim output format target.

    This affects how data is encoded and structured during export.

    - `SkyrimSe`: 64-bit Skyrim Special Edition (SkyrimSE)
    - `SkyrimLe`: 32-bit Skyrim Legendary Edition (SkyrimLE)
    """

    SkyrimSe: "OutPutTarget"
    SkyrimLe: "OutPutTarget"

class LogLevel(Enum):
    """
    Specifies the verbosity level for tracing logs.

    - `Trace`: Extremely detailed logs for low-level debugging.
    - `Debug`: Detailed information useful during development.
    - `Info`: General information about the process (default).
    - `Warn`: Warnings about potential issues.
    - `Error`: Serious errors that may interrupt execution.
    """

    Trace: "LogLevel"
    Debug: "LogLevel"
    Info: "LogLevel"
    Warn: "LogLevel"
    Error: "LogLevel"

class Status:
    """
    Status update object representing progress in the behavior generation process.

    This class corresponds to a Rust enum used internally to track and report
    the progress of various stages during behavior generation, such as reading
    patches, parsing, applying patches, generating output files, or signaling completion or error.

    Instances of this class are passed to the `status_report` callback to notify
    clients about the current operation and progress metrics.

    Factory Methods:
        ReadingPatches(index: int, total: int) -> Status
            Create a status indicating reading patches progress.
            Args:
                index (int): Zero-based index of the current patch being read.
                total (int): Total number of patches to read.

        ParsingPatches(index: int, total: int) -> Status
            Create a status indicating parsing patches progress.
            Args:
                index (int): Zero-based index of the current patch being parsed.
                total (int): Total number of patches to parse.

        ApplyingPatches(index: int, total: int) -> Status
            Create a status indicating applying patches progress.
            Args:
                index (int): Zero-based index of the current patch being applied.
                total (int): Total number of patches to apply.

        GeneratingHkxFiles(index: int, total: int) -> Status
            Create a status indicating generation progress of HKX files.
            Args:
                index (int): Zero-based index of the current HKX file being generated.
                total (int): Total number of HKX files to generate.

        Done() -> Status
            Create a status indicating the behavior generation process is complete.

        Error(msg: str) -> Status
            Create a status indicating an error has occurred.
            Args:
                msg (str): The error message describing the problem.
    """

    def __str__(self) -> str:
        """Return a concise human-readable status message."""
        ...

    def __repr__(self) -> str:
        """Return a detailed debug representation of the status."""
        ...

    @staticmethod
    def ReadingPatches(index: int, total: int) -> "Status":
        """
        Create a status indicating reading patches progress.

        Args:
            index (int): Zero-based index of the current patch being read.
            total (int): Total number of patches to read.

        Returns:
            Status: A status instance representing the reading patches phase.
        """
        ...

    @staticmethod
    def ParsingPatches(index: int, total: int) -> "Status":
        """
        Create a status indicating parsing patches progress.

        Args:
            index (int): Zero-based index of the current patch being parsed.
            total (int): Total number of patches to parse.

        Returns:
            Status: A status instance representing the parsing patches phase.
        """
        ...

    @staticmethod
    def ApplyingPatches(index: int, total: int) -> "Status":
        """
        Create a status indicating applying patches progress.

        Args:
            index (int): Zero-based index of the current patch being applied.
            total (int): Total number of patches to apply.

        Returns:
            Status: A status instance representing the applying patches phase.
        """
        ...

    @staticmethod
    def GeneratingHkxFiles(index: int, total: int) -> "Status":
        """
        Create a status indicating HKX file generation progress.

        Args:
            index (int): Zero-based index of the current HKX file being generated.
            total (int): Total number of HKX files to generate.

        Returns:
            Status: A status instance representing the HKX generation phase.
        """
        ...

    @staticmethod
    def Done() -> "Status":
        """
        Create a status indicating the completion of the behavior generation process.

        Returns:
            Status: A status instance indicating that the process is done.
        """
        ...

    @staticmethod
    def Error(msg: str) -> "Status":
        """
        Create a status indicating that an error occurred.

        Args:
            msg (str): A message describing the error.

        Returns:
            Status: A status instance representing the error state.
        """
        ...

class Config:
    """
    Configuration options for behavior merging, logging, and debug output.

    This object mirrors the internal Rust `Config` structure used to control the merging
    process. You can use it to configure resource directories, output targets, debug flags,
    and logging behavior.

    Attributes:
        resource_dir (str): The directory containing HKX template resources.
            Example: "assets/templates"

        output_dir (str): The directory where merged output files will be written.

        output_target (OutPutTarget): Target Skyrim version for the output format.
            Choose between `OutPutTarget.SkyrimSe` and `OutPutTarget.SkyrimLe`.

        cast_ragdoll_event (bool): Enables compatibility fixes for invalid ragdoll data in mods.
            Substitutes invalid field names with valid ones in ragdoll modifiers.

        output_patch_json (bool): Outputs the parsed patch JSON for debugging.

        output_merged_json (bool): Outputs the merged JSON before binary export.

        output_merged_xml (bool): Outputs the merged XML before `.hkx` conversion.

        log_path (Optional[str]): Optional file path to write tracing logs to. If `None`, logs go to stderr.

        log_level (Optional[LogLevel]): Optional log verbosity level. Defaults to `Info` if not specified.
    """

    def __init__(
        self,
        resource_dir: str,
        output_dir: str,
        output_target: OutPutTarget,
        cast_ragdoll_event: bool,
        output_patch_json: bool,
        output_merged_json: bool,
        output_merged_xml: bool,
        log_path: Optional[str] = None,
        log_level: Optional[LogLevel] = None,
    ) -> None: ...

def behavior_gen(
    nemesis_paths: List[str],
    config: Config,
    status_report: Optional[Callable[[Status], None]] = None,
) -> Awaitable[None]:
    """
    Starts the async behavior generation process using the given paths and configuration.

    This function wraps the Rust `behavior_gen()` async task and launches it
    on the Tokio runtime. It accepts paths to behavior files and a configuration
    object and applies patching/merging logic based on those inputs.

    Args:
        nemesis_paths (List[str]): A list of directories or files to use as behavior sources.
            These typically point to Nemesis output folders.

        config (Config): The configuration specifying how the merge should be performed.

        status_report (Optional[Callable[[Status], None]]): Optional callback
            invoked with progress updates during generation.

    Returns:
        Awaitable[None]: An awaitable coroutine that resolves when the process completes.

    Raises:
        RuntimeError: If behavior merging fails internally (e.g., malformed input, I/O errors, internal Rust panics).

    Example:
        ```python
        from d_merge_python import Config, OutPutTarget, LogLevel, Status, behavior_gen
        import asyncio, time

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

        nemesis_paths = open("../../dummy/ids.txt", "r", encoding="utf-8").read().splitlines()

        async def run():
            # IMPORTANT: Don't forget to await this!
            await behavior_gen(nemesis_paths, config, status_report=on_status)

        asyncio.run(run())
        ```
    """
    ...
