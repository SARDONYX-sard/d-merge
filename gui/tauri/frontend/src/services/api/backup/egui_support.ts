import { z } from 'zod';
import { OBJECT } from '@/lib/object-utils';
import { PRIVATE_CACHE_OBJ, PUB_CACHE_OBJ } from '@/lib/storage/cacheKeys';
import { logLevelSchema } from '@/services/api/log';
import { type PatchOptions, patchOptionsSchema } from '@/services/api/patch';

import type { Cache } from '@/lib/storage';

// NOTE: Keep egui settings ordering
const EguiModItemSchema = z.object({
  enabled: z.boolean(),
  /**
   * Mod-specific dir name.
   * - Nemesis/FNIS(vfs): e.g. `aaaa`
   * - Nemesis(manual): e.g. `<skyrim data dir>/Nemesis_Engine/mod/aaaa`
   * - FNIS(manual): e.g. `<skyrim data dir>/meshes/actors/character/animations/aaaa`
   */
  id: z.string(),
  name: z.string(),
  site: z.string(),
  priority: z.number(),
  mod_type: z.enum(['nemesis', 'fnis']),
});
const EguiModListSchema = z.array(EguiModItemSchema);

const EguiSettingsSchema = z.looseObject({
  mode: z.enum(['vfs', 'manual']).optional(),
  target_runtime: z.enum(['SE', 'LE', 'VR']).optional(),
  template_dir: z.string().optional(),
  output_dir: z.string().optional(),
  auto_remove_meshes: z.boolean().optional(),
  enable_debug_output: z.boolean().optional(),
  generate_fnis_esp: z.boolean().optional(),
  log_level: logLevelSchema.optional(),
  filter_text: z.string().optional(),
  font_path: z.string().nullable().optional(),
  sort_asc: z.boolean().optional(),
  sort_column: z.string().optional(),
  transparent: z.boolean().optional(),
  window_height: z.number().optional(),
  window_maximized: z.boolean().optional(),
  window_pos_x: z.number().optional(),
  window_pos_y: z.number().optional(),
  window_width: z.number().optional(),
  vfs_skyrim_data_dir: z.string().optional(),
  vfs_mod_list: EguiModListSchema.optional(),
  skyrim_data_dir: z.string().optional(),
  mod_list: EguiModListSchema.optional(),
});

type EguiSettings = z.infer<typeof EguiSettingsSchema>;

/**
 * Attempting to parse as an egui setting.
 *
 * # Errors
 * If not an egui setting, null is returned.
 */
export function parseEguiSettings(egui_settings_string: string): EguiSettings | null {
  return EguiSettingsSchema.parse(JSON.parse(egui_settings_string));
}

/**
 * Convert egui settings to tauri settings.
 */
export function convertEguiSettings(settings: EguiSettings): Cache {
  console.log(JSON.stringify(settings, null, 2));

  const output = {
    'patch-is-vfs-mode': JSON.stringify(settings.mode === 'vfs'),
    'patch-options': JSON.stringify({
      hackOptions: {
        castRagdollEvent: true,
        boneWeightOutsideHkparam: true,
      },
      debug: {
        outputPatchJson: settings.enable_debug_output ?? false,
        outputMergedJson: settings.enable_debug_output ?? false,
        outputMergedXml: settings.enable_debug_output ?? false,
      },
      outputTarget: settings.target_runtime === 'SE' ? 'SkyrimSE' : 'SkyrimLE',
      autoRemoveMeshes: settings.auto_remove_meshes ?? false,
      generateFnisEsp: settings.generate_fnis_esp ?? false,
      useProgressReporter: true,
    } as const satisfies PatchOptions),
    'patch-output': settings.output_dir ? JSON.stringify(settings.output_dir) : undefined,
    'log-level': settings.log_level ? JSON.stringify(settings.log_level) : undefined,

    'patch-vfs-skyrim-data-dir': settings.vfs_skyrim_data_dir
      ? JSON.stringify(settings.vfs_skyrim_data_dir)
      : undefined,
    'patch-vfs-mod-list': settings.vfs_mod_list?.length ? JSON.stringify(settings.vfs_mod_list) : undefined,

    'patch-skyrim-data-dir': settings.skyrim_data_dir ? JSON.stringify(settings.skyrim_data_dir) : undefined,
    'patch-mod-list': settings.mod_list?.length ? JSON.stringify(settings.mod_list) : undefined,
  } as const satisfies Cache;

  // Remove null/undefined
  OBJECT.keys(output).forEach((k) => {
    if (output[k] === undefined || output[k] === null) {
      delete output[k];
    }
  });

  return output;
}

