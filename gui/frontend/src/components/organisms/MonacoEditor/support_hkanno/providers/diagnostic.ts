import type { OnMount } from '@monaco-editor/react';
import * as monaco from 'monaco-editor';
import { type ParsedHkanno, parseHkannoLine } from '../parser';

export const updateHkannoDiagnostics: OnMount = (editor, monacoEnv) => {
  const model = editor.getModel();
  if (!model) {
    return;
  }

  const markers: monaco.editor.IMarkerData[] = [];
  const lines = model.getLinesContent();

  for (let lineNumber = 1; lineNumber <= lines.length; lineNumber++) {
    const line = lines[lineNumber - 1];
    const parsed: ParsedHkanno = parseHkannoLine(line, lineNumber);

    if (parsed.errors && parsed.errors.length) {
      for (const err of parsed.errors) {
        markers.push({
          severity: monaco.MarkerSeverity.Error,
          message: err,
          startLineNumber: lineNumber,
          endLineNumber: lineNumber,
          startColumn: 1,
          endColumn: line.length + 1,
        });
      }
    }

    // Additional structural checks
    if (parsed.type === 'motion' && parsed.args?.length !== 3) {
      markers.push({
        severity: monaco.MarkerSeverity.Error,
        message: `animmotion requires exactly 3 numeric arguments (x, y, z). Found ${parsed.args?.length ?? 0}.`,
        startLineNumber: lineNumber,
        endLineNumber: lineNumber,
        startColumn: 1,
        endColumn: line.length + 1,
      });
    }

    if (parsed.type === 'rotation' && parsed.args?.length !== 1) {
      markers.push({
        severity: monaco.MarkerSeverity.Error,
        message: `animrotation requires exactly 1 numeric degree argument.`,
        startLineNumber: lineNumber,
        endLineNumber: lineNumber,
        startColumn: 1,
        endColumn: line.length + 1,
      });
    }
  }

  monacoEnv.editor.setModelMarkers(model, 'hkanno-diagnostics', markers);
};
