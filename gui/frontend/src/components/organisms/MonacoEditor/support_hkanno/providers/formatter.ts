import * as monaco from 'monaco-editor';
import { HKANNO_LANGUAGE_ID } from '..';
import type { HkannoNode } from '../parser/strict/nodes';
import { parseHkannoLine } from '../parser/strict/parser';

/**
 * Registers a document formatting provider for HKANNO language in Monaco.
 * Uses a precise parser to preserve spacing and positional info.
 */
export const registerDocumentFormattingEditProvider = (monacoEnv: typeof monaco) => {
  monacoEnv.languages.registerDocumentFormattingEditProvider(HKANNO_LANGUAGE_ID, {
    provideDocumentFormattingEdits(model) {
      return [
        {
          range: model.getFullModelRange(),
          text: formatHkannoText(model.getValue()),
        },
      ];
    },
  });
};

/**
 * Format the entire HKANNO text by parsing each line into Nodes and reconstructing with preserved spacing.
 * @param text Raw HKANNO text
 */
const formatHkannoText = (text: string): string => {
  const lines = text.split('\n');
  const formattedLines: string[] = [];

  for (let i = 0; i < lines.length; i++) {
    const node = parseHkannoLine(lines[i], i + 1);
    formattedLines.push(formatNode(node));
  }

  return formattedLines.join('\n');
};

/**
 * Format a single HKANNO node.
 * Rules:
 * - space0: ignored
 * - space1: collapsed to a single space
 */
const formatNode = (node: HkannoNode): string => {
  const formatValue = (v: string | number | undefined) => {
    if (typeof v === 'number') return v.toFixed(6);
    return v ?? '';
  };

  const joinSpace1 = (...values: (string | number | undefined)[]) =>
    values
      .filter((v) => v !== undefined)
      .map(formatValue)
      .join(' ');

  switch (node.kind) {
    case 'rotation':
      return joinSpace1(node.time?.value, node.event?.value, node.degrees?.value);

    case 'motion':
      return joinSpace1(node.time?.value, node.event?.value, node.x?.value, node.y?.value, node.z?.value);

    case 'text':
      return joinSpace1(node.time?.value, node.text?.value);

    case 'trackName':
      return `trackName: ${node.name?.value?.trim() ?? ''}`;

    case 'comment':
      return joinSpace1('#', node.comment?.value);
  }
};
