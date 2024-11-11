import { type Dispatch, type SetStateAction, createContext, useContext } from 'react';
import { useState } from 'react';
import { useEffect } from 'react';

import { useStorageState } from '@/components/hooks/useStorageState';
import { NOTIFY } from '@/lib/notify';
import { PRIVATE_CACHE_OBJ } from '@/lib/storage/cacheKeys';
import { type ModInfo, loadModsInfo } from '@/services/api/patch';

import type React from 'react';

type ContextType = {
  activateMods: readonly string[];
  /** Loading info.ini for each Nemesis Mod? */
  loading: boolean;
  /** Data dir of Skyrim where each Nemesis Mod exists */
  modInfoDir: string;
  modInfoList: ModInfo[];
  output: string;
  setActivateMods: (value: readonly string[]) => void;
  setModInfoDir: (value: string) => void;
  setModInfoList: Dispatch<SetStateAction<ModInfo[]>>;
  setOutput: (value: string) => void;
};
const Context = createContext<ContextType | undefined>(undefined);

export const PatchProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [modInfoDir, setModInfoDir] = useStorageState(PRIVATE_CACHE_OBJ.patchInput, '');
  const [output, setOutput] = useStorageState(PRIVATE_CACHE_OBJ.patchOutput, '');
  const [modInfoList, setModInfoList] = useState<ModInfo[]>([]);
  const [activateMods, setActivateMods] = useStorageState<readonly string[]>(PRIVATE_CACHE_OBJ.patchActivateIds, []);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    NOTIFY.asyncTry(async () => {
      setLoading(true);
      const modsInfo = await loadModsInfo(modInfoDir);
      setLoading(false);
      setModInfoList(modsInfo);
    });
  }, [modInfoDir]);

  const context = {
    activateMods,
    loading,
    modInfoDir,
    modInfoList,
    output,
    setActivateMods,
    setModInfoDir,
    setModInfoList,
    setOutput,
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
