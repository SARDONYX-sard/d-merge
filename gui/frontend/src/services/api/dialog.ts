import { isTauri } from '@tauri-apps/api/core';
import { type OpenDialogOptions, open, SaveDialogOptions, save as tauriSave } from '@tauri-apps/plugin-dialog';
import { electronApi, isElectron as isElectron } from './electron';

type OpenOptions = {
  /**
   * path setter.
   * - If we don't get the result within this function, somehow the previous value comes in.(React component)
   * @param path
   * @returns
   */
  setPath?: (path: string) => void;
} & OpenDialogOptions;

/**
 * Open a file or Dir
 * @returns selected path or cancelled null
 * @throws
 */
export async function openPath(path: string, options: OpenOptions = {}): Promise<string | string[] | null> {
  const { setPath, ...dialogOptions } = options;

  const res = await (async () => {
    if (isTauri()) {
      return await open({ defaultPath: path, ...dialogOptions });
    }

    if (isElectron()) {
      return await electronApi.open({ defaultPath: path, ...dialogOptions });
    }

    throw new Error('Unsupported platform: Neither Tauri nor Electron');
  })();

  typeof res === 'string' && setPath?.(res);
  return res;
}

/**
 * Open a file/directory save dialog.
 *
 * @throws Error
 */
export async function save(options: SaveDialogOptions) {
  if (isTauri()) {
    return await tauriSave(options);
  } else if (isElectron()) {
    return await electronApi.save(options);
  } else {
    throw new Error('Unsupported platform: Neither Tauri nor Electron');
  }
}
