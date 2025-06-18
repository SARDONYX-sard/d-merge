import { openPath as _openPath, openUrl as _openUrl } from '@tauri-apps/plugin-opener';

import { NOTIFY } from '@/lib/notify';

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
  await NOTIFY.asyncTry(async () => await _openUrl(path));
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
  await NOTIFY.asyncTry(async () => await _openPath(path));
}
