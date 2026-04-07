import * as monaco from 'monaco-editor';
import { HKANNO_LANGUAGE_ID } from '..';

/** Monarch fallback tokenizer */
export const registerMonarchTokensProvider = (monacoEnv: typeof monaco) => {
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

  // Pair color seems to only work when done in the following manner.
  // See: https://github.com/microsoft/monaco-editor/issues/3907#issuecomment-1502932923
  monacoEnv.languages.setLanguageConfiguration(HKANNO_LANGUAGE_ID, {
    brackets: [
      ['(', ')'],
      ['{', '}'],
      ['[', ']'],
    ],
    comments: {
      lineComment: '#',
    },
  });
};
