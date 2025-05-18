import { type RefObject, useEffect } from 'react';

import type { GridApi, GridInitialState } from '@mui/x-data-grid';

export function useGridStatePersistence(apiRef: RefObject<GridApi | null>, storageKey: string) {
  // 初期化時に保存された状態を復元
  useEffect(() => {
    const saved = localStorage.getItem(storageKey);
    if (saved) {
      try {
        const state: GridInitialState = JSON.parse(saved);
        apiRef.current?.restoreState(state);
      } catch (e) {
        // biome-ignore lint/suspicious/noConsole: <explanation>
        console.warn('Failed to restore grid state:', e);
      }
    }
  }, [apiRef, storageKey]);

  useEffect(() => {
    const saveState = () => {
      try {
        const state = apiRef.current?.exportState();
        localStorage.setItem(storageKey, JSON.stringify(state));
      } catch (e) {
        // biome-ignore lint/suspicious/noConsole: <explanation>
        console.warn('Failed to export grid state:', e);
      }
    };

    const unsubscribe = apiRef.current?.subscribeEvent('stateChange', saveState);
    return () => unsubscribe?.();
  }, [apiRef, storageKey]);
}
