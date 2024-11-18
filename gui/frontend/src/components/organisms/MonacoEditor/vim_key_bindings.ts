import { start } from '../../../services/api/shell';

import type { MonacoEditor, VimModeRef, VimStatusRef } from './MonacoEditor';
import type MonacoVim from 'monaco-vim';
import type { Vim } from 'monaco-vim';

const defineVimExCommand = (
  vim: Vim,
  exCommand: string,
  editor: MonacoEditor,
  actionId: string,
  key: string,
  mode: 'normal' | 'insert' | 'visual',
) => {
  vim.defineEx(exCommand, exCommand, () => {
    editor.getAction(actionId)?.run();
  });
  vim.map(key, `:${exCommand}`, mode);
};

const getWordAtCursor = (editor: MonacoEditor) => {
  const position = editor.getPosition();
  if (!position) {
    return;
  }
  const wordInfo = editor.getModel()?.getWordAtPosition(position);
  if (wordInfo) {
    const word = wordInfo.word;
    return word;
  }
};

const setCustomVimKeyConfig = (editor: MonacoEditor, vim: Vim) => {
  for (const key of ['jj', 'jk', 'kj'] as const) {
    vim.map(key, '<Esc>', 'insert');
  }

  // Fix the problem that the default `%` is one-way and we can't go back.
  defineVimExCommand(vim, 'goToBracket', editor, 'editor.action.jumpToBracket', '%', 'normal');
  defineVimExCommand(vim, 'showHover', editor, 'editor.action.showHover', 'K', 'normal');

  vim.defineEx('GoTo', 'GoTo', async () => {
    const url = getWordAtCursor(editor);
    if (url) {
      await start(url);
    }
  });
  vim.map('gx', ':Goto', 'normal');
};

type VimKeyLoader = (props: { editor: MonacoEditor; vimModeRef: VimModeRef; vimStatusRef: VimStatusRef }) => void;
export const loadVimKeyBindings: VimKeyLoader = ({ editor, vimModeRef, vimStatusRef }) => {
  // @ts-ignore
  window.require.config({
    paths: {
      'monaco-vim': 'https://unpkg.com/monaco-vim/dist/monaco-vim',
    },
  });
  // @ts-ignore
  window.require(['monaco-vim'], (monacoVim: typeof MonacoVim) => {
    const { Vim } = monacoVim.VimMode;
    setCustomVimKeyConfig(editor, Vim);

    if (vimStatusRef.current) {
      vimModeRef.current = monacoVim.initVimMode(editor, vimStatusRef.current);
    }
  });
};
