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
    const isFirstVisit = sessionStorage.getItem('visitedHome') !== 'true';

    if (!isFirstVisit) {
      return;
    }

    sessionStorage.setItem('visitedHome', 'true');

    // Import backup settings
    const doImport = async () => {
      // biome-ignore lint/suspicious/noConsole: <explanation>
      console.log('Backup read once');

      if (!(isTauri() && autoDetectEnabled) || modInfoDir === '') {
        return;
      }

      const importTask = async () => {
        const settings = await readTextFile(settingsPath);
        const newSettings = await BACKUP.importRaw(settings);
        if (newSettings) {
          STORAGE.setAll(newSettings);
        }
      };

      await NOTIFY.asyncTry(importTask);
    };

    // Register export on close
    const registerCloseListener = async () => {
      await listen('tauri://close-requested', async () => {
        // biome-ignore lint/suspicious/noConsole: <explanation>
        console.log('Export settings on window close');

        if (!(isTauri() && autoDetectEnabled) || modInfoDir === '') {
          return;
        }

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
