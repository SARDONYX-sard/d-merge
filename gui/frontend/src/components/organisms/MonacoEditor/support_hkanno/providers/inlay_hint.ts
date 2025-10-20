import * as monaco from 'monaco-editor';
import { HKANNO_LANGUAGE_ID } from '..';
import { parseHkannoLine } from '../parser/simple';

export const registerInlayHintsProvider = (monacoEnv: typeof monaco) => {
  monacoEnv.languages.registerInlayHintsProvider(HKANNO_LANGUAGE_ID, {
    provideInlayHints(model, range, _token) {
      const hints: monaco.languages.InlayHint[] = [];

      for (let lineNumber = range.startLineNumber; lineNumber <= range.endLineNumber; lineNumber++) {
        const line = model.getLineContent(lineNumber);
        const parsed = parseHkannoLine(line, lineNumber);
        if (!parsed || parsed.type === 'none' || parsed.type === 'meta') continue;

        const addHint = (label: string, pos: { line: number; startColumn: number; length: number }) => {
          hints.push({
            position: { lineNumber: pos.line, column: pos.startColumn },
            label,
            kind: monacoEnv.languages.InlayHintKind.Type,
            paddingLeft: true,
          });
        };

        // time
        if (parsed.time !== undefined && parsed.tokenPositions?.time) {
          addHint(`time: `, parsed.tokenPositions.time);
        }

        // verb / event
        if (parsed.eventName && parsed.tokenPositions?.verb) {
          addHint(`event: `, parsed.tokenPositions.verb);
        }

        // args
        if (parsed.args && parsed.tokenPositions?.argPositions) {
          const labels =
            parsed.type === 'motion' ? (['x', 'y', 'z'] as const) : (['degree', 'invalid', 'invalid'] as const);
          parsed.args.forEach((_, i) => {
            const pos = parsed.tokenPositions!.argPositions![i];
            if (pos) addHint(`${labels[i] ?? `arg${i}`}: `, pos);
          });
        }
      }

      return { hints, dispose: () => {} };
    },
  });
};
