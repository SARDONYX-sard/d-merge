import { type OnMount } from '@monaco-editor/react';

import { registerCompletionProvider } from './providers/completion';
import { updateHkannoDiagnostics } from './providers/diagnostic';
import { registerDocumentFormattingEditProvider } from './providers/formatter';
import { registerHoverProvider } from './providers/hover';
import { registerInlayHintsProvider } from './providers/inlay_hint';
import { registerDocumentSemanticTokensProvider } from './providers/semantic_token';
import { registerSignatureHelpProvider } from './providers/signature';

export const HKANNO_LANGUAGE_ID = 'hkanno';

export const supportHkanno: OnMount = (editor, monacoEnv) => {
  if (monacoEnv.languages.getLanguages().some((l) => l.id === HKANNO_LANGUAGE_ID)) {
    return;
  }

  monacoEnv.languages.register({ id: HKANNO_LANGUAGE_ID });
  monacoEnv.languages.setLanguageConfiguration(HKANNO_LANGUAGE_ID, {
    comments: {
      lineComment: '#',
    },
  });

  editor.onDidChangeModelContent(() => {
    if (editor.getModel()?.getLanguageId() === HKANNO_LANGUAGE_ID) {
      updateHkannoDiagnostics(editor, monacoEnv);
    }
  });

  registerCompletionProvider(monacoEnv);
  registerDocumentFormattingEditProvider(monacoEnv);
  registerHoverProvider(monacoEnv);
  registerInlayHintsProvider(monacoEnv);
  registerSignatureHelpProvider(editor, monacoEnv);

  // Monarch fallback tokenizer
  monacoEnv.languages.setMonarchTokensProvider(HKANNO_LANGUAGE_ID, {
    tokenizer: {
      root: [
        [/#.*/, 'comment'],

        [/\b\d+\.\d+\b/, 'number.float'],
        [/\b\d+\b/, 'number'],

        [/\bPIE|pie\b/, 'variable'],

        [/\banimmotion\b/, 'identifier'],
        [/\banimrotation\b/, 'identifier'],
        [/\bSoundPlay\b/, 'identifier'],
        [/\bSpecialFrames_Invincible\b/, 'identifier'],

        [/-?\d+\.\d+|-?\d+/, 'number.float'],

        [/".*?"/, 'string'],
        [/[A-Za-z0-9_]+/, 'white'],
      ],
    },
  });

  registerDocumentSemanticTokensProvider(monacoEnv);
};
