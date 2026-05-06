import { HKANNO_LANGUAGE_ID, NewProviderFn } from '..';

import type * as monaco from 'monaco-editor';

export const registerCodeActionProvider: NewProviderFn = (monacoEnv) => {
  return monacoEnv.languages.registerCodeActionProvider(HKANNO_LANGUAGE_ID, {
    provideCodeActions(model, _range, context, _token) {
      const actions: monaco.languages.CodeAction[] = [];

      for (const marker of context.markers) {
        if (marker.code !== 'fix-iframe-key') continue;

        const range = new monacoEnv.Range(
          marker.startLineNumber,
          marker.startColumn,
          marker.endLineNumber,
          marker.endColumn,
        );

        const textInRange = model.getValueInRange(range).trim();

        let fixedText = textInRange;

        try {
          const parsed = JSON.parse(textInRange) as Record<string, unknown>;
          const normalized: Record<string, unknown> = {};
          for (const [_key, value] of Object.entries(parsed)) {
            normalized['Duration'] = value;
          }
          fixedText = JSON.stringify(normalized);
        } catch {
          continue;
        }

        actions.push({
          title: 'Fix IFrame key → "Duration"',
          diagnostics: [marker],
          kind: 'quickfix',
          edit: {
            edits: [
              {
                resource: model.uri,
                textEdit: {
                  range,
                  text: fixedText,
                },
                versionId: undefined,
              },
            ],
          },
          isPreferred: true,
        });
      }

      return { actions, dispose: () => {} };
    },
  });
};
