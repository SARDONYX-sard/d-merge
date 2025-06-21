import { describe, expect, it } from 'vitest';

import { stripGlob } from './index';

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
