import type { TreeViewBaseItem } from '@mui/x-tree-view';
import { invoke } from '@tauri-apps/api/core';

export type OutFormat = 'amd64' | 'win32' | 'xml' | 'json' | 'yaml';

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

/**
 * Whether the converter supports json and yaml conversion as well?
 *
 * @throws If the backend API (`invoke`) could not be called.
 */
export async function isSupportedExtraFmt() {
  return await invoke<boolean>('is_supported_extra_fmt');
}

export async function loadDirNode(dirs: string[]) {
  return await invoke<TreeViewBaseItem[]>('load_dir_node', { dirs });
}
