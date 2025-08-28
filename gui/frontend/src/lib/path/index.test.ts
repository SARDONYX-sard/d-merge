import { describe, expect, it } from 'vitest';

import { getPathId, stripGlob } from './index';

describe('stripGlob', () => {
  it('returns the same path when no glob is present', () => {
    const input = 'D:\\Games\\Skyrim\\mods';
    expect(stripGlob(input)).toBe('D:\\Games\\Skyrim\\mods');
  });

  it('removes trailing * from path', () => {
    const input = 'D:\\Games\\Skyrim\\mods\\*';
    expect(stripGlob(input)).toBe('D:\\Games\\Skyrim\\mods');
  });

  it('removes nested glob patterns', () => {
    const input = 'D:\\Games\\Skyrim\\mods\\**\\*.esp';
    expect(stripGlob(input)).toBe('D:\\Games\\Skyrim\\mods');
  });

  it('removes trailing slashes after glob strip', () => {
    const input = 'C:\\mods\\folder\\*/';
    expect(stripGlob(input)).toBe('C:\\mods\\folder');
  });
});

describe('getPathId', () => {
  it('should get last segment from POSIX path', () => {
    expect(getPathId('/foo/bar/baz.txt')).toBe('baz.txt');
  });

  it('should get last segment from Windows path', () => {
    expect(getPathId('C:\\foo\\bar\\baz.txt')).toBe('baz.txt');
  });

  it('should return null for trailing slash', () => {
    expect(getPathId('/foo/bar/')).toBeNull();
    expect(getPathId('C:\\foo\\bar\\')).toBeNull();
  });

  it('should return null for empty input', () => {
    expect(getPathId('')).toBeNull();
  });

  it('should return null for root paths', () => {
    expect(getPathId('/')).toBeNull();
    expect(getPathId('C:\\')).toBeNull();
    expect(getPathId('C:/')).toBeNull();
  });
});
