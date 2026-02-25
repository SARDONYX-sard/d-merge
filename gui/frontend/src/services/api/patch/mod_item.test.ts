import { describe, expect, it } from 'vitest';
import type { FetchedModInfo, ModItem } from '.';
import { mergeModInfoList, toPatches } from './mod_item';

const DEFAULT = {
  id: '',
  name: '',
  author: '',
  site: '',
  mod_type: 'nemesis',
  auto: '',

  enabled: false,
  priority: 0,
} as const satisfies ModItem;

describe('mergeModInfoList', () => {
  it('normalizes gaps and non-zero start', () => {
    const prev = [
      { ...DEFAULT, id: 'b', priority: 10, enabled: true },
      { ...DEFAULT, id: 'a', priority: 3, enabled: true },
    ] as const satisfies ModItem[];

    const fetched = [
      { ...DEFAULT, id: 'a' },
      { ...DEFAULT, id: 'b' },
      { ...DEFAULT, id: 'c' },
    ] as const satisfies FetchedModInfo[];

    const result = mergeModInfoList(prev, fetched);

    expect(result.map((m) => m.priority)).toEqual([0, 1, 2]);
    expect(result.map((m) => m.id)).toEqual(['a', 'b', 'c']);
  });

  it('preserves relative order of existing mods', () => {
    const prev: ModItem[] = [
      { ...DEFAULT, id: 'low', priority: 1, enabled: true },
      { ...DEFAULT, id: 'high', priority: 100, enabled: true },
    ];

    const fetched: FetchedModInfo[] = [
      { ...DEFAULT, id: 'high' },
      { ...DEFAULT, id: 'low' },
    ];

    const result = mergeModInfoList(prev, fetched);

    expect(result.map((m) => m.id)).toEqual(['low', 'high']);
  });
});

describe('toPatches', () => {
  it('filters disabled mods', () => {
    const mods: ModItem[] = [
      { ...DEFAULT, id: 'a', enabled: true, priority: 1 },
      { ...DEFAULT, id: 'b', enabled: false, priority: 2 },
    ];

    const result = toPatches('/data', false, mods);

    expect(result.nemesis_entries).toEqual({
      a: 1,
    });

    expect(result.fnis_entries).toEqual({});
  });

  it('creates correct Nemesis path in VFS mode', () => {
    const mods: ModItem[] = [
      {
        ...DEFAULT,
        id: 'aaa',
        enabled: true,
        priority: 5,
        mod_type: 'nemesis',
      },
    ];

    const result = toPatches('/skyrim/Data', true, mods);

    expect(result.nemesis_entries).toEqual({
      '/skyrim/Data/Nemesis_Engine/mod/aaa': 5,
    });

    expect(result.fnis_entries).toEqual({});
  });

  it('uses raw id for Nemesis in manual mode', () => {
    const mods: ModItem[] = [
      {
        ...DEFAULT,
        id: '/Nemesis_Engine/mod/aaa',
        enabled: true,
        priority: 3,
        mod_type: 'nemesis',
      },
    ];

    const result = toPatches('/ignored', false, mods);

    expect(result.nemesis_entries).toEqual({
      '/Nemesis_Engine/mod/aaa': 3,
    });
  });

  it('handles FNIS correctly', () => {
    const mods: ModItem[] = [
      {
        ...DEFAULT,
        id: '/meshes/actors/character/animations/aaa',
        enabled: true,
        priority: 9,
        mod_type: 'fnis',
      },
    ];

    const result = toPatches('/data', true, mods);

    expect(result.fnis_entries).toEqual({
      '/meshes/actors/character/animations/aaa': 9,
    });

    expect(result.nemesis_entries).toEqual({});
  });

  it('separates nemesis and fnis entries', () => {
    const mods: ModItem[] = [
      {
        ...DEFAULT,
        id: 'nem',
        enabled: true,
        priority: 1,
        mod_type: 'nemesis',
      },
      {
        ...DEFAULT,
        id: 'fnis',
        enabled: true,
        priority: 2,
        mod_type: 'fnis',
      },
    ];

    const result = toPatches('/data', false, mods);

    expect(result.nemesis_entries).toEqual({ nem: 1 });
    expect(result.fnis_entries).toEqual({ fnis: 2 });
  });

  it('later duplicates override earlier ones (JS object behavior)', () => {
    const mods: ModItem[] = [
      { ...DEFAULT, id: 'dup', enabled: true, priority: 1, mod_type: 'fnis' },
      { ...DEFAULT, id: 'dup', enabled: true, priority: 5, mod_type: 'fnis' },
    ];

    const result = toPatches('/data', false, mods);

    expect(result.fnis_entries).toEqual({
      dup: 5,
    });
  });
});
