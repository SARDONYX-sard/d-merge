import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { getActualURL } from './url_resolver';

describe('getActualURL', () => {
  it('should map /frontend/ to index.html', () => {
    const input = 'file:///C:/frontend/';
    const expected = join(__dirname, '..', 'frontend', 'index.html');
    expect(getActualURL(input)).toBe(expected);
  });

  it('should map /frontend to index.html', () => {
    const input = 'file:///D:/frontend';
    const expected = join(__dirname, '..', 'frontend', 'index.html');
    expect(getActualURL(input)).toBe(expected);
  });

  it('should map /frontend/page.html to local path', () => {
    const input = 'file:///D:/frontend/page.html';
    const expected = join(__dirname, '..', 'frontend', '/page.html');
    expect(getActualURL(input)).toBe(expected);
  });

  it('should ignore urls without /frontend', () => {
    const input = 'file:///D:/other/path/index.html';
    expect(getActualURL(input)).toBe(input);
  });
});
