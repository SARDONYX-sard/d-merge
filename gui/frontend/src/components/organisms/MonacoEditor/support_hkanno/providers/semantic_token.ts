import type * as monaco from 'monaco-editor';
import { HKANNO_LANGUAGE_ID } from '..';
import { Pos } from '../parser/strict/nodes';
import { parseHkannoLineExt } from '../parser/strict/parser';

export const registerDocumentSemanticTokensProvider = (monacoEnv: typeof monaco) => {
  monacoEnv.languages.registerDocumentSemanticTokensProvider(HKANNO_LANGUAGE_ID, {
    getLegend: () => ({ tokenTypes: TOKEN_TYPES.slice(), tokenModifiers: TOKEN_MODIFIERS }),

    provideDocumentSemanticTokens(model) {
      const lines = model.getLinesContent();
      const data: number[] = [];

      let lastLine = 0;
      let lastChar = 0;

      const pushToken = (pos: Pos | undefined, type: TokenType) => {
        if (pos === undefined) return;
        const tokenTypeIndex = TOKEN_TYPES.indexOf(type);
        if (tokenTypeIndex === -1) return;

        const deltaLine = pos.line - 1 - lastLine;
        const deltaStart = deltaLine === 0 ? pos.startColumn - 1 - lastChar : pos.startColumn - 1;
        const length = pos.endColumn - pos.startColumn;

        data.push(deltaLine, deltaStart, length, tokenTypeIndex, 0);

        lastLine = pos.line - 1;
        lastChar = pos.startColumn - 1;
      };

      for (let lineNumber = 0; lineNumber < lines.length; lineNumber++) {
        const lineText = lines[lineNumber];
        const node = parseHkannoLineExt(lineText, lineNumber + 1);

        switch (node.kind) {
          case 'payload_instruction': {
            pushToken(node?.event?.pos, 'variable');
            if (!node.instruction) break;

            const { prefix: atSymbol, name, parameters } = node.instruction;
            pushToken(atSymbol?.pos, 'keyword');
            pushToken(name?.pos, 'variable.function');

            parameters?.items?.forEach((parameter) => {
              pushToken(parameter?.separator?.pos, 'operator');

              const value = parameter?.value;
              if (Number.isNaN(parseFloat(value?.value ?? ''))) {
                pushToken(value?.pos, 'string');
              } else {
                pushToken(value?.pos, 'number');
              }
            });
            break;
          }
          case 'trackName': {
            pushToken(node.literal?.pos, 'variable');
            pushToken(node.name?.pos, 'string');
            break;
          }

          case 'motion': {
            if (node.event?.pos) pushToken(node.event.pos, 'identifier');
            [node.x, node.y, node.z].forEach((n) => pushToken(n?.pos, 'number'));
            break;
          }

          case 'rotation': {
            pushToken(node.event?.pos, 'identifier');
            pushToken(node.degrees?.pos, 'number');
            break;
          }

          case 'iframe': {
            pushToken(node.event?.pos, 'identifier');
            break;
          }

          case 'text': {
            pushToken(node.time?.pos, 'number');
            pushToken(node.text?.pos, 'string');
            break;
          }

          default:
            break;
        }
      }

      return { data: new Uint32Array(data) };
    },

    releaseDocumentSemanticTokens: () => {
      // no-op
    },
  });
};

/**
 * color refers to the `rules` in atom_onedark_pro.ts.
 */
const TOKEN_TYPES = [
  'support.class',
  'variable.function',
  'type',
  'variable',
  'identifier',
  'keyword',
  'comment',
  'string',
  'number',

  'operator',
] as const;
type TokenType = (typeof TOKEN_TYPES)[number];

const TOKEN_MODIFIERS = [
  'declaration',
  'definition',
  'readonly',
  'static',
  'deprecated',
  'abstract',
  'async',
  'modification',
  'documentation',
  'defaultLibrary',
];
