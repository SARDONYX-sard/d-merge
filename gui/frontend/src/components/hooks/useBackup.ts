import { isTauri } from '@tauri-apps/api/core';
import { useEffect } from 'react';
import { usePatchContext } from '@/components/providers/PatchProvider';
import { NOTIFY } from '@/lib/notify';
import { STORAGE } from '@/lib/storage';
import { BACKUP } from '@/services/api/backup';
import { listen } from '@/services/api/event';
import { exists, readFile } from '@/services/api/fs';
import { destroyCurrentWindow } from '@/services/api/window';

export const useBackup = () => {
  useAutoImportBackup();
  useAutoExportBackup();
};

const useAutoImportBackup = () => {
  const { isVfsMode: autoDetectEnabled, skyrimDataDir: modInfoDir } = usePatchContext();
  const settingsPath = `${modInfoDir}/.d_merge/settings.json` as const;

  useEffect(() => {
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

      try {
        if (!(await exists(settingsPath))) {
          return;
        }

        NOTIFY.info(`Backups are being automatically loaded from ${settingsPath}...`);

        const newSettings = await BACKUP.fromStr(await readFile(settingsPath));
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
  }, [autoDetectEnabled, modInfoDir, settingsPath]);
};

const useAutoExportBackup = () => {
  const { isVfsMode, skyrimDataDir } = usePatchContext();
  const settingsPath = `${skyrimDataDir}/.d_merge/settings.json` as const;

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
            NOTIFY.info(`Backups are being automatically written to ${settingsPath}...`);
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
  }, [isVfsMode, skyrimDataDir, settingsPath]);
};
