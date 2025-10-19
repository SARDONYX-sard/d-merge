import * as monaco from 'monaco-editor';
import { ParsedHkanno, parseHkannoLine } from '../parser';

export function registerDocumentSemanticTokensProvider(monacoEnv: typeof monaco) {
  const tokenTypes = ['number', 'keyword', 'variable', 'comment', 'invalid'];
  const tokenModifiers: string[] = [];

  monacoEnv.languages.registerDocumentSemanticTokensProvider('hkanno', {
    getLegend() {
      return { tokenTypes, tokenModifiers };
    },

    provideDocumentSemanticTokens(model) {
      const lines = model.getLinesContent();
      const data: number[] = [];

      let lastLine = 0;
      let lastChar = 0;

      for (let lineNumber = 0; lineNumber < lines.length; lineNumber++) {
        const lineText = lines[lineNumber];
        const parsed: ParsedHkanno = parseHkannoLine(lineText, lineNumber + 1);
        if (!parsed.tokenPositions) continue;

        const pushToken = (pos: { line: number; startColumn: number; length: number }, type: string) => {
          const tokenTypeIndex = tokenTypes.indexOf(type);
          if (tokenTypeIndex === -1) return;

          const deltaLine = pos.line - 1 - lastLine;
          const deltaStart = deltaLine === 0 ? pos.startColumn - 1 - lastChar : pos.startColumn - 1;

          data.push(deltaLine, deltaStart, pos.length, tokenTypeIndex, 0);

          lastLine = pos.line - 1;
          lastChar = pos.startColumn - 1;
        };

        const { time, verb, argPositions } = parsed.tokenPositions;

        if (time) pushToken(time, 'number'); // f32 → number
        if (verb) pushToken(verb, parsed.type === 'meta' ? 'comment' : 'keyword'); // event → keyword or meta → comment
        if (argPositions) {
          for (const arg of argPositions) pushToken(arg, 'number'); // x/y/z or degrees → number
        }
        if (parsed.type === 'invalid') {
          if (time) pushToken(time, 'invalid');
          if (argPositions) argPositions.forEach((arg) => pushToken(arg, 'invalid'));
        }
      }

      return { data: new Uint32Array(data) };
    },

    releaseDocumentSemanticTokens() {
      // no-op
    },
  });
}
