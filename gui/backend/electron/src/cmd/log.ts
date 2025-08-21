import { changeLogLevel } from 'd_merge_node';
import { app, ipcMain } from 'electron';

export const logDir = app.getPath('logs');
export const logFileName = `d_merge.log`;

ipcMain.handle('log:changeLevel', async (_, level: string = 'error') => {
  let isValid = false;
  if (['error', 'warn', 'info', 'debug', 'trace'].includes(level)) {
    isValid = true;
  }
  changeLogLevel(level);
});

ipcMain.handle('app:getLogDir', () => logDir);
ipcMain.handle('app:getName', () => 'd_merge');
