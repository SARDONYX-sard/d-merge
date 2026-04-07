import { type MouseEventHandler, useCallback } from 'react';
import { usePatchContext } from '@/components/providers/PatchProvider';
import { patch } from '@/services/api/patch';
import { toPatches } from '@/services/api/patch/mod_item';
import { type Status, statusListener } from '@/services/api/patch/patch_listener';

type Props = {
  start: () => void;
  setLoading: (b: boolean) => void;
  onStatus: (s: Status, unlisten: (() => void) | null) => void;
  onError?: (err: unknown) => void;
};

/**
 * Handles the patch process lifecycle including backend event listening,
 * status updates, loading state, timer, and notifications.
 */
export function usePatchHandler({ start, setLoading, onStatus, onError }: Props) {
  const { isVfsMode, vfsSkyrimDataDir, skyrimDataDir, output, patchOptions, modList, vfsModList } = usePatchContext();

  const handleClick: MouseEventHandler<HTMLButtonElement> = useCallback(async () => {
    start();

    await statusListener(
      'd_merge://progress/patch', // event name emitted from Tauri backend
      async () => {
        const patchMaps = toPatches(vfsSkyrimDataDir, isVfsMode, isVfsMode ? vfsModList : modList);
        const config = {
          ...patchOptions,
          skyrimDataDirGlob: isVfsMode ? vfsSkyrimDataDir : skyrimDataDir,
        };
        await patch(output, patchMaps, config);
      },
      { setLoading, onStatus, onError },
    );
  }, [
    output,
    patchOptions,
    vfsSkyrimDataDir,
    skyrimDataDir,
    isVfsMode,
    modList,
    vfsModList,
    start,
    setLoading,
    onStatus,
    onError,
  ]);

  return { handleClick };
}
