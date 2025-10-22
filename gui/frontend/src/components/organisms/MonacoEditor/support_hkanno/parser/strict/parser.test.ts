import { describe, expect, it } from 'vitest';
import type { CommentNode, IFrameNode, MotionNode, RotationNode, TextNode } from './nodes';
import { parseAnimMotionLine, parseAnimRotationLine, parseCommentLine, parseIFrameLine, parseTextLine } from './parser';

describe('Hkanno parsers', () => {
  it('parses a comment line with spaces', () => {
    const line = ' # This is a comment';
    const node = parseCommentLine(line, 1);

    expect(node).toEqual({
      kind: 'comment',
      space0First: { kind: 'space', rawText: ' ', pos: { line: 1, startColumn: 1, endColumn: 2 } },
      hash: { value: '#', pos: { line: 1, startColumn: 2, endColumn: 3 } },
      space0HashToComment: { kind: 'space', rawText: ' ', pos: { line: 1, startColumn: 3, endColumn: 4 } },
      comment: { value: 'This is a comment', pos: { line: 1, startColumn: 4, endColumn: 21 } },
      space0AfterComment: undefined,
    } as const satisfies CommentNode);
  });

  it('parses a text line with spaces', () => {
    const line = ' 2.5 Hello World';
    const node = parseTextLine(line, 1);

    expect(node).toEqual({
      kind: 'text',
      space0First: { kind: 'space', rawText: ' ', pos: { line: 1, startColumn: 1, endColumn: 2 } },
      time: { value: 2.5, pos: { line: 1, startColumn: 2, endColumn: 5 } },
      space1TimeToText: { kind: 'space', rawText: ' ', pos: { line: 1, startColumn: 5, endColumn: 6 } },
      text: { value: 'Hello World', pos: { line: 1, startColumn: 6, endColumn: 17 } },
      space0AfterText: undefined,
    } as const satisfies TextNode);
  });

  it('parses a complete animmotion line with spaces', () => {
    const line = ' 1.0 animmotion 1.0 2.0 3.0 ';
    const node = parseAnimMotionLine(line, 1);

    expect(node).toEqual({
      kind: 'motion',
      space0First: { kind: 'space', rawText: ' ', pos: { line: 1, startColumn: 1, endColumn: 2 } },
      time: { value: 1.0, pos: { line: 1, startColumn: 2, endColumn: 5 } },
      space1TimeToEvent: { kind: 'space', rawText: ' ', pos: { line: 1, startColumn: 5, endColumn: 6 } },
      event: { value: 'animmotion', pos: { line: 1, startColumn: 6, endColumn: 16 } },
      space1EventToX: { kind: 'space', rawText: ' ', pos: { line: 1, startColumn: 16, endColumn: 17 } },
      x: { value: 1.0, pos: { line: 1, startColumn: 17, endColumn: 20 } },
      space1XToY: { kind: 'space', rawText: ' ', pos: { line: 1, startColumn: 20, endColumn: 21 } },
      y: { value: 2.0, pos: { line: 1, startColumn: 21, endColumn: 24 } },
      space1YToZ: { kind: 'space', rawText: ' ', pos: { line: 1, startColumn: 24, endColumn: 25 } },
      z: { value: 3.0, pos: { line: 1, startColumn: 25, endColumn: 28 } },
      space0AfterZ: { kind: 'space', rawText: ' ', pos: { line: 1, startColumn: 28, endColumn: 29 } },
    } as const satisfies MotionNode);
  });
});

