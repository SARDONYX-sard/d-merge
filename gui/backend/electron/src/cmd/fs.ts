import { readFile, writeFile } from 'node:fs/promises';
import { ipcMain } from 'electron';

ipcMain.handle('fs:readFile', async (_, path: string) => {
  return await readFile(path, 'utf-8');
});

ipcMain.handle('fs:writeFile', async (_, { path, content }: { path: string; content: string }) => {
  await writeFile(path, content, 'utf-8');
});
