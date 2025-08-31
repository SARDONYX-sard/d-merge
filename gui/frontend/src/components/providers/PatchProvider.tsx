// NOTE: This state is not normally necessary globally, but it must be placed globally because it needs to be accessible to everything for automatic backup.

import type { Dispatch, FC, ReactNode, SetStateAction } from 'react';
import { createContext, useContext, useEffect, useTransition } from 'react';
import { useDebounce } from '@/components/hooks/useDebounce';
import { useStorageState } from '@/components/hooks/useStorageState';
import { useModInfoState } from '@/components/organisms/PatchContainer/hooks/useModInfoState';
import { NOTIFY } from '@/lib/notify';
import { PRIVATE_CACHE_OBJ, PUB_CACHE_OBJ } from '@/lib/storage/cacheKeys';
import { boolSchema, stringSchema } from '@/lib/zod/schema-utils';
import {
  FetchedModInfo,
  loadModsInfo,
  type ModInfo,
  type PatchOptions,
  patchOptionsSchema,
} from '@/services/api/patch';

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

  /** For Manual mode */
  skyrimDataDir: string;
  setSkyrimDataDir: Dispatch<SetStateAction<string>>;

  modInfoList: ModInfo[];
  setModInfoList: Dispatch<SetStateAction<ModInfo[]>>;

  /////////////////////////////////////////////////////////////////////
  // No cached

  /** Loading info.ini for each Nemesis Mod? */
  loading: boolean;
};
const Context = createContext<ContextType | undefined>(undefined);

export const PatchProvider: FC<{ children: ReactNode }> = ({ children }) => {
  const [output, setOutput] = useStorageState(PRIVATE_CACHE_OBJ.patchOutput, stringSchema);

  const [isVfsMode, setIsVfsMode] = useStorageState(PUB_CACHE_OBJ.isVfsMode, boolSchema);
  const [patchOptions, setPatchOptions] = useStorageState(PUB_CACHE_OBJ.patchOptions, patchOptionsSchema);

  const [vfsSkyrimDataDir, setVfsSkyrimDataDir] = useStorageState(
    PRIVATE_CACHE_OBJ.patchVfsSkyrimDataDir,
    stringSchema,
  );

  const [skyrimDataDir, setSkyrimDataDir] = useStorageState(PRIVATE_CACHE_OBJ.patchSkyrimDataDir, stringSchema);

  const [modInfoList, setModInfoList] = useModInfoState(isVfsMode);
  const [loading, startTransition] = useTransition();

  // NOTE: Use this instead of `useDeferredValue` to delay API calls.
  const deferredModInfoDir = useDebounce(isVfsMode ? vfsSkyrimDataDir : skyrimDataDir, 450);
  useEffect(() => {
    if (!deferredModInfoDir) return;

    startTransition(() => {
      NOTIFY.asyncTry(async () => {
        const modInfos = await loadModsInfo(deferredModInfoDir);
        console.log('fetch mod infos');
        setModInfoList(addDefaultsToModInfoList(modInfos));
      });
    });
  }, [deferredModInfoDir, isVfsMode]);

  const context = {
    output,
    setOutput,

    isVfsMode,
    setIsVfsMode,

    patchOptions,
    setPatchOptions,

    vfsSkyrimDataDir,
    setVfsSkyrimDataDir,

    skyrimDataDir,
    setSkyrimDataDir,

    modInfoList,
    setModInfoList,

    loading,
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
 * Ensures each modInfo in modInfoList has enabled and priority.
 * Existing fields are preserved. New fields are added only if missing.
 */
const addDefaultsToModInfoList = (modInfoList: FetchedModInfo[]): ModInfo[] => {
  for (let i = 0; i < modInfoList.length; i++) {
    const mod = modInfoList[i];

    if (mod.enabled === undefined) {
      mod.enabled = false;
    }

    if (mod.priority === undefined) {
      mod.priority = i + 1; // 1based
    }
  }

  return modInfoList as ModInfo[];
};
