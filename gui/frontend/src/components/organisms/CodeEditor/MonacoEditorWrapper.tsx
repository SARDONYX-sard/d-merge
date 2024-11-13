// Copyright (c) 2023 Luma <lumakernel@gmail.com>
// SPDX-License-Identifier: MIT or Apache-2.0
//
// issue: https://github.com/suren-atoyan/monaco-react/issues/136#issuecomment-731420078
import Editor, { type OnMount } from '@monaco-editor/react';
import { type ComponentPropsWithoutRef, type MutableRefObject, memo, useCallback, useEffect, useRef } from 'react';

import { atomOneDarkPro } from './atom_onedark_pro';

import type monaco from 'monaco-editor/esm/vs/editor/editor.api';
import type { VimMode } from 'monaco-vim';

type VimModeRef = MutableRefObject<VimMode | null>;
type VimStatusRef = MutableRefObject<HTMLDivElement | null>;
type KeyLoaderArgs = {
  editor: monaco.editor.IStandaloneCodeEditor;
  vimModeRef: VimModeRef;
  vimStatusRef: VimStatusRef;
};
type KeyLoader = (props: KeyLoaderArgs) => void;
const loadVimKeyBindings: KeyLoader = ({ editor, vimModeRef, vimStatusRef }) => {
  // NOTE: need setup key bindings before monaco-vim setup
  // editor.addAction({
  //   id: 'show-hover',
  //   label: 'show-hover',
  //   keybindings: [monaco.KeyMod.Shift | monaco.KeyCode.KeyK],
  //   run: (editor) => {
  //     editor.getAction('editor.action.showHover')?.run();
  //   },
  // });

  // setup monaco-vim
  // @ts-ignore
  window.require.config({
    paths: {
      'monaco-vim': 'https://unpkg.com/monaco-vim/dist/monaco-vim',
    },
  });
  // @ts-ignore
  window.require(['monaco-vim'], (monacoVim: VimMode) => {
    if (vimStatusRef.current) {
      vimModeRef.current = monacoVim.initVimMode(editor, vimStatusRef.current);
    }
  });
};

type Props = ComponentPropsWithoutRef<typeof Editor> & {
  id?: string;
  /** use vim key binding? */
  readonly vimMode?: boolean;
};

export const MonacoEditorWrapper = memo(function MonacoEditorWrapper({
  id,
  vimMode = false,
  onMount,
  ...params
}: Props) {
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  const vimModeRef: VimModeRef = useRef(null);
  const vimStatusRef: VimStatusRef = useRef(null);

  const handleDidMount: OnMount = useCallback(
    (editor, monaco) => {
      editorRef.current = editor;
      if (vimMode) {
        loadVimKeyBindings({ editor, vimModeRef, vimStatusRef });
      }

      editor.updateOptions({
        theme: 'onedark',
      });
      onMount?.(editor, monaco);
    },
    [onMount, vimMode],
  );

  // NOTE: If we do not set the key bindings within `useEffect`, the switching will not work.
  //       If we do it in `handleDidMount`, the key bindings will not switch unless we reload the page.
  useEffect(() => {
    vimModeRef.current?.dispose();
    if (vimMode && editorRef.current) {
      loadVimKeyBindings({ editor: editorRef.current, vimModeRef, vimStatusRef });
    }
  }, [vimMode]);

  return (
    <>
      <Editor
        theme='vs-dark'
        {...params}
        beforeMount={(monaco) => monaco.editor.defineTheme('onedark', atomOneDarkPro)}
        onMount={handleDidMount}
      />
      <div style={{ display: 'flex', justifyContent: 'center', width: '100%' }}>
        <div ref={vimStatusRef} />
      </div>
    </>
  );
});
