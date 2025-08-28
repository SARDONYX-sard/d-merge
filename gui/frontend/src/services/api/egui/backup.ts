import { getPathId } from '@/lib/path';

/** egui mod information */
export type ModItem = {
  enabled: boolean;
  id: string;
  name: string;
  site?: string;
  priority?: number;
};

/**
 * Build patch-activate-ids from vfs_mod_list and patch-priority-ids
 * Keeps the order of vfs_mod_list
 * If the same id appears multiple times in patchPriorityIds,
 * all matching paths will be included.
 */
export function buildActivateIds(vfsModList: ModItem[], patchPriorityIds: string[]): string[] {
  const result: string[] = [];

  for (const mod of vfsModList) {
    if (!mod.enabled) continue;

    // find all paths in priorityIds that end with this mod's id
    const matches = patchPriorityIds.filter((path) => getPathId(path) === mod.id);

    result.push(...matches);
  }

  return result;
}
