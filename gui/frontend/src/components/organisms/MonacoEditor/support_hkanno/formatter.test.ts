import { describe, expect, it } from 'vitest';
import { formatHkannoText } from './formatter';

describe('formatHkannoText', () => {
  it('removes leading spaces and preserves <time> <text>', () => {
    const text = '   0.5 Hello\n# numOriginalFrames: 10';
    const formatted = formatHkannoText(text);
    expect(formatted).toBe('0.5 Hello\n# numOriginalFrames: 10');
  });
});
