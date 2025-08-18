import type { TreeViewBaseItem } from '@mui/x-tree-view';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { electronApi, isElectron } from './electron/setup';

// NOTE: Do not use yaml because it cannot be reversed.
export type OutFormat = 'amd64' | 'win32' | 'xml' | 'json';

/**
 * Convert xml/hkx => hkx/xml.
 * - `inputs`: Files or dirs.
 * - `output`: Output dir.
 * - `roots`: The root dir of inputs. (This is needed to preserve the dir structure in the output after conversion in Tree mode.)
 * @throws Error
 */
export async function convert(inputs: string[], output: string, format: OutFormat, roots?: string[]) {
  if (isTauri()) {
    await invoke('convert', { inputs, output, format, roots });
  } else if (isElectron()) {
    await electronApi.convert(inputs, output, format, roots);
  } else {
    throw new Error('Unsupported platform');
  }
  await invoke('convert', { inputs, output, format, roots });
}

export async function loadDirNode(dirs: string[]) {
  if (isTauri()) {
    return await invoke<TreeViewBaseItem[]>('load_dir_node', { dirs });
  } else if (isElectron()) {
    return await electronApi.loadDirNode(dirs);
  } else {
    throw new Error('Unsupported platform');
  }
}
