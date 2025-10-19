import * as monaco from 'monaco-editor';
import { parseHkannoLine } from './hover';

export function registerHkannoInlayHints(monacoEnv: typeof monaco) {
  monacoEnv.languages.registerInlayHintsProvider('hkanno', {
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
        if (parsed.verb && parsed.tokenPositions?.verb) {
          addHint(`event: `, parsed.tokenPositions.verb);
        }

        // args
        if (parsed.args && parsed.tokenPositions?.argPositions) {
          const labels = parsed.type === 'motion' ? ['x', 'y', 'z'] : ['arg0', 'arg1', 'arg2'];
          parsed.args.forEach((_, i) => {
            const pos = parsed.tokenPositions!.argPositions![i];
            if (pos) addHint(`${labels[i] ?? `arg${i}`}: `, pos);
          });
        }
      }

      return { hints, dispose: () => {} };
    },
  });
}
