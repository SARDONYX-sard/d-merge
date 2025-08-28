import { describe, expect, it } from 'vitest';
import { buildActivateIds, type ModItem } from './backup';

describe('buildActivateIds', () => {
  it('keeps enabled mods in vfs_mod_list order', () => {
    const vfs_mod_list = [
      { enabled: true, id: 'amco', name: 'Attack MCO' },
      { enabled: false, id: 'slide', name: 'Crouch Sliding' },
      { enabled: true, id: 'dmco', name: 'Dodge MCO' },
    ] as const satisfies ModItem[];

    const patchPriorityIds = [
      'D:\\\\GAME\\\\mods\\\\Attack MCO\\\\Nemesis_Engine\\\\mod\\\\amco',
      'D:\\\\GAME\\\\mods\\\\Crouch Sliding\\\\Nemesis_Engine\\\\mod\\\\slide',
      'D:\\\\GAME\\\\mods\\\\DMCO\\\\Nemesis_Engine\\\\mod\\\\dmco',
    ];

    const result = buildActivateIds(vfs_mod_list, patchPriorityIds);

    expect(result).toEqual([
      'D:\\\\GAME\\\\mods\\\\Attack MCO\\\\Nemesis_Engine\\\\mod\\\\amco',
      'D:\\\\GAME\\\\mods\\\\DMCO\\\\Nemesis_Engine\\\\mod\\\\dmco',
    ]);
  });

  it('returns empty array if no mods are enabled', () => {
    const vfs_mod_list = [
      { enabled: false, id: 'amco', name: 'Attack MCO' },
      { enabled: false, id: 'slide', name: 'Crouch Sliding' },
    ] as const satisfies ModItem[];

    const patchPriorityIds = [
      'D:\\\\GAME\\\\mods\\\\Attack MCO\\\\Nemesis_Engine\\\\mod\\\\amco',
      'D:\\\\GAME\\\\mods\\\\Crouch Sliding\\\\Nemesis_Engine\\\\mod\\\\slide',
    ];

    const result = buildActivateIds(vfs_mod_list, patchPriorityIds);

    expect(result).toEqual([]);
  });

  it('includes all matching paths when ids are duplicated', () => {
    const vfs_mod_list = [{ enabled: true, id: 'slmco', name: 'ModernStaggerLock' }] as const satisfies ModItem[];

    const patchPriorityIds = [
      'D:\\\\GAME\\\\mods\\\\ModernStaggerLock -AE\\\\Nemesis_Engine\\\\mod\\\\slmco',
      'D:\\\\GAME\\\\mods\\\\ModernStaggerLock -SE\\\\Nemesis_Engine\\\\mod\\\\slmco',
    ];

    const result = buildActivateIds(vfs_mod_list, patchPriorityIds);

    expect(result).toEqual([
      'D:\\\\GAME\\\\mods\\\\ModernStaggerLock -AE\\\\Nemesis_Engine\\\\mod\\\\slmco',
      'D:\\\\GAME\\\\mods\\\\ModernStaggerLock -SE\\\\Nemesis_Engine\\\\mod\\\\slmco',
    ]);
  });
});
