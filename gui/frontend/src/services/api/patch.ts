import { invoke } from '@tauri-apps/api/core';
import { z } from 'zod';

/**
 * Get skyrim directory
 * @throws Error
 */
export async function getSkyrimDir(runtime: PatchOptions['outputTarget']) {
  switch (runtime) {
    case 'SkyrimLE':
      return await invoke<string>('get_skyrim_data_dir', { runtime: 'LE' });
    default:
      return await invoke<string>('get_skyrim_data_dir', { runtime: 'SE' });
  }
}

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

/** must be same as `GuiOption` serde */
export type PatchOptions = {
  hackOptions: {
    castRagdollEvent: boolean;
  };
  debug: {
    outputPatchJson: boolean;
    outputMergedJson: boolean;
    outputMergedXml: boolean;
  };
  outputTarget: 'SkyrimSE' | 'SkyrimLE';
  /** Delete the meshes in the output destination each time the patch is run. */
  autoRemoveMeshes: boolean;
};

export const patchOptionsSchema = z
  .object({
    hackOptions: z.object({
      castRagdollEvent: z.boolean(),
    }),
    debug: z.object({
      outputPatchJson: z.boolean(),
      outputMergedJson: z.boolean(),
      outputMergedXml: z.boolean(),
    }),
    outputTarget: z.union([z.literal('SkyrimSE'), z.literal('SkyrimLE')]),
    autoRemoveMeshes: z.boolean(),
  })
  .catch({
    hackOptions: {
      castRagdollEvent: true,
    },
    debug: {
      outputMergedJson: true,
      outputPatchJson: true,
      outputMergedXml: false,
    },
    outputTarget: 'SkyrimSE',
    autoRemoveMeshes: true,
  } as const satisfies PatchOptions);

/**
 * Patch mods to hkx files.
 * @example
 * ```ts
 * const ids = *['C:/Nemesis_Engine/mod/aaa', 'C:/Nemesis_Engine/mod/bbb']
 * const output = 'C:/output/path';
 * const patchOptions = { ... }; // See `PatchOptions`
 * await patch(output, ids);
 * ```
 * @throws Error
 */
export async function patch(output: string, ids: ModIds, options: PatchOptions) {
  await invoke('patch', { output, ids, options });
}

/**
 * Cancel patch
 * @throws Error
 */
export async function cancelPatch() {
  await invoke('cancel_patch');
}

/**
 * set vfs mode flag.(If enabled, close window manually.)
 * @throws Error
 */
export async function setVfsMode(value: boolean) {
  await invoke('set_vfs_mode', { value });
}
