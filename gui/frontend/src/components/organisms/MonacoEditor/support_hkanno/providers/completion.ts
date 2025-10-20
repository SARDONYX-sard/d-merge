import * as monaco from 'monaco-editor';
import { HKANNO_LANGUAGE_ID } from '..';
import { providePieCompletions } from '../parser/payload_interpreter/completion';
import { parseHkannoLine } from '../parser/strict/parser';

export const registerCompletionProvider = (monacoEnv: typeof monaco) => {
  monacoEnv.languages.registerCompletionItemProvider(HKANNO_LANGUAGE_ID, {
    triggerCharacters: [' ', '.', '0', '@'],
    provideCompletionItems(document, position) {
      const lineContent = document.getLineContent(position.lineNumber);
      const node = parseHkannoLine(lineContent, position.lineNumber);

      const suggestions: monaco.languages.CompletionItem[] = [];

      const range = {
        startLineNumber: position.lineNumber,
        endLineNumber: position.lineNumber,
        startColumn: position.column,
        endColumn: lineContent.length + 1,
      };

      // ---------------------------
      // CommentNode
      // ---------------------------
      if (node.kind === 'comment') {
        return {
          suggestions: [
            {
              label: '# numAnnotations:',
              kind: monaco.languages.CompletionItemKind.Keyword,
              insertText: '# numAnnotations: 0',
              range,
              documentation: {
                value: `\`\`\`hkanno
# numAnnotations: <number>
\`\`\`
Declare the number of annotations in this document.`,
                isTrusted: true,
              },
            },
          ],
        };
      }

      // ---------------------------
      // MotionNode
      // ---------------------------
      if (node.kind === 'motion') {
        ['x', 'y', 'z'].forEach((axis, i) => {
          const arg = [node.x, node.y, node.z][i];
          if (!arg?.value) {
            suggestions.push({
              label: axis,
              kind: monaco.languages.CompletionItemKind.Value,
              insertText: '0.0',
              range,
              documentation: {
                value: `\`\`\`hkanno
<time: f32> animmotion <x: f32> <y: f32> <z: f32>
\`\`\`
Set the ${axis.toUpperCase()} coordinate for the animmotion event.`,
                isTrusted: true,
              },
            });
          }
        });

        return { suggestions };
      }

      // ---------------------------
      // RotationNode
      // ---------------------------
      if (node.kind === 'rotation') {
        if (!node.degrees?.value) {
          suggestions.push({
            label: 'degrees',
            kind: monaco.languages.CompletionItemKind.Value,
            insertText: '0.0',
            range,
            documentation: {
              value: `\`\`\`hkanno
<time: f32> animrotation <degrees: f32>
\`\`\`
Insert an animrotation event with a rotation in degrees.`,
              isTrusted: true,
            },
          });
        }
        return { suggestions };
      }

      // ---------------------------
      // TextNode (fallback)
      // ---------------------------
      if (node.kind === 'text') {
        const hasTime = node.time;

        if (!hasTime) {
          suggestions.push(
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
            {
              label: '# numAnnotations:',
              kind: monaco.languages.CompletionItemKind.Keyword,
              insertText: '# numAnnotations: 0',
              sortText: '<x',
              range,
              documentation: {
                value: `\`\`\`hkanno
# numAnnotations: <number>
\`\`\`
Declare the number of annotations in this document.`,
                isTrusted: true,
              },
            },
          );
        }

        if (hasTime) {
          suggestions.push(
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
              label: 'SoundPlay',
              kind: monaco.languages.CompletionItemKind.Function,
              insertText: 'SoundPlay.',
              range,
              documentation: {
                value: `\`\`\`hkanno
SoundPlay.<event>
\`\`\`
Play a sound effect on the actor
`,
                isTrusted: true,
              },
            },
            {
              label: 'animmotion',
              kind: monaco.languages.CompletionItemKind.Function,
              insertText: 'animmotion',
              range,
              documentation: {
                value: `\`\`\`hkanno
animmotion <x: f32> <y: f32> <z: f32>
\`\`\`
Insert an animmotion event with X, Y, Z coordinates.
(Need \`AMR\` Mod)
`,
                isTrusted: true,
              },
            },
            {
              label: 'animrotation',
              kind: monaco.languages.CompletionItemKind.Function,
              insertText: 'animrotation',
              range,
              documentation: {
                value: `\`\`\`hkanno
animrotation <degrees: f32>
\`\`\`
Insert an animrotation event with a rotation in degrees.
(Need \`AMR\` Mod)
`,
                isTrusted: true,
              },
            },
            {
              label: 'PIE',
              kind: monaco.languages.CompletionItemKind.Function,
              insertText: 'PIE.',
              range,
              documentation: {
                value: `\`\`\`hkanno
<time: f32> PIE.@<inst>|...
\`\`\`
Dummy event that hosts payload instructions, does nothing by itself
(Need \`PayloadInterpreter\` Mod)
`,
                isTrusted: true,
              },
            },
          );
        }

        return { suggestions };
      }

      if (node.kind === 'payload_instruction') {
        suggestions.push(...providePieCompletions(node, range));
      }

      return { suggestions };
    },
  });
};
