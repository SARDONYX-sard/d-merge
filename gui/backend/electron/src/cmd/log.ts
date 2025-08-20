import { changeLogLevel } from 'd_merge_node';
import { app, ipcMain } from 'electron';

ipcMain.handle('log:changeLevel', async (_, level: string = 'error') => {
  let isValid = false;
  if (['error', 'warn', 'info', 'debug', 'trace'].includes(level)) {
    isValid = true;
  }
  changeLogLevel(level);
});

ipcMain.handle('app:getLogDir', () => app.getPath('logs'));
ipcMain.handle('app:getName', () => app.getName());
