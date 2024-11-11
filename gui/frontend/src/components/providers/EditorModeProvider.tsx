import { type ReactNode, createContext, useContext, useState } from 'react';

import { EDITOR_MODE, type EditorMode } from '@/lib/editor-mode';

type ContextType = {
  editorMode: EditorMode;
  setEditorMode: (value: EditorMode) => void;
};
const Context = createContext<ContextType | undefined>(undefined);

type Props = { children: ReactNode };
export const EditorModeProvider = ({ children }: Props) => {
  const [editorMode, setEditorMode] = useState<EditorMode>(EDITOR_MODE.get());

  const setHook = (value: EditorMode) => {
    setEditorMode(value);
    EDITOR_MODE.set(value);
  };

  return <Context.Provider value={{ editorMode, setEditorMode: setHook }}>{children}</Context.Provider>;
};

/**
 * @throws `useEditorModeContext must be used within a EditorModeProvider`
 */
export const useEditorModeContext = () => {
  const context = useContext(Context);
  if (!context) {
    throw new Error('useEditorModeContext must be used within a EditorModeProvider');
  }
  return context;
};
