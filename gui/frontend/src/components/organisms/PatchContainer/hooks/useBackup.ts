import { isTauri } from '@tauri-apps/api/core';
import { TauriEvent } from '@tauri-apps/api/event';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { readTextFile } from '@tauri-apps/plugin-fs';
import { useEffect } from 'react';

import { usePatchContext } from '@/components/organisms/PatchContainer/PatchProvider';
import { NOTIFY } from '@/lib/notify';
import { OBJECT } from '@/lib/object-utils';
import { STORAGE } from '@/lib/storage';
import { BACKUP } from '@/services/api/backup';

/**
 * @param enabled Enable auto backup read/write.
 */
export const useBackup = () => {
  const { autoDetectEnabled, modInfoDir } = usePatchContext();
  const settingsPath = `${modInfoDir}/.d_merge/settings.json`;

  useEffect(() => {
    if (typeof window !== 'undefined' && isTauri()) {
      return;
    }

    if (!autoDetectEnabled || modInfoDir === '') {
      return;
    }

    let unlistenFn: (() => void) | undefined;

    const listenTauriCreate = async () => {
      unlistenFn = await getCurrentWebview().listen(TauriEvent.WINDOW_CREATED, async () => {
        // biome-ignore lint/complexity/noExcessiveCognitiveComplexity: <explanation>
        const importTask = async () => {
          const settings = await readTextFile(settingsPath);
          const newSettings = await BACKUP.importRaw(settings);
          if (newSettings) {
            for (const [key, value] of OBJECT.entries(newSettings)) {
              if (value) {
                STORAGE.set(key, value);
              }
            }
          }
        };

        await NOTIFY.asyncTry(importTask);
      });
    };

    listenTauriCreate();

    return () => unlistenFn?.();
  }, [autoDetectEnabled, modInfoDir, settingsPath]);

  useEffect(() => {
    if (typeof window !== 'undefined' && isTauri()) {
      return;
    }

    if (!autoDetectEnabled || modInfoDir === '') {
      return;
    }

    let unlistenFn: (() => void) | undefined;

    const listenTauriClose = async () => {
      unlistenFn = await getCurrentWebview().listen(TauriEvent.WINDOW_CLOSE_REQUESTED, async () => {
        await NOTIFY.asyncTry(async () => {
          await BACKUP.exportRaw(settingsPath, STORAGE.getAll());
        });
      });
    };

    listenTauriClose();

    return () => unlistenFn?.();
  }, [autoDetectEnabled, modInfoDir, settingsPath]);
};
