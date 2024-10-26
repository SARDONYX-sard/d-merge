import { type Dispatch, type ReactNode, type SetStateAction, createContext, useContext, useState } from 'react';

import { useStorageState } from '@/components/hooks/useStorageState';
import { PUB_CACHE_OBJ } from '@/lib/storage/cacheKeys';
import type { OutFormat } from '@/services/api/serde_hkx';

export type SelectionType = 'files' | 'dir';

export const normalize = (value: string): SelectionType => {
  switch (value) {
    case 'files':
    case 'dir':
      return value;
    default:
      return 'files';
  }
};

export type ConvertStatusPayload = {
  pathId: number;
  /** 0: pending, 1: processing, 2: done, 3: error */
  status: 0 | 1 | 2 | 3;
};

export type ConvertStatusesMap = Map<number, ConvertStatusPayload['status']>;

type ContextType = {
  selectionType: SelectionType;
  setSelectionType: (pathMode: SelectionType) => void;
  selectedFiles: string[];
  setSelectedFiles: (value: string[]) => void;
  selectedDirs: string[];
  setSelectedDirs: (value: string[]) => void;
  output: string;
  setOutput: (value: string) => void;
  fmt: 'amd64' | 'win32' | 'xml';
  setFmt: (value: OutFormat) => void;

  convertStatuses: ConvertStatusesMap;

  setConvertStatuses: Dispatch<SetStateAction<ConvertStatusesMap>>;
};
const Context = createContext<ContextType | undefined>(undefined);

type Props = { children: ReactNode };
export const ConvertProvider = ({ children }: Props) => {
  const [selectionType, setSelectionType] = useStorageState<SelectionType>(PUB_CACHE_OBJ.convertSelectionType, 'files');
  const [selectedFiles, setSelectedFiles] = useStorageState<string[]>(PUB_CACHE_OBJ.convertSelectedFiles, []);
  const [selectedDirs, setSelectedDirs] = useStorageState<string[]>(PUB_CACHE_OBJ.convertSelectedDirs, []);
  const [output, setOutput] = useStorageState('convert-output', '');
  const [fmt, setFmt] = useStorageState<OutFormat>('convert-output-fmt', 'amd64');

  const [convertStatuses, setConvertStatuses] = useState<ConvertStatusesMap>(new Map());

  return (
    <Context.Provider
      value={{
        selectionType,
        setSelectionType,
        selectedFiles,
        setSelectedFiles,
        selectedDirs,
        setSelectedDirs,
        output,
        setOutput,
        fmt,
        setFmt,

        convertStatuses,
        setConvertStatuses,
      }}
    >
      {children}
    </Context.Provider>
  );
};

/**
 * @throws `useConvertContext must be used within a ConvertProvider`
 */
export const useConvertContext = () => {
  const context = useContext(Context);
  if (!context) {
    throw new Error('useConvertContext must be used within a ConvertProvider');
  }
  return context;
};
