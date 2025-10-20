import type * as monaco from 'monaco-editor';
import { useRef } from 'react';

/**
 * useMonacoSyncJump
 *
 * Synchronizes cursor movement between two Monaco Editors (left and right).
 * When the cursor in the left editor moves, the right editor automatically
 * jumps to the corresponding <hkparam name="time"> line in the XML preview.
 *
 * Assumptions:
 * - <hkaSplineCompressedAnimation> line is the base anchor.
 * - The first <hkparam name="time"> appears 13 lines below that base.
 * - Each annotation block consists of 4 lines.
 */
export function useMonacoSyncJump() {
  const leftEditorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  const rightEditorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  const baseLineRef = useRef<number | null>(null);

  /** Register the left Monaco editor and attach cursor sync logic */
  const registerLeft = (editor: monaco.editor.IStandaloneCodeEditor) => {
    leftEditorRef.current = editor;
    setupLeftCursorSync(editor);
  };

  /** Register the right Monaco editor */
  const registerRight = (editor: monaco.editor.IStandaloneCodeEditor) => {
    rightEditorRef.current = editor;
  };

  /** Detects the base line of <hkaSplineCompressedAnimation> in the XML */
  const updateBaseLine = (xmlText: string) => {
    const base = findAnimationBaseLine(xmlText);
    baseLineRef.current = base;
  };

  /** Internal: listens for left cursor movement and jumps right editor */
  const setupLeftCursorSync = (editor: monaco.editor.IStandaloneCodeEditor) => {
    editor.onDidChangeCursorPosition((e) => {
      const model = editor.getModel();
      const right = rightEditorRef.current;
      const baseLine = baseLineRef.current;
      if (!model || !right || baseLine == null) return;

      const BASE_OFFSET = 13;
      const BLOCK_SIZE = 4;

      let index = 0;
      for (let i = 1; i < e.position.lineNumber; i++) {
        const l = model.getLineContent(i).trim();
        if (!isNaN(parseFloat(l))) index++;
      }

      const targetLine = baseLine + BASE_OFFSET + BLOCK_SIZE * index;

      right.revealLineInCenter(targetLine);
      right.setPosition({ lineNumber: targetLine, column: 1 });
    });
  };

  return { registerLeft, registerRight, updateBaseLine };
}

/**
 * Finds the line number of <hkaSplineCompressedAnimation> in the XML text.
 */
function findAnimationBaseLine(xmlText: string): number {
  const lines = xmlText.split(/\r?\n/);
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].includes(' class="hkaSplineCompressedAnimation" signature="0x792ee0bb">')) return i + 1;
  }
  return 0;
}
