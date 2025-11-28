import { initVimMode, type Vim, VimMode } from 'monaco-vim';
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
  const hovers = document.querySelectorAll('.monaco-hover');
  const isHoverVisible = Array.from(hovers).some((h) => !h.classList.contains('hidden'));

  if (isHoverVisible) {
    // Double press detected → show definition preview hover
    return await editor.getAction('editor.action.showDefinitionPreviewHover')?.run();
  }

  await editor.getAction('editor.action.showHover')?.run();
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

  const vimExCommands = [
    { actionId: 'editor.action.jumpToBracket', key: '%' },
    { actionId: 'editor.action.openLink', key: 'gx' },
    { actionId: 'editor.action.revealDefinition', key: 'gd' },
  ] as const satisfies Omit<DefineVimExCommand, 'vim' | 'editor'>[];

  for (const command of vimExCommands) {
    defineVimExCommand({ ...command, vim, editor });
  }

  vim.defineEx('hover', 'hover', async () => {
    await hover(editor);
  });
  vim.map('K', ':hover', 'normal'); // FIXME: For some reason, it doesn't work.
};

type VimKeyLoader = (props: { editor: MonacoEditor; vimModeRef: VimModeRef; vimStatusRef: VimStatusRef }) => void;
export const loadVimKeyBindings: VimKeyLoader = ({ editor, vimModeRef, vimStatusRef }) => {
  if (vimStatusRef.current) {
    const vim = initVimMode(editor, vimStatusRef.current);
    vimModeRef.current = vim;
    setCustomVimKeyConfig(editor, VimMode.Vim);
  }
};