describe('parseAnimRotationLine', () => {
  it('parses a complete line with spaces correctly', () => {
    const line = ' 0.5 animrotation 90 ';
    const node = parseAnimRotationLine(line, 1);

    expect(node).toEqual({
      kind: 'rotation',
      space0First: { rawText: ' ', kind: 'space', pos: { line: 1, startColumn: 1, endColumn: 2 } },
      time: { value: 0.5, pos: { line: 1, startColumn: 2, endColumn: 5 } },
      space1TimeToEvent: { rawText: ' ', kind: 'space', pos: { line: 1, startColumn: 5, endColumn: 6 } },
      event: { value: 'animrotation', pos: { line: 1, startColumn: 6, endColumn: 18 } },
      space1EventToDegrees: { rawText: ' ', kind: 'space', pos: { line: 1, startColumn: 18, endColumn: 19 } },
      degrees: { value: 90, pos: { line: 1, startColumn: 19, endColumn: 21 } },
      space0AfterDegrees: { rawText: ' ', kind: 'space', pos: { line: 1, startColumn: 21, endColumn: 22 } },
    } as const satisfies RotationNode);
  });

  it('parses line without optional leading/trailing spaces', () => {
    const line = '0.5 animrotation 90';
    const node = parseAnimRotationLine(line, 1);

    expect(node).toEqual({
      kind: 'rotation',
      space0First: undefined,
      time: { value: 0.5, pos: { line: 1, startColumn: 1, endColumn: 4 } },
      space1TimeToEvent: { rawText: ' ', kind: 'space', pos: { line: 1, startColumn: 4, endColumn: 5 } },
      event: { value: 'animrotation', pos: { line: 1, startColumn: 5, endColumn: 17 } },
      space1EventToDegrees: { rawText: ' ', kind: 'space', pos: { line: 1, startColumn: 17, endColumn: 18 } },
      degrees: { value: 90, pos: { line: 1, startColumn: 18, endColumn: 20 } },
      space0AfterDegrees: undefined,
    } as const satisfies RotationNode);
  });

  it('handles missing mandatory spaces gracefully', () => {
    const line = '0.5animrotation90';
    const node = parseAnimRotationLine(line, 1);

    expect(node).toEqual({
      kind: 'rotation',
      space0First: undefined,
      time: { value: 0.5, pos: { line: 1, startColumn: 1, endColumn: 4 } },
      space1TimeToEvent: undefined,
      event: { value: 'animrotation', pos: { line: 1, startColumn: 4, endColumn: 16 } },
      space1EventToDegrees: undefined,
      degrees: { value: 90, pos: { line: 1, startColumn: 16, endColumn: 18 } },
      space0AfterDegrees: undefined,
    } as const satisfies RotationNode);
  });

  it('handles negative numbers and decimals', () => {
    const line = '  -1.25 animrotation -90.5 ';
    const node = parseAnimRotationLine(line, 1);

    expect(node).toEqual({
      kind: 'rotation',
      space0First: { rawText: '  ', kind: 'space', pos: { line: 1, startColumn: 1, endColumn: 3 } },
      time: { value: -1.25, pos: { line: 1, startColumn: 3, endColumn: 8 } },
      space1TimeToEvent: { rawText: ' ', kind: 'space', pos: { line: 1, startColumn: 8, endColumn: 9 } },
      event: { value: 'animrotation', pos: { line: 1, startColumn: 9, endColumn: 21 } },
      space1EventToDegrees: { rawText: ' ', kind: 'space', pos: { line: 1, startColumn: 21, endColumn: 22 } },
      degrees: { value: -90.5, pos: { line: 1, startColumn: 22, endColumn: 27 } },
      space0AfterDegrees: { rawText: ' ', kind: 'space', pos: { line: 1, startColumn: 27, endColumn: 28 } },
    } as const satisfies RotationNode);
  });
});

describe('parseIFrameLine', () => {
  it('parses a simple IFrame line', () => {
    const line = '0.500000 SpecialFrames_Invincible{"Duration":0.5}';
    const node = parseIFrameLine(line, 1);

    expect(node).toEqual({
      kind: 'iframe',
      space0First: undefined,
      time: { value: 0.5, pos: { line: 1, startColumn: 1, endColumn: 9 } },
      space1TimeToEvent: { rawText: ' ', kind: 'space', pos: { line: 1, startColumn: 9, endColumn: 10 } },
      event: { value: 'SpecialFrames_Invincible', pos: { line: 1, startColumn: 10, endColumn: 34 } },
      json: { value: { Duration: 0.5 }, pos: { line: 1, startColumn: 34, endColumn: 50 } },
      space0AfterJson: undefined,
    } as const satisfies IFrameNode);
  });

  it('parses IFrame line with leading/trailing spaces', () => {
    const line = '  0.25 SpecialFrames_Invincible{"Duration":1} ';
    const node = parseIFrameLine(line, 1);

    expect(node.space0First?.rawText).toBe('  ');
    expect(node.json?.value).toEqual({ Duration: 1 });
    expect(node.space0AfterJson?.rawText).toBeUndefined();
  });

  it('handles invalid JSON gracefully', () => {
    const line = '0.5 SpecialFrames_Invincible{invalid}';
    const node = parseIFrameLine(line, 1);
    expect(node.json?.value).toBeUndefined();
    expect(node.jsonParseError).toBeDefined();
  });
});
