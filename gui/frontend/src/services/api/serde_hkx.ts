import { invoke } from '@tauri-apps/api/core';

type OutFormat = 'amd64' | 'win32' | 'xml';

/**
 * Convert xml/hkx => hkx/xml.
 * @throws Error
 */
export async function convert(input: string, output: string, format: OutFormat) {
  await invoke('convert', { input, output, format });
}
