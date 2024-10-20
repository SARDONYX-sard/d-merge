import { save } from '@tauri-apps/plugin-dialog';

import { CACHE_KEYS, type Cache, STORAGE } from '@/lib/storage';

import { readFile, writeFile } from './fs';

const IMPORT_PATH_CACHE_KEY = 'import-backup-path';
const EXPORT_PATH_CACHE_KEY = 'export-settings-path';
const SETTINGS_FILE_NAME = 'settings';

export const BACKUP = {
  /** @throws Error */
  async import() {
    const settings = await readFile(IMPORT_PATH_CACHE_KEY, SETTINGS_FILE_NAME);
    if (settings) {
      const obj = JSON.parse(settings);

      // Validate
      for (const key of Object.keys(obj)) {
        if (key === IMPORT_PATH_CACHE_KEY) {
          continue; // The import path does not need to be overwritten.
        }

        // Remove invalid settings values
        const isInvalidKey = !CACHE_KEYS.some((cacheKey) => cacheKey === key);
        if (isInvalidKey) {
          delete obj[key];
        }
      }

      return obj as Cache;
    }
  },

  /** @throws Json parse Error */
  async export(settings: Cache) {
    const cachedPath = STORAGE.get(EXPORT_PATH_CACHE_KEY);
    const path = await save({
      defaultPath: cachedPath ?? undefined,
      filters: [{ name: SETTINGS_FILE_NAME, extensions: ['json'] }],
    });

    if (typeof path === 'string') {
      await writeFile(path, `${JSON.stringify(settings, null, 2)}\n`);
      return path;
    }
    return null;
  },
} as const;
