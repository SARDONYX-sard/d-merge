import { useCallback, useMemo, useState } from 'react';
import { PRIVATE_CACHE_OBJ } from '@/lib/storage/cacheKeys';
import { schemaStorage } from '@/lib/storage/schemaStorage';
import { type ModItem, ModListSchema } from '@/services/api/egui/backup';
import type { ModInfo } from '@/services/api/patch';

/**
 * Convert ModInfo[] into a ModItem[] for storage.
 */
const toModList = (mods: ModInfo[]): ModItem[] =>
  mods.map((item) => ({
    id: item.id,
    enabled: item.enabled,
    priority: item.priority,
  }));

/**
 * Synchronize and sort ModInfo list with cached ModList.
 * This hook guarantees that:
 * - The active mod list is stored in both React state and schemaStorage
 * - Returned modInfoList is always sorted by activeModList
 */
export const useModInfoState = (isVfsMode: boolean) => {
  const cacheKey = isVfsMode ? PRIVATE_CACHE_OBJ.patchVfsModList : PRIVATE_CACHE_OBJ.patchModList;

  // Active mod list (truth source, synced with schemaStorage)
  const [activeModList, setActiveModList] = useState<ModItem[]>(() => schemaStorage.get(cacheKey, ModListSchema) ?? []);

  // Raw mod info list (fetched from API)
  const [modInfoListRaw, setModInfoListRaw] = useState<ModInfo[]>([]);

  /**
   * Setter for local edits:
   * - Updates React state AND schemaStorage
   * - Keeps activeModList in sync
   */
  const setModInfoListActive = useCallback(
    (updater: React.SetStateAction<ModInfo[]>) => {
      setModInfoListRaw((prev) => {
        const next = typeof updater === 'function' ? updater(prev) : updater;
        const nextList = toModList(next);

        setActiveModList(nextList);
        schemaStorage.set(cacheKey, nextList);

        return next;
      });
    },
    [cacheKey],
  );

  /**
   * Sorted mod info list:
   * - Matches enabled/priority with activeModList
   * - Sorted by priority
   */
  const modInfoList = useMemo(() => sortModInfoList(modInfoListRaw, activeModList), [modInfoListRaw, activeModList]);

  return {
    modInfoList,
    /** for initial fetch, does NOT update activeModList */
    setModInfoListRaw,
    /** for local edits, syncs with cache + activeModList */
    setModInfoListActive,
  } as const;
};

/**
 * Sorts modInfoList according to activeModList priorities,
 * and synchronizes `enabled` / `priority` values.
 */
const sortModInfoList = (modInfoList: ModInfo[], modList: ModItem[]): ModInfo[] => {
  if (!modList || modList.length === 0) return modInfoList;

  const modMap = new Map(modList.map((m) => [m.id, m]));

  return modInfoList
    .map((modInfo) => {
      const ref = modMap.get(modInfo.id);
      return ref ? { ...modInfo, enabled: ref.enabled, priority: ref.priority } : modInfo;
    })
    .toSorted((a, b) => {
      const modA = modMap.get(a.id);
      const modB = modMap.get(b.id);

      if (!modA && !modB) return 0;
      if (!modA) return 1;
      if (!modB) return -1;

      return (modA.priority ?? 0) - (modB.priority ?? 0);
    });
};
