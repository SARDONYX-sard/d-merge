import { invoke } from '@tauri-apps/api/core';

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
export async function openUrl(path: string, openWith?: string) {
  await NOTIFY.asyncTry(async () => await invoke('open', { path, openWith }));
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
export async function openPath(path: string, openWith?: string) {
  await NOTIFY.asyncTry(async () => await invoke('open', { path, openWith }));
}
