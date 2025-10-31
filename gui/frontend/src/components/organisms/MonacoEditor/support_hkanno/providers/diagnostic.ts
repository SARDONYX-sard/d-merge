import type { OnMount } from '@monaco-editor/react';
import type * as monaco from 'monaco-editor';
import { HKANNO_LANGUAGE_ID } from '..';
import { PIE_NATIVE_INSTRUCTIONS } from '../parser/payload_interpreter/completion';
import type { PayloadInstructionNode } from '../parser/payload_interpreter/nodes';
import { IFrameNode } from '../parser/strict/nodes';
import { parseHkannoLineExt } from '../parser/strict/parser';

export const registerCodeLen: OnMount = (editor, monacoEnv) => {
  // first
  if (editor.getModel()?.getLanguageId() === HKANNO_LANGUAGE_ID) {
    updateHkannoDiagnostics(editor, monacoEnv);
  }
  editor.onDidChangeModelContent(() => {
    if (editor.getModel()?.getLanguageId() === HKANNO_LANGUAGE_ID) {
      updateHkannoDiagnostics(editor, monacoEnv);
    }
  });
};

const updateHkannoDiagnostics: OnMount = (editor, monacoEnv) => {
  const model = editor.getModel();
  if (!model) return;

  const markers: monaco.editor.IMarkerData[] = [];
  const lines = model.getLinesContent();

  for (let lineNumber = 1; lineNumber <= lines.length; lineNumber++) {
    const line = lines[lineNumber - 1];
    const node = parseHkannoLineExt(line, lineNumber);

    if (node.kind == 'iframe') {
      markers.push(...provideIFrameDiagnostics(node, lineNumber, line.length, monacoEnv));
    }

    // --- motion ---
    if (node.kind === 'motion') {
      (['x', 'y', 'z'] as const).forEach((axis) => {
        if (node[axis]?.value === undefined) {
          const pos = node[axis]?.pos;
          const startCol = pos?.startColumn ?? 1;
          const endCol = pos?.endColumn ?? line.length + 1;
          markers.push({
            severity: monacoEnv.MarkerSeverity.Error,
            message: `Missing ${axis} value in animmotion.`,
            startLineNumber: lineNumber,
            endLineNumber: lineNumber,
            startColumn: startCol,
            endColumn: endCol,
          });
        }
      });
    }

    // --- rotation ---
    if (node.kind === 'rotation') {
      if (node.degrees?.value === undefined) {
        const startCol = node.degrees?.pos?.startColumn ?? 1;
        const endCol = node.degrees?.pos?.endColumn ?? line.length + 1;
        markers.push({
          severity: monacoEnv.MarkerSeverity.Error,
          message: `Missing degrees value in animrotation.`,
          startLineNumber: lineNumber,
          endLineNumber: lineNumber,
          startColumn: startCol,
          endColumn: endCol,
        });
      }
    }

    // --- payload instruction (PIE) ---
    if (node.kind === 'payload_instruction') {
      const pieNode = node as PayloadInstructionNode;
      const name = pieNode.instruction?.name?.value?.toUpperCase();
      if (name) {
        const def = PIE_NATIVE_INSTRUCTIONS.find((i) => i.name.toUpperCase() === name);
        if (def) {
          const provided = pieNode.instruction?.parameters?.items.filter((item) => item.value).length ?? 0;
          const expected = (def.snippet.match(/\$\{[0-9]+:/g) ?? []).length;
          if (provided < expected) {
            const startCol = pieNode.instruction?.prefix?.pos?.startColumn ?? 1;
            const endCol =
              pieNode.instruction?.parameters?.pos?.endColumn ??
              pieNode.instruction?.name?.pos?.endColumn ??
              line.length + 1;
            markers.push({
              severity: monacoEnv.MarkerSeverity.Error,
              message: `PIE instruction '${name}' expects ${expected} parameters, but ${provided} provided.`,
              startLineNumber: lineNumber,
              endLineNumber: lineNumber,
              startColumn: startCol,
              endColumn: endCol,
            });
          }
        }
      }
    }

    if (node.kind === 'text') {
      if (node.time && !node.text?.value) {
        const startCol = node.space1TimeToText?.pos?.startColumn ?? node.time?.pos?.endColumn ?? 1;
        const endCol = node.space0AfterText?.pos?.endColumn ?? line.length + 1;
        markers.push({
          severity: monacoEnv.MarkerSeverity.Warning,
          message: `Text annotation is missing.`,
          startLineNumber: lineNumber,
          endLineNumber: lineNumber,
          startColumn: startCol,
          endColumn: endCol,
        });
      }
    }
  }

  monacoEnv.editor.setModelMarkers(model, 'hkanno-diagnostics', markers);
};

const provideIFrameDiagnostics = (
  node: IFrameNode,
  lineNumber: number,
  lineLength: number,
  monacoEnv: typeof monaco,
): monaco.editor.IMarkerData[] => {
  const markers: monaco.editor.IMarkerData[] = [];

  // time
  if (node.time?.value === undefined) {
    const startCol = node.time?.pos?.startColumn ?? 1;
    const endCol = node.time?.pos?.endColumn ?? lineLength + 1;
    markers.push({
      severity: monacoEnv.MarkerSeverity.Error,
      message: `Missing time value in IFrame event.`,
      startLineNumber: lineNumber,
      endLineNumber: lineNumber,
      startColumn: startCol,
      endColumn: endCol,
    });
  }

  // event
  if (node.event?.value !== 'SpecialFrames_Invincible') {
    const startCol = node.event?.pos?.startColumn ?? 1;
    const endCol = node.event?.pos?.endColumn ?? lineLength + 1;
    markers.push({
      severity: monacoEnv.MarkerSeverity.Error,
      message: `Invalid IFrame event name (expected 'SpecialFrames_Invincible').`,
      startLineNumber: lineNumber,
      endLineNumber: lineNumber,
      startColumn: startCol,
      endColumn: endCol,
    });
  }

  // JSON parse error
  if (node.jsonParseError) {
    const startCol = node.json?.pos?.startColumn ?? 1;
    const endCol = node.json?.pos?.endColumn ?? lineLength + 1;
    markers.push({
      severity: monacoEnv.MarkerSeverity.Error,
      message: `IFrame JSON parse error: ${node.jsonParseError.message}`,
      startLineNumber: lineNumber,
      endLineNumber: lineNumber,
      startColumn: startCol,
      endColumn: endCol,
    });
  } else if (node.json?.value === undefined) {
    const startCol = node.json?.pos?.startColumn ?? 1;
    const endCol = node.json?.pos?.endColumn ?? lineLength + 1;
    markers.push({
      severity: monacoEnv.MarkerSeverity.Error,
      message: `IFrame JSON is missing or invalid. Only "Duration" is supported (IFrame Generator RE v1.03).`,
      startLineNumber: lineNumber,
      endLineNumber: lineNumber,
      startColumn: startCol,
      endColumn: endCol,
    });
  }

  // JSON object contains unsupported keys
  if (node.json?.value) {
    const keys = Object.keys(node.json.value);

    for (const key of keys) {
      if (key !== 'Duration') {
        const startCol = node.json?.pos?.startColumn ?? 1;
        const endCol = node.json?.pos?.endColumn ?? lineLength + 1;
        markers.push({
          severity: monacoEnv.MarkerSeverity.Warning,
          message: `Possible typo: "${key}". Did you mean "Duration"? (IFrame Generator RE v1.03 supports only "Duration")`,
          startLineNumber: lineNumber,
          endLineNumber: lineNumber,
          startColumn: startCol,
          endColumn: endCol,
          code: 'fix-iframe-key',
        });
      }
    }
  }

  return markers;
};
