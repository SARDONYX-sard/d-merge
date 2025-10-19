import * as monaco from 'monaco-editor';
import { type ParsedHkanno, parseHkannoLine } from '../parser';

export function registerHoverProvider(monacoEnv: typeof monaco) {
  monacoEnv.languages.registerHoverProvider('hkanno', {
    provideHover(model, position) {
      const lineContent = model.getLineContent(position.lineNumber);
      const parsed = parseHkannoLine(lineContent, position.lineNumber);

      if (parsed.type === 'none') return null;

      // markdown化
      const markdown = buildHoverMarkdown(parsed);
      if (!markdown) return null;

      return { contents: [{ value: markdown }] };
    },
  });
}

/**
 * Build markdown hover text from ParsedHkanno result
 */
function buildHoverMarkdown(parsed: ParsedHkanno): string | null {
  switch (parsed.type) {
    case 'meta':
      return `**${parsed.eventName}**: ${parsed.rawText}`;
    case 'motion': {
      const [x, y, z] = parsed.args ?? [];
      return [`**animmotion** — Translation at ${parsed.time}s`, '', `- X: ${x}`, `- Y: ${y}`, `- Z: ${z}`].join('\n');
    }
    case 'rotation': {
      const [deg] = parsed.args ?? [];
      return [`**animrotation** — Rotation at ${parsed.time}s`, '', `- Degrees: ${deg}°`].join('\n');
    }
    // case 'text':
    //   return parsed.verb ? `**Event**: ${parsed.verb}` : `**Text**: ${parsed.rawText}`;
    case 'invalid':
      return `⚠ **Invalid line**\n\n${parsed.errors?.join('\n')}`;
    default:
      return null;
  }
}
