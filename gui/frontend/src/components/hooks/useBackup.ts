import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { exists, readTextFile } from '@tauri-apps/plugin-fs';
import { useEffect } from 'react';

import { usePatchContext } from '@/components/providers/PatchProvider';
import { NOTIFY } from '@/lib/notify';
import { STORAGE } from '@/lib/storage';
import { BACKUP } from '@/services/api/backup';
import { setVfsMode } from '@/services/api/patch';

export const useBackup = () => {
  useAutoImportBackup();
  useAutoExportBackup();
};

const useAutoImportBackup = () => {
  const { autoDetectEnabled, modInfoDir } = usePatchContext();
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

      if (!(await exists(settingsPath))) {
        return;
      }

      NOTIFY.info(`Backups are being automatically loaded from ${settingsPath}...`);

      try {
        const newSettings = await BACKUP.fromStr(await readTextFile(settingsPath));
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
  const { autoDetectEnabled, modInfoDir } = usePatchContext();
  const settingsPath = `${modInfoDir}/.d_merge/settings.json` as const;

  useEffect(() => {
    if (!(isTauri() && autoDetectEnabled) || modInfoDir === '') {
      setVfsMode(false);
      return;
    }

    setVfsMode(true);

    let unlisten: (() => void) | undefined;

    const registerCloseListener = async () => {
      const unlistenFn = await listen('tauri://close-requested', async () => {
        try {
          NOTIFY.info(`Backups are being automatically written to ${settingsPath}...`);
          await BACKUP.exportRaw(settingsPath, STORAGE.getAll());
        } catch (e) {
          NOTIFY.error(`${e}`);
        } finally {
          await getCurrentWindow().destroy();
        }
      });

      unlisten = unlistenFn;
    };

    registerCloseListener();

    return () => {
      unlisten?.();
    };
  }, [autoDetectEnabled, modInfoDir, settingsPath]);
};
