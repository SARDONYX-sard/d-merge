import { ZodArray, ZodDefault, ZodEffects, ZodNullable, ZodObject, ZodOptional, type ZodTypeAny, type z } from 'zod';

export type ParseError = {
  path: (string | number)[];
  expected: string;
  received: unknown;
  message: string;
};

type InferPartial<T extends ZodTypeAny> = T extends ZodObject<infer Shape>
  ? { [K in keyof Shape]?: InferPartial<Shape[K]> }
  : T extends ZodArray<infer Item>
    ? InferPartial<Item>[]
    : T extends ZodDefault<infer Inner>
      ? InferPartial<Inner>
      : T extends ZodEffects<infer Inner>
        ? InferPartial<Inner>
        : T extends ZodOptional<infer Inner>
          ? InferPartial<Inner> | undefined
          : T extends ZodNullable<infer Inner>
            ? InferPartial<Inner> | null
            : z.infer<T>;

export type DeepParseResult<T> = {
  data?: Partial<T>;
  errors: ParseError[];
};

export function deepSafeParse<T extends ZodTypeAny>(schema: T, input: unknown): DeepParseResult<InferPartial<T>> {
  const errors: ParseError[] = [];

  function unwrap(s: ZodTypeAny): ZodTypeAny {
    while (s instanceof ZodDefault || s instanceof ZodOptional || s instanceof ZodNullable || s instanceof ZodEffects) {
      if (s instanceof ZodEffects) {
        s = s._def.schema;
      } else {
        s = s._def.innerType;
      }
    }
    return s;
  }

  function walk<S extends ZodTypeAny>(s: S, val: unknown, path: (string | number)[]): InferPartial<S> | undefined {
    const base = unwrap(s);

    if (base instanceof ZodObject && typeof val === 'object' && val !== null) {
      const shape = base.shape;

      const result = {} as InferPartial<S>;
      for (const key in shape) {
        const v = (val as Record<string, unknown>)[key];
        const sub = walk(shape[key], v, [...path, key]);
        if (sub !== undefined) {
          result[key as keyof InferPartial<S>] = sub;
        }
      }
      return result;
    }

    if (base instanceof ZodArray && Array.isArray(val)) {
      return val.map((item, i) => walk(base.element, item, [...path, i])) as InferPartial<S>;
    }

    const result = s.safeParse(val);
    if (result.success) {
      return result.data as InferPartial<S>;
    }

    for (const issue of result.error.issues) {
      errors.push({
        path: [...path, ...issue.path],
        expected: issue.code,
        received: val,
        message: issue.message,
      });
    }
    return undefined;
  }

  const data = walk(schema, input, []);
  return { data, errors };
}

export function formatParseErrors(errors: ParseError[]): string {
  return errors
    .map((err) => {
      const path = err.path.length > 0 ? err.path.join('.') : '(root)';
      return `- ❌ ${path}: ${err.message} (expected: ${err.expected}, received: ${JSON.stringify(err.received)})`;
    })
    .join('\n');
}
