import { describe, expect, it } from 'vitest';

import { hashDjb2 } from './index';

describe('hashDJB2', () => {
  it('should return consistent hash for the same input', () => {
    const input = 'example';
    const hash1 = hashDjb2(input);
    const hash2 = hashDjb2(input);
    expect(hash1).toBe(hash2);
  });

  it('should return different hashes for different inputs', () => {
    const hash1 = hashDjb2('example1');
    const hash2 = hashDjb2('example2');
    expect(hash1).not.toBe(hash2);
  });

  it('should return a 32-bit unsigned integer', () => {
    const hash = hashDjb2('test');
    const u32Max = 0xffffffff;
    expect(hash).toBeGreaterThanOrEqual(0);
    expect(hash).toBeLessThanOrEqual(u32Max);
  });

  it('should handle empty strings correctly', () => {
    const hash = hashDjb2('');
    expect(hash).toBe(5381);
  });
});
