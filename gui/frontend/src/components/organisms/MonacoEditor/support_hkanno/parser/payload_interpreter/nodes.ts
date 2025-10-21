import type { FieldNode, Pos, SpaceNode } from '../strict/nodes';

/**
 * Payload instruction line (e.g., "PIE.@CASTSPELL|0x01|MyMod.esp|...")
 *
 * Structure:
 *   <event> "." <instruction>
 */
export interface PayloadInstructionNode {
  kind: 'payload_instruction';

  /** Optional space before time */
  space0First?: SpaceNode;
  /** Time field (number) */
  time?: FieldNode<number>;
  /** Mandatory space between time and event */
  space1TimeToEvent?: SpaceNode;

  /** Event host, typically "PIE" */
  event?: FieldNode<string>;

  /** Dot between event and instruction */
  dot?: FieldNode<'.'>;

  /** Actual instruction (starts with '@' | '$' | '!') */
  instruction?: InstructionNode;
}

/**
 * Represents an instruction (e.g., "@CASTSPELL|0x01|MyMod.esp|...")
 *
 * Structure:
 *   "@" <name> [<parameters>]
 */
export interface InstructionNode {
  kind: 'instruction';

  /**
   * - `@`: native instruction
   * - `$`: custom instruction
   * - `!`: async instruction
   */
  prefix?: FieldNode<'@' | '$' | '!'>;

  /** Instruction name (e.g., CASTSPELL, SGVF, etc.) */
  name?: FieldNode<string>;

  /** Optional parameter list */
  parameters?: ParametersNode;
}

/**
 * Represents a list of parameters separated by '|'
 *
 * Structure:
 *   "|" <param1> "|" <param2> "|" ...
 */
export interface ParametersNode {
  kind: 'parameters';

  /** Each parameter and its separators */
  items: ParameterItemNode[];

  /** Overall position */
  pos?: Pos;
}

/**
 * Represents a single parameter (and the separator before it)
 */
export interface ParameterItemNode {
  /** The preceding '|' (or undefined if it's the first parameter) */
  separator?: FieldNode<'|'>;

  /** Parameter value (usually string or number) */
  value?: FieldNode<string>;
}
