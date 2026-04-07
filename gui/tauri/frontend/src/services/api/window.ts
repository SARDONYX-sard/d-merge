import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

/**
 * Since the window turns white while it is being prepared, this process is performed in the background,
 * and once the drawing is complete, the front end requests the window to be displayed, thereby suppressing
 * the annoying white screen.
 *
 * @see HACK: Avoid blank white screen on load.
 * - https://github.com/tauri-apps/tauri/issues/5170#issuecomment-2176923461
 * - https://github.com/tauri-apps/tauri/issues/7488
 *
 * @requires
 * tauri.config.json
 * ```json
 * "windows": [
 *   {
 *     "visible": false,
 *   }
 * ```
 */
export async function showWindow() {
  if (typeof window !== 'undefined' && isTauri()) {
    await getCurrentWindow().show();
  }
}

/**
 * Cross-platform window destroy helper.
 *
 * - On Tauri: calls `getCurrentWindow().destroy()`
 * - On Electron: calls `electronApi.destroyWindow()`
 *
 * @throws Error if neither Tauri nor Electron is detected.
 */
export async function destroyCurrentWindow(): Promise<void> {
  if (isTauri()) {
    return await getCurrentWindow().destroy();
  }

  throw new Error('Unsupported platform: neither Tauri nor Electron');
}
