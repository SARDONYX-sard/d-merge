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
    };

    doImport().catch((e) => NOTIFY.warn(`Import backup error ${e}.`));
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
      if (!isTauri()) {
        return;
      }

      const eventHandler = async () => {
        try {
          LOG.log('info', `Backups are being automatically written to ${settingsPath}...`);
          await BACKUP.exportRaw(settingsPath, STORAGE.getAll());
        } catch (e) {
          if (e instanceof Error) {
            NOTIFY.error(e.message);
          } else if (typeof e === 'string') {
            NOTIFY.error(e);
          }
        } finally {
          await destroyCurrentWindow();
        }
      };

      unlisten = await listen('tauri://close-requested', eventHandler);
    };

    registerCloseListener().catch((e) =>
      NOTIFY.error(`Failed to register close listener for auto backup. Error: ${e}`),
    );

    return () => {
      unlisten?.();
    };
  }, [skyrimDataDir, settingsPath]);
};
