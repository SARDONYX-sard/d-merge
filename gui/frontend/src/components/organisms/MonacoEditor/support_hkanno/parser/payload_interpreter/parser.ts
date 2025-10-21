import { makePos, ParserState, parseFieldUntil, parseNumberField, parseSpace } from '../strict/parser';
import type { InstructionNode, ParameterItemNode, ParametersNode, PayloadInstructionNode } from './nodes';

/**
 * Parse PIE.@INSTRUCTION|param1|param2|... line
 */
export const parsePayloadInstructionLine = (line: string, lineNumber = 1): PayloadInstructionNode => {
  const state: ParserState = { line, i: 0, lineNumber, len: line.length };
  const node: PayloadInstructionNode = { kind: 'payload_instruction' };

  node.space0First = parseSpace(state);
  node.time = parseNumberField(state);
  node.space1TimeToEvent = parseSpace(state);

  // --- Parse event (usually "PIE")
  const eventField = parseFieldUntil(state, '.');
  if (eventField) node.event = eventField;

  // --- Parse dot
  if (state.line[state.i] === '.') {
    node.dot = { value: '.', pos: makePos(state.lineNumber, state.i, state.i + 1) };
    state.i++;
  }

  // --- Parse '@'
  const ch = state.line[state.i];
  if (ch === '@' || ch === '$' || ch === '!') {
    const instruction: InstructionNode = { kind: 'instruction' };
    instruction.prefix = { value: ch, pos: makePos(state.lineNumber, state.i, state.i + 1) };
    state.i++;

    // --- Parse instruction name until '|' or end
    const nameField = parseFieldUntil(state, '|');
    if (nameField) instruction.name = nameField;

    // --- Parse parameters
    if (state.i < state.len && state.line[state.i] === '|') {
      const params: ParametersNode = { kind: 'parameters', items: [] };
      const paramStart = state.i;

      while (state.i < state.len) {
        const paramItem: ParameterItemNode = {};

        // separator
        if (state.line[state.i] === '|') {
          paramItem.separator = {
            value: '|',
            pos: makePos(state.lineNumber, state.i, state.i + 1),
          };
          state.i++;
        }

        // parameter value
        const valueField = parseFieldUntil(state, '|');
        if (valueField) paramItem.value = valueField;

        params.items.push(paramItem);
      }

      params.pos = makePos(state.lineNumber, paramStart, state.i);
      instruction.parameters = params;
    }

    node.instruction = instruction;
  }

  return node;
};
