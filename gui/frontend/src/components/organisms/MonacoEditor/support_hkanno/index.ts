import { type OnMount } from '@monaco-editor/react';

import { registerCompletionProvider } from './providers/completion';
import { updateHkannoDiagnostics } from './providers/diagnostic';
import { registerDocumentFormattingEditProvider } from './providers/formatter';
import { registerHoverProvider } from './providers/hover';
import { registerInlayHintsProvider } from './providers/inlay_hint';
import { registerDocumentSemanticTokensProvider } from './providers/semantic_token';
import { registerSignatureHelpProvider } from './providers/signature';

export const supportHkanno: OnMount = (editor, monacoEnv) => {
  if (monacoEnv.languages.getLanguages().some((l) => l.id === 'hkanno')) {
    return;
  }

  monacoEnv.languages.register({ id: 'hkanno' });
  monacoEnv.languages.setLanguageConfiguration('hkanno', {
    comments: {
      lineComment: '#',
    },
  });

  updateHkannoDiagnostics(editor, monacoEnv);

  registerCompletionProvider(monacoEnv);
  registerDocumentFormattingEditProvider(monacoEnv);
  registerDocumentSemanticTokensProvider(monacoEnv);
  registerHoverProvider(monacoEnv);
  registerInlayHintsProvider(monacoEnv);
  registerSignatureHelpProvider(editor, monacoEnv);

  // Monarch fallback tokenizer
  monacoEnv.languages.setMonarchTokensProvider('hkanno', {
    tokenizer: {
      root: [
        [/#.*/, 'comment'],
        [/\d+\.\d+/, 'number.float'],
        [/\d+/, 'number'],
        [/".*?"/, 'string'],
        [/[a-zA-Z0-9_]+:/, 'keyword'],
      ],
    },
  });
};
