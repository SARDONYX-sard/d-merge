import type MonacoVim from 'monaco-vim';
import type { Vim } from 'monaco-vim';
import type { MonacoEditor, VimModeRef, VimStatusRef } from './MonacoEditor';

/**
 * Handles single vs double 'K' presses:
 * - Single K → show hover
 * - Double K (quickly) → show definition preview hover
 *
 * # Hack
 * Since hover and preview cannot be registered simultaneously, use the following hack.
 * Switch the hover detection logic based on whether the .hidden class is applied to the .monaco-hover class.
 * This enables hover preview by pressing K twice.
 */
const hover = async (editor: MonacoEditor) => {
  const hovers = document.querySelectorAll('.monaco-editor .monaco-hover');
  const isHoverVisible = Array.from(hovers).some((h) => !h.classList.contains('hidden'));

  if (isHoverVisible) {
    // Double press detected → show definition preview hover
    await editor.getAction('editor.action.showDefinitionPreviewHover')?.run();
  } else {
    await editor.getAction('editor.action.showHover')?.run();
  }
};

type DefineVimExCommand = {
  actionId: string;
  editor: MonacoEditor;
  /** - `actionId: 'editor.action.jumpToBracket'` => `exCommand: 'jumpToBracket'` */
  exCommand?: string;
  key: string;
  mode?: 'normal' | 'insert' | 'visual';
  vim: Vim;
};

const defineVimExCommand = ({ vim, exCommand, editor, actionId, key, mode }: DefineVimExCommand) => {
  const cmd = exCommand ?? actionId.split('.').at(-1) ?? actionId;
  vim.defineEx(cmd, cmd, async () => {
    await editor.getAction(actionId)?.run();
  });
  vim.map(key, `:${cmd}`, mode ?? 'normal');
};

const setCustomVimKeyConfig = (editor: MonacoEditor, vim: Vim) => {
  for (const key of ['jj', 'jk', 'kj'] as const) {
    vim.map(key, '<Esc>', 'insert');
  }

  vim.defineEx('hover', 'hover', async () => {
    await hover(editor);
  });
  vim.map('K', ':hover', 'normal');

  const vimExCommands = [
    { actionId: 'editor.action.jumpToBracket', key: '%' },
    { actionId: 'editor.action.openLink', key: 'gx' },
    { actionId: 'editor.action.revealDefinition', key: 'gd' },
  ] as const satisfies Omit<DefineVimExCommand, 'vim' | 'editor'>[];

  for (const command of vimExCommands) {
    defineVimExCommand({ ...command, vim, editor });
  }
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
