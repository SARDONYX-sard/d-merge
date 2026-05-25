import { z } from 'zod';
import { ModListSchema, PatchOptions } from '../patch';
import { convertOutputTargetToRuntime, convertRuntimeToOutputTarget } from './egui_support';
import { PRIVATE_CACHE_OBJ, PUB_CACHE_OBJ } from '@/lib/storage/cacheKeys';

import type { Cache } from '@/lib/storage';

export const V2ConfigSchema = z.object({
  app_version: z.string().default('2.0.0'),

  behavior: z.object({
    mode: z.enum(['vfs', 'manual']),
    target_runtime: z.enum(['SE', 'LE', 'VR']),
    auto_run: z.boolean(),
    auto_remove_meshes: z.boolean(),
    enable_debug_output: z.boolean(),
    generate_fnis_esp: z.boolean(),
    template_dir: z.string(),
  }),

  ui: z.object({
    theme: z.enum(['dark', 'light', 'system']),
    transparent: z.boolean(),
    font_path: z.string().nullable(),
    i18n_path: z.string().nullable(),

    mod_list: z.object({
      filter_column: z.string(),
      sort_column: z.string(),
      sort_asc: z.boolean(),
    }),

    window: z.object({
      pos_x: z.number(),
      pos_y: z.number(),
      width: z.number(),
      height: z.number(),
      maximized: z.boolean(),
    }),
  }),

  log: z.object({
    dir_path: z.string(),
    level: z.string(),
  }),

  vfs: z.object({
    skyrim_data_dir: z.string(),
    output_dir: z.string(),
    mod_list: z.array(ModListSchema),
  }),
  manual: z.object({
    skyrim_data_dir: z.string(),
    output_dir: z.string(),
    mod_list: z.array(ModListSchema),
  }),
});

export type V2Config = z.infer<typeof V2ConfigSchema>;

export function v2ToCache(v2: V2Config): Cache {
  const cache: Cache = {
    // behavior
    [PUB_CACHE_OBJ.isVfsMode]: JSON.stringify(v2.behavior.mode === 'vfs'),
    [PUB_CACHE_OBJ.patchOptions]: JSON.stringify({
      outputTarget: convertRuntimeToOutputTarget(v2.behavior.target_runtime),
      autoRemoveMeshes: v2.behavior.auto_remove_meshes,
      generateFnisEsp: v2.behavior.generate_fnis_esp,
      debug: {
        outputPatchJson: v2.behavior.enable_debug_output,
        outputMergedJson: v2.behavior.enable_debug_output,
        outputMergedXml: v2.behavior.enable_debug_output,
      },
      hackOptions: {
        castRagdollEvent: true,
        boneWeightOutsideHkparam: true,
      },
      useProgressReporter: true,
    } satisfies PatchOptions),

    // ui
    [PRIVATE_CACHE_OBJ.patchOutput]: JSON.stringify(v2.vfs.output_dir),
    [PUB_CACHE_OBJ.logLevel]: JSON.stringify(v2.log.level),

    [PRIVATE_CACHE_OBJ.patchVfsSkyrimDataDir]: JSON.stringify(v2.vfs.skyrim_data_dir),
    [PRIVATE_CACHE_OBJ.patchVfsModList]: JSON.stringify(v2.vfs.mod_list),

    [PRIVATE_CACHE_OBJ.patchSkyrimDataDir]: JSON.stringify(v2.manual.skyrim_data_dir),
    [PRIVATE_CACHE_OBJ.patchModList]: JSON.stringify(v2.manual.mod_list),
  };

  return cache;
}

function parse<T>(v: unknown): T | undefined {
  if (v == null) return undefined;
  if (typeof v === 'string') {
    try {
      return JSON.parse(v);
    } catch {
      return v as T;
    }
  }
  return v as T;
}

export function cacheToV2(cache: Cache): V2Config {
  const patchOptions = parse<PatchOptions>(cache[PUB_CACHE_OBJ.patchOptions]);

  const isVfs = parse<boolean>(cache[PUB_CACHE_OBJ.isVfsMode]) ?? false;

  return {
    app_version: '1.8.0',

    behavior: {
      mode: isVfs ? 'vfs' : 'manual',
      target_runtime: patchOptions?.outputTarget ? convertOutputTargetToRuntime(patchOptions?.outputTarget) : 'SE',

      auto_run: false,

      auto_remove_meshes: patchOptions?.autoRemoveMeshes ?? false,
      enable_debug_output:
        patchOptions?.debug?.outputPatchJson ??
        patchOptions?.debug?.outputMergedJson ??
        patchOptions?.debug?.outputMergedXml ??
        false,

      generate_fnis_esp: patchOptions?.generateFnisEsp ?? false,

      template_dir: './assets/templates',
    },

    ui: {
      theme: 'dark',
      transparent: false,
      font_path: null,

      i18n_path: null,

      mod_list: {
        filter_column: 'name',
        sort_column: 'priority',
        sort_asc: true,
      },

      window: {
        pos_x: 0,
        pos_y: 0,
        width: 1280,
        height: 720,
        maximized: false,
      },
    },

    log: {
      dir_path: './.d_merge/logs',
      level: parse<string>(cache[PUB_CACHE_OBJ.logLevel]) ?? 'info',
    },

    vfs: {
      skyrim_data_dir: parse<string>(cache[PRIVATE_CACHE_OBJ.patchVfsSkyrimDataDir]) ?? '',
      output_dir: parse<string>(cache[PRIVATE_CACHE_OBJ.patchOutput]) ?? '',
      mod_list: parse<any[]>(cache[PRIVATE_CACHE_OBJ.patchVfsModList]) ?? [],
    },
    manual: {
      skyrim_data_dir: parse<string>(cache[PRIVATE_CACHE_OBJ.patchSkyrimDataDir]) ?? '',
      output_dir: parse<string>(cache[PRIVATE_CACHE_OBJ.patchOutput]) ?? '',
      mod_list: parse<any[]>(cache[PRIVATE_CACHE_OBJ.patchModList]) ?? [],
    },
  };
}
