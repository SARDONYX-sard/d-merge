import { isTauri } from '@tauri-apps/api/core';
import { openPath as tauriOpenPath, openUrl as tauriOpenUrl } from '@tauri-apps/plugin-opener';
import { NOTIFY } from '@/lib/notify';
import { electronApi, isElectron } from './electron';

/**
 * Wrapper tauri's `open` with `notify.error`
 *
 * # Why need this?
 * Use the backend api to jump to the link so that it can be opened in the default browser without opening it in the webview.
 *
 * @export
 * @param {string} path
 * @param {string} [openWith]
 */
export async function openUrl(path: string) {
  await NOTIFY.asyncTry(async () => {
    if (isTauri()) {
      return await tauriOpenUrl(path);
    }

    if (isElectron()) {
      return await electronApi.openUrl(path);
    }

    throw new Error('Unsupported platform: Neither Tauri nor Electron');
  });
}

/**
 * Wrapper tauri's `openPath` with `notify.error`
 *
 * # Why need this?
 * Use the backend api to jump to the link so that it can be opened in the default browser without opening it in the webview.
 *
 * @export
 * @param {string} path
 * @param {string} [openWith]
 */
export async function openPath(path: string) {
  await NOTIFY.asyncTry(async () => {
    if (isTauri()) {
      await tauriOpenPath(path);
    } else if (isElectron()) {
      await electronApi.openPath(path);
    } else {
      throw new Error('Unsupported platform: Neither Tauri nor Electron');
    }
  });
}
