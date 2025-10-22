import * as monaco from 'monaco-editor';
import { HKANNO_LANGUAGE_ID } from '..';
import { providePieCompletions } from '../parser/payload_interpreter/completion';
import type { MotionNode, Pos, RotationNode, TextNode } from '../parser/strict/nodes';
import { parseHkannoLineExt } from '../parser/strict/parser';

export const registerCompletionProvider = (monacoEnv: typeof monaco) => {
  monacoEnv.languages.registerCompletionItemProvider(HKANNO_LANGUAGE_ID, {
    triggerCharacters: [' ', '.', '@'],
    provideCompletionItems(document, position) {
      const lineContent = document.getLineContent(position.lineNumber);
      const node = parseHkannoLineExt(lineContent, position.lineNumber);
      const cursorNumber = position.column;

      const range = {
        startLineNumber: position.lineNumber,
        endLineNumber: position.lineNumber,
        startColumn: position.column,
        endColumn: lineContent.length + 1,
      };

      switch (node.kind) {
        case 'motion':
          return { suggestions: provideMotionCompletions(node, range, cursorNumber) };
        case 'rotation':
          return { suggestions: provideRotationCompletions(node, range) };
        case 'text':
          return { suggestions: provideTextCompletions(node, range, monacoEnv) };
        case 'iframe':
          // return { suggestions: provideIFrameCompletions(node, range) };
          return { suggestions: [] };
        case 'payload_instruction':
          return { suggestions: providePieCompletions(node, range) };
        default:
          return { suggestions: [] };
      }
    },
  });
};

const hkannoSnippets = [
  {
    label: '# numAnnotations:',
    insertText: '# numAnnotations: ${1:usize}',
    documentation: '```hkanno\n# numAnnotations: <number>\n```\nDeclare the number of annotations in this document.',
  },
  {
    label: 'trackName:',
    insertText: 'trackName: ${1:Name}',
    documentation: '```hkanno\ntrackName: <name>\n```\nDeclare the track name for this annotation track.',
  },
] as const;

const newStartSnippets = (range: monaco.IRange) =>
  hkannoSnippets.map(
    (snip) =>
      ({
        label: snip.label,
        kind: monaco.languages.CompletionItemKind.Keyword,
        insertText: snip.insertText,
        insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
        range,
        documentation: {
          value: snip.documentation,
          isTrusted: true,
        },
        sortText: 'z', // This brings the candidate to the very bottom.
      }) as const,
  ) satisfies readonly monaco.languages.CompletionItem[];

/** Check if the cursor is immediately after the specified node */
export const isAfter = (pos: Pos | undefined, range: monaco.IRange) => {
  if (!pos) return false;
  return range.startLineNumber === pos.line && range.startColumn >= pos.endColumn;
};

type ValueField<T extends string> = { label: T; value?: number | string; spaceBefore?: Pos };
const suggestIfMissing = <T extends string>(
  field: ValueField<T>,
  range: monaco.IRange,
  documentation: string,
): monaco.languages.CompletionItem[] => {
  if (!field.value && field.spaceBefore) {
    return [
      {
        label: field.label,
        kind: monaco.languages.CompletionItemKind.Value,
        insertText: typeof field.value === 'number' || field.value === undefined ? '0.0' : '',
        range,
        documentation: { value: documentation, isTrusted: true },
      },
    ];
  }
  return [];
};

