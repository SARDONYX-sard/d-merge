import { type OnMount } from '@monaco-editor/react';
import { registerHkannoCompletion } from './cmp';
import { updateHkannoDiagnostics } from './diagnostic';
import { registerHkannoFormatter } from './formatter';
import { registerHkannoHover } from './hover';
import { registerHkannoInlayHints } from './inlayhint';
import { registerHkannoSemanticTokens } from './semantic_token';

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

  registerHkannoCompletion(monacoEnv);
  registerHkannoFormatter(monacoEnv);
  registerHkannoHover(monacoEnv);
  registerHkannoInlayHints(monacoEnv);
  registerHkannoSemanticTokens(monacoEnv);

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
