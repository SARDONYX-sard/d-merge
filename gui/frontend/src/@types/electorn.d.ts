/**
 * Type definition for the Electron preload bridge.
 *
 * All APIs exposed via `contextBridge.exposeInMainWorld("__ELECTRON__", { ... })`
 * must conform to this interface.
 *
 * File location: gui/backend/electron/src/preload.ts
 */

import type { OpenDialogOptions, SaveDialogOptions } from '@tauri-apps/plugin-dialog';
import type { ModIds, ModInfo, PatchOptions } from '@/services/api/patch';

/**
 * Electron APIs exposed to the frontend.
 *
 * These functions are called from the browser context
 * and forwarded to the Electron main process via IPC.
 */
export interface ElectronApi {
  // --- Dialog APIs ---
  /** Open a native save file dialog and return selected file path or null. */
  save(options?: SaveDialogOptions): Promise<string | null>;
  /** Open a native open file/directory dialog and return selected path(s) or null. */
  open(options?: OpenDialogOptions): Promise<string | string[] | null>;

  // --- File System APIs ---
  /** Checks if a file or directory exists. */
  exists(path: string): Promise<boolean>;
  /** Read the contents of a file as string. */
  readFile(path: string): Promise<string>;
  /** Write string content to a file. */
  writeFile(path: string, content: string): Promise<void>;

  // --- Shell / Opener APIs ---
  /** Open a URL in the system default browser. */
  openUrl(path: string): Promise<void>;

  /** Open a file or directory with the default OS handler. */
  openPath(path: string): Promise<void>;

  // --- Log Info APIs ---
  /** Change the application's log level. */
  changeLogLevel(level?: string): Promise<void>;
  /** Get the OS-specific log directory path. */
  getAppLogDir(): Promise<string>;
  /** Get the application name. */
  getAppName(): Promise<string>;

  // --- Patch APIs ---
  /** Get the Skyrim data directory for the given runtime. */
  getSkyrimDir(runtime: 'SkyrimSE' | 'SkyrimLE'): Promise<string>;
  /** Load mods info from `info.ini` files matching the glob. */
  loadModsInfo(searchGlob: string): Promise<ModInfo[]>;
  /** Patch mods to hkx files with the specified options. */
  patch(output: string, ids: ModIds, options: PatchOptions): Promise<void>;

  /** Cancel the current patch operation. */
  cancelPatch(): Promise<void>;
  /** Set virtual file system (VFS) mode. */
  setVfsMode(value: boolean): Promise<void>;

  // --- Patch Status Listener ---
  /**
   * Listen to a backend status event for long-running operations.
   * Returns a function to unsubscribe.
   * @category Event
   */
  listen<T>(eventName: string, listener: (payload: T) => void): Promise<() => void>;

  // --- serde_hkx APIs ---
  /** Convert files or directories to a different format (`hkx`/`xml`/`json`/`amd64`/`win32`). */
  convert(inputs: string[], output: string, format: OutFormat, roots?: string[]): Promise<void>;
  /** Load a directory structure as `TreeViewBaseItem[]` for UI display. */
  loadDirNode(dirs: string[]): Promise<TreeViewBaseItem[]>;

  // --- Misc / Window APIs ---
  /** Destroy the current window. */
  destroyWindow(): Promise<void>;
  /** Show a custom context menu defined in main process. */
  showContextMenu({ x, y, selectionText }: { x: number; y: number; selectionText: string }): Promise<void>;

  /** Adjust the zoom factor of the current window. */
  zoom(delta: number): Promise<void>;
}

/**
 * Global type extension for `window`.
 */
declare global {
  interface Window {
    __ELECTRON__: ElectronApi;
  }
}

export {};
