import * as monaco from 'monaco-editor';
import { HKANNO_LANGUAGE_ID } from '..';

export const registerDocumentFormattingEditProvider = (monacoEnv: typeof monaco) => {
  monacoEnv.languages.registerDocumentFormattingEditProvider(HKANNO_LANGUAGE_ID, {
    provideDocumentFormattingEdits(model) {
      return [{ range: model.getFullModelRange(), text: formatHkannoText(model.getValue()) }];
    },
  });
};

export const formatHkannoText = (text: string): string => {
  const lines = text.split('\n');
  const formatted: string[] = [];
  for (const line of lines) {
    const trimmed = line.trimStart();
    if (trimmed.startsWith('#')) {
      formatted.push(trimmed);
      continue;
    }
    const parts = trimmed.split(/\s+/);
    if (parts.length >= 2) {
      formatted.push(`${parts[0]} ${parts.slice(1).join(' ')}`);
    } else {
      formatted.push(trimmed);
    }
  }
  return formatted.join('\n');
};
