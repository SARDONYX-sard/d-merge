import { type ReactNode, createContext, useContext } from 'react';

import { useStorageState } from '../../hooks/useStorageState';

type ContextType = {
  pathMode: 'path' | 'dir';
  setPathMode: (pathMode: 'path' | 'dir') => void;
  input: string;
  setInput: (value: string) => void;
  output: string;
  setOutput: (value: string) => void;
  fmt: 'amd64' | 'win32' | 'xml';
  setFmt: (value: 'amd64' | 'win32' | 'xml') => void;
};
const Context = createContext<ContextType | undefined>(undefined);

type Props = { children: ReactNode };
export const ConvertProvider = ({ children }: Props) => {
  const [pathMode, setPathMode] = useStorageState<'path' | 'dir'>('convert-path-mode', 'dir');
  const [input, setInput] = useStorageState('convert-input', '');
  const [output, setOutput] = useStorageState('convert-output', '');
  const [fmt, setFmt] = useStorageState<'amd64' | 'win32' | 'xml'>('convert-output-fmt', 'amd64');

  return (
    <Context.Provider value={{ pathMode, setPathMode, input, setInput, output, setOutput, fmt, setFmt }}>
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
