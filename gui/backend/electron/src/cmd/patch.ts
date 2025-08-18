import { ipcMain } from 'electron';
import { ModInfo, PatchArguments } from './types/patch';

// Skyrim path
ipcMain.handle('skyrim:getDir', async (_, runtime: 'SkyrimSE' | 'SkyrimLE') => {
  // TODO: Rust FFI
  // @ts-ignore
  return await getSkyrimDir(runtime); // -> string(skyrim Data dir path)
});

// Mods info
ipcMain.handle('patch:loadModsInfo', async (_, glob: string): Promise<ModInfo[]> => {
  // TODO: Rust FFI
  // @ts-ignore
  return await loadModsInfo(glob); // -> Promise<ModInfo[]>(mods info array)
});

// Patch operation
ipcMain.handle('patch:patch', async (_, { output, ids, options }: PatchArguments) => {
  // TODO: Rust FFI
  // @ts-ignore
  return await patch(output, ids, options); // -> void
});

ipcMain.handle('patch:cancel', async () => {
  // TODO: Cancel patch operation
});

ipcMain.handle('patch:setVfsMode', async (_, value) => {
  // TODO: Enable/disable VFS mode
});
