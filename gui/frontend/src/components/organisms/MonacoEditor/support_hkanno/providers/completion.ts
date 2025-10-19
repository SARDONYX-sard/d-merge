import * as monaco from 'monaco-editor';
import { parseHkannoLine } from '../parser';

export function registerCompletionProvider(monacoEnv: typeof monaco) {
  monacoEnv.languages.registerCompletionItemProvider('hkanno', {
    provideCompletionItems(model, position) {
      const lineContent = model.getLineContent(position.lineNumber);
      const parsed = parseHkannoLine(lineContent, position.lineNumber);
      const range = {
        startLineNumber: position.lineNumber,
        endLineNumber: position.lineNumber,
        startColumn: 1,
        endColumn: lineContent.length + 1,
      };

      const suggestions: monaco.languages.CompletionItem[] = [];

      if (parsed.type === 'meta' || lineContent.trim() === '') {
        // Meta suggestions
        suggestions.push(
          {
            label: '# numAnnotations:',
            kind: monaco.languages.CompletionItemKind.Keyword,
            insertText: '# numAnnotations: 0',
            range,
          },
          {
            label: '<time>',
            kind: monaco.languages.CompletionItemKind.Value,
            insertText: '0.0',
            range,
          },
        );
      } else if (parsed.type === 'motion' || parsed.type === 'rotation' || parsed.type === 'text') {
        const cursorCol = position.column;

        // After time → suggest verb if empty
        if (
          !parsed.eventName ||
          cursorCol > (parsed.tokenPositions?.time.startColumn ?? 0) + (parsed.tokenPositions?.time.length ?? 0)
        ) {
          suggestions.push(
            {
              label: 'animmotion',
              kind: monaco.languages.CompletionItemKind.Function,
              insertText: 'animmotion',
              range,
            },
            {
              label: 'animrotation',
              kind: monaco.languages.CompletionItemKind.Function,
              insertText: 'animrotation',
              range,
            },
          );
        }

        // After verb → suggest args for motion/rotation
        if (parsed.eventName?.toLowerCase() === 'animmotion') {
          const argsCount = parsed.args?.length ?? 0;
          if (argsCount < 3) {
            suggestions.push({
              label: ['x', 'y', 'z'][argsCount],
              kind: monaco.languages.CompletionItemKind.Value,
              insertText: '0.0',
              range,
            });
          }
        } else if (parsed.eventName?.toLowerCase() === 'animrotation') {
          if (!parsed.args?.[0]) {
            suggestions.push({
              label: 'degrees',
              kind: monaco.languages.CompletionItemKind.Value,
              insertText: '0.0',
              range,
            });
          }
        }

        // After verb/args → suggest <text>
        if (parsed.type === 'text') {
          suggestions.push({
            label: '<text>',
            kind: monaco.languages.CompletionItemKind.Text,
            insertText: 'Annotation text',
            range,
          });
        }
      }

      return { suggestions };
    },
  });
}