const provideMotionCompletions = (
  node: MotionNode,
  range: monaco.IRange,
  cursorColumn: number,
): monaco.languages.CompletionItem[] => {
  // X
  const isXRange =
    cursorColumn >= (node.space1EventToX?.pos?.endColumn ?? 0) &&
    cursorColumn < (node.space1XToY?.pos?.startColumn ?? Infinity);

  if (isXRange) {
    return [
      {
        label: 'x',
        kind: monaco.languages.CompletionItemKind.Value,
        insertText: String(node.x?.value ?? 0.0),
        range,
        documentation: { value: 'X coordinate for animmotion event.', isTrusted: true },
      },
    ];
  }

  // Y
  const isYRange =
    cursorColumn >= (node.space1XToY?.pos?.endColumn ?? 0) &&
    cursorColumn < (node.space1YToZ?.pos?.startColumn ?? Infinity);

  if (isYRange) {
    return [
      {
        label: 'y',
        kind: monaco.languages.CompletionItemKind.Value,
        insertText: String(node.y?.value ?? 0.0),
        range,
        documentation: { value: 'Y coordinate for animmotion event.', isTrusted: true },
      },
    ];
  }

  // Z
  const isZRange = cursorColumn >= (node.space1YToZ?.pos?.endColumn ?? 0);

  if (isZRange) {
    return [
      {
        label: 'z',
        kind: monaco.languages.CompletionItemKind.Value,
        insertText: String(node.z?.value ?? 0.0),
        range,
        documentation: { value: 'Z coordinate for animmotion event.', isTrusted: true },
      },
    ];
  }

  return [];
};

const provideRotationCompletions = (node: RotationNode, range: monaco.IRange) => {
  return suggestIfMissing(
    { label: 'degrees', value: node.degrees?.value, spaceBefore: node.space1EventToDegrees?.pos },
    range,
    `<time: f32> animrotation <degrees: f32>\nInsert an animrotation event with a rotation in degrees`,
  );
};

const provideTextCompletions = (
  node: TextNode,
  range: monaco.IRange,
  monacoEnv: typeof monaco,
): monaco.languages.CompletionItem[] => {
  if (!node.time) {
    return [
      {
        label: '<time>',
        kind: monaco.languages.CompletionItemKind.Value,
        insertText: '0.0',
        range,
        documentation: {
          value: `\`\`\`hkanno
<time: f32>
\`\`\`
The timestamp at which this annotation occurs.`,
          isTrusted: true,
        },
      },
      ...newStartSnippets(range),
    ];
  }

  if (!isAfter(node.space1TimeToText?.pos, range)) return [];

  return [
    {
      label: '<eventName>',
      kind: monaco.languages.CompletionItemKind.Snippet,
      insertText: '${1:eventName}',
      insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
      range,
      documentation: {
        value: `\`\`\`hkanno
\${1:eventName}
\`\`\`
Annotation text event name(e.g. \`weaponSwing\`).`,
        isTrusted: true,
      },
    },
    {
      label: 'animmotion',
      kind: monaco.languages.CompletionItemKind.Function,
      insertText: 'animmotion ${1:0.0} ${2:0.0} ${3:0.0}',
      insertTextRules: monacoEnv.languages.CompletionItemInsertTextRule.InsertAsSnippet,
      range,
      documentation: {
        value: `\`\`\`hkanno
animmotion <x: f32> <y: f32> <z: f32>
\`\`\`
Insert an animmotion event with X, Y, Z coordinates.
(Need \`AMR\` Mod)`,
        isTrusted: true,
      },
    },
    {
      label: 'animrotation',
      kind: monaco.languages.CompletionItemKind.Function,
      insertText: 'animrotation ${1:0}',
      insertTextRules: monacoEnv.languages.CompletionItemInsertTextRule.InsertAsSnippet,
      range,
      documentation: {
        value: `\`\`\`hkanno
animrotation <degrees: f32>
\`\`\`
Insert an animrotation event with a rotation in degrees.
(Need \`AMR\` Mod)`,
        isTrusted: true,
      },
    },
    {
      label: 'SpecialFrames_Invincible',
      kind: monaco.languages.CompletionItemKind.Function,
      insertText: 'SpecialFrames_Invincible{"Duration":${1:0.5}}',
      insertTextRules: monacoEnv.languages.CompletionItemInsertTextRule.InsertAsSnippet,
      range,
      documentation: {
        value: `\`\`\`hkanno
SpecialFrames_Invincible{"<key>": <value>}
\`\`\`
Insert an IFrame event (Duration supported).`,
        isTrusted: true,
      },
    },
  ];
};
