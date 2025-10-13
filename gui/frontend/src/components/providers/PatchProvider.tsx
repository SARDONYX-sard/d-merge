// NOTE: This state is not normally necessary globally, but it must be placed globally because it needs to be accessible to everything for automatic backup.

import type { Dispatch, FC, ReactNode, SetStateAction } from 'react';
import { createContext, useContext, useEffect, useState, useTransition } from 'react';
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

  /** When sorting, locked drag & drop */
  lockedDnd: boolean;
  setLockedDnd: Dispatch<SetStateAction<boolean>>;
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

  const { modInfoList, setModInfoListActive: setModInfoList, setModInfoListRaw } = useModInfoState(isVfsMode);
  const [loading, startTransition] = useTransition();
  const [lockedDnd, setLockedDnd] = useState(false);

  // NOTE: Use this instead of `useDeferredValue` to delay API calls.
  const deferredModInfoDir = useDebounce(isVfsMode ? vfsSkyrimDataDir : skyrimDataDir, 450);

  setPatchOptions((options) => ({
    ...options,
    skyrimDataDirGlob: deferredModInfoDir,
  }));

  useEffect(() => {
    if (!deferredModInfoDir) return;

    startTransition(() => {
      NOTIFY.asyncTry(async () => {
        console.log('fetching');
        setModInfoListRaw(intoModInfoList(await loadModsInfo(deferredModInfoDir)));
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

    lockedDnd,
    setLockedDnd,
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
 * Convert FetchedModInfo[] to ModInfo(add default `enabled` & `priority`)
 * Existing fields are preserved. New fields are added only if missing.
 */
const intoModInfoList = (modInfoList: FetchedModInfo[]): ModInfo[] => {
  return modInfoList.map(({ id, name, author, site, auto, modType: mod_type }, index) => {
    return {
      id,
      name,
      author,
      site,
      auto,
      modType: mod_type,
      enabled: false,
      priority: index + 1,
    } as const satisfies ModInfo;
  });
};
