import type { TreeViewBaseItem } from '@mui/x-tree-view';
import { invoke } from '@tauri-apps/api/core';

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
  await invoke('convert', { inputs, output, format, roots });
}

export async function loadDirNode(dirs: string[]) {
  return await invoke<TreeViewBaseItem[]>('load_dir_node', { dirs });
}
