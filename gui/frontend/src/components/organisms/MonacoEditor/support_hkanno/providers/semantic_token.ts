import * as monaco from 'monaco-editor';
import { HKANNO_LANGUAGE_ID } from '..';
import { ParsedHkanno, parseHkannoLine, TokenPos } from '../parser/simple';

export const registerDocumentSemanticTokensProvider = (monacoEnv: typeof monaco) => {
  const tokenTypes = ['number', 'keyword', 'variable', 'comment', 'invalid'];
  const tokenModifiers: string[] = [];

  monacoEnv.languages.registerDocumentSemanticTokensProvider(HKANNO_LANGUAGE_ID, {
    getLegend: () => ({ tokenTypes, tokenModifiers }),

    provideDocumentSemanticTokens(model) {
      const lines = model.getLinesContent();
      const data: number[] = [];

      let lastLine = 0;
      let lastChar = 0;

      // Arrow function for pushing semantic tokens
      const pushToken = (pos: TokenPos, type: string) => {
        const tokenTypeIndex = tokenTypes.indexOf(type);
        if (tokenTypeIndex === -1) return;

        const deltaLine = pos.line - 1 - lastLine;
        const deltaStart = deltaLine === 0 ? pos.startColumn - 1 - lastChar : pos.startColumn - 1;

        data.push(deltaLine, deltaStart, pos.length, tokenTypeIndex, 0);

        lastLine = pos.line - 1;
        lastChar = pos.startColumn - 1;
      };

      for (let lineNumber = 0; lineNumber < lines.length; lineNumber++) {
        const lineText = lines[lineNumber];
        const parsed: ParsedHkanno = parseHkannoLine(lineText, lineNumber + 1);
        if (!parsed.tokenPositions) continue;

        const { time, verb, argPositions } = parsed.tokenPositions;

        // Time → number
        if (time) pushToken(time, 'number');

        // Verb → reserved keyword or string variable
        if (verb) {
          const verbText = parsed.eventName?.toLowerCase();
          if (verbText === 'animmotion' || verbText === 'animrotation') {
            pushToken(verb, 'keyword');
          } else if (parsed.type === 'meta') {
            pushToken(verb, 'comment');
          } else {
            pushToken(verb, 'variable');
          }
        }

        // Arguments → number
        if (argPositions) {
          for (const arg of argPositions) pushToken(arg, 'number');
        }

        // Invalid tokens
        if (parsed.type === 'invalid') {
          if (time) pushToken(time, 'invalid');
          if (argPositions) argPositions.forEach((arg) => pushToken(arg, 'invalid'));
        }
      }

      return { data: new Uint32Array(data) };
    },

    releaseDocumentSemanticTokens: () => {
      // no-op
    },
  });
};
