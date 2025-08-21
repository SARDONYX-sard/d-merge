import type { EventName } from '@tauri-apps/api/event';
import type { ReactNode } from 'react';
import { NOTIFY } from '@/lib/notify';
import { listen } from './event';

type ListenerProps = {
  setLoading: (b: boolean) => void;
  onStatus: (s: Status, unlisten: (() => void) | null) => void;
  onSuccess?: () => void;
  onError?: (err: unknown) => void;
  error?: string | ReactNode;
};

type StatusIndexing = {
  /** 1 based index */
  index: number;
  total: number;
};
/** Error message from the backend */
type ErrorPayload = string;

/**
 * Backend status enum for merge operation (defined in Rust).
 *
 * The backend emits these status values using `window.emit(...)` during various stages.
 */
export type Status =
  | { type: 'ReadingPatches'; content: StatusIndexing }
  | { type: 'ParsingPatches'; content: StatusIndexing }
  | { type: 'ApplyingPatches'; content: StatusIndexing }
  | { type: 'GeneratingHkxFiles'; content: StatusIndexing }
  | { type: 'Done' }
  | { type: 'Error'; content: ErrorPayload };

/**
 * Tauri status listener for backend merge progress.
 *
 * This function sets up a Tauri event listener for a specific status event emitted by the backend.
 * It invokes a user-defined promise function (`promiseFn`), and handles loading state, status updates,
 * and success/error notifications.
 *
 * @param eventName - The name of the event to listen for (must match backend `window.emit`).
 * @param promiseFn - The async function that triggers the backend operation.
 * @param setLoading - Function to control loading spinner visibility.
 * @param onStatus - Function to receive status updates from the backend.
 * @param success - Message or React node to display on success.
 * @param error - Optional message or React node to display on error.
 *
 * @example
 * ```tsx
 * const handleStart = async () => {
 *   await statusListener("d_merge://progress/patch", () => invoke("start_merge"), {
 *     setLoading: setIsLoading,
 *     onStatus: (status) => {
 *       switch (status) {
 *         case 'ReadingTemplatesAndPatches':
 *           setMessage("Loading templates...");
 *           break;
 *         case 'ApplyingPatches':
 *           setMessage("Applying patches...");
 *           break;
 *         case 'GenerateHkxFiles':
 *           setMessage("Generating HKX files...");
 *           break;
 *         case 'Done':
 *           setMessage("Done!");
 *           break;
 *       }
 *     },
 *     success: "Merge complete!",
 *     error: "Merge failed.",
 *   });
 * };
 * ```
 */
export async function statusListener(
  eventName: EventName,
  promiseFn: () => Promise<void>,
  { setLoading, onStatus, onSuccess, onError, error }: ListenerProps,
) {
  setLoading(true);

  let unlisten: (() => void) | null = null;

  try {
    unlisten = await listen<Status>(eventName, (payload) => {
      onStatus(payload, unlisten);
    });

    await promiseFn();
    onSuccess?.();
  } catch (err) {
    if (onError) {
      onError(err);
    } else {
      NOTIFY.error(error ?? `${err}`);
    }
  }
}
