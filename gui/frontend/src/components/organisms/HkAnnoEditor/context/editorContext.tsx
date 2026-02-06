import { createContext, Dispatch, useContext } from 'react';
import type { EditorAction, EditorState } from './editorTypes';

/** Shared editor context */
export const EditorContext = createContext<[EditorState, Dispatch<EditorAction>] | null>(null);

/** Access editor state and dispatch */
export const useEditorContext = () => {
  const ctx = useContext(EditorContext);
  if (!ctx) {
    throw new Error('EditorProvider is missing');
  }
  return ctx;
};
