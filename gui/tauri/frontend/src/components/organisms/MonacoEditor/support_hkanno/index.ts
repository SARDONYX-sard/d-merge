import { type OnMount } from '@monaco-editor/react';
import { registerCodeActionProvider } from './providers/code_action';
import { registerCompletionProvider } from './providers/completion';
import { registerCodeLen } from './providers/diagnostic';
import { registerDocumentFormattingEditProvider } from './providers/formatter';
import { registerHoverProvider } from './providers/hover';
import { registerInlayHintsProvider } from './providers/inlay_hint';
import { registerMonarchTokensProvider } from './providers/monarch_token';
import { registerDocumentSemanticTokensProvider } from './providers/semantic_token';
import { registerSignatureHelpProvider } from './providers/signature';

export const HKANNO_LANGUAGE_ID = 'hkanno';

export const supportHkanno: OnMount = (editor, monacoEnv) => {
  if (monacoEnv.languages.getLanguages().some(({ id }) => id === HKANNO_LANGUAGE_ID)) {
    return;
  }
  monacoEnv.languages.register({ id: HKANNO_LANGUAGE_ID });

  registerCodeLen(editor, monacoEnv);

  registerCodeActionProvider(monacoEnv);
  registerCompletionProvider(monacoEnv);
  registerDocumentFormattingEditProvider(monacoEnv);
  registerDocumentSemanticTokensProvider(monacoEnv);
  registerHoverProvider(monacoEnv);
  registerInlayHintsProvider(monacoEnv);
  registerMonarchTokensProvider(monacoEnv);
  registerSignatureHelpProvider(editor, monacoEnv);
};
