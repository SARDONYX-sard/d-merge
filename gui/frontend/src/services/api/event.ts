import { isTauri } from '@tauri-apps/api/core';
import { listen as tauriListen, type UnlistenFn } from '@tauri-apps/api/event';

/**
 * Cross-platform event listener for Tauri and Electron.
 *
 * This function abstracts away the differences between
 * Tauri’s and Electron’s event systems, providing a unified
 * way to subscribe to events.
 *
 * @template T - The type of the event payload.
 *
 * @param eventName - The name of the event to listen for.
 * @param eventHandler - A callback invoked with the event payload whenever the event fires.
 *
 * @returns - A function to stop listening to the event.
 *
 * @throws {Error} If the platform is not Tauri or Electron.
 *
 * @example
 * ```ts
 * // Listen for a string event
 * const unlisten = await listen<string>("status-update", (payload) => {
 *   console.log("Status update:", payload);
 * });
 *
 * // Later, stop listening
 * unlisten();
 * ```
 */
export async function listen<T>(eventName: string, eventHandler: (payload: T) => void): Promise<UnlistenFn> {
  if (isTauri()) {
    return await tauriListen<T>(eventName, (event) => {
      eventHandler(event.payload);
    });
  } else {
    throw new Error('Unsupported platform: Non Tauri');
  }
}
