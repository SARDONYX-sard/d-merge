// Copyright (c) 2023 Luma <lumakernel@gmail.com>
// SPDX-License-Identifier: MIT or Apache-2.0

// NOTE: Type is not read correctly when using the `import` statement because it was obtained via CDN and does not exist in `node_modules`.
declare module 'monaco-vim' {
  class VimMode {
    dispose(): void;
    initVimMode(editor: editor.IStandaloneCodeEditor, statusbarNode?: Element | null): VimMode;
    /**
     * # Example
     * ```typescript
     * VimMode.Vim.defineEx('write', 'w', function() {
     *   // your own implementation on what you want to do when :w is pressed
     *   localStorage.setItem('editorvalue', editor.getValue());
     * });
     * ```
     */
    defineEx(name: string, shorthand: string, callback: () => void);
  }

  export function initVimMode(editor: editor.IStandaloneCodeEditor, statusbarNode?: HTMLElement): VimMode;
}
