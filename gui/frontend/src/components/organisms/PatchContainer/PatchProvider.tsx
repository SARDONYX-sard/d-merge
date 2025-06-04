import { useState, useEffect, createContext, useContext, useMemo, useTransition } from 'react';

import { useDebounce } from '@/components/hooks/useDebounce';
import { useStorageState } from '@/components/hooks/useStorageState';
import { NOTIFY } from '@/lib/notify';
import { PRIVATE_CACHE_OBJ, PUB_CACHE_OBJ } from '@/lib/storage/cacheKeys';
import { boolSchema, stringArraySchema, stringSchema } from '@/lib/zod/schema-utils';
import { type ModInfo, type PatchOptions, loadModsInfo, patchOptionsSchema } from '@/services/api/patch';

import type { Dispatch, ReactNode, SetStateAction, FC } from 'react';

type ContextType = {
  activateMods: string[];
  setActivateMods: Dispatch<SetStateAction<string[]>>;

  /** Loading info.ini for each Nemesis Mod? */
  loading: boolean;

  /** Data dir of Skyrim where each Nemesis Mod exists */
  cacheModInfoDir: string;
  setCacheModInfoDir: Dispatch<SetStateAction<string>>;

  modInfoDir: string;
  setModInfoDir: Dispatch<SetStateAction<string>>;

  /** Auto detect skyrim data directory.(To get modInfoDir) */
  autoDetectEnabled: boolean;
  setAutoDetectEnabled: Dispatch<SetStateAction<boolean>>;

  modInfoList: ModInfo[];
  setModInfoList: Dispatch<SetStateAction<ModInfo[]>>;

  output: string;
  setOutput: Dispatch<SetStateAction<string>>;
  /** priority ids */
  priorities: string[];
  setPriorities: Dispatch<SetStateAction<string[]>>;

  patchOptions: PatchOptions;
  setPatchOptions: Dispatch<SetStateAction<PatchOptions>>;
};
const Context = createContext<ContextType | undefined>(undefined);

export const PatchProvider: FC<{ children: ReactNode }> = ({ children }) => {
  const [activateMods, setActivateMods] = useStorageState(PRIVATE_CACHE_OBJ.patchActivateIds, stringArraySchema);
  const [cacheModInfoDir, setCacheModInfoDir] = useStorageState(PRIVATE_CACHE_OBJ.patchInput, stringSchema);
  const [modInfoDir, setModInfoDir] = useState(cacheModInfoDir);
  const [autoDetectEnabled, setAutoDetectEnabled] = useStorageState(PUB_CACHE_OBJ.autoDetectEnabled, boolSchema);

  const [patchOptions, setPatchOptions] = useStorageState(PUB_CACHE_OBJ.patchOptions, patchOptionsSchema);

  const [output, setOutput] = useStorageState(PRIVATE_CACHE_OBJ.patchOutput, stringSchema);
  const [priorities, setPriorities] = useStorageState(PRIVATE_CACHE_OBJ.patchPriorityIds, stringArraySchema);

  const [modInfoList, setModInfoList] = useState<ModInfo[]>([]);
  const [loading, startTransition] = useTransition();

  // NOTE: Use this instead of `useDeferredValue` to delay API calls.
  const deferredModInfoDir = useDebounce(modInfoDir, 450);
  useEffect(() => {
    startTransition(() => {
      NOTIFY.asyncTry(async () => {
        const modsInfo = await loadModsInfo(deferredModInfoDir);
        setModInfoList(modsInfo);
      });
    });
  }, [deferredModInfoDir]);

  // NOTE: Priority sorting in `useEffect` will cause a query to the backend each time, so separate it to prevent the query.
  const sortedModInfoList = useMemo(() => {
    return modInfoList.toSorted((a, b) => priorities.indexOf(a.id) - priorities.indexOf(b.id));
  }, [modInfoList, priorities]);

  const context = {
    activateMods,
    setActivateMods,

    cacheModInfoDir,
    setCacheModInfoDir,

    modInfoDir,
    setModInfoDir,

    autoDetectEnabled,
    setAutoDetectEnabled,

    patchOptions,
    setPatchOptions,

    output,
    setOutput,

    priorities,
    setPriorities,

    modInfoList: sortedModInfoList,
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
