declare module 'd_merge_node' {
  export type StatusIndexing = {
    /** 1 based index */
    index: number;
    total: number;
  };
  /** Error message from the backend */
  export type ErrorPayload = string;

  /**
   * Backend status enum for merge operation (defined in Rust).
   *
   * The backend emits these status values using `window.emit(...)` during various stages.
   * - Mirrors Rust enum with serde(tag="type", content="content").
   */
  export type Status =
    | { type: 'ReadingPatches'; content: StatusIndexing }
    | { type: 'ParsingPatches'; content: StatusIndexing }
    | { type: 'ApplyingPatches'; content: StatusIndexing }
    | { type: 'GeneratingHkxFiles'; content: StatusIndexing }
    | { type: 'Done' }
    | { type: 'Error'; content: ErrorPayload };

  export type ModIds = readonly string[];
  export type ModInfo = {
    id: string;
    name: string;
    author: string;
    site: string;
    auto: string;
  };

  export type OutputFormat = 'amd64' | 'win32' | 'xml' | 'json';
  export type TreeViewBaseItem = {
    id: string;
    label: string;
    children?: TreeViewBaseItem[];
  };

  // -- log api --
  /**
   * Initialize the global tracing logger.
   *
   * @param logDir - Directory where log files will be stored.
   * @param logName - Base name for the log file.
   * @returns A Promise that resolves when the logger is successfully initialized.
   */
  export function loggerInit(logDir: string, logName: string): Promise<void>;
  /**
   * Change the global log level at runtime.
   *
   * @param level - New log level ("trace", "info", "warn", "error").
   * @returns A Promise that resolves when the log level has been updated.
   */
  export function changeLogLevel(level: string): Promise<void>;
  /**
   * Log a message at TRACE level.
   *
   * @param message - The message to log. Should be stringifyable.
   */
  export function logTrace(message: string): void;
  /**
   * Log a message at DEBUG level.
   *
   * @param message - The message to log. Should be stringifyable.
   */
  export function logDebug(message: string): void;
  /**
   * Log a message at INFO level.
   *
   * @param message - The message to log. Should be stringifyable.
   */
  export function logInfo(message: string): void;
  /**
   * Log a message at WARN level.
   *
   * @param message - The message to log. Should be stringifyable.
   */
  export function logWarn(message: string): void;
  /**
   * Log a message at ERROR level.
   *
   * @param message - The message to log. Should be stringifyable.
   */
  export function logError(message: string): void;

  // -- serde_hkx api --
  export function loadDirNode(dirs: string[]): Promise<TreeViewBaseItem[]>;
  /**
   *
   * @param inputs input path
   * @param output output path
   * @param format output format
   * @param roots  input root paths(For multi convert)
   */
  export function convert(inputs: string[], output: string, format: OutputFormat, roots?: string[]): Promise<void>;

  // -- patch api --
  export function behaviorGen(ids: ModIds, config: Config): Promise<void>;
  export function getSkyrimDataDir(runtime: 'SkyrimSE' | 'SkyrimLE'): Promise<string>;
  export function loadModsInfo(glob: string): Promise<ModInfo[]>;
  export function cancelPatch(): Promise<void>;

  /** patch configuration */
  export type Config = {
    /**
     * The directory containing the HKX templates you want to patch.
     * Typically something like "assets/templates".
     * The actual patch target should be a subdirectory (e.g., "assets/templates/meshes").
     */
    resourceDir: string;

    /**
     * The directory where the output files will be saved.
     * This will also contain a `.debug` subdirectory if debug output is enabled.
     */
    outputDir: string;

    /**
     * Generation target (SkyrimSE | SkyrimLE).
     */
    outputTarget: OutPutTarget;

    /**
     * An optional callback function that reports the current status of the process.
     * It receives `Status` updates (progress, errors, runtime events).
     */
    statusReport?: StatusReporterFn;

    /**
     * Enables lenient parsing for known issues in unofficial or modded patches.
     */
    hackOptions?: HackOptions;

    /**
     * Options controlling the output of debug artifacts.
     */
    debug: DebugOptions;
  };

  /**
   * Output type — mirrors Rust `OutPutTarget`.
   */
  export type OutPutTarget = 'SkyrimSE' | 'SkyrimLE';

  // Status reporter callback is optional (Option<...> in Rust)
  export type StatusReporterFn = (status: Status) => void;
  /**
   * Hack options that enable non-standard parsing behavior.
   * These exist to handle cases where mods/tools produce invalid or inconsistent data.
   */
  export type HackOptions = {
    /**
     * Enables compatibility hacks for invalid fields in the
     * `BSRagdollContactListenerModifier` class.
     *
     * Fixes common mistakes such as:
     * - Substituting `event` with `contactEvent`
     * - Substituting `anotherBoneIndex` with `bones`
     */
    castRagdollEvent: boolean;
  };

  /**
   * Flags to enable debug output of intermediate files.
   */
  export type DebugOptions = {
    /**
     * If true, outputs the raw patch JSON to `<output_dir>/.debug/.d_merge`.
     */
    outputPatchJson: boolean;

    /**
     * If true, outputs the merged JSON (after all patches, before HKX conversion).
     */
    outputMergedJson: boolean;

    /**
     * If true, outputs the intermediate merged XML (before HKX conversion).
     */
    outputMergedXml: boolean;
  };
}
