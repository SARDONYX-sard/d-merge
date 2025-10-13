import type { MouseEventHandler } from 'react';
import { usePatchContext } from '@/components/providers/PatchProvider';
import { type ModInfo, type PatchMaps, patch } from '@/services/api/patch';
import { type Status, statusListener } from '@/services/api/patch_listener';

type Params = {
  start: () => void;
  setLoading: (b: boolean) => void;
  onStatus: (s: Status, unlisten: (() => void) | null) => void;
  onError?: (err: unknown) => void;
};

/**
 * Handles the patch process lifecycle including backend event listening,
 * status updates, loading state, timer, and notifications.
 */
export function usePatchHandler({ start, setLoading, onStatus, onError }: Params) {
  const { output, isVfsMode, patchOptions, vfsSkyrimDataDir, modInfoList } = usePatchContext();

  const handleClick: MouseEventHandler<HTMLButtonElement> = async () => {
    start();

    await statusListener(
      'd_merge://progress/patch', // event name emitted from Tauri backend
      async () => {
        await patch(output, toPatches(vfsSkyrimDataDir, isVfsMode, modInfoList), patchOptions);
      },
      { setLoading, onStatus, onError },
    );
  };

  return { handleClick };
}

function toPatches(vfsSkyrimDataDir: string, isVfsMode: boolean, modInfoList: ModInfo[]): PatchMaps {
  const nemesis_entries: Record<string, number> = {};
  const fnis_entries: Record<string, number> = {};

  for (const mod of modInfoList) {
    if (!mod.enabled) continue;

    let path: string;
    if (mod.modType === 'nemesis') {
      path = isVfsMode ? `${vfsSkyrimDataDir}/Nemesis_Engine/mod/${mod.id}` : mod.id;
      nemesis_entries[path] = mod.priority;
    } else if (mod.modType === 'fnis') {
      // Note that duplicates may cause malfunctions due to FNIS specifications.
      path = mod.id;
      fnis_entries[path] = mod.priority;
    }
  }

  return { nemesisEntries: nemesis_entries, fnisEntries: fnis_entries };
}
