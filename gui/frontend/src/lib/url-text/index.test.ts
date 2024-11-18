import { describe, it, expect } from 'vitest';

import { extractUrlFromLine } from './';

describe('extractUrlFromLine function', () => {
  it('should extract a full URL when the cursor is in the middle', () => {
    const line = 'Visit https://www.example.com for more info';
    const result = extractUrlFromLine(line, 15);
    expect(result).toBe('https://www.example.com');
  });

  it('should return undefined when the cursor is on a non-URL character', () => {
    const line = 'Check this out: hello world';
    const result = extractUrlFromLine(line, 5);
    expect(result).toBeUndefined();
  });

  it('should extract a URL at the start of the line', () => {
    const line = 'http://start.com/path?query=123';
    const result = extractUrlFromLine(line, 5);
    expect(result).toBe('http://start.com/path?query=123');
  });

  it('should extract a URL at the end of the line', () => {
    const line = 'Go to this link: https://end.com/path';
    const result = extractUrlFromLine(line, 30);
    expect(result).toBe('https://end.com/path');
  });

  it('should handle URLs with special characters correctly', () => {
    const line = 'Secure link: https://example.com/resource?param=value&flag=true';
    const result = extractUrlFromLine(line, 25);
    expect(result).toBe('https://example.com/resource?param=value&flag=true');
  });
});
