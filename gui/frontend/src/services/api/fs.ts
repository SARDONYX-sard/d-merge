import { invoke } from '@tauri-apps/api/core';
import { readTextFile } from '@tauri-apps/plugin-fs';

import { type CacheKey, STORAGE } from '@/lib/storage';

import { openPath } from './dialog';

/**
 * Read the entire contents of a file into a string.
 * @param pathKey - target path cache key
 * @return contents
 * @throws `Error`
 */
export async function readFile(pathKey: CacheKey, filterName: string, extensions = ['json']) {
  let path = STORAGE.get(pathKey) ?? '';

  const setPath = (newPath: string) => {
    path = newPath;
    STORAGE.set(pathKey, path);
  };

  if (await openPath(path, { setPath, filters: [{ name: filterName, extensions }] })) {
    return await readTextFile(path);
  }
  return null;
}

/**
 * Alternative file writing API to avoid tauri API bug.
 *
 * # NOTE
 * We couldn't use `writeTextFile`!
 * - The `writeTextFile` of tauri's api has a bug that the data order of some contents is unintentionally swapped.
 * @param path - path to write
 * @param content - string content
 * @throws Error
 */
export async function writeFile(path: string, content: string) {
  await invoke('write_file', { path, content });
}
