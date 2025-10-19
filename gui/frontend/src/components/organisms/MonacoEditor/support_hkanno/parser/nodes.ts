/**
 * - `<space0> # <comment until \n>`
 * - `<space0> <time> <space1> <text until \n but trim>`
 *
 * special pattern
 * - `<space0> <time> <space1> animmotion <space1> <x: f32> <space1> <y: f32> <space1> <z: f32> <space0>`
 * - `<space0> <time> <space1> animrotation <space1> <degrees: f32> <space0>`
 */
export type HkAnnoNode = RotationNode | MotionNode | TextNode | CommentNode;

/**
 * Represents a position in the text (1-based line and column).
 */
export type Pos = {
  line: number;
  /** Indicates the nth character starting from index 1. */
  startColumn: number;
  /** Although it is 1-based, this nth character is not included. */
  endColumn: number;
};

/**
 * Represents a space token in the line.
 * Can be mandatory (space1) or optional (space0).
 *
 * e.g., the space between `time` and `event` or between `x` and `y`.
 */
export interface SpaceNode {
  kind: 'space';
  /** Raw text of the space (e.g., " ", "    ") */
  rawText: string;
  /** Position of the space in the line */
  pos: Pos;
}

/**
 * Generic field node that holds a value and its position.
 * Optional because in LSP parsing the value might not exist yet.
 *
 * e.g., a `time` field, `x` coordinate, or `text` string.
 */
export interface FieldNode<T extends string | number> {
  /** The actual value of the field */
  value?: T;
  /** Position of the value in the line */
  pos?: Pos;
}

/**
 * Comment line node.
 *
 * Pattern: <space0> # <comment until \n>
 *
 * e.g., "  # This is a comment"
 */
export type CommentNode = {
  kind: 'comment';
  /** Optional space at the beginning of the line */
  space0First?: SpaceNode;
  /** The comment symbol */
  hash?: FieldNode<'#'>;

  /** Optional space between # and comment */
  space0HashToComment?: SpaceNode;

  /** The comment text (after #) */
  comment?: FieldNode<string>;

  /** Optional space after comment */
  space0AfterComment?: SpaceNode;
};

/**
 * Text line node.
 *
 * Pattern: <space0> <time> <space1> <text until \n but trim>
 *
 * e.g., " 0.5 Hello world"
 */
export type TextNode = {
  kind: 'text';
  /** Optional space before time */
  space0First?: SpaceNode;
  /** Time field (number) */
  time?: FieldNode<number>;
  /** Mandatory space between time and text */
  space1TimeToText?: SpaceNode;
  /** Text field */
  text?: FieldNode<string>;
  /** Optional space after text */
  space0AfterText?: SpaceNode;
};

/**
 * Motion line node.
 *
 * Pattern: <space0> <time> <space1> animmotion <space1> <x> <space1> <y> <space1> <z> <space0>
 *
 * e.g., " 0.5 animmotion 1 2 3 "
 */
export type MotionNode = {
  kind: 'motion';

  /** Optional space before time */
  space0First?: SpaceNode;
  /** Time field */
  time?: FieldNode<number>;

  /** Mandatory space between time and event */
  space1TimeToEvent?: SpaceNode;
  /** Event name (fixed 'animmotion') */
  event?: FieldNode<'animmotion'>;

  /** Mandatory space between event and x */
  space1EventToX?: SpaceNode;

  /** X coordinate */
  x?: FieldNode<number>;
  /** Mandatory space between x and y */
  space1XToY?: SpaceNode;
  /** Y coordinate */
  y?: FieldNode<number>;
  /** Mandatory space between y and z */
  space1YToZ?: SpaceNode;
  /** Z coordinate */
  z?: FieldNode<number>;
  /** Optional space after z */
  space0AfterZ?: SpaceNode;
};

/**
 * Rotation line node.
 *
 * Pattern: <space0> <time> <space1> animrotation <space1> <degrees> <space0>
 *
 * e.g., "0.5 animrotation 90 "
 */
export type RotationNode = {
  kind: 'rotation';

  /** Optional space before time */
  space0First?: SpaceNode;
  /** Time field */
  time?: FieldNode<number>;
  /** Mandatory space between time and event */
  space1TimeToEvent?: SpaceNode;
  /** Event name (fixed 'animrotation') */
  event?: FieldNode<'animrotation'>;
  /** Mandatory space between event and degrees */
  space1EventToDegrees?: SpaceNode;
  /** Degrees value */
  degrees?: FieldNode<number>;
  /** Optional space after degrees */
  space0AfterDegrees?: SpaceNode;
};
