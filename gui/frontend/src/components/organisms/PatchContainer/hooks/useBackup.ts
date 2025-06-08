import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { readTextFile } from '@tauri-apps/plugin-fs';
import { useEffect } from 'react';

import { usePatchContext } from '@/components/organisms/PatchContainer/PatchProvider';
import { NOTIFY } from '@/lib/notify';
import { STORAGE } from '@/lib/storage';
import { BACKUP } from '@/services/api/backup';

/**
 * @param enabled Enable auto backup read/write.
 */
export const useBackup = () => {
  const { autoDetectEnabled, modInfoDir } = usePatchContext();
  const settingsPath = `${modInfoDir}/.d_merge/settings.json` as const;

  useEffect(() => {
    let unlistenFn: (() => void) | undefined;

    const listenTauriCreate = async () => {
      unlistenFn = await getCurrentWebview().listen('tauri://webview-created', async () => {
        if (typeof window !== 'undefined' && isTauri()) {
          return;
        }
        if (!autoDetectEnabled || modInfoDir === '') {
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
      });
    };

    listenTauriCreate();

    return () => unlistenFn?.();
  }, [autoDetectEnabled, modInfoDir, settingsPath]);

  useEffect(() => {
    let unlistenFn: (() => void) | undefined;

    const listenTauriClose = async () => {
      unlistenFn = await getCurrentWebview().listen('tauri://close-requested', async () => {
        if (typeof window !== 'undefined' && isTauri()) {
          return;
        }
        if (!autoDetectEnabled || modInfoDir === '') {
          return;
        }

        await NOTIFY.asyncTry(async () => {
          await BACKUP.exportRaw(settingsPath, STORAGE.getAll());
        });
      });
    };

    listenTauriClose();

    return () => unlistenFn?.();
  }, [autoDetectEnabled, modInfoDir, settingsPath]);
};