export const TAURI_KEYS_USED_BY_EGUI = [
  PUB_CACHE_OBJ.isVfsMode,
  PUB_CACHE_OBJ.patchOptions,
  PUB_CACHE_OBJ.patchOptions,
  PUB_CACHE_OBJ.logLevel,
  PRIVATE_CACHE_OBJ.patchVfsSkyrimDataDir,
  PRIVATE_CACHE_OBJ.patchVfsModList,
  PRIVATE_CACHE_OBJ.patchSkyrimDataDir,
  PRIVATE_CACHE_OBJ.patchModList,
] as const satisfies readonly (keyof Cache)[];

const jsonString = <T extends z.ZodTypeAny>(schema: T) =>
  z.preprocess((val) => {
    if (typeof val !== 'string') return val;
    try {
      return JSON.parse(val);
    } catch {
      return val;
    }
  }, schema);

const TauriCacheForEguiSchema = z
  .object({
    [PUB_CACHE_OBJ.patchOptions]: jsonString(patchOptionsSchema).optional(),

    [PRIVATE_CACHE_OBJ.patchVfsModList]: jsonString(EguiModListSchema).optional(),

    [PRIVATE_CACHE_OBJ.patchModList]: jsonString(EguiModListSchema).optional(),

    [PUB_CACHE_OBJ.isVfsMode]: jsonString(z.boolean()).optional(),

    [PRIVATE_CACHE_OBJ.patchOutput]: jsonString(z.string()).optional(),

    [PUB_CACHE_OBJ.logLevel]: jsonString(logLevelSchema).optional(),

    [PRIVATE_CACHE_OBJ.patchVfsSkyrimDataDir]: jsonString(z.string()).optional(),

    [PRIVATE_CACHE_OBJ.patchSkyrimDataDir]: jsonString(z.string()).optional(),
  })
  .passthrough();

const TauriToEguiSchema = TauriCacheForEguiSchema.transform((cache): EguiSettings => {
  const patchOptions = cache[PUB_CACHE_OBJ.patchOptions];

  return {
    mode: cache[PUB_CACHE_OBJ.isVfsMode] ? 'vfs' : 'manual',

    target_runtime: patchOptions
      ? ((runtime: PatchOptions['outputTarget']) => {
          switch (runtime) {
            case 'SkyrimSE':
              return 'SE';
            case 'SkyrimLE':
              return 'LE';
          }
        })(patchOptions.outputTarget)
      : undefined,

    auto_remove_meshes: patchOptions?.autoRemoveMeshes,
    enable_debug_output:
      patchOptions?.debug.outputMergedJson ||
      patchOptions?.debug.outputMergedXml ||
      patchOptions?.debug.outputPatchJson,
    generate_fnis_esp: patchOptions?.generateFnisEsp ?? false,

    output_dir: cache[PRIVATE_CACHE_OBJ.patchOutput],
    log_level: cache[PUB_CACHE_OBJ.logLevel],

    vfs_skyrim_data_dir: cache[PRIVATE_CACHE_OBJ.patchVfsSkyrimDataDir],
    vfs_mod_list: cache[PRIVATE_CACHE_OBJ.patchVfsModList],

    skyrim_data_dir: cache[PRIVATE_CACHE_OBJ.patchSkyrimDataDir],
    mod_list: cache[PRIVATE_CACHE_OBJ.patchModList],
  } as const satisfies EguiSettings;
});

/**
 * Convert tauri settings (Cache) into egui settings (EguiSettings).
 *
 * @throws
 * - If the string cannot be parsed as JSON, or if the resulting object does not match the expected schema, an error is thrown.
 */
export function convertToEguiSettings(cache: Cache): EguiSettings {
  return TauriToEguiSchema.parse(cache);
}
