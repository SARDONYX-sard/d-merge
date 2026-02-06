import { useCallback, useMemo, useState } from 'react';
import { useStorageState } from '@/components/hooks/useStorageState';
import { PRIVATE_CACHE_OBJ } from '@/lib/storage/cacheKeys';
import { type ActiveModItem, ModListSchema } from '@/services/api/egui/backup';
import type { ModInfo } from '@/services/api/patch';

/**
 * Synchronize and sort ModInfo list with cached ModList.
 * This hook guarantees that:
 * - The active mod list is stored in both React state and schemaStorage
 * - Returned modInfoList is always sorted by activeModList
 */
export const useModInfoState = (isVfsMode: boolean) => {
  const cacheKey = isVfsMode ? PRIVATE_CACHE_OBJ.patchVfsModList : PRIVATE_CACHE_OBJ.patchModList;

  // Active mod list (truth source, synced with schemaStorage)
  const [activeModList, setActiveModList] = useStorageState(cacheKey, ModListSchema.catch([]));

  // Raw mod info list (fetched from API)
  const [fetchedInfoList, setFetchedModInfoList] = useState<ModInfo[]>([]);

  /**
   * Setter for local edits:
   * - Updates React state AND schemaStorage
   * - Keeps activeModList in sync
   */
  const setModInfoList = useCallback(
    (updater: React.SetStateAction<ModInfo[]>) => {
      setFetchedModInfoList((prev) => {
        const next = typeof updater === 'function' ? updater(prev) : updater;
        const nextList = toActiveList(next);
        setActiveModList(nextList);

        return next;
      });
    },
    [cacheKey],
  );

  const modInfoList = useMemo(
    () => applyActiveModList(fetchedInfoList, activeModList),
    [fetchedInfoList, activeModList],
  );

  return {
    /**
     * Sorted mod info list:
     * - Matches enabled/priority with activeModList
     * - Sorted by priority
     */
    modInfoList,
    /** for initial fetch, does NOT update activeModList */
    setFetchedModInfoList,
    /** for local edits, syncs with cache + activeModList */
    setModInfoList,
  } as const;
};

/**
 * Convert ModInfo[] into a ModItem[] for storage.
 */
const toActiveList = (mods: ModInfo[]): ActiveModItem[] =>
  mods.map(({ id, enabled, priority, mod_type }) => ({
    id,
    enabled,
    priority,
    mod_type,
  }));

/**
 * Sorts modInfoList according to activeModList priorities,
 * and synchronizes `enabled` / `priority` values.
 */
const applyActiveModList = (fetchedModInfoList: ModInfo[], activeModList: ActiveModItem[]): ModInfo[] => {
  if (!activeModList || activeModList.length === 0) return fetchedModInfoList;

  const activeModMap = new Map(activeModList.map((m) => [m.id, m]));

  const result = fetchedModInfoList.map((modInfo) => {
    const ref = activeModMap.get(modInfo.id);
    return ref ? { ...modInfo, enabled: ref.enabled, priority: ref.priority } : modInfo;
  });

  result.sort((a, b) => a.priority - b.priority);

  return result;
};
