import { CommentNode, FieldNode, HkannoNode, MotionNode, Pos, RotationNode, SpaceNode, TextNode } from './nodes';

type ParserState = {
  line: string;
  i: number;
  lineNumber: number;
  len: number;
};

const makePos = (lineNumber: number, start: number, end: number): Pos => ({
  line: lineNumber,
  startColumn: start + 1,
  endColumn: end + 1,
});

const parseSpace = (state: ParserState): SpaceNode | undefined => {
  const start = state.i;
  while (state.i < state.len && /\s/.test(state.line[state.i])) state.i++;
  if (state.i > start) {
    return {
      kind: 'space',
      rawText: state.line.slice(start, state.i),
      pos: makePos(state.lineNumber, start, state.i),
    };
  }
  return undefined;
};

const parseNumberField = (state: ParserState): FieldNode<number> | undefined => {
  const start = state.i;
  if (state.i >= state.len) return undefined;

  let str = '';
  if (state.line[state.i] === '+' || state.line[state.i] === '-') str += state.line[state.i++];

  while (state.i < state.len && /[0-9.]/.test(state.line[state.i])) str += state.line[state.i++];

  if (state.i < state.len && (state.line[state.i] === 'e' || state.line[state.i] === 'E')) {
    str += state.line[state.i++];
    if (state.line[state.i] === '+' || state.line[state.i] === '-') str += state.line[state.i++];
    while (state.i < state.len && /[0-9]/.test(state.line[state.i])) str += state.line[state.i++];
  }

  if (!str) return undefined;
  return { value: Number(str), pos: makePos(state.lineNumber, start, state.i) };
};

const parseLiteralField = <T extends string>(state: ParserState, literal: T): FieldNode<T> | undefined => {
  const start = state.i;
  if (state.line.slice(start, start + literal.length) === literal) {
    state.i += literal.length;
    return { value: literal, pos: makePos(state.lineNumber, start, state.i) };
  }
  return undefined;
};

/**
 * Parse a literal starting at `state.i` until one of the stop characters is reached.
 * Returns the consumed string and its position.
 *
 * @param state Parser state with current line and cursor
 * @param untilChars String containing all characters to stop at (e.g. '\n' or ' ')
 * @returns FieldNode<string> or undefined if nothing consumed
 *
 * @example
 * // line = "# comment text"
 * parseLiteralFieldUntil(state, '\n')
 * // returns { value: "# comment text", pos: { line, startColumn, endColumn } }
 */
const parseLiteralFieldUntil = (state: ParserState, untilChars: string): FieldNode<string> | undefined => {
  const start = state.i;
  let value = '';
  while (state.i < state.len && !untilChars.includes(state.line[state.i])) {
    value += state.line[state.i++];
  }
  if (!value) return undefined;
  return { value, pos: makePos(state.lineNumber, start, state.i) };
};

/**
 * Parse a single comment line.
 * # Pattern
 * `<space0> # <text>`
 */
export const parseCommentLine = (line: string, lineNumber = 1): CommentNode => {
  const state: ParserState = { line, i: 0, lineNumber, len: line.length };
  const node: CommentNode = { kind: 'comment' };

  node.space0First = parseSpace(state);

  node.hash = parseLiteralField(state, '#');
  node.space0HashToComment = parseSpace(state);
  node.comment = parseLiteralFieldUntil(state, '\n');
  node.space0AfterComment = parseSpace(state);

  return node;
};

/**
 * Parse a single animrotation line.
 * # Pattern
 * `<space0> <time> <space1> <text> <space0>`
 */
export const parseTextLine = (line: string, lineNumber = 1): TextNode => {
  const state: ParserState = { line, i: 0, lineNumber, len: line.length };
  const node: TextNode = { kind: 'text' };

  node.space0First = parseSpace(state);
  node.time = parseNumberField(state);
  node.space1TimeToText = parseSpace(state);
  node.text = parseLiteralFieldUntil(state, '\n');
  node.space0AfterText = parseSpace(state);

  return node;
};

/**
 * Parse a single animrotation line.
 *
 * # Pattern
 * `<space0> <time> <space1> animmotion <space1> <x: f32> <space1> <y: f32> <space1> <z: f32> <space0>`
 */
export const parseAnimMotionLine = (line: string, lineNumber = 1): MotionNode => {
  const state: ParserState = { line, i: 0, lineNumber, len: line.length };
  const node: MotionNode = { kind: 'motion' };

  node.space0First = parseSpace(state);
  node.time = parseNumberField(state);
  node.space1TimeToEvent = parseSpace(state);
  node.event = parseLiteralField(state, 'animmotion');
  node.space1EventToX = parseSpace(state);
  node.x = parseNumberField(state);
  node.space1XToY = parseSpace(state);
  node.y = parseNumberField(state);
  node.space1YToZ = parseSpace(state);
  node.z = parseNumberField(state);
  node.space0AfterZ = parseSpace(state);

  return node;
};

/**
 * Parse a single animrotation line.
 *
 * # Pattern
 * `<space0> <time> <space1> animrotation <space1> <degrees> <space0>`
 */
export function parseAnimRotationLine(line: string, lineNumber = 1): RotationNode {
  const state: ParserState = { line, i: 0, lineNumber, len: line.length };
  const node: RotationNode = { kind: 'rotation' };

  node.space0First = parseSpace(state);
  node.time = parseNumberField(state);
  node.space1TimeToEvent = parseSpace(state);
  node.event = parseLiteralField(state, 'animrotation');
  node.space1EventToDegrees = parseSpace(state);
  node.degrees = parseNumberField(state);
  node.space0AfterDegrees = parseSpace(state);

  return node;
}

/**
 * Parse a single hkanno line and return the appropriate Node.
 * Delegates to specialized parsers based on the content.
 *
 * @param line A single line of hkanno text
 * @param lineNumber The line number (1-based)
 * @returns HkannoNode (RotationNode | MotionNode | TextNode | CommentNode)
 */
export const parseHkannoLine = (line: string, lineNumber = 1): HkannoNode => {
  const trimmed = line.trimStart();

  // Comment line starts with #
  if (trimmed.startsWith('#')) {
    return parseCommentLine(line, lineNumber);
  }

  // animrotation line
  if (trimmed.includes('animrotation')) {
    return parseAnimRotationLine(line, lineNumber);
  }

  // animmotion line
  if (trimmed.includes('animmotion')) {
    return parseAnimMotionLine(line, lineNumber);
  }

  // Fallback: treat as text line
  return parseTextLine(line, lineNumber);
};
