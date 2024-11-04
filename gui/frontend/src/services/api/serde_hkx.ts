import { invoke } from '@tauri-apps/api/core';

import type { TreeViewBaseItem } from '@mui/x-tree-view';

export type OutFormat = 'amd64' | 'win32' | 'xml' | 'json' | 'yaml';

/**
 * Convert xml/hkx => hkx/xml.
 * - `inputs`: Files or dirs.
 * - `output`: Output dir.
 * @throws Error
 */
export async function convert(inputs: string[], output: string, format: OutFormat) {
  await invoke('convert', { inputs, output, format });
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
