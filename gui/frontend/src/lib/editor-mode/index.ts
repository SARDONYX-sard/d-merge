import { z } from 'zod';

import { NOTIFY } from '@/lib/notify';
import { STORAGE } from '@/lib/storage';

const DEFAULT: EditorMode = 'default';
const CACHE_KEY = 'editor-mode';

const literalSchema = z.union([z.string(), z.number(), z.boolean(), z.null()]);
type Literal = z.infer<typeof literalSchema>;
type Json = Literal | { [key: string]: Json } | Json[];
const jsonSchema: z.ZodType<Json> = z.lazy(() => z.union([literalSchema, z.array(jsonSchema), z.record(jsonSchema)]));

/**
 * A utility function that returns a Zod schema which:
 * - Parses a JSON string.
 * - Validates the parsed object using the provided schema `T`.
 *
 * @param schema - The Zod schema to validate the parsed object.
 * @returns A Zod schema that parses JSON and validates the result.
 *
 * @see [Parsing a JSON string with zod](https://github.com/colinhacks/zod/discussions/2215#discussion-4977685)
 */
const stringToJsonSchema = z.string().transform((str, ctx): z.infer<typeof jsonSchema> => {
  try {
    return JSON.parse(str);
  } catch (e) {
    ctx.addIssue({ code: 'custom', message: `${e}` });
    return z.NEVER;
  }
});

/** Zod schema for validating the editor mode value. */
const EditorModeSchema = z.enum(['default', 'vim']);
/** Automatically inferred `EditorMode` type from the schema. */
export type EditorMode = z.infer<typeof EditorModeSchema>;

/**
 * Normalizes the editor mode value.
 * If the provided mode is invalid, it falls back to the default mode.
 *
 * @param mode - The editor mode value (can be string or null/undefined).
 * @returns A valid `EditorMode`. Defaults to `'default'` if invalid.
 */
const normalize = (mode?: string | null): EditorMode => {
  if (mode === null || mode === undefined) {
    return DEFAULT;
  }

  const result = EditorModeSchema.safeParse(mode);
  if (result.success) {
    return result.data;
  }

  const errMsg = result.error.errors.map((error) => error.message).join(', ');
  NOTIFY.error(`Invalid editor mode: ${errMsg}`);
  return DEFAULT;
};

export const EDITOR_MODE = {
  /** The default editor mode. */
  default: DEFAULT,

  /**
   * Fallback to `'default'` if the value is `null` or `undefined`.
   *
   * @param mode - The editor mode value to normalize.
   * @returns A valid `EditorMode`.
   */
  normalize,

  /**
   * Retrieves the current editor mode from `localStorage`.
   * If the value is invalid, it returns the default mode.
   *
   * @returns The normalized editor mode.
   */
  get(): EditorMode {
    const mode = STORAGE.get(CACHE_KEY);
    if (mode) {
      const result = stringToJsonSchema.pipe(EditorModeSchema).safeParse(mode);
      if (result.success) {
        return result.data;
      }
    }
    return DEFAULT;
  },

  /**
   * Sets the editor mode in `localStorage`.
   * The value is validated before saving.
   *
   * @param level - The editor mode to set in `localStorage`.
   */
  set(level: EditorMode) {
    // Stringify and save the validated value to localStorage.
    STORAGE.set(CACHE_KEY, JSON.stringify(level));
  },
};
