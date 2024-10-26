import { invoke } from '@tauri-apps/api/core';

export type OutFormat = 'amd64' | 'win32' | 'xml';

/**
 * Convert xml/hkx => hkx/xml.
 * - `inputs`: Files or dirs.
 * - `output`: Output dir.
 * @throws Error
 */
export async function convert(inputs: string[], output: string, format: OutFormat) {
  await invoke('convert', { inputs, output, format });
}
