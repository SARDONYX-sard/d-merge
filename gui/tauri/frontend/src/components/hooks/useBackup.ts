import { isTauri } from '@tauri-apps/api/core';
import { useEffect } from 'react';
import { usePatchContext } from '@/components/providers/PatchProvider';
import { NOTIFY } from '@/lib/notify';
import { STORAGE } from '@/lib/storage';
import { BACKUP } from '@/services/api/backup';
import { listen } from '@/services/api/event';
import { exists, readFile } from '@/services/api/fs';
import { LOG } from '@/services/api/log';
import { destroyCurrentWindow } from '@/services/api/window';

export const useBackup = () => {
  useAutoImportBackup();
  useAutoExportBackup();
};

const useAutoImportBackup = () => {
  const { skyrimDataDir: modInfoDir } = usePatchContext();
  const settingsPath = `./.d_merge/tauri_settings.json` as const;

  useEffect(() => {
    const doImport = async () => {
      if (!isTauri() || modInfoDir === '') {
        return;
      }

      const key = 'registeredAutoBackupImporter';
      const once = sessionStorage.getItem(key) !== 'true';
      if (!once) {
        return;
      }
      sessionStorage.setItem(key, 'true');

      try {
        if (!(await exists(settingsPath))) {
          LOG.log('info', `No backup found at ${settingsPath}. Skipping auto import.`);
          return;
        }
        LOG.log('info', `Backups are being automatically loaded from ${settingsPath}...`);

        const newSettings = BACKUP.fromStr(await readFile(settingsPath));
        if (newSettings) {
          newSettings['last-path'] = '/';
          STORAGE.setAll(newSettings);
          window.location.reload();
        }
      } catch (e) {
        NOTIFY.warn(`Import backup error ${e}.`);
      }
    };

    doImport();
  }, [modInfoDir, settingsPath]);
};

const useAutoExportBackup = () => {
  const { skyrimDataDir } = usePatchContext();
  const settingsPath = './.d_merge/tauri_settings.json' as const;

  useEffect(() => {
    let unlisten: (() => void) | null;

    /**
     * Register close listener for auto backup.
     */
    const registerCloseListener = async () => {
      try {
        if (!isTauri()) {
          return;
        }

        const unlistenFn = await listen('tauri://close-requested', async () => {
          try {
            LOG.log('info', `Backups are being automatically written to ${settingsPath}...`);
            await BACKUP.exportRaw(settingsPath, STORAGE.getAll());
          } catch (e) {
            NOTIFY.error(`${e}`);
          } finally {
            await destroyCurrentWindow();
          }
        });

        unlisten = unlistenFn;
      } catch (e) {
        NOTIFY.error(`Failed to register close listener for auto backup. Error: ${e}`);
      }
    };

    registerCloseListener();

    return () => {
      unlisten?.();
    };
  }, [skyrimDataDir, settingsPath]);
};
