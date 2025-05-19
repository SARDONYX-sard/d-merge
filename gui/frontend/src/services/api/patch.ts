import { invoke } from '@tauri-apps/api/core';

export type ModInfo = {
  id: string;
  name: string;
  author: string;
  site: string;
  auto: string;
};

export type ModIds = readonly string[];

/**
 * Load mods `info.ini`
 * @throws Error
 */
export async function loadModsInfo(searchGlob: string) {
  return await invoke<ModInfo[]>('load_mods_info', { glob: searchGlob });
}

/**
 * Patch mods to hkx files.
 * @example
 * ```ts
 * const ids = *['C:/Nemesis_Engine/mod/aaa', 'C:/Nemesis_Engine/mod/bbb']
 * const output = 'C:/output/path';
 * await patch(output, ids);
 * ```
 * @throws Error
 */
export async function patch(output: string, ids: ModIds) {
  await invoke('patch', { output, ids });
}

/**
 * Cancel patch
 * @throws Error
 */
export async function cancelPatch() {
  await invoke('cancel_patch');
}
