import { type Dispatch, type SetStateAction, createContext, useContext } from 'react';
import { useState } from 'react';
import { useEffect } from 'react';

import { useStorageState } from '@/components/hooks/useStorageState';
import { NOTIFY } from '@/lib/notify';
import { PRIVATE_CACHE_OBJ } from '@/lib/storage/cacheKeys';
import { type ModInfo, loadActivateMods, loadModsInfo } from '@/services/api/patch';

import type React from 'react';

type ContextType = {
  /** Search target dir for `Nemesis_Engine/mods/<id>/info.ini` */
  modInfoDir: string;
  setModInfoDir: (value: string) => void;
  output: string;
  setOutput: (value: string) => void;
  rows: ModInfo[];
  setRows: Dispatch<SetStateAction<ModInfo[]>>;
  selectionModel: readonly string[];
  setSelectionModel: (value: readonly string[]) => void;
};
const Context = createContext<ContextType | undefined>(undefined);

export const PatchProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [modInfoDir, setModInfoDir] = useStorageState(PRIVATE_CACHE_OBJ.patchInput, '');
  const [output, setOutput] = useStorageState(PRIVATE_CACHE_OBJ.patchOutput, '');
  const [rows, setRows] = useState<ModInfo[]>([]);
  const [selectionModel, setSelectionModel] = useState<readonly string[]>([]);

  useEffect(() => {
    NOTIFY.asyncTry(async () => {
      const modsInfo = await loadModsInfo(modInfoDir);
      const activateMods = await loadActivateMods();
      setRows(modsInfo);
      setSelectionModel(activateMods);
    });
  }, [modInfoDir]);

  const context = {
    modInfoDir,
    setModInfoDir,
    output,
    setOutput,
    rows,
    setRows,
    selectionModel,
    setSelectionModel,
  } as const satisfies ContextType;

  return <Context.Provider value={context}>{children}</Context.Provider>;
};

/**
 * @throws `usePatchContext` must be used within a `PatchProvider`
 */
export const usePatchContext = () => {
  const context = useContext(Context);
  if (!context) {
    throw new Error('useCssContext must be used within a PatchProvider');
  }
  return context;
};
