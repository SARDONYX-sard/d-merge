import { ipcMain, shell } from 'electron';

ipcMain.handle('opener:openUrl', async (_, path: string) => {
  await shell.openExternal(path);
});

ipcMain.handle('opener:openPath', async (_, path: string) => {
  await shell.openPath(path);
});
