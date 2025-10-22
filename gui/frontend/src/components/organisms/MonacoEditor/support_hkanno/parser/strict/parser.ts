import { stringToJsonSchema } from '@/lib/zod/json-validation';
import { parsePayloadInstructionLine } from '../payload_interpreter/parser';
import {
  CommentNode,
  FieldNode,
  HkannoNode,
  HkannoNodeExt,
  IFrameNode,
  MotionNode,
  Pos,
  RotationNode,
  SpaceNode,
  TextNode,
} from './nodes';

export type ParserState = {
  line: string;
  i: number;
  lineNumber: number;
  len: number;
};

export const makePos = (lineNumber: number, start: number, end: number): Pos => ({
  line: lineNumber,
  startColumn: start + 1,
  endColumn: end + 1,
});

export const parseSpace = (state: ParserState): SpaceNode | undefined => {
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

export const parseNumberField = (state: ParserState): FieldNode<number> | undefined => {
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
 * @param stopChars String containing all characters to stop at (e.g. '\n' or ' ')
 * @returns FieldNode<string> or undefined if nothing consumed
 *
 * @example
 * // line = "# comment text"
 * parseLiteralFieldUntil(state, '\n')
 * // returns { value: "# comment text", pos: { line, startColumn, endColumn } }
 */
export const parseFieldUntil = (state: ParserState, stopChars: string): FieldNode<string> | undefined => {
  const start = state.i;
  let value = '';
  while (state.i < state.len && !stopChars.includes(state.line[state.i])) {
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
  node.comment = parseFieldUntil(state, '\n');
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
  node.text = parseFieldUntil(state, '\n');
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

export const parseIFrameLine = (line: string, lineNumber = 1): IFrameNode => {
  const state: ParserState = { line, i: 0, lineNumber, len: line.length };
  const node: IFrameNode = { kind: 'iframe' };

  node.space0First = parseSpace(state);
  node.time = parseNumberField(state);
  node.space1TimeToEvent = parseSpace(state);
  node.event = parseLiteralField(state, 'SpecialFrames_Invincible');
  node.json = parseFieldUntil(state, '\n');
  if (node.json?.value) {
    const result = stringToJsonSchema.safeParse(node.json.value);
    if (result.success) {
      node.json.value = result.data;
    } else {
      node.json.value = undefined;
      node.jsonParseError = result.error;
    }
  }

  node.space0AfterJson = parseSpace(state);

  return node;
};

const commonParsers = [
  {
    check: (line: string) => line.trimStart().startsWith('#'),
    parser: parseCommentLine,
  },
  {
    check: (line: string) => line.toLowerCase().includes('specialframes_invincible'),
    parser: parseIFrameLine,
  },
  {
    check: (line: string) => line.toLowerCase().includes('animrotation'),
    parser: parseAnimRotationLine,
  },
  {
    check: (line: string) => line.toLowerCase().includes('animmotion'),
    parser: parseAnimMotionLine,
  },
];

const pieParser = {
  check: (line: string) => line.toLowerCase().includes('pie'),
  parser: parsePayloadInstructionLine,
};

/**
 * Parse a single hkanno line and return the appropriate Node.(With PIE.)
 * Delegates to specialized parsers based on the content.
 *
 * @param line A single line of hkanno text
 * @param lineNumber The line number (1-based)
 */
export const parseHkannoLineExt = (line: string, lineNumber = 1): HkannoNodeExt => {
  const parsers = [pieParser, ...commonParsers];
  for (const { check, parser } of parsers) {
    if (check(line)) return parser(line, lineNumber);
  }
  // Fallback: text line
  return parseTextLine(line, lineNumber);
};

/**
 * Parser with lenient parsing for formatter applications, no PIE
 *
 * @param line A single line of hkanno text
 * @param lineNumber The line number (1-based)
 */
export const parseHkannoLine = (line: string, lineNumber = 1): HkannoNode => {
  for (const { check, parser } of commonParsers) {
    if (check(line)) return parser(line, lineNumber);
  }
  // Fallback: text line
  return parseTextLine(line, lineNumber);
};
