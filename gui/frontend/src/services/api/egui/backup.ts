import { z } from 'zod';
import { OBJECT } from '@/lib/object-utils';
import type { Cache } from '@/lib/storage';
import { logLevelSchema } from '@/services/api/log';

export type ModItem = z.infer<typeof ModItemSchema>;
export const ModItemSchema = z.object({
  enabled: z.boolean(),
  /**
   * - vfs: e.g. `aaaa`
   * - manual: e.g. `path/to/aaaaa`
   */
  id: z.string(),
  priority: z.number(),
});
export const ModListSchema = z.array(ModItemSchema);

const EguiSettingsSchema = z.object({
  mode: z.enum(['vfs', 'manual']).optional(),
  target_runtime: z.enum(['SE', 'LE', 'VR']).optional(),
  template_dir: z.string().optional(),
  output_dir: z.string().optional(),
  auto_remove_meshes: z.boolean().optional(),
  enable_debug_output: z.boolean().optional(),
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
  vfs_mod_list: ModListSchema.optional(),
  skyrim_data_dir: z.string().optional(),
  mod_list: ModListSchema.optional(),
});

type EguiSettings = z.infer<typeof EguiSettingsSchema>;

/**
 * Attempting to parse as an egui setting.
 *
 * # Errors
 * If not an egui setting, null is returned.
 */
export function parseEguiSettings(egui_settings_string: string): EguiSettings | null {
  let parsed: any;
  try {
    parsed = JSON.parse(egui_settings_string);
  } catch {
    return null;
  }

  const result = EguiSettingsSchema.safeParse(parsed);
  const validData = result.success ? result.data : {};

  if (!result.success) {
    return null;
  }

  return validData;
}

/**
 * Convert egui settings to tauri settings.
 */
export function convertEguiSettings(settings: EguiSettings): Cache {
  const output = {
    'patch-is-vfs-mode': JSON.stringify(settings.mode === 'vfs'),
    'patch-options': JSON.stringify({
      hackOptions: { castRagdollEvent: true },
      debug: {
        outputPatchJson: settings.enable_debug_output,
        outputMergedJson: settings.enable_debug_output,
        outputMergedXml: settings.enable_debug_output,
      },
      outputTarget: settings.target_runtime === 'SE' ? 'SkyrimSE' : 'SkyrimLE',
      autoRemoveMeshes: settings.auto_remove_meshes,
    }),
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
