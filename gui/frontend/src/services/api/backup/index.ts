import { NOTIFY } from '@/lib/notify';
import { CACHE_KEYS, type Cache, STORAGE } from '@/lib/storage';
import { PRIVATE_CACHE_OBJ } from '@/lib/storage/cacheKeys';
import { stringToJsonSchema } from '@/lib/zod/json-validation';
import { save } from '../dialog';
import { readFileWithDialog, writeFile } from '../fs';
import { convertEguiSettings, convertToEguiSettings, parseEguiSettings } from './egui_support';

const SETTINGS_FILE_NAME = 'settings';

export const BACKUP = {
  /** @throws Error | JsonParseError */
  async import(parserMode: 'egui' | 'tauri'): Promise<Cache | undefined> {
    const settings = await readFileWithDialog(PRIVATE_CACHE_OBJ.importSettingsPath, SETTINGS_FILE_NAME);
    if (settings === null) {
      return undefined;
    }

    if (parserMode === 'tauri') {
      return this.fromStr(settings);
    } else if (parserMode === 'egui') {
      return convertEguiSettings(parseEguiSettings(settings) ?? {});
    }

    const eguiSettings = parseEguiSettings(settings);
    if (eguiSettings) {
      return convertEguiSettings(eguiSettings);
    } else {
      return this.fromStr(settings);
    }
  },

  /** @throws Error | JsonParseError */
  fromStr(settings: string | null): Cache | undefined {
    if (settings) {
      const json = stringToJsonSchema.parse(settings);

      // Validate
      if (typeof json === 'object' && !Array.isArray(json) && json !== null) {
        const invalidKeys: string[] = [];
        for (const key of Object.keys(json)) {
          // NOTE: The import path selected immediately before should remain selectable the next time, so do not overwrite it.
          if (key === PRIVATE_CACHE_OBJ.importSettingsPath) {
            continue;
          }

          const isInvalidKey = !CACHE_KEYS.some((cacheKey) => cacheKey === key);
          if (isInvalidKey) {
            invalidKeys.push(key);
            delete json[key];
          }
        }

        if (invalidKeys.length > 0) {
          NOTIFY.warn(`The following keys are not recognized and have been ignored: ${invalidKeys.join(', ')}`);
        }

        return json;
      }
    }
  },

  /** @throws SaveError */
  async export(settings: Cache, parserMode: 'egui' | 'tauri' = 'tauri'): Promise<string | null> {
    const cachedPath = STORAGE.get(PRIVATE_CACHE_OBJ.exportSettingsPath);
    const path = await save({
      defaultPath: cachedPath ?? 'settings.json',
      filters: [{ name: SETTINGS_FILE_NAME, extensions: ['json'] }],
    });

    if (typeof path === 'string') {
      await this.exportRaw(path, settings, parserMode);
      return path;
    }
    return null;
  },

  /**
   * Write to path.
   * - `path`: e.g. `<output_dir>/d_merge_settings.json`
   * @throws
   * - If `mode` is 'egui', throws if settings cannot be converted to egui format.
   * - SaveError
   */
  async exportRaw(path: string, settings: Cache, mode: 'tauri' | 'egui' = 'tauri') {
    if (mode === 'egui') {
      const eguiSettings = convertToEguiSettings(settings);
      await writeFile(path, `${JSON.stringify(eguiSettings, null, 2)}\n`);
    } else {
      await writeFile(path, `${JSON.stringify(settings, null, 2)}\n`);
    }
  },
} as const;
