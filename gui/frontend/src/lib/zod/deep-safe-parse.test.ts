import { describe, expect, it } from 'vitest';
import { z } from 'zod';

import { deepSafeParse } from './deep-safe-parse';

const schema = z.object({
  user: z.object({
    name: z.string(),
    age: z.number(),
  }),
  isActive: z.boolean(),
});

describe('deepSafeParse', () => {
  it('returns partial valid data and collects errors', () => {
    const input = {
      user: {
        name: 'Taro',
        age: 'NaN',
      },
      isActive: true,
    };

    const result = deepSafeParse(schema, input);

    expect(result.data).toEqual({
      user: {
        name: 'Taro',
      },
      isActive: true,
    });

    expect(result.errors.length).toBe(1);
    expect(result.errors[0].path).toEqual(['user', 'age']);
    expect(result.errors[0].message).toBe('Expected number, received string');
  });
});
