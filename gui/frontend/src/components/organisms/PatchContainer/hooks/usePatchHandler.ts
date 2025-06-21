import type { MouseEventHandler } from 'react';
import { usePatchContext } from '@/components/providers/PatchProvider';
import { patch } from '@/services/api/patch';
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
  const { output, activateMods, patchOptions } = usePatchContext();

  const handleClick: MouseEventHandler<HTMLButtonElement> = async () => {
    start();

    await statusListener(
      'd_merge://progress/patch', // event name emitted from Tauri backend
      async () => {
        await patch(output, activateMods, patchOptions);
      },
      {
        setLoading,
        onStatus,
        onError,
      },
    );
  };

  return { handleClick };
}
