import { type Dispatch, type SetStateAction, createContext, useContext, useMemo } from 'react';
import { useState } from 'react';
import { useEffect } from 'react';

import { useStorageState } from '@/components/hooks/useStorageState';
import { NOTIFY } from '@/lib/notify';
import { PRIVATE_CACHE_OBJ } from '@/lib/storage/cacheKeys';
import { stringArraySchema, stringSchema } from '@/lib/zod/schema-utils';
import { type ModInfo, loadModsInfo } from '@/services/api/patch';

import type React from 'react';

type ContextType = {
  activateMods: string[];
  /** Loading info.ini for each Nemesis Mod? */
  loading: boolean;
  /** Data dir of Skyrim where each Nemesis Mod exists */
  modInfoDir: string;
  modInfoList: ModInfo[];
  output: string;
  priorities: string[];
  setActivateMods: (value: string[]) => void;
  setModInfoDir: (value: string) => void;
  setModInfoList: Dispatch<SetStateAction<ModInfo[]>>;
  setOutput: (value: string) => void;
  setPriorities: (value: string[]) => void;
};
const Context = createContext<ContextType | undefined>(undefined);

export const PatchProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [activateMods, setActivateMods] = useStorageState(PRIVATE_CACHE_OBJ.patchActivateIds, stringArraySchema);
  const [modInfoDir, setModInfoDir] = useStorageState(PRIVATE_CACHE_OBJ.patchInput, stringSchema);
  const [output, setOutput] = useStorageState(PRIVATE_CACHE_OBJ.patchOutput, stringSchema);
  const [priorities, setPriorities] = useStorageState(PRIVATE_CACHE_OBJ.patchPriorityIds, stringArraySchema);

  const [loading, setLoading] = useState(false);
  const [modInfoList, setModInfoList] = useState<ModInfo[]>([]);

  useEffect(() => {
    NOTIFY.asyncTry(async () => {
      setLoading(true);
      const modsInfo = await loadModsInfo(modInfoDir);
      setLoading(false);
      setModInfoList(modsInfo);
    });
  }, [modInfoDir]);

  // NOTE: Priority sorting in `useEffect` will cause a query to the backend each time, so separate it to prevent the query.
  const sortedModInfoList = useMemo(() => {
    return modInfoList.toSorted((a, b) => priorities.indexOf(a.id) - priorities.indexOf(b.id));
  }, [modInfoList, priorities]);

  const context = {
    activateMods,
    loading,
    modInfoDir,
    modInfoList: sortedModInfoList,
    output,
    priorities,
    setActivateMods,
    setModInfoDir,
    setModInfoList,
    setOutput,
    setPriorities,
  } as const satisfies ContextType;

  return <Context.Provider value={context}>{children}</Context.Provider>;
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
