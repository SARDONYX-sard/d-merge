import type { FetchedModInfo, ModItem, PatchMaps } from '.';

/**
 * Merge fetched list into previous ModInfo[] by `id`.
 * - Preserve `enabled` and `priority` from prev when id matches.
 * - Keep latest name/author/site/auto/modType from fetched.
 * - If priority missing in prev, assign index+1 (fetched order).
 */
export const mergeModInfoList = (prev: ModItem[], fetched: FetchedModInfo[]): ModItem[] => {
  const prevMap = new Map<string, ModItem>();
  for (const p of prev) {
    prevMap.set(p.id, p);
  }

  const withOld: ModItem[] = [];
  const withoutOld: ModItem[] = [];

  for (const f of fetched) {
    const existing = prevMap.get(f.id);

    const base: ModItem = {
      id: f.id,
      name: f.name,
      author: f.author,
      site: f.site,
      auto: f.auto,
      mod_type: f.mod_type,
      enabled: existing?.enabled ?? false,
      priority: existing?.priority ?? 0, // temporary
    };

    if (existing) {
      withOld.push(base);
    } else {
      withoutOld.push(base);
    }
  }

  withOld.sort((a, b) => a.priority - b.priority);

  // New ID alphabetical order
  withoutOld.sort((a, b) => a.id.localeCompare(b.id));

  const merged = [...withOld, ...withoutOld];

  // Normalize(0..n-1)
  merged.forEach((item, index) => {
    item.priority = index;
  });

  return merged;
};

export const toPatches = (vfsSkyrimDataDir: string, isVfsMode: boolean, modInfos: ModItem[]): PatchMaps => {
  const nemesisEntries: Record<string, number> = {};
  const fnisEntries: Record<string, number> = {};

  for (const mod of modInfos) {
    if (!mod.enabled) continue;

    switch (mod.mod_type) {
      case 'nemesis': {
        const path = isVfsMode ? `${vfsSkyrimDataDir}/Nemesis_Engine/mod/${mod.id}` : mod.id;
        nemesisEntries[path] = mod.priority;
        break;
      }
      case 'nemesis_ext': {
        const path = isVfsMode ? `${vfsSkyrimDataDir}/Nemesis_EngineExt/mod/${mod.id}` : mod.id;
        nemesisEntries[path] = mod.priority;
        break;
      }
      case 'fnis': {
        // Note that duplicates may cause malfunctions due to FNIS specifications.
        const path = mod.id;
        fnisEntries[path] = mod.priority;
        break;
      }
    }
  }

  return { nemesis_entries: nemesisEntries, fnis_entries: fnisEntries };
};
