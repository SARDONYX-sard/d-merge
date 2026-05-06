import React, { useReducer } from 'react';
import z from 'zod';
import { DEFAULT_HKANNO_LSP_OPTIONS, HkAnnoLspOptionsSchema } from '../../MonacoEditor/support_hkanno';
import { FileTabSchema } from '../types/FileTab';
import { EditorContext } from './editorContext';
import { editorReducer } from './editorReducer';
import { PRIVATE_CACHE_OBJ } from '@/lib/storage/cacheKeys';
import { schemaStorage } from '@/lib/storage/schemaStorage';

import type { EditorState } from './editorTypes';

/** Provides hkanno editor state backed by schemaStorage */
export const HkAnnoEditorProvider: React.FC<React.PropsWithChildren> = ({ children }) => {
  const tabs = schemaStorage.get(PRIVATE_CACHE_OBJ.hkannoFileTabs, z.array(FileTabSchema));
  const active = schemaStorage.get(PRIVATE_CACHE_OBJ.hkannoActiveTab, z.number());
  const showPreview = schemaStorage.get(PRIVATE_CACHE_OBJ.hkannoShowPreview, z.boolean());
  const lspOptions = schemaStorage.get(PRIVATE_CACHE_OBJ.hkannoLspOptions, HkAnnoLspOptionsSchema);

  const initState = {
    tabs: tabs ?? [],
    active: active ?? 0,
    showPreview: !!showPreview,
    lspOptions: lspOptions ?? DEFAULT_HKANNO_LSP_OPTIONS,
  } as const satisfies EditorState;

  const [state, dispatch] = useReducer(editorReducer, initState);

  return <EditorContext.Provider value={[state, dispatch]}>{children}</EditorContext.Provider>;
};
