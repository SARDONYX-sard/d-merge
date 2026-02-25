import type { GridApi, GridInitialState } from '@mui/x-data-grid';
import { type RefObject, useEffect } from 'react';

export function useGridStatePersistence(apiRef: RefObject<GridApi | null>, storageKey: string) {
  useEffect(() => {
    const saved = localStorage.getItem(storageKey);
    if (saved) {
      try {
        const state: GridInitialState = JSON.parse(saved);
        apiRef.current?.restoreState(state);
      } catch (e) {
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
        console.warn('Failed to export grid state:', e);
      }
    };

    const unsubscribe = apiRef.current?.subscribeEvent('stateChange', saveState);
    return () => unsubscribe?.();
  }, [apiRef, storageKey]);
}
