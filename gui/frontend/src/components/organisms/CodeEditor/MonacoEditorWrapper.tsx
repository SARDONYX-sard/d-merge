// Copyright (c) 2023 Luma <lumakernel@gmail.com>
// SPDX-License-Identifier: MIT or Apache-2.0
//
// issue: https://github.com/suren-atoyan/monaco-react/issues/136#issuecomment-731420078
'use client';
import Editor, { type OnMount } from '@monaco-editor/react';
import { type ComponentPropsWithoutRef, memo, useCallback, useRef } from 'react';

import { atomOneDarkPro } from './atom_onedark_pro';

import type monaco from 'monaco-editor/esm/vs/editor/editor.api';
import type { VimMode } from 'monaco-vim';

const loadVimKeyBindings: OnMount = (editor, _monaco) => {
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
    const statusNode = document.getElementById('status-node');
    monacoVim.initVimMode(editor, statusNode);
  });
};

type Props = ComponentPropsWithoutRef<typeof Editor> & {
  /** use vim key binding? */
  readonly vimMode?: boolean;
};

export const MonacoEditorWrapper = memo(function MonacoEditorWrapper({ vimMode = false, onMount, ...params }: Props) {
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);

  const handleDidMount: OnMount = useCallback(
    (editor, monaco) => {
      editorRef.current = editor;
      if (vimMode) {
        loadVimKeyBindings(editor, monaco);
      }

      editor.updateOptions({
        theme: 'onedark',
      });
      onMount?.(editor, monaco);
    },
    [onMount, vimMode],
  );

  return (
    <Editor
      theme='vs-dark'
      {...params}
      beforeMount={(monaco) => monaco.editor.defineTheme('onedark', atomOneDarkPro)}
      onMount={handleDidMount}
    />
  );
});
