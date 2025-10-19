import * as monaco from 'monaco-editor';

export type TokenPos = {
  line: number;
  startColumn: number;
  length: number;
};

export type HkannoType = 'meta' | 'motion' | 'rotation' | 'text' | 'none' | 'invalid';

export type ParsedHkanno = {
  type: HkannoType;
  time?: number;
  rawText?: string;
  verb?: string | null;
  args?: number[];
  tokenPositions?: { time: TokenPos; verb?: TokenPos; argPositions?: TokenPos[] };
  errors?: string[];
};

/**
 * Parse a single hkanno line (no trailing newline required)
 */
export function parseHkannoLine(line: string, lineNumber = 1): ParsedHkanno {
  const res: ParsedHkanno = { type: 'none', errors: [] };
  if (!line || line.trim() === '') return res;

  let i = 0;
  const trimmed = line.replace(/\r$/, '');
  const len = trimmed.length;

  // Helper: skip whitespace
  const skipWhitespace = () => {
    while (i < len && /\s/.test(trimmed[i])) i++;
  };

  skipWhitespace();

  // Meta line: # key: value
  if (trimmed[i] === '#') {
    i++;
    skipWhitespace();
    const keyStart = i;
    while (i < len && trimmed[i] !== ':' && !/\s/.test(trimmed[i])) i++;
    const keyEnd = i;
    skipWhitespace();
    if (trimmed[i] === ':') i++;
    skipWhitespace();
    const valueStart = i;
    const key = trimmed.slice(keyStart, keyEnd);
    const value = trimmed.slice(valueStart);
    res.type = 'meta';
    res.verb = key;
    res.rawText = value;
    res.tokenPositions = {
      time: { line: lineNumber, startColumn: 1, length: 0 },
      verb: { line: lineNumber, startColumn: keyStart + 1, length: keyEnd - keyStart },
      argPositions: [],
    };
    return res;
  }

  // Parse time
  const timeStart = i;
  while (i < len && !/\s/.test(trimmed[i])) i++;
  const timeToken = trimmed.slice(timeStart, i);
  const timeVal = Number(timeToken);
  if (!Number.isFinite(timeVal)) {
    res.type = 'invalid';
    res.rawText = trimmed.slice(timeStart);
    res.errors!.push(`Invalid time token: "${timeToken}"`);
    return res;
  }
  res.time = timeVal;
  res.tokenPositions = { time: { line: lineNumber, startColumn: timeStart + 1, length: i - timeStart } };

  skipWhitespace();

  if (i >= len) {
    res.type = 'text';
    res.rawText = '';
    return res;
  }

  // Parse verb token
  const verbStart = i;
  while (i < len && !/\s/.test(trimmed[i])) i++;
  const verbEnd = i;
  const verb = trimmed.slice(verbStart, verbEnd);
  res.verb = verb;
  res.tokenPositions.verb = { line: lineNumber, startColumn: verbStart + 1, length: verbEnd - verbStart };

  skipWhitespace();

  const args: number[] = [];
  const argPositions: TokenPos[] = [];
  const verbLower = verb.toLowerCase();

  // Helper: parse numbers char by char
  const parseNumber = (): { value: number; start: number; end: number } | null => {
    skipWhitespace();
    if (i >= len) return null;
    const start = i;
    let numStr = '';
    if (trimmed[i] === '+' || trimmed[i] === '-') numStr += trimmed[i++];
    while (i < len && /[0-9.]/.test(trimmed[i])) numStr += trimmed[i++];
    if (i < len && (trimmed[i] === 'e' || trimmed[i] === 'E')) {
      numStr += trimmed[i++];
      if (i < len && (trimmed[i] === '+' || trimmed[i] === '-')) numStr += trimmed[i++];
      while (i < len && /[0-9]/.test(trimmed[i])) numStr += trimmed[i++];
    }
    const value = Number(numStr);
    if (!Number.isFinite(value)) return null;
    return { value, start, end: i };
  };

  if (verbLower === 'animmotion') {
    res.type = 'motion';
    for (let k = 0; k < 3; k++) {
      const n = parseNumber();
      if (n) {
        args.push(n.value);
        argPositions.push({ line: lineNumber, startColumn: n.start + 1, length: n.end - n.start });
      }
    }
    res.args = args;
    res.tokenPositions.argPositions = argPositions;
    res.rawText = trimmed.slice(verbEnd + 1);
    return res;
  }

  if (verbLower === 'animrotation') {
    res.type = 'rotation';
    const n = parseNumber();
    if (n) {
      args.push(n.value);
      argPositions.push({ line: lineNumber, startColumn: n.start + 1, length: n.end - n.start });
      res.args = args;
      res.tokenPositions.argPositions = argPositions;
      res.rawText = trimmed.slice(verbEnd + 1);
    } else {
      res.type = 'invalid';
      res.errors!.push('animrotation missing numeric angle');
    }
    return res;
  }

  // fallback: treat remainder as text
  res.type = 'text';
  res.rawText = trimmed.slice(verbEnd + 1);
  return res;
}

export function registerHkannoHover(monacoEnv: typeof monaco) {
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
      return `**${parsed.verb}**: ${parsed.rawText}`;
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
