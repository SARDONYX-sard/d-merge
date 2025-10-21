import { describe, expect, it } from 'vitest';
import type { PayloadInstructionNode } from './nodes';
import { parsePayloadInstructionLine } from './parser';

describe('parsePayloadInstructionLine', () => {
  it('parses a full payload instruction with multiple parameters', () => {
    const line = '0.3 PIE.@CASTSPELL|0X001|Apocalypse.esp|1|2.0|0|100|10|0|0|0|0';
    const result = parsePayloadInstructionLine(line, 1);

    expect(result).toMatchObject({
      kind: 'payload_instruction',
      space0First: undefined,
      time: {
        value: 0.3,
        pos: {
          line: 1,
          startColumn: 1,
          endColumn: 4,
        },
      },
      space1TimeToEvent: {
        kind: 'space',
        rawText: ' ',
        pos: {
          line: 1,
          startColumn: 4,
          endColumn: 5,
        },
      },
      event: {
        value: 'PIE',
        pos: { line: 1, startColumn: 5, endColumn: 8 },
      },
      dot: {
        value: '.',
        pos: { line: 1, startColumn: 8, endColumn: 9 },
      },
      instruction: {
        kind: 'instruction',
        prefix: {
          value: '@',
          pos: { line: 1, startColumn: 9, endColumn: 10 },
        },
        name: {
          value: 'CASTSPELL',
          pos: { line: 1, startColumn: 10, endColumn: 19 },
        },
        parameters: {
          kind: 'parameters',
          items: [
            {
              separator: { value: '|', pos: { line: 1, startColumn: 19, endColumn: 20 } },
              value: { value: '0X001', pos: { line: 1, startColumn: 20, endColumn: 25 } },
            },
            {
              separator: { value: '|', pos: { line: 1, startColumn: 25, endColumn: 26 } },
              value: { value: 'Apocalypse.esp', pos: { line: 1, startColumn: 26, endColumn: 40 } },
            },
            {
              separator: { value: '|', pos: { line: 1, startColumn: 40, endColumn: 41 } },
              value: { value: '1', pos: { line: 1, startColumn: 41, endColumn: 42 } },
            },
            {
              separator: { value: '|', pos: { line: 1, startColumn: 42, endColumn: 43 } },
              value: { value: '2.0', pos: { line: 1, startColumn: 43, endColumn: 46 } },
            },
            {
              separator: { value: '|', pos: { line: 1, startColumn: 46, endColumn: 47 } },
              value: { value: '0', pos: { line: 1, startColumn: 47, endColumn: 48 } },
            },
            {
              separator: { value: '|', pos: { line: 1, startColumn: 48, endColumn: 49 } },
              value: { value: '100', pos: { line: 1, startColumn: 49, endColumn: 52 } },
            },
            {
              separator: { value: '|', pos: { line: 1, startColumn: 52, endColumn: 53 } },
              value: { value: '10', pos: { line: 1, startColumn: 53, endColumn: 55 } },
            },
            {
              separator: { value: '|', pos: { line: 1, startColumn: 55, endColumn: 56 } },
              value: { value: '0', pos: { line: 1, startColumn: 56, endColumn: 57 } },
            },
            {
              separator: { value: '|', pos: { line: 1, startColumn: 57, endColumn: 58 } },
              value: { value: '0', pos: { line: 1, startColumn: 58, endColumn: 59 } },
            },
            {
              separator: { value: '|', pos: { line: 1, startColumn: 59, endColumn: 60 } },
              value: { value: '0', pos: { line: 1, startColumn: 60, endColumn: 61 } },
            },
            {
              separator: { value: '|', pos: { line: 1, startColumn: 61, endColumn: 62 } },
              value: { value: '0', pos: { line: 1, startColumn: 62, endColumn: 63 } },
            },
          ],
        },
      },
    } as const satisfies PayloadInstructionNode);
  });
});
