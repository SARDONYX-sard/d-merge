// NOTE: This state is not normally necessary globally, but it must be placed globally because it needs to be accessible to everything for automatic backup.

import type { Dispatch, FC, ReactNode, SetStateAction } from 'react';
import { createContext, useContext, useEffect, useMemo, useState, useTransition } from 'react';
import z from 'zod';
import { useDebounce } from '@/components/hooks/useDebounce';
import { useStorageState } from '@/components/hooks/useStorageState';
import { NOTIFY } from '@/lib/notify';
import { PRIVATE_CACHE_OBJ, PUB_CACHE_OBJ } from '@/lib/storage/cacheKeys';
import { boolSchema, stringSchema } from '@/lib/zod/schema-utils';
import { loadModsInfo, type ModInfo, type PatchOptions, patchOptionsSchema } from '@/services/api/patch';

type ContextType = {
  output: string;
  setOutput: Dispatch<SetStateAction<string>>;

  isVfsMode: boolean;
  setIsVfsMode: Dispatch<SetStateAction<boolean>>;

  patchOptions: PatchOptions;
  setPatchOptions: Dispatch<SetStateAction<PatchOptions>>;

  /** For Vfs(MO2 etc.)mode */
  vfsSkyrimDataDir: string;
  setVfsSkyrimDataDir: Dispatch<SetStateAction<string>>;
  vfsModList: ModItem[];
  setVfsModList: Dispatch<SetStateAction<ModItem[]>>;

  /** For Manual mode */
  skyrimDataDir: string;
  setSkyrimDataDir: Dispatch<SetStateAction<string>>;
  modList: ModItem[];
  setModList: Dispatch<SetStateAction<ModItem[]>>;

  /////////////////////////////////////////////////////////////////////
  // No cached

  /** Loading info.ini for each Nemesis Mod? */
  loading: boolean;
  modInfoList: ModInfo[];
  setModInfoList: Dispatch<SetStateAction<ModInfo[]>>;
};
const Context = createContext<ContextType | undefined>(undefined);

export type ModItem = z.infer<typeof ModItemSchema>;
export const ModItemSchema = z.object({
  enabled: z.boolean(),
  /**
   * - vfs: e.g. `aaaa`
   * - manual: e.g. `path/to/aaaaa`
   */
  id: z.string(),
  priority: z.number(),
});
export const ModListSchema = z.array(ModItemSchema).catch([]);

export const PatchProvider: FC<{ children: ReactNode }> = ({ children }) => {
  const [output, setOutput] = useStorageState(PRIVATE_CACHE_OBJ.patchOutput, stringSchema);

  const [isVfsMode, setIsVfsMode] = useStorageState(PUB_CACHE_OBJ.isVfsMode, boolSchema);
  const [patchOptions, setPatchOptions] = useStorageState(PUB_CACHE_OBJ.patchOptions, patchOptionsSchema);

  const [vfsSkyrimDataDir, setVfsSkyrimDataDir] = useStorageState(
    PRIVATE_CACHE_OBJ.patchVfsSkyrimDataDir,
    stringSchema,
  );
  const [vfsModList, setVfsModList] = useStorageState(PRIVATE_CACHE_OBJ.patchVfsModList, ModListSchema);

  const [skyrimDataDir, setSkyrimDataDir] = useStorageState(PRIVATE_CACHE_OBJ.patchSkyrimDataDir, stringSchema);
  const [modList, setModList] = useStorageState(PRIVATE_CACHE_OBJ.patchModList, ModListSchema);

  const [modInfoList, setModInfoList] = useState<ModInfo[]>([]);
  const [loading, startTransition] = useTransition();

  // NOTE: Use this instead of `useDeferredValue` to delay API calls.
  const deferredModInfoDir = useDebounce(isVfsMode ? vfsSkyrimDataDir : skyrimDataDir, 450);
  useEffect(() => {
    if (!deferredModInfoDir) return;

    startTransition(() => {
      NOTIFY.asyncTry(async () => setModInfoList(await loadModsInfo(deferredModInfoDir)));
    });
  }, [deferredModInfoDir, isVfsMode]);

  // NOTE: Priority sorting in `useEffect` will cause a query to the backend each time, so separate it to prevent the query.
  const sortedModInfoList = useMemo(() => {
    const activeModList = isVfsMode ? vfsModList : modList;
    return sortModInfoList(modInfoList, activeModList);
  }, [modInfoList, modList, vfsModList, isVfsMode]);

  const context = {
    output,
    setOutput,

    isVfsMode,
    setIsVfsMode,

    patchOptions,
    setPatchOptions,

    vfsSkyrimDataDir,
    setVfsSkyrimDataDir,
    vfsModList,
    setVfsModList,

    skyrimDataDir,
    setSkyrimDataDir,
    modList,
    setModList,

    loading,
    modInfoList: sortedModInfoList,
    setModInfoList,
  } as const satisfies ContextType;

  return <Context value={context}>{children}</Context>;
};

/**
 * @throws `usePatchContext` must be used within a `PatchProvider`
 */
export const usePatchContext = () => {
  const context = useContext(Context);
  if (!context) {
    throw new Error('usePatchContext must be used within a PatchProvider');
  }
  return context;
};

/**
 * Sorts the modInfoList by priority, referencing modList.
 * Also updates each ModInfo.enabled to match modList.
 */
function sortModInfoList(modInfoList: ModInfo[], modList: ModItem[]): ModInfo[] {
  if (!modList || modList.length === 0) return modInfoList;

  // Create a map for quick lookup: id -> ModItem
  const modMap = new Map(modList.map((m) => [m.id, m]));

  return modInfoList
    .map((modInfo) => {
      const ref = modMap.get(modInfo.id);
      return ref
        ? {
            ...modInfo,
            enabled: ref.enabled,
            priority: ref.priority,
          }
        : modInfo;
    })
    .toSorted((a, b) => {
      const modA = modMap.get(a.id);
      const modB = modMap.get(b.id);

      // Fallback: if neither exists in modMap
      if (!modA && !modB) return 0;
      if (!modA) return 1;
      if (!modB) return -1;

      // Then sort by priority
      return (modA.priority ?? 0) - (modB.priority ?? 0);
    });
}
