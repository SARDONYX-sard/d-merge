import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { readTextFile } from '@tauri-apps/plugin-fs';
import { useEffect } from 'react';

import { usePatchContext } from '@/components/organisms/PatchContainer/PatchProvider';
import { NOTIFY } from '@/lib/notify';
import { STORAGE } from '@/lib/storage';
import { BACKUP } from '@/services/api/backup';

export const useBackup = () => {
  const { autoDetectEnabled, modInfoDir } = usePatchContext();
  const settingsPath = `${modInfoDir}/.d_merge/settings.json` as const;

  useEffect(() => {
    // Import backup settings
    const doImport = async () => {
      if (!(isTauri() && autoDetectEnabled) || modInfoDir === '') {
        return;
      }

      const key = 'registeredAutoBackupImporter';
      const once = sessionStorage.getItem(key) !== 'true';
      if (!once) {
        return;
      }
      sessionStorage.setItem(key, 'true');

      // biome-ignore lint/suspicious/noConsole: <explanation>
      console.log(`Backup read once from ${settingsPath}`);

      try {
        const settings = await readTextFile(settingsPath);
        const newSettings = await BACKUP.importRaw(settings);
        if (newSettings) {
          STORAGE.setAll(newSettings);
        }
      } catch (_) {
        // biome-ignore lint/suspicious/noConsole: <explanation>
        console.info('Try to import backup. But not found yet.');
      }
    };

    // Register export on close
    const registerCloseListener = async () => {
      if (!(isTauri() && autoDetectEnabled) || modInfoDir === '') {
        return;
      }

      const key = 'registeredAutoBackupExporter';
      const once = sessionStorage.getItem(key) !== 'true';
      if (!once) {
        return;
      }
      sessionStorage.setItem(key, 'true');
      // biome-ignore lint/suspicious/noConsole: <explanation>
      console.log('Export settings on window close once');

      await listen('tauri://close-requested', async () => {
        await NOTIFY.asyncTry(async () => {
          await BACKUP.exportRaw(settingsPath, STORAGE.getAll());
        });
      });
    };

    // Run both in parallel
    doImport();
    registerCloseListener();
  }, [autoDetectEnabled, modInfoDir, settingsPath]);
};
