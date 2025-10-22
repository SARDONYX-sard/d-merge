import type * as monaco from 'monaco-editor';
import { useRef } from 'react';

type AnnotationPos = Readonly<{ trackName: string; lines: number[] }>;

/**
 * useMonacoSyncJump (with cached annotation line map)
 *
 * Synchronizes cursor movement between two Monaco Editors (left and right),
 * using a precomputed map of annotation lines for fast jumps.
 */
export function useMonacoSyncJump() {
  const leftEditorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  const rightEditorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);

  /** Cached annotation line map: [trackName, lineNumbers[]] */
  const annotationMapRef = useRef<AnnotationPos[]>([]);

  /** Register the left Monaco editor */
  const registerLeft = (editor: monaco.editor.IStandaloneCodeEditor) => {
    leftEditorRef.current = editor;
    setupLeftCursorSync(editor);
  };

  /** Register the right Monaco editor */
  const registerRight = (editor: monaco.editor.IStandaloneCodeEditor) => {
    rightEditorRef.current = editor;
  };

  /**
   * Update base line and rebuild annotation map.
   * Call this whenever the XML preview is updated.
   */
  const updateBaseLine = (xmlText: string) => {
    const base = findAnimationBaseLine(xmlText);
    annotationMapRef.current = buildAnnotationMap(xmlText, base);
  };

  /** Listen for left cursor movement and jump right editor */
  const setupLeftCursorSync = (editor: monaco.editor.IStandaloneCodeEditor) => {
    editor.onDidChangeCursorPosition((e) => {
      const right = rightEditorRef.current;
      const map = annotationMapRef.current;
      if (!right || !map.length) return;

      const model = editor.getModel();
      if (!model) return;

      // Count which annotation index the cursor is at
      let annotationIndex = 0;
      for (let i = 1; i < e.position.lineNumber; i++) {
        const line = model.getLineContent(i).trim();
        if (!isNaN(parseFloat(line))) annotationIndex++;
      }

      // Find target line from cached map
      let accumulated = 0;
      for (const track of map) {
        if (annotationIndex < accumulated + track.lines.length) {
          const targetLine = track.lines[annotationIndex - accumulated];
          right.revealLineInCenter(targetLine);
          right.setPosition({ lineNumber: targetLine, column: 1 });
          return;
        }
        accumulated += track.lines.length;
      }
    });
  };

  return { registerLeft, registerRight, updateBaseLine };
}

/** Finds <hkaSplineCompressedAnimation> base line */
const findAnimationBaseLine = (xmlText: string): number => {
  const lines = xmlText.split(/\r?\n/);
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].includes(' class="hkaSplineCompressedAnimation" signature="0x792ee0bb">')) return i + 1;
  }
  return 0;
};

/** Build cached annotation map: trackName -> line numbers of <hkparam name="time"> */
const buildAnnotationMap = (xmlText: string, baseLine: number): AnnotationPos[] => {
  const lines = xmlText.split(/\r?\n/);
  const map: { trackName: string; lines: number[] }[] = [];

  let currentTrack: { trackName: string; lines: number[] } | null = null;

  for (let i = baseLine; i < lines.length; i++) {
    const line = lines[i];

    const trackMatch = line.match(/<hkparam name="trackName">(.*)<\/hkparam>/);
    if (trackMatch) {
      if (currentTrack) map.push(currentTrack);
      currentTrack = { trackName: trackMatch[1], lines: [] };
      continue;
    }

    const timeMatch = line.match(/<hkparam name="time">([\d.]+)<\/hkparam>/);
    if (timeMatch && currentTrack) {
      currentTrack.lines.push(i + 1); // Monaco is 1-based
    }
  }

  if (currentTrack) map.push(currentTrack);
  return map;
};
