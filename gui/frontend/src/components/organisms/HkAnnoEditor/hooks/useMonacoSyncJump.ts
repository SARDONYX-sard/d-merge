import type * as monaco from 'monaco-editor';
import { useRef } from 'react';

/**
 * useMonacoSyncJump (index-based jump)
 *
 * Synchronizes cursor movement between two Monaco Editors (left and right)
 * based on line *index* (order of appearance) rather than trackName text.
 *
 * When the cursor moves on the left editor:
 *  - If it's on a `trackName:` line → jumps to the corresponding <hkparam name="trackName"> line on the right.
 *  - If it's on a time (float) line → jumps to the corresponding <hkparam name="time"> line on the right.
 */
export const useMonacoSyncJump = () => {
  const leftEditorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  const rightEditorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);

  /** Mapping tables: left line → right line */
  const trackLineMapRef = useRef<Map<number, number>>(new Map());
  const timeLineMapRef = useRef<Map<number, number>>(new Map());

  /** Register editors */
  const registerLeft = (editor: monaco.editor.IStandaloneCodeEditor) => {
    leftEditorRef.current = editor;
    setupLeftCursorSync(editor);
  };

  const registerRight = (editor: monaco.editor.IStandaloneCodeEditor) => {
    rightEditorRef.current = editor;
  };

  /**
   * Update both mapping tables whenever left or right text changes.
   */
  const updateBaseLine = (leftText: string, xmlText: string) => {
    const { trackMap, timeMap } = buildIndexMaps(leftText, xmlText);
    trackLineMapRef.current = trackMap;
    timeLineMapRef.current = timeMap;
  };

  /**
   * Cursor listener on the left editor — jumps to corresponding line on the right.
   */
  const setupLeftCursorSync = (editor: monaco.editor.IStandaloneCodeEditor) => {
    const trackNameRegex = /^\s*trackName\s*:?\s*(.+)?$/i;
    editor.onDidChangeCursorPosition((e) => {
      const right = rightEditorRef.current;
      if (!right) return;

      const model = editor.getModel();
      if (!model) return;

      const lineNum = e.position.lineNumber;
      const line = model.getLineContent(lineNum).trim();

      const trackMap = trackLineMapRef.current;
      const timeMap = timeLineMapRef.current;

      // --- Handle "trackName:" lines ---
      if (trackNameRegex.test(line)) {
        // NOTE: We considered using a range, but abandoned it because it would prevent O(1) retrieval.
        const target = trackMap.get(lineNum);
        if (target) {
          right.revealLineInCenter(target);
          right.setPosition({ lineNumber: target, column: 1 });
        }
        return;
      }

      // --- Handle "time" (float) lines ---
      const first = parseFloat(line.split(/\s+/)[0]);
      if (!Number.isNaN(first)) {
        const target = timeMap.get(lineNum);
        if (target) {
          right.revealLineInCenter(target);
          right.setPosition({ lineNumber: target, column: 1 });
        }
      }
    });
  };

  return { registerLeft, registerRight, updateBaseLine };
};

/**
 * Build mapping tables between left and right editors by index order.
 *
 * Each occurrence of:
 *  - `trackName:` (left) ↔ <hkparam name="trackName"> (right)
 *  - time line (left) ↔ <hkparam name="time"> (right)
 */
const buildIndexMaps = (leftText: string, xmlText: string) => {
  const leftLines = leftText.split(/\r?\n/);
  const xmlLines = xmlText.split(/\r?\n/);

  const trackMap = new Map<number, number>();
  const timeMap = new Map<number, number>();

  // --- Collect all relevant line numbers on the left ---
  const leftTrackLines: number[] = [];
  const leftTimeLines: number[] = [];

  leftLines.forEach((line, i) => {
    if (line.trim().startsWith('trackName')) {
      leftTrackLines.push(i + 1);
    } else if (!isNaN(parseFloat(line.trim().split(/\s+/)[0]))) {
      leftTimeLines.push(i + 1);
    }
  });

  // --- Collect all relevant line numbers on the right ---
  const rightTrackLines: number[] = [];
  const rightTimeLines: number[] = [];

  xmlLines.forEach((line, i) => {
    if (/<hkparam name="trackName">/.test(line)) {
      rightTrackLines.push(i + 1);
    } else if (/<hkparam name="time">/.test(line)) {
      rightTimeLines.push(i + 1);
    }
  });

  // --- Create index-based mappings ---
  const minTrackCount = Math.min(leftTrackLines.length, rightTrackLines.length);
  for (let i = 0; i < minTrackCount; i++) {
    trackMap.set(leftTrackLines[i], rightTrackLines[i]);
  }

  const minTimeCount = Math.min(leftTimeLines.length, rightTimeLines.length);
  for (let i = 0; i < minTimeCount; i++) {
    timeMap.set(leftTimeLines[i], rightTimeLines[i]);
  }

  return { trackMap, timeMap };
};
