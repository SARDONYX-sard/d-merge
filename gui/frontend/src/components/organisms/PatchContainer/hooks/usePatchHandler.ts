import type { MouseEventHandler } from 'react';
import { usePatchContext } from '@/components/providers/PatchProvider';
import { type ModInfo, patch } from '@/services/api/patch';
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
        await patch(output, getCheckedPath(isVfsMode, vfsSkyrimDataDir, modInfoList), patchOptions);
      },
      { setLoading, onStatus, onError },
    );
  };

  return { handleClick };
}

function getCheckedPath(isVfsMode: boolean, vfsSkyrimDataDir: string, modInfoList: ModInfo[]): string[] {
  let res: string[] = [];

  if (isVfsMode) {
    for (const mod of modInfoList) {
      res.push(`${vfsSkyrimDataDir}/Nemesis_Engine/mod/${mod.id}`);
    }
    return res;
  }

  for (const mod of modInfoList) {
    res.push(mod.id);
  }
  return res;
}
