import * as monaco from 'monaco-editor';
import { HKANNO_LANGUAGE_ID } from '..';
import { PayloadInstructionNode } from '../parser/payload_interpreter/nodes';
import type { FieldNode, HkannoNodeExt } from '../parser/strict/nodes';
import { parseHkannoLineExt } from '../parser/strict/parser';

const UNKNOWN = '<unknown>';

export const registerHoverProvider = (monacoEnv: typeof monaco) => {
  monacoEnv.languages.registerHoverProvider(HKANNO_LANGUAGE_ID, {
    provideHover(model, position) {
      const lineContent = model.getLineContent(position.lineNumber);
      const node = parseHkannoLineExt(lineContent, position.lineNumber);

      const markdown = buildHoverMarkdown(node, position.column);
      if (!markdown) return null;

      return { contents: [{ value: markdown }] };
    },
  });
};

const isCursorInside = <T extends string>(field: FieldNode<T> | undefined, column: number) =>
  field?.pos && column >= field.pos.startColumn && column <= field.pos.endColumn;

const buildHoverMarkdown = (node: HkannoNodeExt, cursorColumn: number): string | null => {
  switch (node.kind) {
    case 'motion': {
      // Hover on keyword
      if (isCursorInside(node.event, cursorColumn)) {
        return `# Anim Motion
Applies linear motion to the animation.
- required: [Animation Motion Revolution](https://www.nexusmods.com/skyrimspecialedition/mods/50258)

# Format
\`\`\`hkanno
animmotion <x: f32> <y: f32> <z: f32>
\`\`\``;
      }

      // Hover on values
      const x = node.x?.value ?? UNKNOWN;
      const y = node.y?.value ?? UNKNOWN;
      const z = node.z?.value ?? UNKNOWN;
      const time = node.time?.value ?? UNKNOWN;
      return `# animmotion values
- Time: ${time}s
- X: ${x}
- Y: ${y}
- Z: ${z}`;
    }

    case 'rotation': {
      if (isCursorInside(node.event, cursorColumn)) {
        return `# Anim Rotation
Applies rotation to the animation.
- required: [Animation Motion Revolution](https://www.nexusmods.com/skyrimspecialedition/mods/50258)

# Format
\`\`\`hkanno
animrotation <degrees: f32>
\`\`\``;
      }

      const deg = node.degrees?.value ?? UNKNOWN;
      const time = node.time?.value ?? UNKNOWN;
      return `# animrotation value
- Time: ${time}s
- Degrees: ${deg}°`;
    }

    case 'text': {
      const text = node.text?.value ?? UNKNOWN;
      const time = node.time?.value ?? UNKNOWN;
      return `# Text annotation
- Time: ${time}s
- Text: \`${text}\``;
    }

    case 'payload_instruction': {
      const pie = node as PayloadInstructionNode;

      // Hover on PIE keyword
      if (isCursorInside(pie.event, cursorColumn)) {
        return `# Payload Interpreter Dummy event (PIE)
Payload instruction.
- required: [Payload Interpreter](https://www.nexusmods.com/skyrimspecialedition/mods/65089)
- See: [Reference](https://github.com/D7ry/PayloadInterpreter?tab=readme-ov-file#list-of-instructions)

# Format
\`\`\`hkanno
PIE.@<instruction>|<param1>|<param2>|...
\`\`\``;
      }

      // Hover on instruction or params
      const name = pie.instruction?.name?.value ?? UNKNOWN;
      const params = pie.instruction?.parameters?.items.map((p) => p.value?.value ?? UNKNOWN) ?? [];
      return [
        `# PIE Instruction`,
        `- Name: \`${name}\``,
        params.length ? `- Parameters: ${params.join(' | ')}` : '- No parameters',
      ].join('\n');
    }

    default:
      return null;
  }
};
