import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { readTextFile } from '@tauri-apps/plugin-fs';
import { useEffect } from 'react';

import { usePatchContext } from '@/components/organisms/PatchContainer/PatchProvider';
import { STORAGE } from '@/lib/storage';
import { BACKUP } from '@/services/api/backup';

export const useBackup = () => {
  const { autoDetectEnabled, modInfoDir } = usePatchContext();

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

      const settingsPath = `${modInfoDir}/.d_merge/settings.json` as const;
      // biome-ignore lint/suspicious/noConsole: <explanation>
      console.log(`Backup read once from ${settingsPath}`);

      try {
        const settings = await readTextFile(settingsPath);
        const newSettings = await BACKUP.importRaw(settings);
        if (newSettings) {
          newSettings['last-path'] = undefined;
          // TODO: It is better to use setState punishment for performance, but reload because there are too many providers.
          STORAGE.setAll(newSettings);
          window.location.reload(); // To enable
        }
      } catch (e) {
        // biome-ignore lint/suspicious/noConsole: <explanation>
        console.warn(`Import backup error ${e}.`);
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

      // NOTE
      // - Get close event in backend but prevent execution. After saving the current data to a file, close the window with destination.
      // - The listener exists only once globally. (To avoid calling unlisten by return)
      await listen('tauri://close-requested', async () => {
        const settings = STORAGE.getAll();
        const latestModInfoDir = settings['patch-input'];
        const settingsPath = `${latestModInfoDir ?? modInfoDir}/.d_merge/settings.json` as const;
        await BACKUP.exportRaw(settingsPath, settings);
        await getCurrentWindow().destroy();
      });
    };

    // Run both in parallel
    doImport();
    registerCloseListener();
  }, [autoDetectEnabled, modInfoDir]);
};
