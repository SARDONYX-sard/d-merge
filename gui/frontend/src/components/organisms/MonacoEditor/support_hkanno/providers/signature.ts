import { OnMount } from '@monaco-editor/react';
import type * as monaco from 'monaco-editor';
import { HKANNO_LANGUAGE_ID } from '..';
import { providePieSignatureHelp } from '../parser/payload_interpreter/signature';
import { parseHkannoLineExt } from '../parser/strict/parser';

export const registerSignatureHelpProvider: OnMount = (_editor, monacoNS) => {
  const provider: monaco.languages.SignatureHelpProvider = {
    signatureHelpTriggerCharacters: [' ', '.', '0'],

    provideSignatureHelp(model, position) {
      const lineNumber = position.lineNumber;
      const lineContent = model.getLineContent(lineNumber);
      const beforeCursor = lineContent.slice(0, position.column - 1);

      const node = parseHkannoLineExt(beforeCursor, lineNumber);

      // PIE instructions
      if (node.kind === 'payload_instruction') {
        return providePieSignatureHelp(node);
      }

      // Motion event
      if (node.kind === 'motion') {
        const activeParameter: 0 | 1 | 2 = (() => {
          if (!node.x?.pos) return 0;
          const cursorCol = position.column;

          if (node.x.pos.endColumn >= cursorCol) return 0;
          if (node.y?.pos === undefined || node.y.pos.endColumn >= cursorCol) return 1;
          if (node.z?.pos === undefined || node.z.pos.endColumn >= cursorCol) return 2;
          return 2;
        })();

        return fnSignature(
          'animmotion <x: f32> <y: f32> <z: f32>',
          'Applies linear motion offset to the animation.',
          ['x', 'y', 'z'],
          activeParameter,
        );
      }

      // Rotation event
      if (node.kind === 'rotation') {
        const argsProvided = node.degrees?.value !== undefined ? 1 : 0;
        return fnSignature(
          'animrotation <angle: f32>',
          'Applies a rotation (in degrees) to the animation.',
          ['angle'],
          argsProvided,
        );
      }

      // Fallback Text line
      if (node.kind === 'text') {
        if (!node.time || !node.space1TimeToText) {
          return valueOf('<time: f32>', 'Timestamp in seconds (e.g., 0.100000)', 'time');
        }

        if (node.space1TimeToText) {
          return valueOf(
            '<text: string>',
            'Annotation label or event name (e.g., `MCO_DodgeOpen`, `animmotion`, `animrotation`)',
            'text',
          );
        }
      }
    },
  };

  monacoNS.languages.registerSignatureHelpProvider(HKANNO_LANGUAGE_ID, provider);
  return provider;
};

/* --- helpers --- */
const valueOf = (label: string, doc: string, paramLabel: string): monaco.languages.SignatureHelpResult => ({
  value: {
    signatures: [{ label, documentation: undefined, parameters: [{ label: paramLabel, documentation: doc }] }],
    activeSignature: 0,
    activeParameter: 0,
  },
  dispose() {},
});

const fnSignature = (
  label: string,
  doc: string,
  params: string[],
  activeParam: number,
): monaco.languages.SignatureHelpResult => ({
  value: {
    signatures: [
      { label, documentation: doc, parameters: params.map((p) => ({ label: p, documentation: `${p} value` })) },
    ],
    activeSignature: 0,
    activeParameter: Math.max(0, Math.min(activeParam, params.length - 1)),
  },
  dispose() {},
});
