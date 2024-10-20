import { invoke } from '@tauri-apps/api/core';

export type ModInfo = {
  id: string;
  name: string;
  author: string;
  site: string;
  auto: string;
};

export type ModIds = readonly string[];

/**
 * Load mods `info.ini`
 * @throws Error
 */
export async function loadModsInfo() {
  return await invoke<ModInfo[]>('load_mods_info');
}

/**
 * Load activate mods id
 * @example ['aaa', 'bbb']
 * @throws Error
 */
export async function loadActivateMods() {
  return await invoke<readonly string[]>('load_activate_mods');
}

/**
 * Load activate mods id
 * @example ['aaa', 'bbb']
 * @throws Error
 */
export async function patch(ids: ModIds) {
  await invoke('patch', { ids });
}

export const createMockModsInfo = () => {
  const modsInfo: Readonly<ModInfo>[] = [];

  for (let i = 1; i < 21; i++) {
    modsInfo.push({
      id: `mod-${i}`,
      name: `Mod ${String.fromCharCode(65 + i)}`,
      author: 'Author A',
      site: `https://www.nexusmods.com/skyrimspecialedition/mods/${i}`,
      auto: 'Yes',
    } as const);
  }

  return modsInfo;
};

export const createMockSelectId = (rows: ModInfo[]) =>
  rows.filter((r) => ['mod-1', 'mod-2'].includes(r.id)).map((r) => r.id);
