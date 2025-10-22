import * as monaco from 'monaco-editor';
import { HKANNO_LANGUAGE_ID } from '..';
import type { PayloadInstructionNode } from '../parser/payload_interpreter/nodes';
import type {
  FieldNode,
  HkannoNodeExt,
  IFrameNode,
  MotionNode,
  RotationNode,
  TextNode,
  TrackNameNode,
} from '../parser/strict/nodes';
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
    case 'trackName':
      return hoverTrackName(node);
    case 'motion':
      return hoverMotion(node, cursorColumn);
    case 'rotation':
      return hoverRotation(node, cursorColumn);
    case 'text':
      return hoverText(node);
    case 'iframe':
      return hoverIFrame(node, cursorColumn);
    case 'payload_instruction':
      return hoverPie(node, cursorColumn);
    default:
      return null;
  }
};

const hoverMotion = (node: MotionNode, cursorColumn: number) => {
  if (isCursorInside(node.event, cursorColumn)) {
    return `# Anim Motion
Applies linear motion to the animation.
- required: [Animation Motion Revolution](https://www.nexusmods.com/skyrimspecialedition/mods/50258)

# Format

\`\`\`hkanno
animmotion <x: f32> <y: f32> <z: f32>
\`\`\``;
  }

  const x = node.x?.value ?? UNKNOWN;
  const y = node.y?.value ?? UNKNOWN;
  const z = node.z?.value ?? UNKNOWN;
  const time = node.time?.value ?? UNKNOWN;
  return `# animmotion values
- Time: ${time}s
- X: ${x}
- Y: ${y}
- Z: ${z}`;
};

const hoverRotation = (node: RotationNode, cursorColumn: number) => {
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
};

const hoverText = (node: TextNode) => {
  const text = node.text?.value ?? UNKNOWN;
  const time = node.time?.value ?? UNKNOWN;
  return `# Text annotation
- Time: ${time}s
- Text: \`${text}\``;
};

const hoverIFrame = (node: IFrameNode, cursorColumn: number) => {
  if (isCursorInside(node.event, cursorColumn)) {
    return `# Invincibility Frames Annotation
- required: [IFrame Generator RE](https://www.nexusmods.com/skyrimspecialedition/mods/74401)
- See: [tutorial](https://github.com/max-su-2019/MaxsuIFrame/blob/main/doc/en/tutorial.md)

# Format

- As of v1.03, it appears to support only \`Duration\`.

\`\`\`hkanno
SpecialFrames_Invincible{"Duration": <value: f32>}
\`\`\`

# Example

Invincible for 0.1 to 0.5 seconds

\`\`\`hkanno
0.1 SpecialFrames_Invincible{"Duration": 0.5}
\`\`\`
`;
  }

  const time = node.time?.value ?? UNKNOWN;
  const json = node.json?.value ? JSON.stringify(node.json.value, null, 2) : `<${node.jsonParseError?.message}>`;
  return ` # I-Frame value
- Time: ${time}s
- JSON Data:
  \`\`\`json
  ${json}
  \`\`\`

`;
};

const hoverPie = (node: PayloadInstructionNode, cursorColumn: number) => {
  const pie = node;

  // Hover on PIE keyword
  if (isCursorInside(pie.event, cursorColumn)) {
    return `# Payload Interpreter Dummy event (PIE)
Payload instruction.
- required: [Payload Interpreter](https://www.nexusmods.com/skyrimspecialedition/mods/65089)
- See: [Reference](https://github.com/D7ry/PayloadInterpreter?tab=readme-ov-file#list-of-instructions)

# Format
- Native instruction
  \`\`\`hkanno
  PIE.@<instruction>|<param1>|<param2>|...
  \`\`\`

- Custom instruction
  \`\`\`hkanno
  PIE.$KEY|instruction1|instruction2|...
  \`\`\`

- Async instruction
  \`\`\`hkanno
  PIE.$[time]<rest>
  \`\`\`
`;
  }

  // Hover on instruction or params
  const name = pie.instruction?.name?.value ?? UNKNOWN;
  const params = pie.instruction?.parameters?.items.map((p) => p.value?.value ?? UNKNOWN) ?? [];
  const getPrefixKindDisplay = (ch?: '@' | '$' | '!') => {
    switch (ch) {
      case '@':
        return 'Native';
      case '$':
        return 'Custom';
      case '!':
        return 'Async';
      default:
        return;
    }
  };
  const kind = getPrefixKindDisplay(pie.instruction?.prefix?.value);

  return [
    `# PIE Instruction`,
    `- Kind: \`${kind}\``,
    `- Name: \`${name}\``,
    params.length ? `- Parameters: ${params.join(' | ')}` : '- No parameters',
  ].join('\n');
};

const hoverTrackName = (node: TrackNameNode) => {
  const name = node.name?.value ?? '<unnamed>';
  return `# Annotation Track
This defines a named annotation track. All following annotations belong to this track until the next trackName or end of file.

- Track name: \`${name}\``;
};
