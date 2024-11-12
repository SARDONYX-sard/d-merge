import { type Dispatch, type ReactNode, type SetStateAction, createContext, useContext, useState } from 'react';

import { useStorageState } from '@/components/hooks/useStorageState';
import { PRIVATE_CACHE_OBJ, PUB_CACHE_OBJ } from '@/lib/storage/cacheKeys';
import type { OutFormat } from '@/services/api/serde_hkx';

import type { TreeViewBaseItem } from '@mui/x-tree-view';

export type SelectionType = 'files' | 'dir' | 'tree';

export const normalize = (value: string): SelectionType => {
  switch (value) {
    case 'files':
    case 'dir':
    case 'tree':
      return value;
    default:
      return 'files';
  }
};

export type ConvertStatusPayload = {
  /**  Djb2 hash algorism */
  pathId: number;
  /** 0: pending, 1: processing, 2: done, 3: error */
  status: 0 | 1 | 2 | 3;
};

/** key: Djb2 hash algorism, value:  */
export type ConvertStatusesMap = Map<number, ConvertStatusPayload['status']>;

export type SelectedTree = {
  selectedItems: string[];
  expandedItems: string[];
  roots: string[];
  tree: TreeViewBaseItem[];
};
export const CONVERT_TREE_INIT_VALUES = {
  expandedItems: [],
  selectedItems: [],
  roots: [],
  tree: [],
} as const satisfies SelectedTree;

type ContextType = {
  selectionType: SelectionType;
  setSelectionType: (pathMode: SelectionType) => void;
  selectedFiles: string[];
  setSelectedFiles: (value: string[]) => void;
  selectedDirs: string[];
  setSelectedDirs: (value: string[]) => void;
  selectedTree: SelectedTree;
  setSelectedTree: (value: SelectedTree) => void;
  output: string;
  setOutput: (value: string) => void;
  fmt: OutFormat;
  setFmt: (value: OutFormat) => void;

  convertStatuses: ConvertStatusesMap;
  setConvertStatuses: Dispatch<SetStateAction<ConvertStatusesMap>>;
};
const Context = createContext<ContextType | undefined>(undefined);

type Props = { children: ReactNode };
export const ConvertProvider = ({ children }: Props) => {
  const [selectionType, setSelectionType] = useStorageState<SelectionType>(PUB_CACHE_OBJ.convertSelectionType, 'files');
  const [selectedFiles, setSelectedFiles] = useStorageState<string[]>(PRIVATE_CACHE_OBJ.convertSelectedFiles, []);
  const [selectedDirs, setSelectedDirs] = useStorageState<string[]>(PRIVATE_CACHE_OBJ.convertSelectedDirs, []);
  /** NOTE: Tree is not cached because it can be a huge file */
  const [selectedTree, setSelectedTree] = useState<SelectedTree>(CONVERT_TREE_INIT_VALUES);
  const [output, setOutput] = useStorageState(PRIVATE_CACHE_OBJ.convertOutput, '');
  const [fmt, setFmt] = useStorageState<OutFormat>(PUB_CACHE_OBJ.convertOutFmt, 'amd64');

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
        selectedTree,
        setSelectedTree,
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
