import { OnMount } from '@monaco-editor/react';
import type * as monaco from 'monaco-editor';
import { HKANNO_LANGUAGE_ID } from '..';
import { parseHkannoLine } from '../parser';

/**
 * Registers hkanno signature help provider
 * @param editor Monaco editor instance
 * @param monaco Monaco namespace
 */
export const registerSignatureHelpProvider: OnMount = (_editor, monacoNS) => {
  const provider: monaco.languages.SignatureHelpProvider = {
    signatureHelpTriggerCharacters: [' ', '.', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9'],

    provideSignatureHelp(model, position) {
      const lineNumber = position.lineNumber;
      const lineContent = model.getLineContent(lineNumber);
      const beforeCursor = lineContent.slice(0, position.column - 1);

      const parsed = parseHkannoLine(beforeCursor, lineNumber);

      // Meta line (#) → # numAnnotations / # numOriginalFrames
      if (parsed.type === 'meta') {
        return valueOf(
          '# numAnnotations: <usize>',
          'Push hkaSplineCompressedAnimation.annotationTracks. (e.g., # numAnnotations: 3)',
          'numAnnotations',
        );
      }

      // 2️⃣ Time is present → <time: float>
      if (!parsed.timeComplete) {
        return valueOf('<time: f32>', 'Timestamp in seconds (e.g., 0.100000)', 'time');
      }

      // 4️⃣ Verb detected → AnimMotion / AnimRotation
      const verb = parsed.eventName?.toLowerCase();
      if (verb === 'animmotion') {
        return verbSignature(
          'animmotion <x: f32> <y: f32> <z: f32>',
          'Applies linear motion offset to the animation.',
          ['x', 'y', 'z'],
          parsed.args?.length ?? 0,
        );
      }

      if (verb === 'animrotation') {
        return verbSignature(
          'animrotation <angle: f32>',
          'Applies a rotation (in degrees) to the animation.',
          ['angle'],
          parsed.args?.length ?? 0,
        );
      }

      // Text after time (free text, not verb) → <text: string>
      if (parsed.timeComplete) {
        return valueOf(
          '<text: string>',
          'Annotation label or event name (e.g., `MCO_DodgeOpen`, `animmotion`, `animrotation`)',
          'text',
        );
      }

      // Fallback → nothing
      return None();
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

const verbSignature = (
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

const None = (): monaco.languages.SignatureHelpResult => ({
  value: { signatures: [], activeSignature: 0, activeParameter: 0 },
  dispose() {},
});
