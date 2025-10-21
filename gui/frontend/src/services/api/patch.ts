import { invoke, isTauri } from '@tauri-apps/api/core';
import { z } from 'zod';
import { electronApi, isElectron } from './electron';

/**
 * Get skyrim directory
 * @throws Error
 */
export async function getSkyrimDir(runtime: PatchOptions['outputTarget']) {
  if (isTauri()) {
    switch (runtime) {
      case 'SkyrimLE':
        return await invoke<string>('get_skyrim_data_dir', { runtime: 'LE' });
      default:
        return await invoke<string>('get_skyrim_data_dir', { runtime: 'SE' });
    }
  }

  if (isElectron()) {
    return await electronApi.getSkyrimDir(runtime);
  }

  throw new Error('Unsupported platform: Neither Tauri nor Electron');
}

export type FetchedModInfo = {
  /**
   * Mod-specific dir name.
   * - Nemesis/FNIS(vfs): e.g. `aaaa`
   * - Nemesis(manual): e.g. `<skyrim data dir>/Nemesis_Engine/mod/aaaa`
   * - FNIS(manual): e.g. `<skyrim data dir>/meshes/actors/character/animations/aaaa`
   */
  id: string;
  name: string;
  author: string;
  site: string;
  auto: string;
  mod_type: 'nemesis' | 'fnis';
};

export type ModInfo = FetchedModInfo & {
  enabled: boolean;
  priority: number;
};

export type PatchMaps = {
  /** Nemesis patch path
   * - key: path until mod_code (e.g.: `<skyrim_data_dir>/meshes/Nemesis_Engine/mod/slide`)
   * - value: priority
   */
  nemesis_entries: Record<string, number>;

  /** FNIS patch path
   * - key: path until namespace (e.g.: `<skyrim_data_dir>/meshes/actors/character/animations/FNISFlyer`)
   * - value: priority
   */
  fnis_entries: Record<string, number>;
};

/**
 * Load mods `info.ini`
 * @throws Error
 */
export async function loadModsInfo(skyrimDataDir: string) {
  if (isTauri()) {
    return await invoke<FetchedModInfo[]>('load_mods_info', { glob: skyrimDataDir });
  }

  if (isElectron()) {
    return await electronApi.loadModsInfo(skyrimDataDir);
  }

  throw new Error('Unsupported platform: Neither Tauri nor Electron');
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
  /** Report progress status +2s */
  useProgressReporter: boolean;
  /** Skyrim data directories glob (required **only when using FNIS**).
   *
   * This must include all directories containing `animations/<namespace>`, otherwise FNIS
   * entries will not be detected and the process will fail.
   **/
  skyrimDataDirGlob?: string;
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
    useProgressReporter: z.boolean(),
    skyrimDataDirGlob: z.optional(z.string()),
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
    useProgressReporter: true,
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
export async function patch(output: string, patches: PatchMaps, options: PatchOptions) {
  if (isTauri()) {
    return await invoke('patch', { output, patches, options });
  }

  if (isElectron()) {
    return await electronApi.patch(output, patches, options);
  }

  throw new Error('Unsupported platform: Neither Tauri nor Electron');
}

/**
 * Cancel patch
 * @throws Error
 */
export async function cancelPatch() {
  if (isTauri()) {
    return await invoke('cancel_patch');
  }

  if (isElectron()) {
    return await electronApi.cancelPatch();
  }

  throw new Error('Unsupported platform: Neither Tauri nor Electron');
}

/**
 * If enabled, close window manually.
 * @throws Error
 */
export async function preventAutoCloseWindow(isEnabled: boolean) {
  if (isTauri()) {
    return await invoke('set_vfs_mode', { value: isEnabled });
  }

  if (isElectron()) {
    return await electronApi.setVfsMode(isEnabled);
  }

  throw new Error('Unsupported platform: Neither Tauri nor Electron');
}
